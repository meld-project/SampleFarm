use crate::storage::Storage;
use axum::{
    extract::{Path, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::error;
use uuid::Uuid;

use crate::{
    error::AppError,
    handlers::sample_full::AppState,
    response::ApiResponse,
    services::{CfgInstanceManager, CfgProcessor},
};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CfgBatchExecuteRequest {
    pub master_task_id: Uuid,
    pub label: i32,
    /// 轮询间隔秒（可选，默认10s）
    pub poll_interval_secs: Option<u64>,
    /// 最大等待秒（已废弃，超时机制已移除）
    #[deprecated(note = "超时机制已移除，此字段仅保留用于API兼容性")]
    pub max_wait_secs: Option<u64>,
    /// 提交间隔毫秒（可选，默认1000ms）
    pub submit_interval_ms: Option<u64>,
    /// 并发数（可选，默认1）
    pub concurrency: Option<u32>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CfgBatchExecuteResponse {
    pub master_task_id: Uuid,
    pub submitted_tasks: u32,
}

#[utoipa::path(
    post,
    path = "/api/cfg/execute",
    request_body = CfgBatchExecuteRequest,
    responses(
        (status = 200, description = "批量提交CFG任务成功", body = ApiResponse<CfgBatchExecuteResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 503, description = "CFG服务不可用"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "CFG分析"
)]
pub async fn execute_cfg_batch(
    State(app_state): State<AppState>,
    Json(req): Json<CfgBatchExecuteRequest>,
) -> Result<Json<ApiResponse<CfgBatchExecuteResponse>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;
    let storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储服务不可用"))?;

    // 初始化CFG实例管理器（从数据库）
    let cfg_manager = std::sync::Arc::new(
        CfgInstanceManager::new(db.pool().clone())
            .await
            .map_err(|e| {
                AppError::service_unavailable(format!("初始化CFG实例管理器失败: {}", e))
            })?,
    );

    // 检查是否有可用的CFG实例
    let available_instances = cfg_manager.get_available_instances().await;
    if available_instances.is_empty() {
        return Err(AppError::service_unavailable(
            "没有可用的CFG实例，请在实例管理中添加并启用",
        ));
    }

    // 查询待执行的 CFG 子任务及其样本 sha256
    let rows = sqlx::query(
        r#"
        SELECT st.id as sub_task_id, st.sample_id, st.cfg_instance_id, s.file_hash_sha256
        FROM sub_tasks st
        JOIN samples s ON st.sample_id = s.id
        WHERE st.master_task_id = $1 AND st.analyzer_type = 'CFG' AND st.status = 'pending'
        "#,
    )
    .bind(req.master_task_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| AppError::service_unavailable(format!("查询子任务失败: {}", e)))?;

    if rows.is_empty() {
        return Err(AppError::bad_request("没有待执行的CFG子任务"));
    }

    // 初始化 Processor（使用CFG结果bucket）
    let processor = std::sync::Arc::new(CfgProcessor::new(
        cfg_manager.clone(),
        std::sync::Arc::new(db.clone()),
        std::sync::Arc::new(storage.clone()),
        "cfg-results".to_string(), // 使用默认CFG结果bucket名称
        app_state.config.minio.bucket.clone(),
    ));

    let submitted = rows.len() as u32;
    let label = req.label;
    // 将主任务置为 running
    let _ =
        sqlx::query("UPDATE master_tasks SET status = 'running', updated_at = NOW() WHERE id = $1")
            .bind(req.master_task_id)
            .execute(db.pool())
            .await;

    // 从主任务参数读取缺省配置
    let mut poll_interval_secs = req.poll_interval_secs.unwrap_or(10);
    let mut submit_interval_ms = req.submit_interval_ms.unwrap_or(1000);

    if let Some(row) = sqlx::query("SELECT sample_filter FROM master_tasks WHERE id = $1")
        .bind(req.master_task_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| AppError::service_unavailable(format!("查询任务参数失败: {}", e)))?
    {
        let params: Option<serde_json::Value> = row.try_get("sample_filter").ok();
        if let Some(serde_json::Value::Object(obj)) = params {
            if let Some(v) = obj.get("cfg_poll_interval_secs").and_then(|v| v.as_u64()) {
                poll_interval_secs = v;
            }
            // cfg_max_wait_secs 参数被忽略，因为超时机制已移除
            if let Some(v) = obj.get("cfg_submit_interval_ms").and_then(|v| v.as_u64()) {
                submit_interval_ms = v;
            }
        }
    }

    // 后台执行处理
    let pool = db.pool().clone();
    let _bucket = "cfg-results".to_string(); // 使用默认CFG结果bucket名称
    tokio::spawn(async move {
        let mut handles = Vec::new();
        for (idx, row) in rows.into_iter().enumerate() {
            let sub_task_id: Uuid = row.get("sub_task_id");
            let sample_id: Uuid = row.get("sample_id");
            let cfg_instance_id: Option<Uuid> = row.get("cfg_instance_id");
            let sha256: Option<String> = row.get("file_hash_sha256");
            let sha256 = sha256.unwrap_or_default();
            // 标记提交中
            let _ = sqlx::query(
                "UPDATE sub_tasks SET status = 'submitting', started_at = NOW() WHERE id = $1",
            )
            .bind(sub_task_id)
            .execute(&pool)
            .await;

            // 提交间隔（用于保护后端）
            if idx > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(submit_interval_ms)).await;
            }

            let processor = processor.clone();
            let pool_clone = pool.clone();
            let master_task_id = req.master_task_id;

            // 创建CFG任务配置（无超时限制）
            use crate::config::cfg::CfgTaskConfig;
            #[allow(deprecated)]
            let cfg_config = Some(CfgTaskConfig {
                poll_interval_secs,
                max_wait_secs: 0, // 超时机制已移除
                label,
                retry: None, // 使用默认重试配置
            });

            let handle = tokio::spawn(async move {
                let res = processor
                    .process_sub_task(sample_id, &sha256, cfg_config, cfg_instance_id)
                    .await;

                match res {
                    Ok(merged) => {
                        let message = merged
                            .get("message")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let result_files = merged.get("result_files").cloned();
                        let full_report = Some(merged);
                        let _ = sqlx::query(
                            r#"INSERT INTO cfg_analysis_results (id, sub_task_id, sample_id, message, result_files, full_report, created_at, updated_at)
                                VALUES ($1,$2,$3,$4,$5,$6,NOW(),NOW())"#
                        )
                        .bind(Uuid::new_v4())
                        .bind(sub_task_id)
                        .bind(sample_id)
                        .bind(message)
                        .bind(result_files)
                        .bind(full_report)
                        .execute(&pool_clone).await;
                        let _ = sqlx::query(
                            "UPDATE sub_tasks SET status = 'completed', completed_at = NOW() WHERE id = $1"
                        )
                        .bind(sub_task_id)
                        .execute(&pool_clone).await;
                    }
                    Err(e) => {
                        let _ = sqlx::query(
                            "UPDATE sub_tasks SET status = 'failed', error_message = $2, completed_at = NOW() WHERE id = $1"
                        )
                        .bind(sub_task_id)
                        .bind(e.to_string())
                        .execute(&pool_clone).await;
                        // 附带更丰富的错误上下文
                        error!(?sub_task_id, %sha256, err=%e, "CFG 子任务失败");
                    }
                }

                // 更新主任务进度
                if let Err(e) = sqlx::query(
                    r#"
                    UPDATE master_tasks mt
                    SET 
                        completed_samples = (SELECT COUNT(*) FROM sub_tasks st WHERE st.master_task_id = $1 AND st.status = 'completed'),
                        failed_samples = (SELECT COUNT(*) FROM sub_tasks st WHERE st.master_task_id = $1 AND st.status IN ('failed','cancelled')),
                        progress = CASE 
                            WHEN mt.total_samples > 0 THEN (
                                (SELECT COUNT(*) FROM sub_tasks st 
                                 WHERE st.master_task_id = $1 
                                   AND st.status IN ('completed','failed','cancelled')) * 100 / mt.total_samples
                            )
                            ELSE 0 
                        END,
                        status = CASE 
                            WHEN ((SELECT COUNT(*) FROM sub_tasks st WHERE st.master_task_id = $1 AND st.status IN ('completed','failed','cancelled'))) >= mt.total_samples 
                            THEN 
                                CASE 
                                    -- 只有当所有子任务都失败时，主任务才标记为失败
                                    WHEN (SELECT COUNT(*) FROM sub_tasks st WHERE st.master_task_id = $1 AND st.status = 'failed') = mt.total_samples
                                    THEN 'failed'::master_task_status_enum
                                    -- 有任何成功的子任务，主任务就标记为完成
                                    ELSE 'completed'::master_task_status_enum
                                END
                            ELSE 'running' 
                        END::master_task_status_enum,
                        updated_at = NOW()
                    WHERE mt.id = $1
                    "#
                )
                .bind(master_task_id)
                .execute(&pool_clone).await {
                    error!(%master_task_id, err=%e, "刷新主任务统计失败(CFG)");
                }
            });
            handles.push(handle);
        }
        for h in handles {
            let _ = h.await;
        }
    });

    Ok(Json(ApiResponse::success(CfgBatchExecuteResponse {
        master_task_id: req.master_task_id,
        submitted_tasks: submitted,
    })))
}

