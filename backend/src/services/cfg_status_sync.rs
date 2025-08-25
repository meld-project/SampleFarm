use sqlx::Row;
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{database::Database, error::AppError};

use super::{cfg_instance_manager::CfgInstanceManager, cfg_processor::CfgProcessor};

pub struct CfgStatusSyncer {
    pub db: Arc<Database>,
    pub manager: Arc<CfgInstanceManager>,
    pub processor: Arc<CfgProcessor>,
    pub interval_secs: u64,
}

impl CfgStatusSyncer {
    pub fn new(
        db: Arc<Database>,
        manager: Arc<CfgInstanceManager>,
        processor: Arc<CfgProcessor>,
        interval_secs: u64,
    ) -> Self {
        Self {
            db,
            manager,
            processor,
            interval_secs,
        }
    }

    pub async fn start_sync_loop(self: Arc<Self>) {
        loop {
            if let Err(e) = self.sync_once().await {
                warn!(error=%e, "CFG 状态同步轮询失败");
            }
            tokio::time::sleep(std::time::Duration::from_secs(self.interval_secs)).await;
        }
    }

    async fn sync_once(&self) -> Result<(), AppError> {
        // 首先暂停属于已暂停主任务的运行中子任务
        let paused_count = self
            .pause_running_tasks_for_paused_masters()
            .await
            .unwrap_or(0);
        if paused_count > 0 {
            warn!("因主任务暂停而暂停了 {} 个CFG子任务", paused_count);
        }

        // 查询处于运行中的 CFG 子任务（排除暂停的主任务）
        let rows = sqlx::query(
            r#"
            SELECT st.id, st.sample_id, st.external_task_id, st.status, st.cfg_instance_id
            FROM sub_tasks st
            JOIN master_tasks mt ON st.master_task_id = mt.id
            WHERE st.analyzer_type = 'CFG' 
              AND st.status IN ('submitted','analyzing','submitting')
              AND st.external_task_id IS NOT NULL
              AND mt.status NOT IN ('paused', 'cancelled', 'failed', 'completed')
            LIMIT 100
            "#,
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询运行中任务失败: {}", e)))?;

        for row in rows {
            let sub_task_id: Uuid = row.get("id");
            let sample_id: Uuid = row.get("sample_id");
            let task_id: String = row
                .get::<Option<String>, _>("external_task_id")
                .unwrap_or_default();
            let cfg_instance_id: Option<Uuid> = row.get("cfg_instance_id");
            if task_id.is_empty() {
                continue;
            }

            // 拉取状态
            let client = if let Some(instance_id) = cfg_instance_id {
                match self.manager.get_client(instance_id).await {
                    Some(client) => client,
                    None => {
                        warn!(%sub_task_id, %task_id, %instance_id, "对应实例的CFG客户端不可用，跳过该子任务");
                        continue;
                    }
                }
            } else {
                match self.manager.client().await {
                    Some(client) => client,
                    None => {
                        warn!(%sub_task_id, %task_id, "没有可用的默认CFG客户端，跳过该子任务");
                        continue;
                    }
                }
            };

            match client.get_task_status(&task_id).await {
                Ok(status_json) => {
                    let state = status_json
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    debug!(%sub_task_id, %task_id, state, instance_id = ?cfg_instance_id, "CFG 轮询状态");

                    if state == "completed" {
                        // 获取结果并处理文件
                        match client.get_result(&task_id).await {
                            Ok(result_json) => {
                                match self
                                    .processor
                                    .process_completed(&task_id, &result_json)
                                    .await
                                {
                                    Ok(merged) => {
                                        let message = merged
                                            .get("message")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        let result_files = merged.get("result_files").cloned();
                                        let full_report = Some(merged);
                                        // 插入结果
                                        let _ = sqlx::query(
                                            r#"INSERT INTO cfg_analysis_results (id, sub_task_id, sample_id, message, result_files, full_report, created_at, updated_at)
                                                VALUES ($1,$2,$3,$4,$5,$6,NOW(),NOW())"#
                                        )
                                        .bind(Uuid::new_v4())
                                        .bind(sub_task_id)
                                        .bind(sample_id)
                                        .bind(message)
                                        .bind(result_files)
                                        .bind(full_report)
                                        .execute(self.db.pool()).await;

                                        // 更新子任务状态
                                        let _ = sqlx::query(
                                            "UPDATE sub_tasks SET status = 'completed', completed_at = NOW() WHERE id = $1"
                                        )
                                        .bind(sub_task_id)
                                        .execute(self.db.pool()).await;
                                    }
                                    Err(e) => {
                                        let _ = sqlx::query(
                                            "UPDATE sub_tasks SET status = 'failed', error_message = $2, completed_at = NOW() WHERE id = $1"
                                        )
                                        .bind(sub_task_id)
                                        .bind(e.to_string())
                                        .execute(self.db.pool()).await;
                                        warn!(%sub_task_id, err=%e, "CFG 处理结果失败");
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(%sub_task_id, %task_id, instance_id = ?cfg_instance_id, err=%e, "获取CFG结果失败");
                            }
                        }
                    } else if state == "failed" {
                        // 仅当远端返回明确错误信息时判为失败，否则保持 analyzing 以便由执行流再核验
                        let err_msg = status_json
                            .get("error")
                            .or_else(|| status_json.get("message"))
                            .and_then(|v| v.as_str());
                        if let Some(msg) = err_msg {
                            let preview = msg.chars().take(300).collect::<String>();
                            let _ = sqlx::query(
                                "UPDATE sub_tasks SET status = 'failed', error_message = $2, completed_at = NOW() WHERE id = $1"
                            )
                            .bind(sub_task_id)
                            .bind(preview)
                            .execute(self.db.pool()).await;
                        } else {
                            // 无明确失败原因，维持 analyzing 并等待后续处理
                            let _ = sqlx::query(
                                "UPDATE sub_tasks SET status = 'analyzing', started_at = COALESCE(started_at, NOW()) WHERE id = $1"
                            )
                            .bind(sub_task_id)
                            .execute(self.db.pool()).await;
                        }
                    } else {
                        // 仍在进行中，保持 analyzing
                        let _ = sqlx::query(
                            "UPDATE sub_tasks SET status = 'analyzing', started_at = COALESCE(started_at, NOW()) WHERE id = $1"
                        )
                        .bind(sub_task_id)
                        .execute(self.db.pool()).await;
                    }
                }
                Err(e) => {
                    // 对短期错误（含404）不落失败，由执行器或下次轮询再处理
                    warn!(%sub_task_id, %task_id, instance_id = ?cfg_instance_id, err=%e, "CFG 轮询状态失败（忽略并重试）");
                }
            }
        }

        Ok(())
    }

    /// 暂停属于已暂停主任务的运行中子任务
    async fn pause_running_tasks_for_paused_masters(&self) -> Result<usize, AppError> {
        let query = r#"
            UPDATE sub_tasks 
            SET status = 'paused', 
                error_message = '主任务已暂停，子任务自动暂停', 
                updated_at = NOW()
            FROM master_tasks mt
            WHERE sub_tasks.master_task_id = mt.id
              AND sub_tasks.analyzer_type = 'CFG'
              AND sub_tasks.status IN ('submitting', 'submitted', 'analyzing')
              AND mt.status IN ('paused', 'cancelled')
        "#;

        let result = sqlx::query(query)
            .execute(self.db.pool())
            .await
            .map_err(|e| AppError::service_unavailable(format!("暂停运行中CFG任务失败: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }
}
