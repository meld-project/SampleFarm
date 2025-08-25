use crate::{
    error::AppError,
    models::{SubTask, SubTaskStatus, cape_result::CapeAnalysisResult},
    repositories::TaskRepository,
    services::cape_client::{CapeClient, TaskExecutionStats},
    storage::{MinioStorage as MinioClient, Storage},
};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// CAPE 分析处理器
#[derive(Debug, Clone)]
pub struct CapeProcessor {
    cape_client: CapeClient,
    task_repository: TaskRepository,
    storage_client: MinioClient,
    pool: PgPool,
}

// 使用config模块中定义的CapeTaskConfig
use crate::config::cape::{CapeTaskConfig, RetryExecutor};

/// 性能统计历史记录
#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    pub avg_analysis_duration: Option<Duration>,
    pub avg_submit_duration: Option<Duration>,
    pub avg_throughput_mbps: Option<f64>,
    pub success_rate: f64,
    pub total_tasks: u32,
}

impl CapeProcessor {
    /// 创建新的 CAPE 处理器
    pub fn new(
        cape_client: CapeClient,
        task_repository: TaskRepository,
        storage_client: MinioClient,
        pool: PgPool,
    ) -> Self {
        Self {
            cape_client,
            task_repository,
            storage_client,
            pool,
        }
    }

