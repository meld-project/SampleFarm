use serde_json::Value as JsonValue;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    config::{cape::RetryExecutor, cfg::CfgTaskConfig},
    database::Database,
    error::{AppError, AppResult},
    models::task::MasterTaskStatus,
    storage::{MinioStorage, Storage},
};

use super::cfg_instance_manager::CfgInstanceManager;

pub struct CfgProcessor {
    pub manager: Arc<CfgInstanceManager>,
    pub database: Arc<Database>,
    pub storage: Arc<MinioStorage>,
    pub result_bucket: String,
    pub sample_bucket: String,
}

impl CfgProcessor {
    pub fn new(
        manager: Arc<CfgInstanceManager>,
        database: Arc<Database>,
        storage: Arc<MinioStorage>,
        result_bucket: String,
        sample_bucket: String,
    ) -> Self {
        Self {
            manager,
            database,
            storage,
            result_bucket,
            sample_bucket,
        }
    }

    /// 下载并上传结果文件到对象存储，返回对象路径
    async fn store_result_file(&self, task_id: &str, filename: &str) -> AppResult<String> {
        let client = self
            .manager
            .client()
            .await
            .ok_or_else(|| AppError::service_unavailable("没有可用的CFG客户端"))?;
        let data = client.download_result_file(task_id, filename).await?;
        let key = format!("cfg/{}/{}", task_id, filename);
        self.storage
            .upload(
                &self.result_bucket,
                &key,
                &data,
                Some("application/octet-stream"),
            )
            .await?;
        Ok(key)
    }

    /// 处理完成态：拉取结果、存储文件、生成结果 JSON（此处骨架，不落库）
    pub async fn process_completed(
        &self,
        task_id: &str,
        result_json: &JsonValue,
    ) -> AppResult<JsonValue> {
        let mut stored = serde_json::Map::new();
        if let Some(files) = result_json.get("result_files").and_then(|v| v.as_object()) {
            for (k, v) in files.iter() {
                if let Some(fname) = v.as_str() {
                    if let Ok(key) = self.store_result_file(task_id, fname).await {
                        stored.insert(k.clone(), JsonValue::String(key));
                    }
                }
            }
        }
        let mut merged = result_json.clone();
        if !stored.is_empty() {
            merged["result_files"] = JsonValue::Object(stored);
        }
        Ok(merged)
    }
    /// 主流程：提交/轮询/完成（推荐使用此方法，直接传递SubTask对象）
    /// 支持重试机制的CFG任务处理
    pub async fn process_sub_task_with_task(
        &self,
        sub_task: &crate::models::SubTask,
        sample_sha256: &str,
        config: Option<CfgTaskConfig>,
    ) -> AppResult<JsonValue> {
        // 直接使用SubTask中的主任务ID，避免查询错误
        self.check_master_task_status(sub_task.master_task_id)
            .await?;

        let config = config.unwrap_or_default();
        let retry_config = config.retry.clone().unwrap_or_default();
        let retry_executor = RetryExecutor::new(retry_config);

        // 带重试的任务提交和处理操作
        let result = retry_executor
            .execute_with_retry(
                || async {
                    // 在重试过程中也要检查主任务状态
                    self.check_master_task_status(sub_task.master_task_id)
                        .await?;

                    self.process_sub_task_internal(
                        sub_task.sample_id,
                        sub_task.master_task_id,
                        sample_sha256,
                        &config,
                        sub_task.cfg_instance_id,
                    )
                    .await
                },
                &sub_task.sample_id.to_string(),
            )
            .await?;

        Ok(result)
    }

