use crate::{
    error::AppError,
    models::{
        FileTypeCount, SampleQueryParams, SampleTypeCount, SourceCount, TaskPreviewRequest,
        TaskPreviewResponse,
    },
    response::ApiResponse,
};
use axum::{
    extract::{Query, State},
    response::Json,
};
use sqlx::{Pool, Postgres, QueryBuilder, Row};

// 使用统一的 AppState（从 sample_full 导入）
use crate::handlers::sample_full::AppState;

/// 任务预览处理器
///
/// 根据筛选条件预览将要分析的样本统计信息，返回匹配的样本数量、总大小、文件类型分布等
#[utoipa::path(
    get,
    path = "/api/tasks/preview",
    params(TaskPreviewRequest),
    responses(
        (status = 200, description = "任务预览成功", body = ApiResponse<TaskPreviewResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "任务管理"
)]
pub async fn preview_task(
    State(app_state): State<AppState>,
    Query(params): Query<TaskPreviewRequest>,
) -> Result<Json<ApiResponse<TaskPreviewResponse>>, AppError> {
    // 检查数据库连接
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用".to_string()))?;

    // 检查分析器是否可用
    if !params.analyzer_type.is_enabled() {
        return Err(AppError::bad_request(format!(
            "分析器 {} 当前不可用",
            params.analyzer_type
        )));
    }

    // 获取样本统计信息 - 现在使用平铺的字段，需要重新构造 SampleQueryParams
    let filter = SampleQueryParams {
        file_name: params.file_name,
        file_type: params.file_type,
        sample_type: params.sample_type,
        file_hash_md5: params.file_hash_md5,
        file_hash_sha1: params.file_hash_sha1,
        file_hash_sha256: params.file_hash_sha256,
        min_size: params.min_size,
        max_size: params.max_size,
        uploader: params.uploader,
        source: params.source,
        labels: params.labels,
        is_container: params.is_container,
        parent_id: params.parent_id,
        start_time: params.start_time,
        end_time: params.end_time,
        // 预览不需要分页和排序
        page: None,
        page_size: None,
        sort_by: None,
        sort_order: None,
    };
    let stats = get_sample_statistics(db.pool(), &filter).await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 获取样本统计信息
async fn get_sample_statistics(
    pool: &Pool<Postgres>,
    filter: &SampleQueryParams,
) -> Result<TaskPreviewResponse, AppError> {
    // 条件拼装助手
    fn apply_filter<'a>(qb: &mut QueryBuilder<'a, Postgres>, f: &'a SampleQueryParams) {
        let mut first = true;
        let mut push_cond = |qb: &mut QueryBuilder<'a, Postgres>| {
            if first {
                qb.push(" WHERE ");
                first = false;
            } else {
                qb.push(" AND ");
            }
        };

        if let Some(v) = &f.file_name {
            push_cond(qb);
            qb.push("file_name ILIKE ").push_bind(format!("%{}%", v));
        }
        if let Some(v) = &f.file_type {
            push_cond(qb);
            qb.push("file_type = ").push_bind(v);
        }
        if let Some(v) = &f.sample_type {
            push_cond(qb);
            qb.push("sample_type = ").push_bind(v);
        }
        if let Some(v) = &f.file_hash_md5 {
            push_cond(qb);
            qb.push("file_hash_md5 = ").push_bind(v);
        }
        if let Some(v) = &f.file_hash_sha1 {
            push_cond(qb);
            qb.push("file_hash_sha1 = ").push_bind(v);
        }
        if let Some(v) = &f.file_hash_sha256 {
            push_cond(qb);
            qb.push("file_hash_sha256 = ").push_bind(v);
        }
        if let Some(v) = f.min_size {
            push_cond(qb);
            qb.push("file_size >= ").push_bind(v);
        }
        if let Some(v) = f.max_size {
            push_cond(qb);
            qb.push("file_size <= ").push_bind(v);
        }
        if let Some(v) = &f.uploader {
            // 当前schema暂无uploader字段，若后续加入可启用该过滤
            let _ = v; // 占位避免未使用警告
        }
        if let Some(v) = &f.source {
            push_cond(qb);
            qb.push("source ILIKE ").push_bind(format!("%{}%", v));
        }
        if let Some(v) = &f.labels {
            if !v.is_empty() {
                push_cond(qb);
                qb.push("labels && ").push_bind(v);
            }
        }
        if let Some(v) = f.is_container {
            push_cond(qb);
            qb.push("is_container = ").push_bind(v);
        }
        if let Some(v) = f.parent_id {
            push_cond(qb);
            qb.push("parent_id = ").push_bind(v);
        }
        if let Some(v) = f.start_time {
            push_cond(qb);
            qb.push("created_at >= ").push_bind(v);
        }
        if let Some(v) = f.end_time {
            push_cond(qb);
            qb.push("created_at <= ").push_bind(v);
        }
    }

    // 1) 计数
    let mut count_q = QueryBuilder::new("SELECT COUNT(*) as count FROM samples");
    apply_filter(&mut count_q, filter);
    let count_result = count_q
        .build()
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询样本数量失败: {}", e)))?;
    let total_samples: i64 = count_result.get("count");

    // 2) 总大小
    let mut size_q = QueryBuilder::new(
        "SELECT CAST(COALESCE(SUM(file_size), 0) AS BIGINT) as total_size FROM samples",
    );
    apply_filter(&mut size_q, filter);
    let size_result = size_q
        .build()
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询文件大小失败: {}", e)))?;
    let total_size: i64 = size_result.get("total_size");

    // 3) 文件类型分布
    let mut ft_q = QueryBuilder::new(
        "SELECT file_type, COUNT(*) as count, CAST(COALESCE(SUM(file_size), 0) AS BIGINT) as size FROM samples",
    );
    apply_filter(&mut ft_q, filter);
    ft_q.push(" GROUP BY file_type ORDER BY count DESC");
    let file_type_rows = ft_q
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询文件类型分布失败: {}", e)))?;
    let file_type_distribution: Vec<FileTypeCount> = file_type_rows
        .into_iter()
        .map(|row| FileTypeCount {
            file_type: row.get("file_type"),
            count: row.get("count"),
            size: row.get("size"),
        })
        .collect();

    // 4) 样本类型分布
    let mut st_q = QueryBuilder::new(
        "SELECT sample_type::text as sample_type, COUNT(*) as count FROM samples",
    );
    apply_filter(&mut st_q, filter);
    st_q.push(" GROUP BY sample_type ORDER BY count DESC");
    let sample_type_rows = st_q
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询样本类型分布失败: {}", e)))?;
    let sample_type_distribution: Vec<SampleTypeCount> = sample_type_rows
        .into_iter()
        .map(|row| SampleTypeCount {
            sample_type: row.get("sample_type"),
            count: row.get("count"),
        })
        .collect();

    // 5) 来源分布（仅非空）
    let mut src_q = QueryBuilder::new("SELECT source, COUNT(*) as count FROM samples");
    apply_filter(&mut src_q, filter);
    src_q.push(" AND source IS NOT NULL GROUP BY source ORDER BY count DESC LIMIT 10");
    let source_rows = src_q
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询来源分布失败: {}", e)))?;
    let source_distribution: Vec<SourceCount> = source_rows
        .into_iter()
        .map(|row| SourceCount {
            source: row.get("source"),
            count: row.get("count"),
        })
        .collect();

    let estimated_duration_minutes = if total_samples > 0 {
        Some((total_samples * 3) as i32)
    } else {
        None
    };

    Ok(TaskPreviewResponse {
        total_samples,
        total_size,
        file_type_distribution,
        sample_type_distribution,
        source_distribution,
        estimated_duration_minutes,
    })
}

#[cfg(test)]
mod tests {
    // 测试代码暂时为空
}
