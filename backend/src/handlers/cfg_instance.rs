use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    config::cfg::CfgConfig,
    error::AppError,
    handlers::sample_full::AppState,
    models::{
        CfgHealthStatus, CfgInstance, CfgInstanceStatus, CreateCfgInstanceRequest, PagedResult,
        Pagination, UpdateCfgInstanceRequest,
    },
    response::ApiResponse,
    services::cfg_client::CfgClient,
};

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CfgInstanceQueryParams {
    #[serde(default)]
    pub enabled_only: bool,
    pub status: Option<String>,
    #[serde(flatten)]
    pub pagination: Pagination,
}

#[utoipa::path(
    get,
    path = "/api/cfg-instances",
    params(CfgInstanceQueryParams),
    responses(
        (status = 200, description = "成功获取CFG实例列表", body = ApiResponse<PagedResult<CfgInstance>>),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "CFG管理"
)]
pub async fn list_cfg_instances(
    State(app_state): State<AppState>,
    Query(params): Query<CfgInstanceQueryParams>,
) -> Result<Json<ApiResponse<PagedResult<CfgInstance>>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;

    let mut conditions: Vec<String> = Vec::new();
    if params.enabled_only {
        conditions.push("enabled = true".to_string());
    }
    if let Some(status) = &params.status {
        conditions.push(format!("status = '{}'", status));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*) as count FROM cfg_instances{}",
        where_clause
    );
    let count_row = sqlx::query(&count_sql)
        .fetch_one(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询实例数量失败: {}", e)))?;
    let total: i64 = count_row.get("count");

    let offset = (params.pagination.page - 1) * params.pagination.page_size;
    let data_sql = format!(
        "SELECT * FROM cfg_instances{} ORDER BY created_at DESC LIMIT {} OFFSET {}",
        where_clause, params.pagination.page_size, offset
    );
    let instances: Vec<CfgInstance> = sqlx::query_as::<_, CfgInstance>(&data_sql)
        .fetch_all(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询实例列表失败: {}", e)))?;

    Ok(Json(ApiResponse::success(PagedResult::new(
        instances,
        total,
        params.pagination.page,
        params.pagination.page_size,
    ))))
}

#[utoipa::path(
    get,
    path = "/api/cfg-instances/{id}",
    params(("id" = Uuid, Path, description = "CFG实例ID")),
    responses((status = 200, description = "成功", body = ApiResponse<CfgInstance>), (status = 404, description = "不存在")),
    tag = "CFG管理"
)]
pub async fn get_cfg_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CfgInstance>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let instance = sqlx::query_as::<_, CfgInstance>("SELECT * FROM cfg_instances WHERE id = $1")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询实例失败: {}", e)))?
        .ok_or_else(|| AppError::not_found("CFG实例不存在"))?;
    Ok(Json(ApiResponse::success(instance)))
}

#[utoipa::path(
    post,
    path = "/api/cfg-instances",
    request_body = CreateCfgInstanceRequest,
    responses((status = 200, description = "创建成功", body = ApiResponse<CfgInstance>)),
    tag = "CFG管理"
)]
pub async fn create_cfg_instance(
    State(app_state): State<AppState>,
    Json(req): Json<CreateCfgInstanceRequest>,
) -> Result<Json<ApiResponse<CfgInstance>>, AppError> {
    req.validate().map_err(AppError::bad_request)?;
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    let row = sqlx::query_as::<_, CfgInstance>(
        r#"INSERT INTO cfg_instances (
            id, name, base_url, description, enabled, timeout_seconds, max_concurrent_tasks, health_check_interval, status, last_health_check, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,true, COALESCE($5,30), COALESCE($6,2), COALESCE($7,30), 'unknown', NULL, $8, $8) RETURNING *"#
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.base_url)
    .bind(&req.description)
    .bind(req.timeout_seconds)
    .bind(req.max_concurrent_tasks)
    .bind(req.health_check_interval)
    .bind(now)
    .fetch_one(db.pool()).await
    .map_err(|e| AppError::service_unavailable(format!("创建实例失败: {}", e)))?;
    Ok(Json(ApiResponse::success(row)))
}

#[utoipa::path(
    put,
    path = "/api/cfg-instances/{id}",
    params(("id" = Uuid, Path, description = "CFG实例ID")),
    request_body = UpdateCfgInstanceRequest,
    responses((status = 200, description = "更新成功", body = ApiResponse<String>)),
    tag = "CFG管理"
)]
pub async fn update_cfg_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCfgInstanceRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let now = Utc::now();
    let _row = sqlx::query(
        r#"UPDATE cfg_instances SET
            name = COALESCE($2, name),
            base_url = COALESCE($3, base_url),
            description = COALESCE($4, description),
            enabled = COALESCE($5, enabled),
            timeout_seconds = COALESCE($6, timeout_seconds),
            max_concurrent_tasks = COALESCE($7, max_concurrent_tasks),
            health_check_interval = COALESCE($8, health_check_interval),
            updated_at = $9
        WHERE id = $1"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.base_url)
    .bind(req.description)
    .bind(req.enabled)
    .bind(req.timeout_seconds)
    .bind(req.max_concurrent_tasks)
    .bind(req.health_check_interval)
    .bind(now)
    .execute(db.pool())
    .await
    .map_err(|e| AppError::service_unavailable(format!("更新实例失败: {}", e)))?;
    Ok(Json(ApiResponse::success("CFG实例更新成功".to_string())))
}