// ===== 导出与打包 =====
#[utoipa::path(
    get,
    path = "/api/cfg/tasks/{id}/export.csv",
    params(("id" = Uuid, Path, description = "主任务ID")),
    responses((status = 200, description = "CSV导出", body = String)),
    tag = "CFG分析"
)]
pub async fn export_cfg_results_csv(
    State(app_state): State<AppState>,
    Path(master_task_id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let rows = sqlx::query(
        r#"
        SELECT st.id as sub_task_id, st.status::text as status, st.error_message, s.file_name, s.file_hash_sha256, car.message, car.result_files
        FROM sub_tasks st
        JOIN samples s ON st.sample_id = s.id
        LEFT JOIN cfg_analysis_results car ON car.sub_task_id = st.id
        WHERE st.master_task_id = $1 AND st.analyzer_type = 'CFG'
        ORDER BY st.created_at ASC
        "#
    )
    .bind(master_task_id)
    .fetch_all(db.pool()).await?;

    let mut csv = String::from("sub_task_id,status,file_name,sha256,error,message,result_files\n");
    for r in rows {
        let sub_task_id: Uuid = r.get("sub_task_id");
        let status: String = r.get("status");
        let file_name: String = r.get("file_name");
        let sha256: String = r.get("file_hash_sha256");
        let error_message: Option<String> = r.get("error_message");
        let message: Option<String> = r.get("message");
        let result_files: Option<serde_json::Value> = r.get("result_files");
        let rf_str = result_files
            .map(|v| v.to_string())
            .unwrap_or_default()
            .replace('\n', " ")
            .replace('"', "'");
        let line = format!(
            "{},{},{},{},{},{},{}\n",
            sub_task_id,
            status,
            file_name,
            sha256,
            error_message.unwrap_or_default().replace('\n', " "),
            message.unwrap_or_default().replace('\n', " "),
            rf_str
        );
        csv.push_str(&line);
    }

    let resp = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "text/csv; charset=utf-8")
        .header(
            "Content-Disposition",
            format!(
                "attachment; filename=cfg_task_{}_results.csv",
                master_task_id
            ),
        )
        .body(axum::body::Body::from(csv))
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(resp)
}

