use crate::{
    error::AppError,
    models::{SubTask, SubTaskStatus},
    repositories::TaskRepository,
    services::{CapeClient, CapeInstanceManager},
};
use std::sync::Arc;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, info, warn};

#[derive(Clone, Debug)]
pub struct CapeStatusPoller {
    task_repo: TaskRepository,
    instance_manager: Arc<CapeInstanceManager>,
    interval_secs: u64,
}

impl CapeStatusPoller {
    pub fn new(
        task_repo: TaskRepository,
        instance_manager: Arc<CapeInstanceManager>,
        interval_secs: u64,
    ) -> Self {
        Self {
            task_repo,
            instance_manager,
            interval_secs,
        }
    }

    pub async fn start(self) {
        let mut ticker = interval(std::time::Duration::from_secs(self.interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = self.tick_once().await {
                warn!(error=%e, "CAPE状态轮询器执行失败");
            }
        }
    }

    async fn tick_once(&self) -> Result<(), AppError> {
        // 按实例分片轮询，避免单实例吃满全局LIMIT导致其他实例饥饿
        let instances = self.instance_manager.get_all_instances().await;
        // 每实例抓取条数（可按需调整）
        let per_instance_limit: i64 = 1000;

        for instance in instances {
            let query = r#"
                SELECT id, master_task_id, sample_id, analyzer_type,
                       cape_instance_id, external_task_id, status, priority,
                       parameters, error_message, retry_count,
                       created_at, started_at, completed_at, updated_at
                FROM sub_tasks
                WHERE analyzer_type = 'CAPE'
                  AND cape_instance_id = $1
                  AND external_task_id IS NOT NULL
                  AND status IN ('submitting','submitted','analyzing','completed')
                ORDER BY
                  CASE WHEN status IN ('submitting','submitted','analyzing') THEN 0 ELSE 1 END ASC,
                  COALESCE(updated_at, started_at) ASC
                LIMIT $2
            "#;

            let rows = sqlx::query(query)
                .bind(instance.id)
                .bind(per_instance_limit)
                .fetch_all(self.task_repo.pool())
                .await
                .map_err(|e| AppError::service_unavailable(format!("查询待轮询任务失败: {}", e)))?;

            for row in rows {
                let task = SubTask {
                    id: row.get("id"),
                    master_task_id: row.get("master_task_id"),
                    sample_id: row.get("sample_id"),
                    analyzer_type: row.get("analyzer_type"),
                    cape_instance_id: row.get("cape_instance_id"),
                    cfg_instance_id: None,
                    external_task_id: row.get("external_task_id"),
                    status: row.get("status"),
                    priority: row.get("priority"),
                    parameters: row.get("parameters"),
                    error_message: row.get("error_message"),
                    retry_count: row.get("retry_count"),
                    created_at: row.get("created_at"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    updated_at: row.get("updated_at"),
                };

                if let Err(e) = self.process_one(task).await {
                    warn!(error=%e, "状态轮询处理单任务失败");
                }
            }
        }

        Ok(())
    }

    async fn process_one(&self, sub_task: SubTask) -> Result<(), AppError> {
        let instance_id = sub_task
            .cape_instance_id
            .ok_or_else(|| AppError::service_unavailable("子任务缺少CAPE实例ID"))?;

        let client: CapeClient = self
            .instance_manager
            .get_client(instance_id)
            .await
            .ok_or_else(|| {
                AppError::service_unavailable(format!("CAPE实例 {} 的客户端不可用", instance_id))
            })?;

        let ext_id: i32 = sub_task
            .external_task_id
            .as_ref()
            .and_then(|s| s.parse::<i32>().ok())
            .ok_or_else(|| AppError::service_unavailable("非法的 external_task_id"))?;

        // 跳过占位 external_task_id（负数）
        if ext_id <= 0 {
            debug!(sub_task_id=%sub_task.id, ext_id, "跳过占位 external_task_id 的任务");
            return Ok(());
        }

        let status = match client.get_task_status(ext_id).await {
            Ok(s) => s,
            Err(e) => {
                // 状态查询失败：写入错误并刷新 updated_at，将任务“往后挪”，避免长期占据轮询队头
                let _ = self
                    .task_repo
                    .update_sub_task_status(
                        sub_task.id,
                        &crate::models::UpdateSubTaskStatusRequest {
                            status: None,
                            external_task_id: None,
                            error_message: Some(format!("CAPE状态查询失败: {}", e)),
                            started_at: None,
                            completed_at: None,
                        },
                    )
                    .await;
                return Ok(());
            }
        };
        if let Some(s) = &status.data {
            match s.as_str() {
                // 运行中/分析中
                "pending" | "running" | "analyzing" => {
                    if matches!(sub_task.status, SubTaskStatus::Submitted) {
                        let _ = self
                            .task_repo
                            .update_sub_task_status(
                                sub_task.id,
                                &crate::models::UpdateSubTaskStatusRequest {
                                    status: Some(SubTaskStatus::Analyzing),
                                    external_task_id: None,
                                    error_message: None,
                                    started_at: None,
                                    completed_at: None,
                                },
                            )
                            .await?;
                    }
                }
                // 将 completed 与 reported 都视为可获取报告的完成态
                "completed" | "reported" => {
                    let _ = self
                        .task_repo
                        .update_sub_task_status(
                            sub_task.id,
                            &crate::models::UpdateSubTaskStatusRequest {
                                status: Some(SubTaskStatus::Completed),
                                external_task_id: None,
                                error_message: None,
                                started_at: None,
                                completed_at: None,
                            },
                        )
                        .await?;
                    info!(
                        "任务 {} {} → 本地标记 completed (ext_id={})",
                        sub_task.id, s, ext_id
                    );
                }
                // 失败状态
                "failed" | "failed_analysis" | "failed_processing" | "failed_reporting" => {
                    let _ = self
                        .task_repo
                        .update_sub_task_status(
                            sub_task.id,
                            &crate::models::UpdateSubTaskStatusRequest {
                                status: Some(SubTaskStatus::Failed),
                                external_task_id: None,
                                error_message: Some(format!("CAPE返回失败状态: {}", s)),
                                started_at: None,
                                completed_at: None,
                            },
                        )
                        .await?;
                    info!(
                        "任务 {} CAPE失败状态 → 本地标记 failed (ext_id={})",
                        sub_task.id, ext_id
                    );
                }
                other => {
                    debug!(sub_task_id=%sub_task.id, status=%other, ext_id, "未知CAPE状态");
                }
            }
        }
        Ok(())
    }
}

use sqlx::Row;