    /// 主流程：提交/轮询/完成（兼容性方法，建议使用process_sub_task_with_task）
    /// 支持重试机制的CFG任务处理
    pub async fn process_sub_task(
        &self,
        sample_id: Uuid,
        sample_sha256: &str,
        config: Option<CfgTaskConfig>,
        cfg_instance_id: Option<Uuid>,
    ) -> AppResult<JsonValue> {
        // 先获取CFG类型的主任务ID并检查状态
        let master_task_id = self.get_master_task_id_by_sample(sample_id).await?;
        self.check_master_task_status(master_task_id).await?;

        let config = config.unwrap_or_default();
        let retry_config = config.retry.clone().unwrap_or_default();
        let retry_executor = RetryExecutor::new(retry_config);

        // 带重试的任务提交和处理操作
        let result = retry_executor
            .execute_with_retry(
                || async {
                    // 在重试过程中也要检查主任务状态
                    self.check_master_task_status(master_task_id).await?;

                    self.process_sub_task_internal(
                        sample_id,
                        master_task_id,
                        sample_sha256,
                        &config,
                        cfg_instance_id,
                    )
                    .await
                },
                &sample_id.to_string(),
            )
            .await?;

        Ok(result)
    }

    /// 内部处理方法（不包含重试逻辑）
    async fn process_sub_task_internal(
        &self,
        sample_id: Uuid,
        master_task_id: Uuid,
        sample_sha256: &str,
        config: &CfgTaskConfig,
        cfg_instance_id: Option<Uuid>,
    ) -> AppResult<JsonValue> {
        // 1) 查询样本文件名与存储路径
        let row = sqlx::query("SELECT file_name, storage_path FROM samples WHERE id = $1")
            .bind(sample_id)
            .fetch_one(self.database.pool())
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询样本信息失败: {}", e)))?;
        let file_name: String = row.get("file_name");
        let storage_path: String = row.get("storage_path");

        // 2) 从 MinIO 下载源文件
        let bytes = self
            .storage
            .download(&self.sample_bucket, &storage_path)
            .await?;

        // 3) 获取CFG客户端（按实例）并上传到 CFG 的 /preprocess_pe，task_id 使用 sha256
        let client = if let Some(instance_id) = cfg_instance_id {
            self.manager.get_client(instance_id).await.ok_or_else(|| {
                AppError::service_unavailable(format!("CFG实例 {} 的客户端不可用", instance_id))
            })?
        } else {
            self.manager
                .client()
                .await
                .ok_or_else(|| AppError::service_unavailable("没有可用的CFG客户端"))?
        };

        // 提交任务到CFG。
        // 若返回“任务ID已存在”，则视为幂等提交命中，直接进入轮询阶段。
        match client
            .submit_preprocess_pe_bytes(&file_name, &bytes, sample_sha256, config.label)
            .await
        {
            Ok(_resp) => {
                // 提交成功后再回写 external_task_id（即 sha256），避免提前写导致的404轮询噪音
                let _ = sqlx::query(
                    "UPDATE sub_tasks SET external_task_id = $1, updated_at = NOW() WHERE master_task_id = $2 AND sample_id = $3 AND analyzer_type = 'CFG'"
                )
                .bind(sample_sha256)
                .bind(master_task_id)
                .bind(sample_id)
                .execute(self.database.pool()).await;
            }
            Err(e) => {
                let msg = e.to_string();
                let already_exists =
                    msg.contains("已存在") || msg.to_ascii_lowercase().contains("already exist");
                if already_exists {
                    tracing::info!(
                        task_id = sample_sha256,
                        "CFG 任务已存在，跳过提交，直接进入轮询"
                    );
                    // 亦可补回 external_task_id 以便后续轮询
                    let _ = sqlx::query(
                        "UPDATE sub_tasks SET external_task_id = $1, updated_at = NOW() WHERE master_task_id = $2 AND sample_id = $3 AND analyzer_type = 'CFG'"
                    )
                    .bind(sample_sha256)
                    .bind(master_task_id)
                    .bind(sample_id)
                    .execute(self.database.pool()).await;
                } else {
                    return Err(e);
                }
            }
        }
        // 轮询（无超时限制）
        let poll_secs = config.poll_interval_secs;
        tracing::info!(
            task_id = sample_sha256,
            "开始CFG任务轮询（无超时限制，直到任务完成或失败）"
        );

        loop {
            // 暂停/取消等状态检查：若主任务不可执行，则终止轮询
            if let Err(e) = self.check_master_task_status(master_task_id).await {
                // 将暂停/取消/失败/完成视为中断原因，返回错误交由调用者区分处理
                return Err(e);
            }
            match client.get_task_status(sample_sha256).await {
                Ok(status) => {
                    let state = status.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    tracing::debug!(task_id = sample_sha256, state = state, "CFG任务状态轮询");

                    if state == "completed" {
                        // 获取结果
                        tracing::info!(task_id = sample_sha256, "CFG任务已完成，开始获取结果");
                        let result = client.get_result(sample_sha256).await?;
                        let merged = self.process_completed(sample_sha256, &result).await?;
                        return Ok(merged);
                    }
                    // 明确失败态
                    if state == "failed" {
                        let msg = status
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("CFG 任务失败");
                        // 将明确的处理失败归类为服务端处理失败，避免被解释为"请求参数错误"
                        tracing::warn!(task_id = sample_sha256, message = msg, "CFG任务处理失败");
                        return Err(AppError::service_unavailable(format!(
                            "CFG 处理失败: {}",
                            msg
                        )));
                    }
                }
                Err(e) => {
                    // 对 404 不立即失败，短暂等待后重试，避免提交与轮询竞态
                    tracing::warn!(task_id = sample_sha256, error = %e, "获取CFG任务状态失败（将重试）");
                    tokio::time::sleep(std::time::Duration::from_secs(poll_secs)).await;
                    continue;
                }
            }
            // 继续轮询，无超时限制
            tokio::time::sleep(std::time::Duration::from_secs(poll_secs)).await;
        }
    }