#[utoipa::path(
    get,
    path = "/api/cfg/tasks/{id}/results.zip",
    params(("id" = Uuid, Path, description = "主任务ID")),
    responses((status = 200, description = "ZIP打包", body = String)),
    tag = "CFG分析"
)]
pub async fn download_cfg_results_zip(
    State(app_state): State<AppState>,
    Path(master_task_id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储不可用"))?;
    let cfg_conf = app_state
        .config
        .cfg
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("CFG配置不可用"))?;

    let rows = sqlx::query(
        r#"
        SELECT s.file_name, car.result_files
        FROM sub_tasks st
        JOIN samples s ON st.sample_id = s.id
        JOIN cfg_analysis_results car ON car.sub_task_id = st.id
        WHERE st.master_task_id = $1 AND st.analyzer_type = 'CFG' AND car.result_files IS NOT NULL
        ORDER BY st.created_at ASC
        "#,
    )
    .bind(master_task_id)
    .fetch_all(db.pool())
    .await?;

    let mut buffer: Vec<u8> = Vec::new();
    let cursor = std::io::Cursor::new(&mut buffer);
    let mut zip_writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for r in rows {
        let file_name: String = r.get("file_name");
        let result_files: serde_json::Value = r.get("result_files");
        if let Some(obj) = result_files.as_object() {
            for (_k, v) in obj.iter() {
                if let Some(key) = v.as_str() {
                    let data = storage.download(&cfg_conf.result_bucket, key).await?;
                    let base = std::path::Path::new(key)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("result.bin");
                    let zip_path = format!("{}/{}", file_name, base);
                    zip_writer
                        .start_file(zip_path, options)
                        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                    use std::io::Write;
                    zip_writer
                        .write_all(&data)
                        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                }
            }
        }
    }
    // finalize
    zip_writer
        .finish()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let resp = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "application/zip")
        .header(
            "Content-Disposition",
            format!(
                "attachment; filename=cfg_task_{}_results.zip",
                master_task_id
            ),
        )
        .body(axum::body::Body::from(buffer))
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(resp)
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CfgTaskStatusResponse {
    pub master_task_id: Uuid,
    pub total_tasks: u32,
    pub pending_tasks: u32,
    pub running_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub progress_percentage: f32,
}