#[utoipa::path(
    delete,
    path = "/api/cfg-instances/{id}",
    params(("id" = Uuid, Path, description = "CFG实例ID")),
    responses((status = 200, description = "删除成功", body = ApiResponse<String>)),
    tag = "CFG管理"
)]
pub async fn delete_cfg_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    sqlx::query("DELETE FROM cfg_instances WHERE id = $1")
        .bind(id)
        .execute(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("删除实例失败: {}", e)))?;
    Ok(Json(ApiResponse::success("CFG实例删除成功".to_string())))
}

#[utoipa::path(
    post,
    path = "/api/cfg-instances/{id}/health-check",
    params(("id" = Uuid, Path, description = "CFG实例ID")),
    responses((status = 200, description = "健康状态", body = ApiResponse<CfgHealthStatus>)),
    tag = "CFG管理"
)]
pub async fn health_check_cfg_instance(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CfgHealthStatus>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let base = sqlx::query("SELECT name, base_url FROM cfg_instances WHERE id = $1")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询实例失败: {}", e)))?
        .ok_or_else(|| AppError::not_found("CFG实例不存在"))?;
    let name: String = base.get("name");
    let base_url: String = base.get("base_url");

    // 不再依赖全局配置中的 [cfg]，改为基于实例 base_url 构建最小可用配置
    let mut override_cfg = CfgConfig::default();
    override_cfg.base_url = base_url;
    override_cfg.enabled = true;
    let client = CfgClient::new(override_cfg)?;
    let start = std::time::Instant::now();
    let (status, error_message) = match client.get_system_status().await {
        Ok(_) => (CfgInstanceStatus::Healthy, None),
        Err(e) => (CfgInstanceStatus::Unhealthy, Some(e.to_string())),
    };
    let elapsed = start.elapsed().as_millis() as u64;

    // 更新实例状态与 last_health_check
    sqlx::query("UPDATE cfg_instances SET status = $2, last_health_check = $3, updated_at = $3 WHERE id = $1")
        .bind(id)
        .bind(status.to_string())
        .bind(Utc::now())
        .execute(db.pool()).await.ok();

    let resp = CfgHealthStatus {
        instance_id: id,
        instance_name: name,
        status,
        response_time_ms: Some(elapsed),
        checked_at: Utc::now(),
        error_message,
    };
    Ok(Json(ApiResponse::success(resp)))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CfgStatsQueryParams {
    pub days: Option<u32>,
}

#[derive(Debug)]
struct CfgInstanceStatistics {
    total_tasks: i64,
    successful_tasks: i64,
    failed_tasks: i64,
    average_processing_time: Option<f64>,
    period_start: chrono::DateTime<chrono::Utc>,
    period_end: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(
    get,
    path = "/api/cfg-instances/{id}/stats",
    params(("id" = Uuid, Path, description = "CFG实例ID"), ("days" = Option<u32>, Query, description = "统计天数，默认7天")),
    responses((status = 200, description = "成功", body = ApiResponse<CfgInstanceStatsResponse>)),
    tag = "CFG管理"
)]
pub async fn get_cfg_instance_stats(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<CfgStatsQueryParams>,
) -> Result<Json<ApiResponse<CfgInstanceStatsResponse>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let days = params.days.unwrap_or(7);
    let stats = get_cfg_instance_statistics(db.pool(), id, days).await?;
    Ok(Json(ApiResponse::success(CfgInstanceStatsResponse {
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

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CfgInstanceStatsResponse {
    pub instance_id: Uuid,
    pub total_tasks: i64,
    pub successful_tasks: i64,
    pub failed_tasks: i64,
    pub average_processing_time: Option<f64>,
    pub success_rate: f64,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

async fn get_cfg_instance_statistics(
    pool: &sqlx::PgPool,
    instance_id: Uuid,
    days: u32,
) -> Result<CfgInstanceStatistics, AppError> {
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
        WHERE cfg_instance_id = $1 AND analyzer_type = 'CFG'
        AND created_at >= $2 AND created_at <= $3
        "#,
    )
    .bind(instance_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::service_unavailable(format!("查询统计信息失败: {}", e)))?;

    Ok(CfgInstanceStatistics {
        total_tasks: row.get::<i64, _>("total_tasks"),
        successful_tasks: row.get::<i64, _>("successful_tasks"),
        failed_tasks: row.get::<i64, _>("failed_tasks"),
        average_processing_time: row.get::<Option<f64>, _>("avg_duration_seconds"),
        period_start,
        period_end,
    })
}
