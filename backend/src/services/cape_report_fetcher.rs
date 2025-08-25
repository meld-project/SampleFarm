use crate::{
    error::AppError,
    models::SubTask,
    repositories::TaskRepository,
    services::{CapeClient, CapeInstanceManager},
};
use sqlx::Row;
use sqlx::types::Json;
use std::sync::Arc;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CapeReportFetcher {
    task_repo: TaskRepository,
    instance_manager: Arc<CapeInstanceManager>,
    interval_secs: u64,
}

impl CapeReportFetcher {
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
                warn!(error=%e, "CAPE报告拉取器执行失败");
            }
        }
    }

    async fn tick_once(&self) -> Result<(), AppError> {
        // 按实例分片扫描：已完成但无报告
        let instances = self.instance_manager.get_all_instances().await;
        let per_instance_limit: i64 = 1000;

        for instance in instances {
            let query = r#"
                SELECT st.id, st.master_task_id, st.sample_id, st.analyzer_type,
                       st.cape_instance_id, st.external_task_id, st.status, st.priority,
                       st.parameters, st.error_message, st.retry_count,
                       st.created_at, st.started_at, st.completed_at, st.updated_at
                FROM sub_tasks st
                LEFT JOIN cape_analysis_results car ON car.sub_task_id = st.id
                WHERE st.analyzer_type = 'CAPE'
                  AND st.cape_instance_id = $1
                  AND st.status = 'completed'
                  AND st.external_task_id IS NOT NULL
                  AND car.id IS NULL
                ORDER BY COALESCE(st.completed_at, st.updated_at, st.started_at) ASC
                LIMIT $2
            "#;

            let rows = sqlx::query(query)
                .bind(instance.id)
                .bind(per_instance_limit)
                .fetch_all(self.task_repo.pool())
                .await
                .map_err(|e| {
                    AppError::service_unavailable(format!("查询已完成但无报告任务失败: {}", e))
                })?;

            for row in rows {
                let sub_task = SubTask {
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
                if let Err(e) = self.fetch_one(&sub_task).await {
                    warn!(error=%e, "拉取单任务报告失败");
                }
            }
        }
        Ok(())
    }

    async fn fetch_one(&self, sub_task: &SubTask) -> Result<(), AppError> {
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

        let cape_task_id: i32 = sub_task
            .external_task_id
            .as_ref()
            .and_then(|s| s.parse::<i32>().ok())
            .ok_or_else(|| AppError::service_unavailable("非法的 external_task_id"))?;

        // 获取报告原始JSON与原始文本
        let cape_report = client.get_report_raw(cape_task_id).await?;
        let mut report_json = cape_report.json;

        // 在入库前清理JSON中PostgreSQL不接受的字符（如 \u0000）
        fn sanitize_json_for_pg(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::String(s) => {
                    if s.contains('\u{0000}') {
                        *s = s.replace('\u{0000}', "\u{FFFD}");
                    }
                }
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        sanitize_json_for_pg(item);
                    }
                }
                serde_json::Value::Object(map) => {
                    for (_, val) in map.iter_mut() {
                        sanitize_json_for_pg(val);
                    }
                }
                _ => {}
            }
        }

        sanitize_json_for_pg(&mut report_json);

        // 入库（复用现有 cape_status_sync 的存储逻辑更理想；此处先直接插入最小字段）
        let now = chrono::Utc::now();
        let result_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO cape_analysis_results (
                id, sub_task_id, sample_id, cape_task_id,
                full_report, full_report_raw, report_summary, created_at, updated_at
            )
            SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9
            WHERE NOT EXISTS (
                SELECT 1 FROM cape_analysis_results WHERE sub_task_id = $2
            )
            "#,
        )
        .bind(result_id)
        .bind(sub_task.id)
        .bind(sub_task.sample_id)
        .bind(cape_task_id)
        .bind(Json(report_json.clone()))
        .bind(cape_report.raw_text.into_bytes())
        .bind("CAPE报告已入库")
        .bind(now)
        .bind(now)
        .execute(self.task_repo.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("报告入库失败: {}", e)))?;

        info!("任务 {} 报告入库成功", sub_task.id);
        Ok(())
    }
}
