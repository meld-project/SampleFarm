use crate::{
    error::AppError,
    handlers::sample_full::AppState,
    models::{
        AnalyzerType, CreateMasterTaskRequest, CreateTaskByFilterRequest, MasterTask, PagedResult,
        Pagination, SubTask, SubTaskFilter, SubTaskStatus, SubTaskWithSample, TaskFilter,
        UpdateMasterTaskRequest, UpdateSubTaskStatusRequest,
    },
    repositories::TaskRepository,
    response::ApiResponse,
};
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// 创建任务的响应
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateTaskResponse {
    pub master_task: MasterTask,
    pub sub_tasks_count: usize,
    pub message: String,
}

/// 任务查询参数
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TaskQueryParams {
    // 分页参数
    pub page: Option<u32>,
    pub page_size: Option<u32>,

    // 过滤参数
    pub analyzer_type: Option<AnalyzerType>,
    pub status: Option<String>,     // 使用字符串，后续转换
    pub start_time: Option<String>, // ISO 8601格式
    pub end_time: Option<String>,
}

/// 子任务查询参数
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SubTaskQueryParams {
    // 分页参数
    pub page: Option<u32>,
    pub page_size: Option<u32>,

    // 过滤参数
    pub master_task_id: Option<Uuid>,
    pub sample_id: Option<Uuid>,
    pub analyzer_type: Option<AnalyzerType>,
    pub status: Option<String>,
    pub keyword: Option<String>, // 新增关键词搜索
}

/// 任务状态计数
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Default)]
pub struct TaskStatusCounts {
    pub pending: i64,
    pub submitting: i64,
    pub submitted: i64,
    pub analyzing: i64,
    pub paused: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
}

/// 任务运行时状态响应
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskRuntimeStatus {
    pub master_task_id: Uuid,
    pub total: i64,
    pub counts: TaskStatusCounts,
    pub progress_percentage: f32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
}

/// 暂停任务请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PauseTaskRequest {
    /// 暂停模式（当前版本只支持soft）
    pub mode: Option<String>,
    /// 暂停原因
    pub reason: Option<String>,
}

