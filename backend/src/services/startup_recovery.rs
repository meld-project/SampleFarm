use crate::{
    config::StartupRecoveryConfig,
    error::AppError,
    models::{AnalyzerType, SubTask},
    services::{CapeManager, CfgInstanceManager, CfgProcessor},
};
use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// 启动恢复服务
/// 负责在程序重启后恢复未完成的任务
#[derive(Clone)]
pub struct StartupRecovery {
    /// 数据库连接池
    pool: PgPool,
    /// CAPE管理器
    cape_manager: Option<Arc<CapeManager>>,
    /// CFG组件（可选）
    cfg_manager: Option<Arc<CfgInstanceManager>>,
    cfg_processor: Option<Arc<CfgProcessor>>,
    /// 配置
    config: StartupRecoveryConfig,
}

/// 恢复统计信息
#[derive(Debug, Default)]
pub struct RecoveryStats {
    pub pending_found: u32,
    pub pending_submitted: u32,
    pub pending_failed: u32,
    pub stuck_found: u32,
    pub stuck_recovered: u32,
    pub stuck_failed: u32,
}

impl StartupRecovery {
    /// 创建新的启动恢复服务
    pub fn new(
        pool: PgPool,
        cape_manager: Option<Arc<CapeManager>>,
        cfg_manager: Option<Arc<CfgInstanceManager>>,
        cfg_processor: Option<Arc<CfgProcessor>>,
        config: StartupRecoveryConfig,
    ) -> Self {
        Self {
            pool,
            cape_manager,
            cfg_manager,
            cfg_processor,
            config,
        }
    }