    /// 根据样本ID获取CFG类型的主任务ID
    async fn get_master_task_id_by_sample(&self, sample_id: Uuid) -> AppResult<Uuid> {
        let row = sqlx::query(
            "SELECT st.master_task_id FROM sub_tasks st WHERE st.sample_id = $1 AND st.analyzer_type = 'CFG' LIMIT 1"
        )
        .bind(sample_id)
        .fetch_one(self.database.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询CFG主任务ID失败: {}。请确认该样本存在CFG类型的子任务", e)))?;

        let master_task_id: Uuid = row
            .try_get("master_task_id")
            .map_err(|e| AppError::service_unavailable(format!("获取CFG主任务ID失败: {}", e)))?;

        Ok(master_task_id)
    }

    /// 检查主任务状态，如果任务被暂停、取消或删除则返回错误
    async fn check_master_task_status(&self, master_task_id: Uuid) -> AppResult<()> {
        let row = sqlx::query("SELECT status FROM master_tasks WHERE id = $1")
            .bind(master_task_id)
            .fetch_optional(self.database.pool())
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询主任务状态失败: {}", e)))?;

        match row {
            Some(row) => {
                let status: MasterTaskStatus = row.try_get("status").map_err(|e| {
                    AppError::service_unavailable(format!("获取主任务状态失败: {}", e))
                })?;

                match status {
                    MasterTaskStatus::Paused => {
                        return Err(AppError::bad_request(format!(
                            "CFG主任务 {} 已暂停，停止处理CFG子任务",
                            master_task_id
                        )));
                    }
                    MasterTaskStatus::Cancelled => {
                        return Err(AppError::bad_request(format!(
                            "CFG主任务 {} 已取消，停止处理CFG子任务",
                            master_task_id
                        )));
                    }
                    MasterTaskStatus::Failed => {
                        return Err(AppError::bad_request(format!(
                            "CFG主任务 {} 已失败，停止处理CFG子任务",
                            master_task_id
                        )));
                    }
                    MasterTaskStatus::Completed => {
                        return Err(AppError::bad_request(format!(
                            "CFG主任务 {} 已完成，无需继续处理CFG子任务",
                            master_task_id
                        )));
                    }
                    MasterTaskStatus::Pending | MasterTaskStatus::Running => {
                        // 这些状态允许继续处理
                        Ok(())
                    }
                }
            }
            None => {
                // CFG主任务不存在（可能已被删除）
                Err(AppError::not_found(format!(
                    "CFG主任务 {} 不存在，可能已被删除",
                    master_task_id
                )))
            }
        }
    }
}
