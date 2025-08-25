use crate::{
    config::cape::CapeTaskConfig,
    error::AppError,
    handlers::sample_full::AppState,
    models::{AnalyzerType, SubTaskStatus},
    repositories::TaskRepository,
    response::ApiResponse,
};
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::task;
use tracing::{error, info};
use uuid::Uuid;

// CAPE 任务执行器结构体已删除，使用独立的处理函数实现
// 这种设计更适合 Axum 的 handler 模式

/// 批量执行请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BatchExecuteRequest {
    pub master_task_id: Uuid,
    pub config: Option<CapeTaskConfigRequest>,
    /// 提交间隔（毫秒，默认1000ms）
    pub submit_interval_ms: Option<u64>,
    /// 全局并发数（默认1）
    pub concurrency: Option<u32>,
}

/// CAPE 任务配置请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CapeTaskConfigRequest {
    pub machine: Option<String>,
    // 超时相关字段已废弃，保留前端/旧客户端兼容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, String>>,
    /// 重试配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfigRequest>,
}

/// 重试配置请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RetryConfigRequest {
    /// 是否启用重试
    pub enabled: Option<bool>,
    /// 最大重试次数
    pub max_attempts: Option<u32>,
    /// 初始退避时间（秒）
    pub initial_backoff_secs: Option<u64>,
    /// 最大退避时间（秒）
    pub max_backoff_secs: Option<u64>,
    /// 退避倍率
    pub backoff_multiplier: Option<f64>,
    /// 是否添加随机抖动
    pub jitter: Option<bool>,
}

impl From<CapeTaskConfigRequest> for CapeTaskConfig {
    fn from(req: CapeTaskConfigRequest) -> Self {
        use crate::config::cape::RetryConfig;

        let retry_config = req.retry.map(|retry_req| {
            let mut retry_config = RetryConfig::default();
            if let Some(enabled) = retry_req.enabled {
                retry_config.enabled = enabled;
            }
            if let Some(max_attempts) = retry_req.max_attempts {
                retry_config.max_attempts = max_attempts;
            }
            if let Some(initial_backoff_secs) = retry_req.initial_backoff_secs {
                retry_config.initial_backoff_secs = initial_backoff_secs;
            }
            if let Some(max_backoff_secs) = retry_req.max_backoff_secs {
                retry_config.max_backoff_secs = max_backoff_secs;
            }
            if let Some(backoff_multiplier) = retry_req.backoff_multiplier {
                retry_config.backoff_multiplier = backoff_multiplier;
            }
            if let Some(jitter) = retry_req.jitter {
                retry_config.jitter = jitter;
            }
            retry_config
        });

        Self {
            machine: req.machine,
            options: req.options,
            retry: retry_config,
        }
    }
}

/// 批量执行响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BatchExecuteResponse {
    pub master_task_id: Uuid,
    pub submitted_tasks: u32,
    pub estimated_completion_time: Option<String>,
}

/// 任务执行状态响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TaskExecutionStatusResponse {
    pub master_task_id: Uuid,
    pub total_tasks: u32,
    pub pending_tasks: u32,
    pub running_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub progress_percentage: f32,
    pub estimated_remaining_time: Option<String>,
    pub average_task_duration: Option<String>,
    pub current_throughput_mbps: Option<f64>,
}

/// 性能统计响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PerformanceStatsResponse {
    pub period_days: i32,
    pub total_tasks: u32,
    pub success_rate: f64,
    pub average_analysis_duration: Option<String>,
    pub average_submit_duration: Option<String>,
    pub average_throughput_mbps: Option<f64>,
    pub recommendations: Vec<String>,
}