    /// 启动一次性恢复扫描
    pub async fn start_initial_scan(&self) {
        if !self.config.enabled {
            info!("启动恢复已禁用，跳过一次性扫描");
            return;
        }

        info!(
            "启动恢复服务：将在 {} 秒后执行一次性扫描",
            self.config.initial_delay_secs
        );

        tokio::spawn({
            let self_clone = self.clone();
            async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    self_clone.config.initial_delay_secs,
                ))
                .await;

                info!("开始执行启动恢复的一次性扫描");
                match self_clone.scan_and_recover_all().await {
                    Ok(stats) => {
                        info!(
                            "一次性恢复扫描完成：pending找到/提交/失败={}/{}/{}, stuck找到/恢复/失败={}/{}/{}",
                            stats.pending_found,
                            stats.pending_submitted,
                            stats.pending_failed,
                            stats.stuck_found,
                            stats.stuck_recovered,
                            stats.stuck_failed
                        );
                    }
                    Err(e) => {
                        error!("一次性恢复扫描失败: {}", e);
                    }
                }
            }
        });
    }

    /// 启动周期性补偿扫描
    pub async fn start_periodic_scan(&self) {
        if !self.config.enabled {
            info!("启动恢复已禁用，跳过周期性扫描");
            return;
        }

        info!(
            "启动恢复服务：周期性补偿扫描间隔 {} 秒",
            self.config.scan_interval_secs
        );

        tokio::spawn({
            let self_clone = self.clone();
            async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
                    self_clone.config.scan_interval_secs,
                ));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                loop {
                    interval.tick().await;

                    debug!("开始执行周期性恢复补偿");
                    match self_clone.scan_and_recover_stuck_only().await {
                        Ok(stats) => {
                            if stats.stuck_found > 0 {
                                info!(
                                    "周期性补偿完成：stuck找到/恢复/失败={}/{}/{}",
                                    stats.stuck_found, stats.stuck_recovered, stats.stuck_failed
                                );
                            } else {
                                debug!("周期性补偿完成：无僵死任务需要恢复");
                            }
                        }
                        Err(e) => {
                            warn!("周期性恢复补偿失败: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// 扫描并恢复所有类型的任务（pending + stuck）
    async fn scan_and_recover_all(&self) -> Result<RecoveryStats, AppError> {
        let mut stats = RecoveryStats::default();

        // 恢复CAPE pending任务
        if let Some(cape_manager) = &self.cape_manager {
            let cape_stats = self
                .scan_and_submit_pending_cape(cape_manager.clone())
                .await?;
            stats.pending_found += cape_stats.pending_found;
            stats.pending_submitted += cape_stats.pending_submitted;
            stats.pending_failed += cape_stats.pending_failed;
        }

        // 恢复CFG pending任务
        if let (Some(cfg_manager), Some(cfg_processor)) = (&self.cfg_manager, &self.cfg_processor) {
            let cfg_stats = self
                .scan_and_submit_pending_cfg(cfg_manager.clone(), cfg_processor.clone())
                .await?;
            stats.pending_found += cfg_stats.pending_found;
            stats.pending_submitted += cfg_stats.pending_submitted;
            stats.pending_failed += cfg_stats.pending_failed;
        }

        // 恢复僵死任务
        let stuck_stats = self.scan_and_recover_stuck().await?;
        stats.stuck_found += stuck_stats.stuck_found;
        stats.stuck_recovered += stuck_stats.stuck_recovered;
        stats.stuck_failed += stuck_stats.stuck_failed;

        Ok(stats)
    }

    /// 仅扫描并恢复僵死任务（周期性补偿用）
    async fn scan_and_recover_stuck_only(&self) -> Result<RecoveryStats, AppError> {
        self.scan_and_recover_stuck().await
    }

    /// 扫描并提交CAPE的pending任务
    async fn scan_and_submit_pending_cape(
        &self,
        cape_manager: Arc<CapeManager>,
    ) -> Result<RecoveryStats, AppError> {
        debug!("开始扫描CAPE pending任务");

        let pending_tasks = self.list_pending_sub_tasks(AnalyzerType::CAPE).await?;
        let mut stats = RecoveryStats::default();
        stats.pending_found = pending_tasks.len() as u32;

        if pending_tasks.is_empty() {
            debug!("没有发现CAPE pending任务");
            return Ok(stats);
        }

        info!("发现 {} 个CAPE pending任务，开始提交", pending_tasks.len());

        // 串行处理任务，避免并发问题
        for sub_task in pending_tasks {
            match Self::submit_single_pending_task(
                cape_manager.clone(),
                sub_task.clone(),
                self.config.clone(),
                self.pool.clone(),
            )
            .await
            {
                Ok(()) => {
                    stats.pending_submitted += 1;
                }
                Err(e) => {
                    warn!("提交pending任务失败: {}", e);
                    stats.pending_failed += 1;
                }
            }

            // 任务间稍作延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!(
            "CAPE pending任务提交完成：提交/失败={}/{}",
            stats.pending_submitted, stats.pending_failed
        );
        Ok(stats)
    }

    /// 扫描并提交CFG的pending任务
    async fn scan_and_submit_pending_cfg(
        &self,
        _cfg_manager: Arc<CfgInstanceManager>,
        cfg_processor: Arc<CfgProcessor>,
    ) -> Result<RecoveryStats, AppError> {
        debug!("开始扫描CFG pending任务");

        let pending_tasks = self.list_pending_sub_tasks(AnalyzerType::CFG).await?;
        let mut stats = RecoveryStats::default();
        stats.pending_found = pending_tasks.len() as u32;

        if pending_tasks.is_empty() {
            debug!("没有发现CFG pending任务");
            return Ok(stats);
        }

        info!("发现 {} 个CFG pending任务，开始提交", pending_tasks.len());

        // 串行处理任务，避免并发问题
        for sub_task in pending_tasks {
            match Self::submit_single_pending_cfg_task(
                cfg_processor.clone(),
                sub_task.clone(),
                self.config.clone(),
                self.pool.clone(),
            )
            .await
            {
                Ok(()) => {
                    stats.pending_submitted += 1;
                }
                Err(e) => {
                    warn!("提交CFG pending任务失败: {}", e);
                    stats.pending_failed += 1;
                }
            }

            // 任务间稍作延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!(
            "CFG pending任务提交完成：提交/失败={}/{}",
            stats.pending_submitted, stats.pending_failed
        );
        Ok(stats)
    }

    /// 提交单个pending任务（静态方法，用于tokio::spawn）
    async fn submit_single_pending_task(
        cape_manager: Arc<CapeManager>,
        sub_task: SubTask,
        _config: StartupRecoveryConfig,
        pool: PgPool,
    ) -> Result<(), AppError> {
        // 1. 使用乐观锁尝试将任务标记为submitting
        let updated_task = Self::try_lock_and_mark_submitting(&pool, sub_task.id).await?;
        if updated_task.is_none() {
            debug!("任务 {} 已被其他进程处理，跳过", sub_task.id);
            return Ok(());
        }

        let mut current_task = updated_task.unwrap();

        // 2. 尝试提交到CAPE
        let submit_result = cape_manager
            .submit_sub_task_to_instance(
                &current_task,
                current_task.cape_instance_id,
                None, // 使用默认配置
            )
            .await;

        // 3. 根据提交结果更新任务状态
        match submit_result {
            Ok(()) => {
                // 提交成功，任务状态应该已经被CAPE manager更新为submitted或analyzing
                debug!("任务 {} 提交成功", current_task.id);
            }
            Err(e) => {
                // 提交失败，回滚为pending并记录错误（不再因为超过最大重试次数而标记失败）
                current_task.retry_count += 1;
                Self::mark_task_as_pending_with_error(&pool, current_task.id, &e.to_string())
                    .await?;
                warn!(
                    "任务 {} 提交失败，已回滚为pending，将继续重试: {}",
                    current_task.id, e
                );
                return Err(e);
            }
        }

        Ok(())
    }

    /// 提交单个CFG pending任务（静态方法，用于tokio::spawn）
    async fn submit_single_pending_cfg_task(
        cfg_processor: Arc<CfgProcessor>,
        sub_task: SubTask,
        _config: StartupRecoveryConfig,
        pool: PgPool,
    ) -> Result<(), AppError> {
        // 1. 使用乐观锁尝试将任务标记为submitting
        let updated_task = Self::try_lock_and_mark_submitting(&pool, sub_task.id).await?;
        if updated_task.is_none() {
            debug!("CFG任务 {} 已被其他进程处理，跳过", sub_task.id);
            return Ok(());
        }

        let mut current_task = updated_task.unwrap();

        // 2. 获取样本SHA256（CFG使用SHA256作为external_task_id）
        let sha256 = match Self::get_sample_sha256(&pool, current_task.sample_id).await {
            Ok(Some(hash)) => hash,
            Ok(None) => {
                let error_msg = "样本SHA256不存在，无法提交CFG任务";
                Self::mark_task_as_failed(&pool, current_task.id, error_msg).await?;
                return Err(AppError::bad_request(error_msg));
            }
            Err(e) => {
                Self::mark_task_as_pending_with_error(&pool, current_task.id, &e.to_string())
                    .await?;
                return Err(e);
            }
        };

        // 3. 设置external_task_id为SHA256
        if let Err(e) = Self::update_external_task_id(&pool, current_task.id, &sha256).await {
            warn!("更新CFG任务external_task_id失败: {}", e);
        }

        // 4. 尝试提交到CFG（使用默认配置，无超时限制）
        use crate::config::cfg::CfgTaskConfig;
        #[allow(deprecated)]
        let cfg_config = Some(CfgTaskConfig {
            poll_interval_secs: 10,
            max_wait_secs: 0, // 超时机制已移除
            label: 1,
            retry: None, // 启动恢复中不使用重试，由上层处理
        });

        // 使用新的process_sub_task_with_task方法，避免样本ID关联错误
        let submit_result = cfg_processor
            .process_sub_task_with_task(&current_task, &sha256, cfg_config)
            .await;

        // 5. 根据提交结果更新任务状态
        match submit_result {
            Ok(merged_result) => {
                // CFG process_sub_task成功意味着整个流程完成，直接入库结果
                let message = merged_result
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let result_files = merged_result.get("result_files").cloned();
                let full_report = Some(merged_result);

                // 插入CFG分析结果
                if let Err(e) = Self::store_cfg_analysis_result(
                    &pool,
                    current_task.id,
                    current_task.sample_id,
                    message,
                    result_files,
                    full_report,
                )
                .await
                {
                    warn!("存储CFG分析结果失败: {}，但任务已完成", e);
                }

                // 标记任务为完成
                Self::mark_task_as_completed(&pool, current_task.id).await?;
                debug!("CFG任务 {} 提交并完成成功", current_task.id);
            }
            Err(e) => {
                // 提交失败，回滚为pending并记录错误（不再因为超过最大重试次数而标记失败）
                current_task.retry_count += 1;
                Self::mark_task_as_pending_with_error(&pool, current_task.id, &e.to_string())
                    .await?;
                warn!(
                    "CFG任务 {} 提交失败，已回滚为pending，将继续重试（当前重试计数 {}）: {}",
                    current_task.id, current_task.retry_count, e
                );
                return Err(e);
            }
        }

        Ok(())
    }

    /// 扫描并恢复僵死的submitting任务
    async fn scan_and_recover_stuck(&self) -> Result<RecoveryStats, AppError> {
        debug!("开始扫描僵死submitting任务");

        let stuck_tasks = self.list_stuck_submitting_sub_tasks().await?;
        let mut stats = RecoveryStats::default();
        stats.stuck_found = stuck_tasks.len() as u32;

        if stuck_tasks.is_empty() {
            debug!("没有发现僵死任务");
            return Ok(stats);
        }

        info!("发现 {} 个僵死submitting任务，开始恢复", stuck_tasks.len());

        for sub_task in stuck_tasks {
            match Self::recover_stuck_task(&self.pool, &sub_task).await {
                Ok(()) => {
                    stats.stuck_recovered += 1;
                    debug!("僵死任务 {} 已恢复为pending", sub_task.id);
                }
                Err(e) => {
                    stats.stuck_failed += 1;
                    warn!("恢复僵死任务 {} 失败: {}", sub_task.id, e);
                }
            }
        }

        info!(
            "僵死任务恢复完成：恢复/失败={}/{}",
            stats.stuck_recovered, stats.stuck_failed
        );
        Ok(stats)
    }

    /// 查询pending状态的子任务（排除已暂停的主任务）
    async fn list_pending_sub_tasks(
        &self,
        analyzer_type: AnalyzerType,
    ) -> Result<Vec<SubTask>, AppError> {
        let query = r#"
            SELECT st.id, st.master_task_id, st.sample_id, st.analyzer_type, st.cape_instance_id, st.cfg_instance_id, st.external_task_id, 
                   st.status, st.priority, st.parameters, st.error_message, st.retry_count,
                   st.created_at, st.started_at, st.completed_at, st.updated_at
            FROM sub_tasks st
            JOIN master_tasks mt ON st.master_task_id = mt.id
            WHERE st.status = 'pending' 
              AND st.analyzer_type = $1
              AND st.external_task_id IS NULL
              AND mt.status NOT IN ('paused','cancelled','failed','completed')
            ORDER BY st.created_at ASC
            LIMIT $2
        "#;

        let rows = sqlx::query(query)
            .bind(analyzer_type)
            .bind(self.config.batch_size as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询pending任务失败: {}", e)))?;

        let tasks = rows
            .into_iter()
            .map(|row| SubTask {
                id: row.get("id"),
                master_task_id: row.get("master_task_id"),
                sample_id: row.get("sample_id"),
                analyzer_type: row.get("analyzer_type"),
                cape_instance_id: row.get("cape_instance_id"),
                cfg_instance_id: row.get("cfg_instance_id"),
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
            })
            .collect();

        Ok(tasks)
    }

    /// 查询僵死的submitting任务（排除已暂停的主任务）
    async fn list_stuck_submitting_sub_tasks(&self) -> Result<Vec<SubTask>, AppError> {
        let threshold =
            Utc::now() - Duration::seconds(self.config.stuck_submitting_threshold_secs as i64);

        let query = r#"
            SELECT st.id, st.master_task_id, st.sample_id, st.analyzer_type, st.cape_instance_id, st.cfg_instance_id, st.external_task_id, 
                   st.status, st.priority, st.parameters, st.error_message, st.retry_count,
                   st.created_at, st.started_at, st.completed_at, st.updated_at
            FROM sub_tasks st
            JOIN master_tasks mt ON st.master_task_id = mt.id
            WHERE st.status = 'submitting' 
              AND st.external_task_id IS NULL
              AND st.updated_at < $1
              AND mt.status NOT IN ('paused','cancelled','failed','completed')
            ORDER BY st.updated_at ASC
            LIMIT $2
        "#;

        let rows = sqlx::query(query)
            .bind(threshold)
            .bind(self.config.batch_size as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询僵死任务失败: {}", e)))?;

        let tasks = rows
            .into_iter()
            .map(|row| SubTask {
                id: row.get("id"),
                master_task_id: row.get("master_task_id"),
                sample_id: row.get("sample_id"),
                analyzer_type: row.get("analyzer_type"),
                cape_instance_id: row.get("cape_instance_id"),
                cfg_instance_id: row.get("cfg_instance_id"),
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
            })
            .collect();

        Ok(tasks)
    }

    /// 使用乐观锁尝试将任务标记为submitting
    async fn try_lock_and_mark_submitting(
        pool: &PgPool,
        sub_task_id: Uuid,
    ) -> Result<Option<SubTask>, AppError> {
        let query = r#"
            UPDATE sub_tasks 
            SET status = 'submitting', started_at = COALESCE(started_at, NOW()), updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            RETURNING id, master_task_id, sample_id, analyzer_type, cape_instance_id, cfg_instance_id, external_task_id, 
                     status, priority, parameters, error_message, retry_count,
                     created_at, started_at, completed_at, updated_at
        "#;

        let result = sqlx::query(query)
            .bind(sub_task_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("锁定任务失败: {}", e)))?;

        if let Some(row) = result {
            let task = SubTask {
                id: row.get("id"),
                master_task_id: row.get("master_task_id"),
                sample_id: row.get("sample_id"),
                analyzer_type: row.get("analyzer_type"),
                cape_instance_id: row.get("cape_instance_id"),
                cfg_instance_id: row.get("cfg_instance_id"),
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
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// 将任务标记为pending并记录错误
    async fn mark_task_as_pending_with_error(
        pool: &PgPool,
        sub_task_id: Uuid,
        error_message: &str,
    ) -> Result<(), AppError> {
        let query = r#"
            UPDATE sub_tasks 
            SET status = 'pending', error_message = $2, retry_count = retry_count + 1, updated_at = NOW()
            WHERE id = $1
        "#;

        sqlx::query(query)
            .bind(sub_task_id)
            .bind(error_message)
            .execute(pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("更新任务状态失败: {}", e)))?;

        Ok(())
    }

    /// 将任务标记为失败
    async fn mark_task_as_failed(
        pool: &PgPool,
        sub_task_id: Uuid,
        error_message: &str,
    ) -> Result<(), AppError> {
        let query = r#"
            UPDATE sub_tasks 
            SET status = 'failed', error_message = $2, completed_at = NOW(), updated_at = NOW()
            WHERE id = $1
        "#;

        sqlx::query(query)
            .bind(sub_task_id)
            .bind(error_message)
            .execute(pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("更新任务状态失败: {}", e)))?;

        Ok(())
    }

    /// 恢复僵死任务
    async fn recover_stuck_task(pool: &PgPool, sub_task: &SubTask) -> Result<(), AppError> {
        let query = r#"
            UPDATE sub_tasks 
            SET status = 'pending', retry_count = retry_count + 1, updated_at = NOW(),
                error_message = 'Recovered from stuck submitting state'
            WHERE id = $1 AND status = 'submitting' AND external_task_id IS NULL
        "#;

        let result = sqlx::query(query)
            .bind(sub_task.id)
            .execute(pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("恢复僵死任务失败: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("任务已被其他进程处理或状态已变更"));
        }

        Ok(())
    }

    /// 获取样本的SHA256哈希值
    async fn get_sample_sha256(pool: &PgPool, sample_id: Uuid) -> Result<Option<String>, AppError> {
        let query = "SELECT file_hash_sha256 FROM samples WHERE id = $1";

        let result = sqlx::query(query)
            .bind(sample_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询样本SHA256失败: {}", e)))?;

        if let Some(row) = result {
            let sha256: Option<String> = row.get("file_hash_sha256");
            Ok(sha256)
        } else {
            Ok(None)
        }
    }

    /// 更新子任务的external_task_id
    async fn update_external_task_id(
        pool: &PgPool,
        sub_task_id: Uuid,
        external_task_id: &str,
    ) -> Result<(), AppError> {
        let query = "UPDATE sub_tasks SET external_task_id = $1, updated_at = NOW() WHERE id = $2";

        sqlx::query(query)
            .bind(external_task_id)
            .bind(sub_task_id)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::service_unavailable(format!("更新external_task_id失败: {}", e))
            })?;

        Ok(())
    }

    /// 存储CFG分析结果
    async fn store_cfg_analysis_result(
        pool: &PgPool,
        sub_task_id: Uuid,
        sample_id: Uuid,
        message: Option<String>,
        result_files: Option<serde_json::Value>,
        full_report: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        let query = r#"
            INSERT INTO cfg_analysis_results (id, sub_task_id, sample_id, message, result_files, full_report, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
        "#;

        sqlx::query(query)
            .bind(Uuid::new_v4())
            .bind(sub_task_id)
            .bind(sample_id)
            .bind(message)
            .bind(result_files)
            .bind(full_report)
            .execute(pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("存储CFG分析结果失败: {}", e)))?;

        Ok(())
    }

    /// 标记任务为完成状态
    async fn mark_task_as_completed(pool: &PgPool, task_id: Uuid) -> Result<(), AppError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE sub_tasks 
            SET status = 'completed', 
                completed_at = $2, 
                updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(task_id)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("更新任务状态为完成失败: {}", e)))?;

        Ok(())
    }
}
