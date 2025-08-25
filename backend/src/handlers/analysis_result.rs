use crate::{
    error::AppError,
    handlers::sample_full::AppState,
    models::cape_result::{CapeAnalysisResult, CapeResultSummary},
    response::ApiResponse,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

/// 任务分析结果查询参数
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AnalysisResultQuery {
    /// 页码，从1开始
    pub page: Option<u32>,
    /// 每页数量，默认20，最大100
    pub page_size: Option<u32>,
    /// 按分析完成时间排序
    pub sort_by: Option<String>,
    /// 排序方向：asc/desc
    pub sort_order: Option<String>,
    /// 按威胁评分筛选（最小值）
    pub min_score: Option<f32>,
    /// 按威胁评分筛选（最大值）
    pub max_score: Option<f32>,
    /// 按判定结果筛选
    pub verdict: Option<String>,
}

/// 分析结果统计信息
#[derive(Debug, Serialize, ToSchema)]
pub struct AnalysisResultStats {
    /// 总分析数量
    pub total_analyses: i64,
    /// 恶意判定数量
    pub malicious_count: i64,
    /// 可疑判定数量
    pub suspicious_count: i64,
    /// 干净判定数量
    pub clean_count: i64,
    /// 平均威胁评分
    pub average_score: Option<f64>,
    /// 最高威胁评分
    pub max_score: Option<f32>,
    /// 最近分析时间
    pub latest_analysis: Option<chrono::DateTime<chrono::Utc>>,
}

/// 样本分析历史响应
#[derive(Debug, Serialize, ToSchema)]
pub struct SampleAnalysisHistory {
    /// 样本ID
    pub sample_id: Uuid,
    /// 样本文件名
    pub sample_name: String,
    /// 分析历史记录
    pub analyses: Vec<CapeResultSummary>,
    /// 统计信息
    pub stats: AnalysisResultStats,
}

/// 获取任务的所有分析结果
///
/// 返回指定任务的所有子任务分析结果，支持分页和筛选
#[utoipa::path(
    get,
    path = "/api/tasks/{id}/results",
    params(
        ("id" = Uuid, Path, description = "任务ID"),
        AnalysisResultQuery
    ),
    responses(
        (status = 200, description = "获取任务分析结果成功", body = ApiResponse<Vec<CapeAnalysisResult>>),
        (status = 404, description = "任务不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "分析结果管理"
)]
pub async fn get_task_results(
    State(app_state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Query(query): Query<AnalysisResultQuery>,
) -> Result<Json<ApiResponse<Vec<CapeAnalysisResult>>>, AppError> {
    // 获取数据库连接
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用".to_string()))?;
    let pool = db.pool();
    // 验证任务是否存在
    let task_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM master_tasks WHERE id = $1)")
            .bind(task_id)
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询任务失败: {}", e)))?;

    if !task_exists {
        return Err(AppError::not_found("任务不存在".to_string()));
    }

    // 设置分页参数
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * page_size;

    // 构建查询条件
    let mut where_conditions = vec!["st.master_task_id = $1".to_string()];
    let mut param_count = 1;

    if query.min_score.is_some() {
        param_count += 1;
        where_conditions.push(format!("car.score >= ${}", param_count));
    }

    if query.max_score.is_some() {
        param_count += 1;
        where_conditions.push(format!("car.score <= ${}", param_count));
    }

    if query.verdict.is_some() {
        param_count += 1;
        where_conditions.push(format!("car.verdict = ${}", param_count));
    }

    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    // 排序
    let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = query.sort_order.as_deref().unwrap_or("desc");
    let order_clause = match sort_by {
        "score" => format!("ORDER BY car.score {}", sort_order),
        "analysis_completed_at" => format!("ORDER BY car.analysis_completed_at {}", sort_order),
        _ => format!("ORDER BY car.created_at {}", sort_order),
    };

    // 查询分析结果
    let query_sql = format!(
        r#"
        SELECT 
            car.id, car.sub_task_id, car.sample_id, car.cape_task_id,
            car.analysis_started_at, car.analysis_completed_at, car.analysis_duration,
            car.score::float4 AS score, car.severity, car.verdict,
            car.signatures, car.behavior_summary,
            car.full_report, car.report_summary,
            car.created_at, car.updated_at,
            st.error_message
        FROM cape_analysis_results car
        JOIN sub_tasks st ON car.sub_task_id = st.id
        {}
        {}
        LIMIT ${} OFFSET ${}
        "#,
        where_clause,
        order_clause,
        param_count + 1,
        param_count + 2
    );

    let mut query_builder = sqlx::query(&query_sql).bind(task_id);

    // 绑定筛选参数
    if query.min_score.is_some() {
        query_builder = query_builder.bind(query.min_score.unwrap());
    }
    if query.max_score.is_some() {
        query_builder = query_builder.bind(query.max_score.unwrap());
    }
    if let Some(ref verdict) = query.verdict {
        query_builder = query_builder.bind(verdict);
    }

    // 绑定分页参数
    query_builder = query_builder.bind(page_size as i64).bind(offset as i64);

    let rows = query_builder
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询分析结果失败: {}", e)))?;

    let results: Vec<CapeAnalysisResult> = rows
        .into_iter()
        .map(|row| CapeAnalysisResult {
            id: row.get("id"),
            sub_task_id: row.get("sub_task_id"),
            sample_id: row.get("sample_id"),
            cape_task_id: row.get("cape_task_id"),
            analysis_started_at: row.get("analysis_started_at"),
            analysis_completed_at: row.get("analysis_completed_at"),
            analysis_duration: row.get("analysis_duration"),
            score: row.get("score"),
            severity: row.get("severity"),
            verdict: row.get("verdict"),
            signatures: row.get("signatures"),
            behavior_summary: row.get("behavior_summary"),
            full_report: row.get("full_report"),
            report_summary: row.get("report_summary"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    Ok(Json(ApiResponse::success(results)))
}

/// 获取样本的分析历史
///
/// 返回指定样本的所有分析历史记录和统计信息
#[utoipa::path(
    get,
    path = "/api/samples/{id}/analysis",
    params(
        ("id" = Uuid, Path, description = "样本ID"),
        AnalysisResultQuery
    ),
    responses(
        (status = 200, description = "获取样本分析历史成功", body = ApiResponse<SampleAnalysisHistory>),
        (status = 404, description = "样本不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "分析结果管理"
)]
pub async fn get_sample_analysis_history(
    State(app_state): State<AppState>,
    Path(sample_id): Path<Uuid>,
    Query(query): Query<AnalysisResultQuery>,
) -> Result<Json<ApiResponse<SampleAnalysisHistory>>, AppError> {
    // 获取数据库连接
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用".to_string()))?;
    let pool = db.pool();
    // 获取样本信息
    let sample_row = sqlx::query("SELECT file_name FROM samples WHERE id = $1")
        .bind(sample_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询样本失败: {}", e)))?;

    let sample_name = match sample_row {
        Some(row) => row.get::<String, _>("file_name"),
        None => return Err(AppError::not_found("样本不存在".to_string())),
    };

    // 设置分页参数
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * page_size;

    // 查询分析历史（摘要）
    let analyses_rows = sqlx::query(
        r#"
        SELECT 
            car.id, car.sub_task_id, car.sample_id, car.cape_task_id,
            car.score::float4 AS score, car.severity, car.verdict,
            car.analysis_completed_at, car.created_at
        FROM cape_analysis_results car
        WHERE car.sample_id = $1
        ORDER BY car.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(sample_id)
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::service_unavailable(format!("查询分析历史失败: {}", e)))?;

    let analyses: Vec<CapeResultSummary> = analyses_rows
        .into_iter()
        .map(|row| CapeResultSummary {
            id: row.get("id"),
            sub_task_id: row.get("sub_task_id"),
            sample_id: row.get("sample_id"),
            cape_task_id: row.get("cape_task_id"),
            score: row.get("score"),
            severity: row.get("severity"),
            verdict: row.get("verdict"),
            analysis_completed_at: row.get("analysis_completed_at"),
            created_at: row.get("created_at"),
        })
        .collect();

    // 查询统计信息
    let stats_row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_analyses,
            COUNT(CASE WHEN verdict = 'malicious' THEN 1 END) as malicious_count,
            COUNT(CASE WHEN verdict = 'suspicious' THEN 1 END) as suspicious_count,
            COUNT(CASE WHEN verdict = 'clean' THEN 1 END) as clean_count,
            AVG(score)::float8 as average_score,
            MAX(score)::float4 as max_score,
            MAX(analysis_completed_at) as latest_analysis
        FROM cape_analysis_results
        WHERE sample_id = $1
        "#,
    )
    .bind(sample_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::service_unavailable(format!("查询统计信息失败: {}", e)))?;

    let stats = AnalysisResultStats {
        total_analyses: stats_row.get("total_analyses"),
        malicious_count: stats_row.get("malicious_count"),
        suspicious_count: stats_row.get("suspicious_count"),
        clean_count: stats_row.get("clean_count"),
        average_score: stats_row.get("average_score"),
        max_score: stats_row.get("max_score"),
        latest_analysis: stats_row.get("latest_analysis"),
    };

    let response = SampleAnalysisHistory {
        sample_id,
        sample_name,
        analyses,
        stats,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取CAPE分析详情
///
/// 返回指定CAPE分析的完整详情，包括报告全文
#[utoipa::path(
    get,
    path = "/api/analysis/cape/{id}",
    params(
        ("id" = Uuid, Path, description = "分析结果ID")
    ),
    responses(
        (status = 200, description = "获取CAPE分析详情成功", body = ApiResponse<CapeAnalysisResult>),
        (status = 404, description = "分析结果不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "分析结果管理"
)]
pub async fn get_cape_analysis_detail(
    State(app_state): State<AppState>,
    Path(analysis_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CapeAnalysisResult>>, AppError> {
    // 获取数据库连接
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用".to_string()))?;
    let pool = db.pool();
    let row = sqlx::query(
        r#"
        SELECT 
            car.id, car.sub_task_id, car.sample_id, car.cape_task_id,
            car.analysis_started_at, car.analysis_completed_at, car.analysis_duration,
            car.score::float4 AS score, car.severity, car.verdict,
            car.signatures, car.behavior_summary,
            car.full_report, car.report_summary,
            car.created_at, car.updated_at,
            st.error_message
        FROM cape_analysis_results car
        JOIN sub_tasks st ON car.sub_task_id = st.id
        WHERE car.id = $1
        "#,
    )
    .bind(analysis_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::service_unavailable(format!("查询分析详情失败: {}", e)))?;

    let result = match row {
        Some(row) => CapeAnalysisResult {
            id: row.get("id"),
            sub_task_id: row.get("sub_task_id"),
            sample_id: row.get("sample_id"),
            cape_task_id: row.get("cape_task_id"),
            analysis_started_at: row.get("analysis_started_at"),
            analysis_completed_at: row.get("analysis_completed_at"),
            analysis_duration: row.get("analysis_duration"),
            score: row.get("score"),
            severity: row.get("severity"),
            verdict: row.get("verdict"),
            signatures: row.get("signatures"),
            behavior_summary: row.get("behavior_summary"),
            full_report: row.get("full_report"),
            report_summary: row.get("report_summary"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        },
        None => {
            // 回退：有些前端可能误传了 sub_task_id，这里尝试按 sub_task_id 查最近一条
            tracing::warn!(
                "按分析结果ID未找到，尝试按 sub_task_id 查询: {}",
                analysis_id
            );
            let fallback = sqlx::query(
                r#"
                SELECT 
                    car.id, car.sub_task_id, car.sample_id, car.cape_task_id,
                    car.analysis_started_at, car.analysis_completed_at, car.analysis_duration,
                    car.score::float4 AS score, car.severity, car.verdict,
                    car.signatures, car.behavior_summary,
                    car.full_report, car.report_summary,
                    car.created_at, car.updated_at,
                    st.error_message
                FROM cape_analysis_results car
                JOIN sub_tasks st ON car.sub_task_id = st.id
                WHERE car.sub_task_id = $1
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(analysis_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                AppError::service_unavailable(format!("按 sub_task_id 查询分析详情失败: {}", e))
            })?;

            match fallback {
                Some(row) => CapeAnalysisResult {
                    id: row.get("id"),
                    sub_task_id: row.get("sub_task_id"),
                    sample_id: row.get("sample_id"),
                    cape_task_id: row.get("cape_task_id"),
                    analysis_started_at: row.get("analysis_started_at"),
                    analysis_completed_at: row.get("analysis_completed_at"),
                    analysis_duration: row.get("analysis_duration"),
                    score: row.get("score"),
                    severity: row.get("severity"),
                    verdict: row.get("verdict"),
                    signatures: row.get("signatures"),
                    behavior_summary: row.get("behavior_summary"),
                    full_report: row.get("full_report"),
                    report_summary: row.get("report_summary"),
                    error_message: row.get("error_message"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                },
                None => return Err(AppError::not_found("分析结果不存在".to_string())),
            }
        }
    };

    Ok(Json(ApiResponse::success(result)))
}

/// CAPE任务运行时快照响应模型
#[derive(Debug, Serialize, ToSchema)]
pub struct CapeRuntimeSnapshot {
    /// 快照状态
    pub status: String,
    /// 完整快照数据
    pub snapshot: serde_json::Value,
    /// 最后更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 获取CAPE任务运行时快照（已弃用：快照表默认不创建）
#[utoipa::path(
    get,
    path = "/api/analysis/cape/{sub_task_id}/runtime",
    params(
        ("sub_task_id" = uuid::Uuid, Path, description = "子任务ID")
    ),
    responses(
        (status = 200, description = "成功获取运行时快照", body = ApiResponse<CapeRuntimeSnapshot>),
        (status = 404, description = "快照不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "分析结果管理"
)]
pub async fn get_cape_runtime_snapshot(
    _state: State<AppState>,
    _sub_task_id: Path<Uuid>,
) -> Result<Json<ApiResponse<CapeRuntimeSnapshot>>, AppError> {
    Err(AppError::service_unavailable(
        "运行时快照功能已禁用。如需启用，请在schema中开启cap_task_status_snapshots并恢复仓库使用"
            .to_string(),
    ))
}
