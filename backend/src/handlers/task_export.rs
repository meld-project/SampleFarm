use crate::storage::Storage;
use crate::{error::AppError, handlers::sample_full::AppState};
use axum::extract::{Path, State};
use sqlx::Row;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/tasks/{id}/export.csv",
    params(("id" = Uuid, Path, description = "主任务ID")),
    responses((status = 200, description = "CSV导出", body = String)),
    tag = "任务导出"
)]
pub async fn export_task_results_csv(
    State(app_state): State<AppState>,
    Path(master_task_id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;

    // 获取任务类型
    let row = sqlx::query("SELECT analyzer_type FROM master_tasks WHERE id = $1")
        .bind(master_task_id)
        .fetch_optional(db.pool())
        .await?;
    let Some(r) = row else {
        return Err(AppError::not_found("任务不存在"));
    };
    let analyzer: String = r.get("analyzer_type");

    if analyzer == "CFG" {
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

        let mut csv =
            String::from("sub_task_id,status,file_name,sha256,error,message,result_files\n");
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
                format!("attachment; filename=task_{}_results.csv", master_task_id),
            )
            .body(axum::body::Body::from(csv))
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        return Ok(resp);
    }

    if analyzer == "CAPE" {
        let rows = sqlx::query(
            r#"
            SELECT 
                st.id as sub_task_id,
                st.status::text as status,
                st.error_message,
                s.file_name,
                s.file_hash_sha256,
                car.cape_task_id,
                car.score::float4 AS score,
                car.severity,
                car.verdict
            FROM sub_tasks st
            JOIN samples s ON st.sample_id = s.id
            LEFT JOIN cape_analysis_results car ON car.sub_task_id = st.id
            WHERE st.master_task_id = $1 AND st.analyzer_type = 'CAPE'
            ORDER BY st.created_at ASC
            "#,
        )
        .bind(master_task_id)
        .fetch_all(db.pool())
        .await?;

        let mut csv = String::from(
            "sub_task_id,status,file_name,sha256,cape_task_id,score,severity,verdict,error\n",
        );
        for r in rows {
            let sub_task_id: Uuid = r.get("sub_task_id");
            let status: String = r.get("status");
            let file_name: String = r.get("file_name");
            let sha256: String = r.get("file_hash_sha256");
            let error_message: Option<String> = r.get("error_message");
            let cape_task_id: Option<i32> = r.get("cape_task_id");
            let score: Option<f32> = r.get("score");
            let severity: Option<String> = r.get("severity");
            let verdict: Option<String> = r.get("verdict");
            let line = format!(
                "{},{},{},{},{},{},{},{},{}\n",
                sub_task_id,
                status,
                file_name,
                sha256,
                cape_task_id.map(|v| v.to_string()).unwrap_or_default(),
                score.map(|v| v.to_string()).unwrap_or_default(),
                severity.unwrap_or_default(),
                verdict.unwrap_or_default(),
                error_message.unwrap_or_default().replace('\n', " ")
            );
            csv.push_str(&line);
        }

        let resp = axum::response::Response::builder()
            .status(axum::http::StatusCode::OK)
            .header("Content-Type", "text/csv; charset=utf-8")
            .header(
                "Content-Disposition",
                format!("attachment; filename=task_{}_results.csv", master_task_id),
            )
            .body(axum::body::Body::from(csv))
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        return Ok(resp);
    }

    Err(AppError::bad_request("当前分析器暂不支持CSV导出"))
}

#[utoipa::path(
    get,
    path = "/api/tasks/{id}/results.zip",
    params(("id" = Uuid, Path, description = "主任务ID")),
    responses((status = 200, description = "ZIP打包", body = String)),
    tag = "任务导出"
)]
pub async fn download_task_results_zip(
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

    // 获取任务类型
    let row = sqlx::query("SELECT analyzer_type FROM master_tasks WHERE id = $1")
        .bind(master_task_id)
        .fetch_optional(db.pool())
        .await?;
    let Some(r) = row else {
        return Err(AppError::not_found("任务不存在"));
    };
    let analyzer: String = r.get("analyzer_type");

    let mut buffer: Vec<u8> = Vec::new();
    let cursor = std::io::Cursor::new(&mut buffer);
    let mut zip_writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    if analyzer == "CFG" {
        let rows = sqlx::query(
            r#"
            SELECT s.file_name, car.result_files
            FROM sub_tasks st
            JOIN samples s ON st.sample_id = s.id
            JOIN cfg_analysis_results car ON car.sub_task_id = st.id
            WHERE st.master_task_id = $1 AND st.analyzer_type = 'CFG' AND car.result_files IS NOT NULL
            ORDER BY st.created_at ASC
            "#
        )
        .bind(master_task_id)
        .fetch_all(db.pool()).await?;

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
    } else if analyzer == "CAPE" {
        // 将 CAPE 的 full_report 以 JSON 文件形式打包
        let rows = sqlx::query(
            r#"
            SELECT s.file_name, car.full_report
            FROM sub_tasks st
            JOIN samples s ON st.sample_id = s.id
            JOIN cape_analysis_results car ON car.sub_task_id = st.id
            WHERE st.master_task_id = $1 AND st.analyzer_type = 'CAPE'
            ORDER BY st.created_at ASC
            "#,
        )
        .bind(master_task_id)
        .fetch_all(db.pool())
        .await?;

        for r in rows {
            let file_name: String = r.get("file_name");
            let full_report: Option<serde_json::Value> = r.get("full_report");
            let json_bytes = if let Some(v) = full_report {
                serde_json::to_vec_pretty(&v).unwrap_or_else(|_| b"{}".to_vec())
            } else {
                b"{}".to_vec()
            };
            let zip_path = format!("{}/report.json", file_name);
            zip_writer
                .start_file(zip_path, options)
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
            use std::io::Write;
            zip_writer
                .write_all(&json_bytes)
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        }
    } else {
        return Err(AppError::bad_request("当前分析器暂不支持ZIP导出"));
    }

    zip_writer
        .finish()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let resp = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "application/zip")
        .header(
            "Content-Disposition",
            format!("attachment; filename=task_{}_results.zip", master_task_id),
        )
        .body(axum::body::Body::from(buffer))
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(resp)
}
