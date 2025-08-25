use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    error::AppError,
    handlers::sample_full::AppState,
    models::{
        CapeHealthStatus, CapeInstance, CapeInstanceStats, CreateCapeInstanceRequest, PagedResult,
        Pagination, UpdateCapeInstanceRequest,
    },
    response::ApiResponse,
};

/// CAPE实例查询参数
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CapeInstanceQueryParams {
    /// 是否只返回启用的实例
    #[serde(default)]
    pub enabled_only: bool,
    /// 健康状态筛选
    pub status: Option<String>,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: Pagination,
}

/// 获取CAPE实例列表
///
/// 获取所有CAPE实例的配置信息
#[utoipa::path(
    get,
    path = "/api/cape-instances",
    params(CapeInstanceQueryParams),
    responses(
        (status = 200, description = "成功获取CAPE实例列表", body = ApiResponse<CapeInstanceListResponse>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn list_cape_instances(
    State(app_state): State<AppState>,
    Query(params): Query<CapeInstanceQueryParams>,
) -> Result<Json<ApiResponse<PagedResult<CapeInstance>>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    let mut instances = instance_manager.get_all_instances().await;

    // 应用筛选
    if params.enabled_only {
        instances.retain(|instance| instance.enabled);
    }

    if let Some(status) = &params.status {
        instances.retain(|instance| instance.status.to_string() == *status);
    }

    // 应用分页
    let total = instances.len() as i64;
    let offset = (params.pagination.page - 1) * params.pagination.page_size;
    let page_size = params.pagination.page_size as usize;

    if offset >= instances.len() as u32 {
        instances.clear();
    } else {
        let start = offset as usize;
        let end = std::cmp::min(start + page_size, instances.len());
        instances = instances[start..end].to_vec();
    }

    let result = PagedResult::new(
        instances,
        total,
        params.pagination.page,
        params.pagination.page_size,
    );

    Ok(Json(ApiResponse::success(result)))
}

/// 获取指定CAPE实例详情
///
/// 根据实例ID获取CAPE实例的详细信息
#[utoipa::path(
    get,
    path = "/api/cape-instances/{id}",
    params(
        ("id" = Uuid, Path, description = "CAPE实例ID")
    ),
    responses(
        (status = 200, description = "成功获取CAPE实例详情", body = ApiResponse<CapeInstance>),
        (status = 404, description = "CAPE实例不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn get_cape_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CapeInstance>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    let instance = instance_manager
        .get_instance(id)
        .await
        .ok_or_else(|| AppError::not_found("CAPE实例不存在"))?;

    Ok(Json(ApiResponse::success(instance)))
}

/// 创建CAPE实例
///
/// 创建新的CAPE实例配置
#[utoipa::path(
    post,
    path = "/api/cape-instances",
    request_body = CreateCapeInstanceRequest,
    responses(
        (status = 200, description = "成功创建CAPE实例", body = ApiResponse<CapeInstance>),
        (status = 400, description = "请求参数错误", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn create_cape_instance(
    State(app_state): State<AppState>,
    Json(request): Json<CreateCapeInstanceRequest>,
) -> Result<Json<ApiResponse<CapeInstance>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    let instance = instance_manager.create_instance(request).await?;

    Ok(Json(ApiResponse::success(instance)))
}

/// 更新CAPE实例
///
/// 更新指定CAPE实例的配置信息
#[utoipa::path(
    put,
    path = "/api/cape-instances/{id}",
    params(
        ("id" = Uuid, Path, description = "CAPE实例ID")
    ),
    request_body = UpdateCapeInstanceRequest,
    responses(
        (status = 200, description = "成功更新CAPE实例", body = ApiResponse<String>),
        (status = 400, description = "请求参数错误", body = ApiResponse<String>),
        (status = 404, description = "CAPE实例不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn update_cape_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateCapeInstanceRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    instance_manager.update_instance(id, request).await?;

    Ok(Json(ApiResponse::success("CAPE实例更新成功".to_string())))
}

/// 删除CAPE实例
///
/// 删除指定的CAPE实例配置
#[utoipa::path(
    delete,
    path = "/api/cape-instances/{id}",
    params(
        ("id" = Uuid, Path, description = "CAPE实例ID")
    ),
    responses(
        (status = 200, description = "成功删除CAPE实例", body = ApiResponse<String>),
        (status = 400, description = "请求参数错误（实例正在使用中）", body = ApiResponse<String>),
        (status = 404, description = "CAPE实例不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn delete_cape_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    instance_manager.delete_instance(id).await?;

    Ok(Json(ApiResponse::success("CAPE实例删除成功".to_string())))
}

/// 测试CAPE实例健康状态
///
/// 对指定CAPE实例进行健康检查
#[utoipa::path(
    post,
    path = "/api/cape-instances/{id}/health-check",
    params(
        ("id" = Uuid, Path, description = "CAPE实例ID")
    ),
    responses(
        (status = 200, description = "健康检查完成", body = ApiResponse<CapeHealthStatus>),
        (status = 404, description = "CAPE实例不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn health_check_cape_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CapeHealthStatus>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    let health_status = instance_manager.health_check_instance(id).await?;

    Ok(Json(ApiResponse::success(health_status)))
}

/// 获取所有CAPE实例的健康状态
///
/// 获取所有CAPE实例的健康检查结果
#[utoipa::path(
    get,
    path = "/api/cape-instances/health",
    responses(
        (status = 200, description = "成功获取健康状态", body = ApiResponse<Vec<CapeHealthStatus>>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn get_all_health_status(
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<CapeHealthStatus>>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    let instances = instance_manager.get_all_instances().await;

    let mut health_statuses = Vec::new();
    for instance in instances {
        match instance_manager.health_check_instance(instance.id).await {
            Ok(status) => health_statuses.push(status),
            Err(e) => {
                tracing::warn!("实例 {} 健康检查失败: {}", instance.id, e);
                // 创建一个错误状态
                use crate::models::CapeInstanceStatus;
                use chrono::Utc;

                health_statuses.push(CapeHealthStatus {
                    instance_id: instance.id,
                    instance_name: instance.name,
                    status: CapeInstanceStatus::Unhealthy,
                    response_time_ms: None,
                    checked_at: Utc::now(),
                    error_message: Some(e.to_string()),
                });
            }
        }
    }

    Ok(Json(ApiResponse::success(health_statuses)))
}

/// 获取CAPE实例统计信息
///
/// 获取指定CAPE实例的任务执行统计
#[utoipa::path(
    get,
    path = "/api/cape-instances/{id}/stats",
    params(
        ("id" = Uuid, Path, description = "CAPE实例ID"),
        ("days" = Option<u32>, Query, description = "统计天数，默认7天")
    ),
    responses(
        (status = 200, description = "成功获取统计信息", body = ApiResponse<CapeInstanceStats>),
        (status = 404, description = "CAPE实例不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "CAPE管理"
)]
pub async fn get_cape_instance_stats(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<StatsQueryParams>,
) -> Result<Json<ApiResponse<CapeInstanceStats>>, AppError> {
    let cape_manager = app_state
        .cape_manager
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CAPE管理器不可用"))?;

    let instance_manager = cape_manager.instance_manager();
    let _instance = instance_manager
        .get_instance(id)
        .await
        .ok_or_else(|| AppError::not_found("CAPE实例不存在"))?;

    let days = params.days.unwrap_or(7);

    // 从数据库获取统计数据
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;

    let stats = get_instance_statistics(db.pool(), id, days).await?;

    Ok(Json(ApiResponse::success(CapeInstanceStats {
        instance_id: id,
        total_tasks: stats.total_tasks,
        successful_tasks: stats.successful_tasks,
        failed_tasks: stats.failed_tasks,
        average_processing_time: stats.average_processing_time,
        success_rate: if stats.total_tasks > 0 {
            stats.successful_tasks as f64 / stats.total_tasks as f64
        } else {
            0.0
        },
        period_start: stats.period_start,
        period_end: stats.period_end,
    })))
}

/// 统计查询参数
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct StatsQueryParams {
    /// 统计天数
    pub days: Option<u32>,
}

/// 内部统计数据结构
#[derive(Debug)]
struct InstanceStatistics {
    total_tasks: i64,
    successful_tasks: i64,
    failed_tasks: i64,
    average_processing_time: Option<f64>,
    period_start: chrono::DateTime<chrono::Utc>,
    period_end: chrono::DateTime<chrono::Utc>,
}

/// 从数据库获取实例统计信息
async fn get_instance_statistics(
    pool: &sqlx::PgPool,
    instance_id: Uuid,
    days: u32,
) -> Result<InstanceStatistics, AppError> {
    use chrono::{Duration, Utc};

    let period_end = Utc::now();
    let period_start = period_end - Duration::days(days as i64);

    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_tasks,
            COUNT(CASE WHEN status = 'completed' THEN 1 END) as successful_tasks,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_tasks,
            AVG(EXTRACT(EPOCH FROM (completed_at - started_at)))::float8 as avg_duration_seconds
        FROM sub_tasks 
        WHERE cape_instance_id = $1 
        AND created_at >= $2 
        AND created_at <= $3
        "#,
    )
    .bind(instance_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::service_unavailable(format!("查询统计信息失败: {}", e)))?;

    Ok(InstanceStatistics {
        total_tasks: row.get::<i64, _>("total_tasks"),
        successful_tasks: row.get::<i64, _>("successful_tasks"),
        failed_tasks: row.get::<i64, _>("failed_tasks"),
        average_processing_time: row.get::<Option<f64>, _>("avg_duration_seconds"),
        period_start,
        period_end,
    })
}