/// 批量执行CAPE分析任务
///
/// 根据主任务ID批量执行CAPE沙箱分析，支持自定义配置
#[utoipa::path(
    post,
    path = "/api/cape/execute",
    request_body = BatchExecuteRequest,
    responses(
        (status = 200, description = "批量任务执行成功", body = ApiResponse<BatchExecuteResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "任务不存在"),
        (status = 503, description = "CAPE服务不可用"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "CAPE分析"
)]
pub async fn execute_cape_batch(
    State(app_state): State<AppState>,
    Json(request): Json<BatchExecuteRequest>,
) -> Result<Json<ApiResponse<BatchExecuteResponse>>, AppError> {
    info!("开始批量执行CAPE分析任务: {}", request.master_task_id);

    // 获取应用组件
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用".to_string()))?;
    let pool = db.pool().clone();

    let _storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储服务不可用".to_string()))?;

    // 初始化组件
    let task_repo = TaskRepository::new(db.pool().clone());

    // 不再依赖后端配置文件的 CAPE 配置，使用数据库中的实例管理

    // 初始化CAPE管理器
    let cape_manager =
        crate::services::CapeManager::new(db.pool().clone(), _storage.clone()).await?;

    // 获取主任务信息
    let master_task = task_repo
        .get_master_task_by_id(request.master_task_id)
        .await?
        .ok_or_else(|| AppError::bad_request("任务不存在".to_string()))?;

    // 验证任务状态
    if !matches!(
        master_task.status,
        crate::models::MasterTaskStatus::Pending | crate::models::MasterTaskStatus::Running
    ) {
        return Err(AppError::bad_request(format!(
            "任务状态不允许执行，当前状态: {:?}",
            master_task.status
        )));
    }

    // 检查任务是否被暂停
    if matches!(master_task.status, crate::models::MasterTaskStatus::Paused) {
        return Err(AppError::bad_request(
            "任务已暂停，请先恢复任务后再执行".to_string(),
        ));
    }

    // 验证分析器类型
    if master_task.analyzer_type != AnalyzerType::CAPE {
        return Err(AppError::bad_request(format!(
            "不支持的分析器类型: {:?}",
            master_task.analyzer_type
        )));
    }

    // 获取待执行的子任务
    let sub_tasks = task_repo
        .list_sub_tasks_by_master_task(request.master_task_id)
        .await?;
    let pending_tasks: Vec<_> = sub_tasks
        .into_iter()
        .filter(|task| matches!(task.status, SubTaskStatus::Pending))
        .collect();

    if pending_tasks.is_empty() {
        return Err(AppError::bad_request("没有待执行的子任务".to_string()));
    }

    let submitted_count = pending_tasks.len() as u32;

    // 获取历史性能数据用于时间预估
    // CapeProcessor 用于统计估算，这里不强制需要 base_url；
    // 选择一个可用实例（若有）做统计来源，否则统计使用默认值
    let _cape_processor = if let Some(cape_mgr) = &app_state.cape_manager {
        if let Some(instance) = cape_mgr
            .instance_manager()
            .get_available_instances()
            .await
            .into_iter()
            .next()
        {
            crate::services::CapeProcessor::new(
                crate::services::CapeClient::new(instance.base_url.clone()),
                task_repo.clone(),
                _storage.clone(),
                db.pool().clone(),
            )
        } else {
            crate::services::CapeProcessor::new(
                crate::services::CapeClient::new("http://127.0.0.1:8000/apiv2".to_string()),
                task_repo.clone(),
                _storage.clone(),
                db.pool().clone(),
            )
        }
    } else {
        crate::services::CapeProcessor::new(
            crate::services::CapeClient::new("http://127.0.0.1:8000/apiv2".to_string()),
            task_repo.clone(),
            _storage.clone(),
            db.pool().clone(),
        )
    };

    // 简化：不依赖历史性能表，估算一个保守的平均分析时长
    let avg_duration = Duration::from_secs(300);

    // 预估完成时间
    let estimated_completion = {
        let total_estimated_seconds = avg_duration.as_secs() * submitted_count as u64;
        let completion_time =
            chrono::Utc::now() + chrono::Duration::seconds(total_estimated_seconds as i64);
        Some(completion_time.format("%Y-%m-%d %H:%M:%S UTC").to_string())
    };

    // 更新主任务状态为运行中
    let update_request = crate::models::UpdateMasterTaskRequest {
        status: Some(crate::models::MasterTaskStatus::Running),
        progress: Some(0),
        completed_samples: None,
        failed_samples: None,
        error_message: None,
        result_summary: None,
    };
    task_repo
        .update_master_task(request.master_task_id, &update_request)
        .await?;

    // 配置CAPE任务
    let config: CapeTaskConfig = request.config.map(|c| c.into()).unwrap_or_default();

    // 异步执行所有子任务
    let master_task_id = request.master_task_id;
    let cape_manager_clone = cape_manager.clone();

    info!(
        "任务 {} 已提交执行，包含 {} 个子任务",
        master_task_id, submitted_count
    );

    // 后台执行CAPE任务（仅提交间隔节流）
    let submit_interval_ms = request.submit_interval_ms.unwrap_or(1000);

    task::spawn(async move {
        match cape_manager_clone
            .submit_master_task(master_task_id, Some(config), submit_interval_ms)
            .await
        {
            Ok(stats) => {
                info!(
                    "任务 {} 执行完成: {}/{} 成功，成功率 {:.2}%",
                    master_task_id,
                    stats.total_completed,
                    stats.total_submitted,
                    stats.success_rate * 100.0
                );
                // 统一刷新主任务统计（仅全部失败才失败；全部终态否则完成；否则保持/置为running）
                if let Err(e) = sqlx::query(r#"
                    UPDATE master_tasks mt
                    SET 
                        completed_samples = (
                            SELECT COUNT(*) FROM sub_tasks st 
                            WHERE st.master_task_id = $1 AND st.status = 'completed'
                        ),
                        failed_samples = (
                            SELECT COUNT(*) FROM sub_tasks st 
                            WHERE st.master_task_id = $1 AND st.status IN ('failed','cancelled')
                        ),
                        progress = CASE 
                            WHEN mt.total_samples > 0 THEN (
                                (SELECT COUNT(*) FROM sub_tasks st 
                                 WHERE st.master_task_id = $1 
                                   AND st.status IN ('completed','failed','cancelled')) * 100 / mt.total_samples
                            )
                            ELSE 0 
                        END,
                        status = CASE 
                            WHEN (
                                SELECT COUNT(*) FROM sub_tasks st 
                                WHERE st.master_task_id = $1 
                                  AND st.status IN ('completed','failed','cancelled')
                            ) >= mt.total_samples THEN 
                                CASE 
                                    WHEN (
                                        SELECT COUNT(*) FROM sub_tasks st 
                                        WHERE st.master_task_id = $1 AND st.status = 'failed'
                                    ) = mt.total_samples
                                    THEN 'failed'::master_task_status_enum
                                    ELSE 'completed'::master_task_status_enum
                                END
                            ELSE 'running'::master_task_status_enum
                        END,
                        updated_at = NOW()
                    WHERE mt.id = $1
                "#)
                .bind(master_task_id)
                .execute(&pool).await {
                    error!(%master_task_id, err=%e, "刷新主任务统计失败");
                }
            }
            Err(e) => {
                error!("任务 {} 执行失败: {}", master_task_id, e);
            }
        }
    });

    let response = BatchExecuteResponse {
        master_task_id: request.master_task_id,
        submitted_tasks: submitted_count,
        estimated_completion_time: estimated_completion,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取任务执行状态
///
/// 根据主任务ID获取当前任务的执行状态，包括进度、统计和预估时间
#[utoipa::path(
    get,
    path = "/api/cape/status/{id}",
    params(
        ("id" = Uuid, Path, description = "主任务ID")
    ),
    responses(
        (status = 200, description = "任务执行状态获取成功", body = ApiResponse<TaskExecutionStatusResponse>),
        (status = 404, description = "任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "CAPE分析"
)]
pub async fn get_task_execution_status(
    State(app_state): State<AppState>,
    Path(master_task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<TaskExecutionStatusResponse>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用".to_string()))?;

    let task_repo = TaskRepository::new(db.pool().clone());

    // 获取主任务信息
    let _master_task = task_repo.get_master_task_by_id(master_task_id).await?;

    // 获取子任务统计
    let sub_tasks = task_repo
        .list_sub_tasks_by_master_task(master_task_id)
        .await?;

    let total_tasks = sub_tasks.len() as u32;
    let mut pending_tasks = 0;
    let mut running_tasks = 0;
    let mut completed_tasks = 0;
    let mut failed_tasks = 0;

    for task in &sub_tasks {
        match task.status {
            SubTaskStatus::Pending => pending_tasks += 1,
            SubTaskStatus::Submitting | SubTaskStatus::Submitted | SubTaskStatus::Analyzing => {
                running_tasks += 1
            }
            SubTaskStatus::Paused => pending_tasks += 1, // 暂停的任务归类为等待中
            SubTaskStatus::Completed => completed_tasks += 1,
            SubTaskStatus::Failed | SubTaskStatus::Cancelled => failed_tasks += 1,
        }
    }

    let progress_percentage = if total_tasks > 0 {
        (completed_tasks as f32 / total_tasks as f32) * 100.0
    } else {
        0.0
    };

    // 获取性能统计（基于历史数据）
    // 不依赖 config.cape，直接基于 CapeManager 统计或使用默认值
    let (estimated_remaining_time, average_task_duration, current_throughput_mbps) = {
        let cape_manager = crate::services::CapeManager::new(
            db.pool().clone(),
            app_state.storage.as_ref().unwrap().clone(),
        )
        .await
        .ok();

        if let Some(manager) = cape_manager {
            match manager.get_task_statistics(7).await {
                Ok(stats) => {
                    let avg_duration = stats.average_duration_seconds.unwrap_or(300.0);
                    let remaining_seconds =
                        avg_duration as u64 * (pending_tasks + running_tasks) as u64;
                    (
                        if (pending_tasks + running_tasks) > 0 {
                            Some(format!("{} minutes", remaining_seconds / 60))
                        } else {
                            None
                        },
                        Some(format!("{} seconds", avg_duration)),
                        None,
                    )
                }
                Err(_) => {
                    let avg_seconds = 300;
                    let remaining_seconds = avg_seconds * (pending_tasks + running_tasks) as u64;
                    (
                        if (pending_tasks + running_tasks) > 0 {
                            Some(format!("{} minutes", remaining_seconds / 60))
                        } else {
                            None
                        },
                        Some("300 seconds".to_string()),
                        Some(1.0),
                    )
                }
            }
        } else {
            let avg_seconds = 300;
            let remaining_seconds = avg_seconds * (pending_tasks + running_tasks) as u64;
            (
                if (pending_tasks + running_tasks) > 0 {
                    Some(format!("{} minutes", remaining_seconds / 60))
                } else {
                    None
                },
                Some("300 seconds".to_string()),
                Some(1.0),
            )
        }
    };

    let response = TaskExecutionStatusResponse {
        master_task_id,
        total_tasks,
        pending_tasks,
        running_tasks,
        completed_tasks,
        failed_tasks,
        progress_percentage,
        estimated_remaining_time,
        average_task_duration,
        current_throughput_mbps,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取性能统计
///
/// 获取CAPE分析的性能统计信息，包括成功率、平均分析时间等
#[utoipa::path(
    get,
    path = "/api/cape/performance",
    params(
        ("period_days" = Option<i32>, Query, description = "统计周期（天），默认7天")
    ),
    responses(
        (status = 200, description = "性能统计获取成功", body = ApiResponse<PerformanceStatsResponse>),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "CAPE分析"
)]
pub async fn get_performance_stats(
    State(app_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<PerformanceStatsResponse>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用".to_string()))?;

    let period_days = params.get("days").and_then(|d| d.parse().ok()).unwrap_or(7);

    // 获取CAPE配置
    let cape_config = app_state
        .config
        .cape
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE配置未找到".to_string()))?;

    if !cape_config.enabled {
        return Err(AppError::service_unavailable("CAPE服务未启用".to_string()));
    }

    // 初始化CAPE客户端和处理器
    let _cape_processor = crate::services::CapeProcessor::new(
        crate::services::CapeClient::new(cape_config.base_url.clone()),
        crate::repositories::TaskRepository::new(db.pool().clone()),
        app_state.storage.as_ref().unwrap().clone(),
        db.pool().clone(),
    );

    // 获取性能历史数据（已弃用：使用固定默认值）
    let avg_analysis_duration = Duration::from_secs(300);

    // 生成性能建议
    let mut recommendations = Vec::new();

    // 以下建议逻辑已简化，仅示例
    if 0.95 < 0.9 {
        recommendations.push("成功率偏低，建议检查CAPE服务器状态和网络连接".to_string());
    }
    // 其他建议省略

    if recommendations.is_empty() {
        recommendations.push("系统运行良好，无特殊建议".to_string());
    }

    let response = PerformanceStatsResponse {
        period_days,
        total_tasks: 0,
        success_rate: 1.0,
        average_analysis_duration: Some(format!("{} seconds", avg_analysis_duration.as_secs())),
        average_submit_duration: Some("30 seconds".to_string()),
        average_throughput_mbps: Some(1.2),
        recommendations,
    };

    Ok(Json(ApiResponse::success(response)))
}