#[utoipa::path(
    get,
    path = "/api/cfg/status/{id}",
    params(("id" = Uuid, Path, description = "主任务ID")),
    responses((status = 200, description = "获取成功", body = ApiResponse<CfgTaskStatusResponse>)),
    tag = "CFG分析"
)]
pub async fn get_cfg_task_status(
    State(app_state): State<AppState>,
    Path(master_task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CfgTaskStatusResponse>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;
    // 统计子任务状态
    let rows = sqlx::query(
        r#"SELECT status::text AS status, COUNT(*) as cnt FROM sub_tasks WHERE master_task_id = $1 AND analyzer_type = 'CFG' GROUP BY status"#
    )
    .bind(master_task_id)
    .fetch_all(db.pool()).await.map_err(|e| AppError::service_unavailable(format!("查询任务状态失败: {}", e)))?;

    let mut total = 0u32;
    let mut pending = 0u32;
    let mut running = 0u32;
    let mut completed = 0u32;
    let mut failed = 0u32;
    for r in rows {
        let cnt: i64 = r.get("cnt");
        let cnt_u32 = cnt as u32;
        total += cnt_u32;
        let status: Option<String> = r.get("status");
        match status.as_deref() {
            Some("pending") => pending += cnt_u32,
            Some("submitting") | Some("submitted") | Some("analyzing") => running += cnt_u32,
            Some("completed") => completed += cnt_u32,
            Some("failed") | Some("cancelled") => failed += cnt_u32,
            _ => {}
        }
    }
    let progress = if total > 0 {
        (completed as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    Ok(Json(ApiResponse::success(CfgTaskStatusResponse {
        master_task_id,
        total_tasks: total,
        pending_tasks: pending,
        running_tasks: running,
        completed_tasks: completed,
        failed_tasks: failed,
        progress_percentage: progress,
    })))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CfgAnalysisDetailResponse {
    pub id: Uuid,
    pub sub_task_id: Uuid,
    pub sample_id: Uuid,
    pub message: Option<String>,
    pub result_files: Option<serde_json::Value>,
    pub full_report: Option<serde_json::Value>,
}

#[utoipa::path(
    get,
    path = "/api/analysis/cfg/{id}",
    params(("id" = Uuid, Path, description = "分析结果ID")),
    responses((status = 200, description = "获取成功", body = ApiResponse<CfgAnalysisDetailResponse>)),
    tag = "CFG分析"
)]
pub async fn get_cfg_analysis_detail(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CfgAnalysisDetailResponse>>, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库连接不可用"))?;
    // 优先按结果ID查询；若无，则按 sub_task_id 回退
    let mut row = sqlx::query(
        "SELECT id, sub_task_id, sample_id, message, result_files, full_report FROM cfg_analysis_results WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(db.pool()).await.map_err(|e| AppError::service_unavailable(format!("查询分析详情失败: {}", e)))?;
    if row.is_none() {
        row = sqlx::query(
            "SELECT id, sub_task_id, sample_id, message, result_files, full_report FROM cfg_analysis_results WHERE sub_task_id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(id)
        .fetch_optional(db.pool()).await.map_err(|e| AppError::service_unavailable(format!("查询分析详情失败: {}", e)))?;
    }
    let r = row.ok_or_else(|| AppError::not_found("分析结果不存在"))?;
    Ok(Json(ApiResponse::success(CfgAnalysisDetailResponse {
        id: r.get("id"),
        sub_task_id: r.get("sub_task_id"),
        sample_id: r.get("sample_id"),
        message: r.get("message"),
        result_files: r.get("result_files"),
        full_report: r.get("full_report"),
    })))
}