/// 创建任务处理器
///
/// 根据提供的样本ID列表和分析器类型创建新的分析任务
#[utoipa::path(
    post,
    path = "/api/tasks",
    request_body = CreateMasterTaskRequest,
    responses(
        (status = 201, description = "任务创建成功", body = ApiResponse<CreateTaskResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn create_task(
    State(app_state): State<AppState>,
    Json(request): Json<CreateMasterTaskRequest>,
) -> Result<Json<ApiResponse<CreateTaskResponse>>, AppError> {
    // 检查数据库连接
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    // 验证分析器是否可用
    if !request.analyzer_type.is_enabled() {
        return Err(AppError::bad_request(format!(
            "分析器 {} 当前不可用",
            request.analyzer_type
        )));
    }

    // 验证样本ID列表不为空
    if request.sample_ids.is_empty() {
        return Err(AppError::bad_request("样本ID列表不能为空"));
    }

    // 创建任务存储库
    let task_repo = TaskRepository::new(db.pool().clone());

    // 创建主任务
    let master_task = task_repo.create_master_task(&request).await?;

    // 创建子任务
    // 兼容：若仅提供单个 cape_instance_id，则转为单元素列表
    let cape_ids = if let Some(list) = request.cape_instance_ids.clone() {
        Some(list)
    } else if let Some(id) = request.cape_instance_id {
        Some(vec![id])
    } else {
        None
    };

    let sub_tasks = task_repo
        .create_sub_tasks(
            master_task.id,
            &request.sample_ids,
            request.analyzer_type,
            cape_ids,
            request.cfg_instance_ids.clone(),
            None, // 默认优先级
            request.parameters.clone(),
        )
        .await?;

    // 自动执行任务（如果是CAPE分析器）
    if request.analyzer_type == crate::models::AnalyzerType::CAPE {
        // 获取CAPE管理器
        if let Some(cape_manager) = &app_state.cape_manager {
            let master_task_id = master_task.id;
            let cape_manager_clone = cape_manager.clone();

            // 异步执行任务
            tokio::spawn(async move {
                tracing::info!("开始执行CAPE任务: {}", master_task_id);
                match cape_manager_clone
                    .submit_master_task(master_task_id, None, 1000)
                    .await
                {
                    Ok(stats) => {
                        tracing::info!(
                            "任务 {} 执行完成: {}/{} 成功，成功率 {:.2}%",
                            master_task_id,
                            stats.total_completed,
                            stats.total_submitted,
                            stats.success_rate * 100.0
                        );
                    }
                    Err(e) => {
                        tracing::error!("任务 {} 执行失败: {}", master_task_id, e);
                    }
                }
            });
        } else {
            tracing::warn!("CAPE管理器不可用，任务 {} 将保持待执行状态", master_task.id);
        }
    }

    let response = CreateTaskResponse {
        master_task,
        sub_tasks_count: sub_tasks.len(),
        message: format!(
            "成功创建任务，包含 {} 个样本，正在开始执行",
            sub_tasks.len()
        ),
    };

    tracing::info!(
        "创建任务成功: {} ({}个样本, 分析器: {:?})",
        request.task_name,
        sub_tasks.len(),
        request.analyzer_type
    );

    Ok(Json(ApiResponse::success(response)))
}

/// 按筛选条件创建任务处理器
///
/// 根据提供的样本筛选条件在数据库侧直接生成子任务队列，避免前端拉取所有 sample_ids
#[utoipa::path(
    post,
    path = "/api/tasks/by-filter",
    request_body = CreateTaskByFilterRequest,
    responses(
        (status = 201, description = "任务创建成功", body = ApiResponse<CreateTaskResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn create_task_by_filter(
    State(app_state): State<AppState>,
    Json(request): Json<CreateTaskByFilterRequest>,
) -> Result<Json<ApiResponse<CreateTaskResponse>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    if !request.analyzer_type.is_enabled() {
        return Err(AppError::bad_request(format!(
            "分析器 {} 当前不可用",
            request.analyzer_type
        )));
    }

    let task_repo = TaskRepository::new(db.pool().clone());
    let (master_task, sub_tasks) = task_repo.create_master_task_by_filter(&request).await?;

    // 可选：如需自动执行CAPE
    if request.analyzer_type == crate::models::AnalyzerType::CAPE {
        if let Some(cape_manager) = &app_state.cape_manager {
            let master_task_id = master_task.id;
            let cape_manager_clone = cape_manager.clone();
            tokio::spawn(async move {
                tracing::info!("开始执行CAPE任务: {}", master_task_id);
                if let Err(e) = cape_manager_clone
                    .submit_master_task(master_task_id, None, 1000)
                    .await
                {
                    tracing::error!("任务 {} 执行失败: {}", master_task_id, e);
                }
            });
        } else {
            tracing::warn!("CAPE管理器不可用，任务 {} 将保持待执行状态", master_task.id);
        }
    }

    let response = CreateTaskResponse {
        master_task,
        sub_tasks_count: sub_tasks.len(),
        message: format!("成功创建任务，包含 {} 个样本", sub_tasks.len()),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取任务列表
///
/// 根据查询条件获取任务列表，支持分页和筛选
#[utoipa::path(
    get,
    path = "/api/tasks",
    params(TaskQueryParams),
    responses(
        (status = 200, description = "任务列表查询成功", body = ApiResponse<PagedResult<MasterTask>>),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn list_tasks(
    State(app_state): State<AppState>,
    Query(params): Query<TaskQueryParams>,
) -> Result<Json<ApiResponse<PagedResult<MasterTask>>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    // 构建分页参数
    let pagination = Pagination {
        page: params.page.unwrap_or(1).max(1),
        page_size: params.page_size.unwrap_or(20).clamp(1, 100),
    };

    // 构建过滤条件
    let filter = TaskFilter {
        analyzer_type: params.analyzer_type,
        task_type: None,  // 暂时不过滤任务类型
        status: None,     // 状态过滤需要额外处理
        start_time: None, // 时间过滤需要解析
        end_time: None,
    };

    let task_repo = TaskRepository::new(db.pool().clone());
    let result = task_repo.list_master_tasks(&filter, &pagination).await?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取任务详情
///
/// 根据任务ID获取指定任务的详细信息
#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    params(
        ("id" = Uuid, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "任务详情获取成功", body = ApiResponse<MasterTask>),
        (status = 404, description = "任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn get_task(
    State(app_state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<MasterTask>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    let task_repo = TaskRepository::new(db.pool().clone());
    let task = task_repo
        .get_master_task_by_id(task_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("任务 {} 不存在", task_id)))?;

    Ok(Json(ApiResponse::success(task)))
}

/// 更新任务状态
///
/// 根据任务ID更新任务状态和相关信息
#[utoipa::path(
    put,
    path = "/api/tasks/{id}",
    params(
        ("id" = Uuid, Path, description = "任务ID")
    ),
    request_body = UpdateMasterTaskRequest,
    responses(
        (status = 200, description = "任务状态更新成功", body = ApiResponse<MasterTask>),
        (status = 404, description = "任务不存在"),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn update_task(
    State(app_state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(request): Json<UpdateMasterTaskRequest>,
) -> Result<Json<ApiResponse<MasterTask>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    let task_repo = TaskRepository::new(db.pool().clone());

    // 检查任务是否存在
    let _existing_task = task_repo
        .get_master_task_by_id(task_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("任务 {} 不存在", task_id)))?;

    // 更新任务
    let updated_task = task_repo.update_master_task(task_id, &request).await?;

    tracing::info!("任务状态更新成功: {}", task_id);

    Ok(Json(ApiResponse::success(updated_task)))
}

/// 删除任务
///
/// 根据任务ID删除指定任务，会级联删除所有相关的子任务
#[utoipa::path(
    delete,
    path = "/api/tasks/{id}",
    params(
        ("id" = Uuid, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "任务删除成功", body = ApiResponse<String>),
        (status = 404, description = "任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn delete_task(
    State(app_state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    let task_repo = TaskRepository::new(db.pool().clone());

    // 检查任务是否存在
    let _existing_task = task_repo
        .get_master_task_by_id(task_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("任务 {} 不存在", task_id)))?;

    // 删除任务（级联删除子任务）
    task_repo.delete_master_task(task_id).await?;

    tracing::info!("任务删除成功: {}", task_id);

    Ok(Json(ApiResponse::success(format!(
        "任务 {} 删除成功",
        task_id
    ))))
}

/// 获取子任务列表
///
/// 根据主任务ID获取所有相关的子任务列表，支持分页和筛选
#[utoipa::path(
    get,
    path = "/api/tasks/{id}/sub-tasks",
    params(
        ("id" = Uuid, Path, description = "主任务ID")
    ),
    responses(
        (status = 200, description = "子任务列表查询成功", body = ApiResponse<PagedResult<SubTask>>),
        (status = 404, description = "主任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn list_sub_tasks(
    State(app_state): State<AppState>,
    Path(master_task_id): Path<Uuid>,
    Query(params): Query<SubTaskQueryParams>,
) -> Result<Json<ApiResponse<PagedResult<SubTaskWithSample>>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    // 构建分页参数
    let pagination = Pagination {
        page: params.page.unwrap_or(1).max(1),
        page_size: params.page_size.unwrap_or(20).clamp(1, 100),
    };

    // 解析状态字符串为枚举
    let status = params.status.as_ref().and_then(|s| match s.as_str() {
        "pending" => Some(SubTaskStatus::Pending),
        "submitting" => Some(SubTaskStatus::Submitting),
        "submitted" => Some(SubTaskStatus::Submitted),
        "analyzing" => Some(SubTaskStatus::Analyzing),
        "completed" => Some(SubTaskStatus::Completed),
        "failed" => Some(SubTaskStatus::Failed),
        "cancelled" => Some(SubTaskStatus::Cancelled),
        _ => None,
    });

    // 构建过滤条件
    let filter = SubTaskFilter {
        master_task_id: Some(master_task_id),
        sample_id: params.sample_id,
        analyzer_type: params.analyzer_type,
        status,
        start_time: None,
        end_time: None,
    };

    let task_repo = TaskRepository::new(db.pool().clone());

    // 如果有关键词搜索，需要使用特殊的方法
    let result = if params.keyword.is_some() {
        task_repo
            .list_sub_tasks_with_sample_and_keyword(filter, pagination, params.keyword)
            .await?
    } else {
        task_repo
            .list_sub_tasks_with_sample(filter, pagination)
            .await?
    };

    Ok(Json(ApiResponse::success(result)))
}

/// 更新子任务状态
///
/// 根据子任务ID更新子任务的状态和相关信息
#[utoipa::path(
    put,
    path = "/api/sub-tasks/{id}",
    params(
        ("id" = Uuid, Path, description = "子任务ID")
    ),
    request_body = UpdateSubTaskStatusRequest,
    responses(
        (status = 200, description = "子任务状态更新成功", body = ApiResponse<SubTask>),
        (status = 404, description = "子任务不存在"),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn update_sub_task_status(
    State(app_state): State<AppState>,
    Path(sub_task_id): Path<Uuid>,
    Json(request): Json<UpdateSubTaskStatusRequest>,
) -> Result<Json<ApiResponse<SubTask>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    let task_repo = TaskRepository::new(db.pool().clone());
    let updated_sub_task = task_repo
        .update_sub_task_status(sub_task_id, &request)
        .await?;

    tracing::info!("子任务状态更新成功: {}", sub_task_id);

    Ok(Json(ApiResponse::success(updated_sub_task)))
}

/// 获取任务统计信息
///
/// 获取系统中所有任务的统计信息，包括各种状态的任务数量和样本统计
#[utoipa::path(
    get,
    path = "/api/tasks/stats",
    responses(
        (status = 200, description = "任务统计信息获取成功", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn get_task_stats(
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    // 查询任务统计
    let stats_query = r#"
        SELECT 
            COUNT(*) as total_tasks,
            COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_tasks,
            COUNT(CASE WHEN status = 'running' THEN 1 END) as running_tasks,
            COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_tasks,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_tasks,
            SUM(total_samples) as total_samples,
            SUM(completed_samples) as completed_samples,
            SUM(failed_samples) as failed_samples
        FROM master_tasks
    "#;

    let row = sqlx::query(stats_query)
        .fetch_one(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询任务统计失败: {}", e)))?;

    let stats = serde_json::json!({
        "total_tasks": row.get::<i64, _>("total_tasks"),
        "pending_tasks": row.get::<i64, _>("pending_tasks"),
        "running_tasks": row.get::<i64, _>("running_tasks"),
        "completed_tasks": row.get::<i64, _>("completed_tasks"),
        "failed_tasks": row.get::<i64, _>("failed_tasks"),
        "total_samples": row.get::<Option<i64>, _>("total_samples").unwrap_or(0),
        "completed_samples": row.get::<Option<i64>, _>("completed_samples").unwrap_or(0),
        "failed_samples": row.get::<Option<i64>, _>("failed_samples").unwrap_or(0),
    });

    Ok(Json(ApiResponse::success(stats)))
}

/// 获取任务运行时状态统计
///
/// 根据任务ID实时统计子任务状态分布、进度、运行时间等信息
#[utoipa::path(
    get,
    path = "/api/tasks/{id}/status",
    params(
        ("id" = Uuid, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "任务状态统计获取成功", body = ApiResponse<TaskRuntimeStatus>),
        (status = 404, description = "任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn get_task_runtime_status(
    State(app_state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<TaskRuntimeStatus>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    // 首先检查任务是否存在
    let task_repo = TaskRepository::new(db.pool().clone());
    let _master_task = task_repo
        .get_master_task_by_id(task_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("任务 {} 不存在", task_id)))?;

    // 查询子任务状态统计
    let stats_query = r#"
        SELECT 
            status::text as status,
            COUNT(*) as count
        FROM sub_tasks 
        WHERE master_task_id = $1 
        GROUP BY status
    "#;

    let stat_rows = sqlx::query(stats_query)
        .bind(task_id)
        .fetch_all(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询子任务状态统计失败: {}", e)))?;

    // 构建状态计数
    let mut counts = TaskStatusCounts::default();
    let mut total = 0i64;

    for row in stat_rows {
        let status: String = row.get("status");
        let count: i64 = row.get("count");
        total += count;

        match status.as_str() {
            "pending" => counts.pending = count,
            "submitting" => counts.submitting = count,
            "submitted" => counts.submitted = count,
            "analyzing" => counts.analyzing = count,
            "paused" => counts.paused = count,
            "completed" => counts.completed = count,
            "failed" => counts.failed = count,
            "cancelled" => counts.cancelled = count,
            _ => {} // 忽略未知状态
        }
    }

    // 计算进度百分比
    let finished_count = counts.completed + counts.failed + counts.cancelled;
    let progress_percentage = if total > 0 {
        (finished_count as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    // 查询运行时间信息
    let time_query = r#"
        SELECT 
            MIN(started_at) as earliest_start,
            CASE 
                WHEN COUNT(*) = COUNT(CASE WHEN status IN ('completed', 'failed', 'cancelled') THEN 1 END)
                THEN MAX(completed_at)
                ELSE NULL
            END as latest_completion,
            COUNT(*) as total_tasks,
            COUNT(CASE WHEN status IN ('completed', 'failed', 'cancelled') THEN 1 END) as finished_tasks
        FROM sub_tasks 
        WHERE master_task_id = $1
    "#;

    let time_row = sqlx::query(time_query)
        .bind(task_id)
        .fetch_one(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询任务时间信息失败: {}", e)))?;

    let started_at: Option<DateTime<Utc>> = time_row.get("earliest_start");
    let completed_at: Option<DateTime<Utc>> = time_row.get("latest_completion");

    // 计算运行时间（秒）
    let duration_seconds = if let Some(start) = started_at {
        let end_time = completed_at.unwrap_or_else(|| Utc::now());
        Some((end_time - start).num_seconds())
    } else {
        None
    };

    let status = TaskRuntimeStatus {
        master_task_id: task_id,
        total,
        counts,
        progress_percentage,
        started_at,
        completed_at,
        duration_seconds,
    };

    Ok(Json(ApiResponse::success(status)))
}

/// 暂停任务处理器
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/pause",
    params(
        ("id" = Uuid, Path, description = "任务ID")
    ),
    request_body = PauseTaskRequest,
    responses(
        (status = 200, description = "任务暂停成功", body = ApiResponse<MasterTask>),
        (status = 400, description = "任务状态不允许暂停"),
        (status = 404, description = "任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn pause_task(
    State(app_state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(request): Json<PauseTaskRequest>,
) -> Result<Json<ApiResponse<MasterTask>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    let task_repo = TaskRepository::new(db.pool().clone());

    // 暂停主任务
    let master_task = task_repo.pause_master_task(task_id, request.reason).await?;

    // 暂停待调度的子任务
    let paused_sub_tasks_count = task_repo.pause_pending_sub_tasks(task_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        master_task,
        format!("任务已暂停，影响 {} 个子任务", paused_sub_tasks_count),
    )))
}

/// 恢复任务处理器
#[utoipa::path(
    post,
    path = "/api/tasks/{id}/resume",
    params(
        ("id" = Uuid, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "任务恢复成功", body = ApiResponse<MasterTask>),
        (status = 409, description = "任务未处于暂停状态"),
        (status = 404, description = "任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn resume_task(
    State(app_state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<MasterTask>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;

    let task_repo = TaskRepository::new(db.pool().clone());

    // 恢复主任务
    let master_task = task_repo.resume_master_task(task_id).await?;

    // 恢复暂停的子任务
    let resumed_sub_tasks_count = task_repo.resume_paused_sub_tasks(task_id).await?;

    // 后台自动触发提交（针对 CAPE 和 CFG 任务），避免用户还要再点一次“执行”
    if master_task.analyzer_type == AnalyzerType::CAPE {
        let pool = db.pool().clone();
        let storage_opt = app_state.storage.clone();
        let master_task_id = task_id;

        tokio::spawn(async move {
            match storage_opt {
                Some(storage) => {
                    match crate::services::CapeManager::new(pool.clone(), storage.clone()).await {
                        Ok(cape_manager) => {
                            // 提交间隔使用 1000ms 的默认节流，配置如需可后续扩展
                            if let Err(e) = cape_manager
                                .submit_master_task(master_task_id, None, 1000)
                                .await
                            {
                                tracing::warn!("恢复后自动提交CAPE任务失败: {}", e);
                            } else {
                                tracing::info!(
                                    "已在恢复后自动触发CAPE任务提交: {}",
                                    master_task_id
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!("初始化CAPE管理器失败，无法自动提交: {}", e);
                        }
                    }
                }
                None => {
                    tracing::warn!("存储服务不可用，无法在恢复后自动提交CAPE任务");
                }
            }
        });
    }

    if master_task.analyzer_type == AnalyzerType::CFG {
        let database = db.clone();
        let storage_opt = app_state.storage.clone();
        let cfg_opt = app_state.config.cfg.clone();
        let master_task_id = task_id;

        tokio::spawn(async move {
            match (storage_opt, cfg_opt) {
                (Some(storage), Some(cfg_conf)) => {
                    // 初始化实例管理器与处理器
                    match crate::services::CfgInstanceManager::new(database.pool().clone()).await {
                        Ok(cfg_manager) => {
                            let processor =
                                std::sync::Arc::new(crate::services::CfgProcessor::new(
                                    std::sync::Arc::new(cfg_manager),
                                    std::sync::Arc::new(database.clone()),
                                    std::sync::Arc::new(storage),
                                    cfg_conf.result_bucket.clone(),
                                    // 传入任意占位字符串，process_sub_task_internal 未使用该字段
                                    "samples".to_string(),
                                ));

                            // 查询当前 master 下的 pending 子任务
                            let rows = sqlx::query(
                                r#"
                                SELECT st.id as sub_task_id, st.sample_id, st.cfg_instance_id, s.file_hash_sha256
                                FROM sub_tasks st
                                JOIN samples s ON st.sample_id = s.id
                                WHERE st.master_task_id = $1 AND st.analyzer_type = 'CFG' AND st.status = 'pending'
                                ORDER BY st.created_at ASC
                                "#
                            )
                            .bind(master_task_id)
                            .fetch_all(database.pool()).await;

                            match rows {
                                Ok(rows) if !rows.is_empty() => {
                                    // 为每个 pending 子任务触发处理
                                    for row in &rows {
                                        let sub_task_id: uuid::Uuid = row.get("sub_task_id");
                                        let sample_id: uuid::Uuid = row.get("sample_id");
                                        let cfg_instance_id: Option<uuid::Uuid> =
                                            row.get("cfg_instance_id");
                                        let sha256: Option<String> = row.get("file_hash_sha256");
                                        let sha256 = sha256.unwrap_or_default();

                                        // 标记提交中并设置 external_task_id = sha256
                                        let _ = sqlx::query(
                                            "UPDATE sub_tasks SET status = 'submitting', external_task_id = $1, started_at = NOW() WHERE id = $2"
                                        )
                                        .bind(&sha256)
                                        .bind(sub_task_id)
                                        .execute(database.pool()).await;

                                        // 构造任务配置（使用CFG默认配置）
                                        #[allow(deprecated)]
                                        let cfg_task = crate::config::cfg::CfgTaskConfig {
                                            poll_interval_secs: cfg_conf.default_poll_interval_secs,
                                            max_wait_secs: cfg_conf.default_max_wait_secs, // 保留用于兼容性，实际不使用
                                            label: cfg_conf.default_label,
                                            retry: None,
                                        };

                                        let processor = processor.clone();
                                        tokio::spawn(async move {
                                            let _ = processor
                                                .process_sub_task(
                                                    sample_id,
                                                    &sha256,
                                                    Some(cfg_task),
                                                    cfg_instance_id,
                                                )
                                                .await;
                                        });
                                    }
                                    tracing::info!(
                                        "CFG 任务恢复后已自动触发 {} 个子任务提交",
                                        rows.len()
                                    );
                                }
                                Ok(_) => {
                                    tracing::info!("CFG 任务恢复：无 pending 子任务可提交");
                                }
                                Err(e) => {
                                    tracing::warn!("查询CFG pending子任务失败: {}", e);
                                }
                            }
                        }
                        Err(e) => tracing::warn!("初始化CFG实例管理器失败: {}", e),
                    }
                }
                _ => tracing::warn!("CFG配置或存储不可用，无法在恢复后自动触发CFG执行"),
            }
        });
    }

    Ok(Json(ApiResponse::success_with_message(
        master_task,
        format!(
            "任务已恢复，影响 {} 个子任务，已自动触发执行",
            resumed_sub_tasks_count
        ),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_task_validation() {
        // 测试创建任务的验证逻辑
        let _request = CreateMasterTaskRequest {
            task_name: "Test Task".to_string(),
            analyzer_type: AnalyzerType::CAPE,
            task_type: crate::models::TaskType::Batch,
            sample_ids: vec![], // 空的样本ID列表应该返回错误
            cape_instance_id: None,
            cape_instance_ids: None,
            cfg_instance_ids: None,
            parameters: None,
        };

        // 这里需要模拟AppState，实际测试时需要设置测试环境
        // 暂时跳过实际的处理器测试
    }
}
