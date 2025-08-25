use crate::{
    config::cape::CapeTaskConfig,
    error::AppError,
    models::{AnalyzerType, SubTask, SubTaskStatus},
    repositories::TaskRepository,
    services::{CapeInstanceManager, CapeProcessor},
    storage::MinioStorage as MinioClient,
};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info};
use uuid::Uuid;

/// CAPE 任务管理器
/// 负责协调多个CAPE实例、处理器和任务队列
#[derive(Debug, Clone)]
pub struct CapeManager {
    /// CAPE实例管理器
    instance_manager: Arc<CapeInstanceManager>,
    /// 任务存储库
    task_repository: TaskRepository,
    /// 存储客户端
    storage_client: MinioClient,
    /// 数据库连接池
    pool: PgPool,
    /// 每个实例的信号量缓存（控制并发任务数量）
    semaphores: Arc<RwLock<HashMap<Uuid, Arc<Semaphore>>>>,
}

/// CAPE 任务统计
#[derive(Debug, Clone)]
pub struct CapeTaskStats {
    pub total_submitted: u32,
    pub total_completed: u32,
    pub total_failed: u32,
    pub average_duration_seconds: Option<f64>,
    pub success_rate: f64,
}

impl CapeManager {
    /// 创建新的 CAPE 管理器
    pub async fn new(pool: PgPool, storage_client: MinioClient) -> Result<Self, AppError> {
        let instance_manager = Arc::new(CapeInstanceManager::new(pool.clone()).await?);
        let task_repository = TaskRepository::new(pool.clone());

        Ok(Self {
            instance_manager,
            task_repository,
            storage_client,
            pool,
            semaphores: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 检查所有CAPE实例的健康状态
    pub async fn health_check(&self) -> Result<bool, AppError> {
        let available_instances = self.instance_manager.get_available_instances().await;
        Ok(!available_instances.is_empty())
    }

    // 移除实例级并发限制，改为完全由任务级并发控制

    /// 提交单个子任务到指定CAPE实例进行分析
    pub async fn submit_sub_task_to_instance(
        &self,
        sub_task: &SubTask,
        cape_instance_id: Option<Uuid>,
        config: Option<CapeTaskConfig>,
    ) -> Result<(), AppError> {
        // 确定使用的CAPE实例
        let instance = if let Some(instance_id) = cape_instance_id {
            self.instance_manager
                .get_instance(instance_id)
                .await
                .ok_or_else(|| AppError::not_found("指定的CAPE实例不存在"))?
        } else {
            self.instance_manager
                .get_default_instance()
                .await
                .ok_or_else(|| AppError::service_unavailable("没有可用的CAPE实例"))?
        };

        if !instance.is_available() {
            return Err(AppError::service_unavailable(format!(
                "CAPE实例 {} 不可用",
                instance.name
            )));
        }

        // 实例级并发限制已移除，统一由外层 submit_master_task 的全局并发控制

        info!(
            "开始处理CAPE子任务: {} (实例: {})",
            sub_task.id, instance.name
        );

        // 获取客户端
        let base_client = self
            .instance_manager
            .get_client(instance.id)
            .await
            .ok_or_else(|| AppError::service_unavailable("CAPE客户端不可用"))?;

        let task_config = config.unwrap_or_default();
        // 注意：现在不使用超时限制，确保获取到报告结果
        let client = base_client;

        // 创建处理器（使用注入后的客户端）
        let cape_processor = CapeProcessor::new(
            client,
            self.task_repository.clone(),
            self.storage_client.clone(),
            self.pool.clone(),
        );

        // 使用处理器执行任务
        match cape_processor
            .process_sub_task(sub_task, Some(task_config))
            .await
        {
            Ok(_) => {
                info!(
                    "CAPE子任务 {} 处理成功 (实例: {})",
                    sub_task.id, instance.name
                );
                Ok(())
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("任务已被其他进程处理") {
                    info!(
                        "CAPE子任务 {} 已被其他执行流占用，跳过 (实例: {})",
                        sub_task.id, instance.name
                    );
                } else if msg.contains("主任务")
                    && (msg.contains("已暂停")
                        || msg.contains("已取消")
                        || msg.contains("已完成")
                        || msg.contains("已失败"))
                {
                    // 主任务不可执行类：视为业务流程控制，降级为 info
                    info!(
                        "CAPE子任务 {} 被跳过 (实例: {}): {}",
                        sub_task.id, instance.name, msg
                    );
                } else {
                    error!(
                        "CAPE子任务 {} 处理失败 (实例: {}): {}",
                        sub_task.id, instance.name, msg
                    );
                }
                Err(e)
            }
        }
    }

    /// 提交单个子任务到CAPE进行分析（向后兼容）
    pub async fn submit_sub_task(
        &self,
        sub_task: &SubTask,
        config: Option<CapeTaskConfig>,
    ) -> Result<(), AppError> {
        self.submit_sub_task_to_instance(sub_task, sub_task.cape_instance_id, config)
            .await
    }

    /// 批量提交主任务下的所有子任务
    pub async fn submit_master_task(
        &self,
        master_task_id: Uuid,
        config: Option<CapeTaskConfig>,
        submit_interval_ms: u64,
    ) -> Result<CapeTaskStats, AppError> {
        // 检查是否有可用的CAPE实例
        if !self.health_check().await? {
            return Err(AppError::service_unavailable(
                "没有可用的CAPE实例".to_string(),
            ));
        }

        info!("开始批量处理CAPE主任务: {}", master_task_id);

        // 获取主任务信息
        let master_task = self
            .task_repository
            .get_master_task_by_id(master_task_id)
            .await?
            .ok_or_else(|| AppError::bad_request("主任务不存在".to_string()))?;

        // 验证分析器类型
        if master_task.analyzer_type != AnalyzerType::CAPE {
            return Err(AppError::bad_request(format!(
                "不支持的分析器类型: {:?}",
                master_task.analyzer_type
            )));
        }

        // 检查主任务是否被暂停
        if matches!(master_task.status, crate::models::MasterTaskStatus::Paused) {
            return Err(AppError::bad_request(
                "主任务已暂停，无法提交子任务".to_string(),
            ));
        }

        // 获取待处理的子任务
        let sub_tasks = self
            .task_repository
            .list_sub_tasks_by_master_task(master_task_id)
            .await?;

        let pending_tasks: Vec<_> = sub_tasks
            .into_iter()
            .filter(|task| matches!(task.status, SubTaskStatus::Pending))
            .collect();

        if pending_tasks.is_empty() {
            return Err(AppError::bad_request("没有待处理的子任务".to_string()));
        }

        let total_tasks = pending_tasks.len();
        let mut completed = 0;
        let mut failed = 0;

        // 改为串行提交，避免并发问题
        for (idx, sub_task) in pending_tasks.into_iter().enumerate() {
            // 每次提交前，重新检查主任务是否仍然存在且可执行，避免删除后的残余提交
            match self
                .task_repository
                .get_master_task_by_id(master_task_id)
                .await?
            {
                Some(master_task) => {
                    if matches!(
                        master_task.status,
                        crate::models::MasterTaskStatus::Paused
                            | crate::models::MasterTaskStatus::Cancelled
                            | crate::models::MasterTaskStatus::Failed
                            | crate::models::MasterTaskStatus::Completed
                    ) {
                        info!(
                            "主任务 {} 状态为 {:?}，终止后续子任务提交",
                            master_task_id, master_task.status
                        );
                        break;
                    }
                }
                None => {
                    info!(
                        "主任务 {} 已不存在（可能被删除），终止后续子任务提交",
                        master_task_id
                    );
                    break;
                }
            }
            // 提交间隔
            if idx > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(submit_interval_ms)).await;
            }

            // 串行执行任务
            match self.submit_sub_task(&sub_task, config.clone()).await {
                Ok(_) => {
                    debug!("子任务 {} 完成", sub_task.id);
                    completed += 1;
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("任务已被其他进程处理") {
                        info!("子任务 {} 已被其他执行流占用，跳过计数", sub_task.id);
                        // 不计入失败/成功
                    } else if msg.contains("主任务")
                        && (msg.contains("已暂停")
                            || msg.contains("已取消")
                            || msg.contains("已完成")
                            || msg.contains("已失败")
                            || msg.contains("不存在"))
                    {
                        // 主任务不可执行类：不计入失败，按跳过处理
                        info!("子任务 {} 因主任务不可执行而跳过", sub_task.id);
                    } else {
                        error!("子任务 {} 失败: {}", sub_task.id, msg);
                        failed += 1;
                    }
                }
            }
        }

        // 更新主任务状态
        let final_status = if failed == 0 {
            crate::models::MasterTaskStatus::Completed
        } else if completed == 0 {
            crate::models::MasterTaskStatus::Failed
        } else {
            crate::models::MasterTaskStatus::Completed // 部分成功也算完成
        };

        let update_request = crate::models::UpdateMasterTaskRequest {
            status: Some(final_status),
            progress: Some(100),
            completed_samples: Some(completed as i32),
            failed_samples: Some(failed as i32),
            error_message: None,
            result_summary: None,
        };

        if let Err(e) = self
            .task_repository
            .update_master_task(master_task_id, &update_request)
            .await
        {
            error!("更新主任务状态失败: {}", e);
        }

        let success_rate = if total_tasks > 0 {
            completed as f64 / total_tasks as f64
        } else {
            0.0
        };

        let stats = CapeTaskStats {
            total_submitted: total_tasks as u32,
            total_completed: completed as u32,
            total_failed: failed as u32,
            average_duration_seconds: None,
            success_rate,
        };

        info!(
            "CAPE主任务 {} 处理完成: {}/{} 成功, 成功率: {:.2}%",
            master_task_id,
            completed,
            total_tasks,
            success_rate * 100.0
        );

        Ok(stats)
    }

    /// 获取CAPE任务执行统计（已弃用：不再依赖 cape_performance_stats 表）
    pub async fn get_task_statistics(&self, _days: i32) -> Result<CapeTaskStats, AppError> {
        Ok(CapeTaskStats {
            total_submitted: 0,
            total_completed: 0,
            total_failed: 0,
            average_duration_seconds: None,
            success_rate: 0.0,
        })
    }

    /// 获取当前并发限制和使用情况（为所有实例汇总）
    pub async fn get_concurrency_info(&self) -> (u32, u32) {
        let instances = self.instance_manager.get_available_instances().await;
        let total_max_concurrent: u32 = instances
            .iter()
            .map(|i| i.max_concurrent_tasks as u32)
            .sum();

        let semaphores = self.semaphores.read().await;
        let total_available: u32 = instances
            .iter()
            .filter_map(|instance| semaphores.get(&instance.id))
            .map(|semaphore| semaphore.available_permits() as u32)
            .sum();

        let current_running = total_max_concurrent.saturating_sub(total_available);
        (total_max_concurrent, current_running)
    }

    /// 获取CAPE实例管理器的引用
    pub fn instance_manager(&self) -> &CapeInstanceManager {
        &self.instance_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::cape::CapeConfig;

    #[test]
    fn test_cape_manager_creation() {
        let config = CapeConfig::default();
        assert!(!config.is_enabled());
        assert_eq!(config.max_concurrent_tasks, 5);
    }

    #[test]
    fn test_cape_task_stats() {
        let stats = CapeTaskStats {
            total_submitted: 100,
            total_completed: 95,
            total_failed: 5,
            average_duration_seconds: Some(240.0),
            success_rate: 0.95,
        };

        assert_eq!(stats.success_rate, 0.95);
        assert_eq!(stats.total_submitted, 100);
    }
}