    /// 处理单个子任务
    pub async fn process_sub_task(
        &self,
        sub_task: &SubTask,
        config: Option<CapeTaskConfig>,
    ) -> Result<CapeAnalysisResult, AppError> {
        let config = config.unwrap_or_default();

        info!("开始处理子任务: {}", sub_task.id);

        // 0. 检查主任务是否仍然可以执行（未暂停/取消/删除）。
        // 如果已暂停/取消/删除，则将该子任务标记为 paused，避免被周期性扫描再次拾起。
        if let Err(e) = self.check_master_task_status(sub_task.master_task_id).await {
            let _ = self
                .task_repository
                .update_sub_task_status(
                    sub_task.id,
                    &crate::models::UpdateSubTaskStatusRequest {
                        status: Some(SubTaskStatus::Paused),
                        external_task_id: None,
                        error_message: Some(
                            "主任务处于暂停/不可执行状态，子任务已暂停".to_string(),
                        ),
                        started_at: None,
                        completed_at: None,
                    },
                )
                .await;
            return Err(e);
        }

        // 1. 生成占位符ID（使用负数），防止重复提交
        let placeholder_id = -1 * (Utc::now().timestamp_millis() % 1000000) as i32; // 使用负数时间戳作为占位符
        info!(
            "设置占位符ID: {} for 子任务: {}",
            placeholder_id, sub_task.id
        );

        // 2. 使用乐观锁尝试更新子任务状态为"提交中"，防止重复处理
        let update_result = self
            .try_lock_task_for_processing(sub_task.id, placeholder_id)
            .await?;
        if !update_result {
            info!("任务 {} 已被其他进程处理，跳过", sub_task.id);
            return Err(AppError::bad_request("任务已被其他进程处理"));
        }

        // 3. 创建重试执行器
        let retry_config = config.retry.clone().unwrap_or_default();
        let retry_executor = RetryExecutor::new(retry_config);

        // 4. 下载样本文件（这个步骤通常不需要重试，因为是从内部存储下载）
        info!(
            "正在下载样本文件: {} (子任务: {})",
            sub_task.sample_id, sub_task.id
        );
        let file_path = match self
            .download_sample_file(sub_task.sample_id, sub_task.id)
            .await
        {
            Ok(path) => {
                info!("样本文件下载成功: {}", path.display());
                path
            }
            Err(e) => {
                error!("下载样本文件失败: {}", e);
                self.update_sub_task_status(sub_task.id, SubTaskStatus::Failed, None, None)
                    .await
                    .ok();
                return Err(e);
            }
        };

        // 4. 带重试的CAPE提交操作
        let result = retry_executor
            .execute_with_retry(
                || async {
                    // 在提交前再次检查主任务状态
                    self.check_master_task_status(sub_task.master_task_id)
                        .await?;

                    // 提交到CAPE
                    info!("正在提交文件到CAPE: {}", file_path.display());
                    let options = config.options.clone().unwrap_or_default();
                    let result = self
                        .cape_client
                        .submit_file(&file_path, config.machine.as_deref(), Some(options))
                        .await?;

                    info!("文件提交CAPE成功，任务ID: {}", result.0);
                    Ok::<(i32, TaskExecutionStats), AppError>(result)
                },
                &sub_task.id.to_string(),
            )
            .await;

        let (cape_task_id, stats) = match result {
            Ok(success) => success,
            Err(e) => {
                // 区分临时性错误和永久性错误
                if self.is_transient_error(&e) {
                    // 临时性错误：先检查是否已经成功提交
                    warn!("提交遇到临时性错误: {}", e);

                    // 等待一小段时间
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

                    // 暂时跳过重新查询的逻辑，因为没有get_sub_task_by_id方法
                    // TODO: 可以通过其他方式查询任务状态

                    // 确实是失败了，回退为 pending
                    let error_msg = format!("临时性提交失败，已回退待重试: {}", e);
                    let _ = self
                        .task_repository
                        .update_sub_task_status(
                            sub_task.id,
                            &crate::models::UpdateSubTaskStatusRequest {
                                status: Some(SubTaskStatus::Pending),
                                external_task_id: None,
                                error_message: Some(error_msg),
                                started_at: None,
                                completed_at: None,
                            },
                        )
                        .await;
                } else {
                    // 永久性错误：直接标记失败
                    error!("提交遇到永久性错误，标记为失败: {}", e);
                    let error_msg = format!("永久性提交失败: {}", e);
                    let _ = self
                        .task_repository
                        .update_sub_task_status(
                            sub_task.id,
                            &crate::models::UpdateSubTaskStatusRequest {
                                status: Some(SubTaskStatus::Failed),
                                external_task_id: None,
                                error_message: Some(error_msg),
                                started_at: None,
                                completed_at: None,
                            },
                        )
                        .await;
                }
                return Err(e);
            }
        };

        // 5. 更新子任务状态为"已提交"
        self.update_sub_task_status(
            sub_task.id,
            SubTaskStatus::Submitted,
            Some(cape_task_id),
            Some(Utc::now()),
        )
        .await?;

        // 6. 记录提交成功的性能统计
        let end_time = Utc::now();
        let submission_duration = end_time.signed_duration_since(stats.submit_start_time);
        let mut final_stats = stats;
        final_stats.submit_end_time = Some(end_time);
        final_stats.submit_duration = Some(Duration::from_secs(
            submission_duration.num_seconds().max(0) as u64,
        ));

        // 性能统计记录已移除

        // 7. 创建占位的分析结果返回值（实际的分析结果将由状态同步器创建）
        // 注意：这里不保存到数据库，只是为了满足返回类型要求
        let cape_result = CapeAnalysisResult {
            id: uuid::Uuid::new_v4(),
            sub_task_id: sub_task.id,
            sample_id: sub_task.sample_id,
            cape_task_id,
            analysis_started_at: Some(Utc::now()),
            analysis_completed_at: None,
            analysis_duration: None,
            score: None,
            severity: None,
            verdict: None,
            signatures: None,
            behavior_summary: None,
            full_report: None,
            report_summary: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 8. 清理临时文件和目录
        // 首先删除文件
        if let Err(e) = tokio::fs::remove_file(&file_path).await {
            // 如果文件不存在，只记录debug级别日志（并发环境下这是正常的）
            if e.kind() == std::io::ErrorKind::NotFound {
                debug!("临时文件已不存在，可能被其他任务清理: {}", e);
            } else {
                warn!("清理临时文件失败: {}", e);
            }
        }

        // 然后删除临时目录（如果为空）
        if let Some(parent_dir) = file_path.parent() {
            if let Err(e) = tokio::fs::remove_dir(parent_dir).await {
                // 目录可能不为空或其他原因，只记录调试信息，不影响主流程
                debug!("清理临时目录失败（可能不为空）: {}", e);
            }
        }

        info!(
            "子任务 {} 已提交到CAPE，任务ID: {}，等待分析完成",
            sub_task.id, cape_task_id
        );
        info!("提交耗时: {:?}", final_stats.submit_duration);

        Ok(cape_result)
    }

    /// 从存储中下载样本文件
    async fn download_sample_file(
        &self,
        sample_id: Uuid,
        sub_task_id: Uuid,
    ) -> Result<std::path::PathBuf, AppError> {
        // 查询样本信息
        let sample_query = "SELECT file_name, storage_path FROM samples WHERE id = $1";
        let sample_row = sqlx::query(sample_query)
            .bind(sample_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询样本信息失败: {}", e)))?;

        let file_name: String = sample_row.get("file_name");
        let storage_path: String = sample_row.get("storage_path");

        // 生成临时文件路径 - 使用原始文件名，但添加UUID前缀避免冲突
        let temp_dir = std::env::temp_dir();

        // 为了避免文件名冲突，在临时目录中创建一个以sub_task_id命名的子目录，确保每个子任务使用独立路径
        let sample_temp_dir = temp_dir.join(format!("cape_sample_{}_{}", sample_id, sub_task_id));

        // 确保子目录存在
        tokio::fs::create_dir_all(&sample_temp_dir)
            .await
            .map_err(|e| AppError::service_unavailable(format!("创建临时目录失败: {}", e)))?;

        // 使用原始文件名，这样提交给CAPE时就是原始文件名
        let temp_file_path = sample_temp_dir.join(&file_name);

        // 从MinIO下载文件
        info!("正在从MinIO下载文件，路径: {}", storage_path);
        let file_content = self
            .storage_client
            .download("samplefarm", &storage_path)
            .await
            .map_err(|e| AppError::service_unavailable(format!("下载样本文件失败: {}", e)))?;

        // 验证下载的文件大小
        let downloaded_size = file_content.len();
        info!("文件下载完成，大小: {} 字节", downloaded_size);

        if downloaded_size == 0 {
            return Err(AppError::service_unavailable("下载的文件为空"));
        }

        // 写入临时文件前检查目录是否存在
        if !sample_temp_dir.exists() {
            return Err(AppError::service_unavailable(format!(
                "临时目录不存在: {:?}",
                sample_temp_dir
            )));
        }

        // 使用更安全的文件写入方法
        match tokio::fs::write(&temp_file_path, &file_content).await {
            Ok(()) => {
                debug!("文件写入操作完成: {:?}", temp_file_path);
            }
            Err(e) => {
                error!("写入临时文件失败: {} (路径: {:?})", e, temp_file_path);
                return Err(AppError::service_unavailable(format!(
                    "写入临时文件失败: {}",
                    e
                )));
            }
        }

        // 立即验证文件是否成功写入
        let written_size = match tokio::fs::metadata(&temp_file_path).await {
            Ok(metadata) => metadata.len(),
            Err(e) => {
                error!("读取临时文件元数据失败: {} (路径: {:?})", e, temp_file_path);
                return Err(AppError::service_unavailable(format!(
                    "读取临时文件信息失败: {}",
                    e
                )));
            }
        };

        info!("临时文件写入完成，大小: {} 字节", written_size);

        if written_size == 0 {
            error!("临时文件写入后大小为0字节，可能存在磁盘空间不足或权限问题");
            return Err(AppError::service_unavailable(
                "临时文件写入后大小为0字节，可能存在磁盘空间不足或权限问题",
            ));
        }

        if written_size != downloaded_size as u64 {
            error!(
                "文件写入不完整：期望 {} 字节，实际 {} 字节",
                downloaded_size, written_size
            );
            return Err(AppError::service_unavailable(format!(
                "文件写入不完整：期望 {} 字节，实际 {} 字节",
                downloaded_size, written_size
            )));
        }

        debug!(
            "样本文件下载到: {:?} (子任务: {})",
            temp_file_path, sub_task_id
        );
        info!("CAPE提交将使用原始文件名: {}", file_name);

        Ok(temp_file_path)
    }

    /// 使用乐观锁尝试锁定任务进行处理
    async fn try_lock_task_for_processing(
        &self,
        sub_task_id: Uuid,
        placeholder_id: i32,
    ) -> Result<bool, AppError> {
        let query = r#"
            UPDATE sub_tasks 
            SET status = 'submitting', 
                external_task_id = $2,
                started_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 
              AND status = 'pending' 
              AND (external_task_id IS NULL OR external_task_id LIKE '-%')
        "#;

        let result = sqlx::query(query)
            .bind(sub_task_id)
            .bind(placeholder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("锁定任务失败: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    /// 更新子任务状态
    async fn update_sub_task_status(
        &self,
        sub_task_id: Uuid,
        status: SubTaskStatus,
        external_task_id: Option<i32>,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        // 构建更新请求
        let mut request = crate::models::UpdateSubTaskStatusRequest {
            status: Some(status),
            external_task_id: external_task_id.map(|id| id.to_string()),
            error_message: None,
            started_at: None,
            completed_at: None,
        };

        // 根据状态类型设置时间戳
        if let Some(ts) = timestamp {
            match status {
                SubTaskStatus::Submitted => {
                    request.started_at = Some(ts);
                }
                SubTaskStatus::Completed => {
                    request.completed_at = Some(ts);
                }
                _ => {}
            }
        }

        // 使用 task_repository 更新状态
        self.task_repository
            .update_sub_task_status(sub_task_id, &request)
            .await?;

        debug!("子任务 {} 状态更新为: {:?}", sub_task_id, status);

        Ok(())
    }

    /// 判断错误是否为临时性/可重试错误（网络、连接、5xx等）
    fn is_transient_error(&self, error: &AppError) -> bool {
        let error_msg = error.to_string().to_lowercase();

        // 网络/连接相关错误
        if error_msg.contains("connection")
            || error_msg.contains("timeout")
            || error_msg.contains("network")
            || error_msg.contains("dns")
            || error_msg.contains("error sending request")
            || error_msg.contains("提交文件到cape失败")
        {
            return true;
        }

        // HTTP 5xx 错误（服务端临时问题）
        if error_msg.contains("cape返回错误状态 5") {
            return true;
        }

        // 服务不可用
        if error_msg.contains("service_unavailable") || error_msg.contains("service unavailable") {
            return true;
        }

        // JSON 解析失败（可能是临时返回了 HTML 错误页）
        if error_msg.contains("解析cape响应失败") || error_msg.contains("解析json") {
            return true;
        }

        // 其他明确的业务错误视为永久失败（如文件太大、参数错误等）
        false
    }

    /// 检查主任务状态，如果任务被暂停、取消或删除则返回错误
    async fn check_master_task_status(&self, master_task_id: Uuid) -> Result<(), AppError> {
        match self
            .task_repository
            .get_master_task_by_id(master_task_id)
            .await?
        {
            Some(master_task) => {
                match master_task.status {
                    crate::models::MasterTaskStatus::Paused => {
                        return Err(AppError::bad_request(format!(
                            "主任务 {} 已暂停，停止处理子任务",
                            master_task_id
                        )));
                    }
                    crate::models::MasterTaskStatus::Cancelled => {
                        return Err(AppError::bad_request(format!(
                            "主任务 {} 已取消，停止处理子任务",
                            master_task_id
                        )));
                    }
                    crate::models::MasterTaskStatus::Failed => {
                        return Err(AppError::bad_request(format!(
                            "主任务 {} 已失败，停止处理子任务",
                            master_task_id
                        )));
                    }
                    crate::models::MasterTaskStatus::Completed => {
                        return Err(AppError::bad_request(format!(
                            "主任务 {} 已完成，无需继续处理子任务",
                            master_task_id
                        )));
                    }
                    crate::models::MasterTaskStatus::Pending
                    | crate::models::MasterTaskStatus::Running => {
                        // 这些状态允许继续处理
                        Ok(())
                    }
                }
            }
            None => {
                // 主任务不存在（可能已被删除）
                Err(AppError::not_found(format!(
                    "主任务 {} 不存在，可能已被删除",
                    master_task_id
                )))
            }
        }
    }

    /// 获取历史性能统计（已弃用：返回默认值，避免依赖cape_performance_stats表）
    pub async fn get_performance_history(
        &self,
        _days: i32,
    ) -> Result<PerformanceHistory, AppError> {
        Ok(PerformanceHistory {
            avg_analysis_duration: Some(Duration::from_secs(300)),
            avg_submit_duration: Some(Duration::from_secs(30)),
            avg_throughput_mbps: None,
            success_rate: 1.0,
            total_tasks: 0,
        })
    }
}
