use crate::storage::Storage;
use crate::{
    error::AppError, handlers::sample_full::AppState, repositories::sample::SampleRepository,
};
use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct BatchDownloadRequest {
    pub ids: Vec<Uuid>,
    pub encrypt: Option<bool>,
    pub password: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/samples/batch/download",
    request_body = BatchDownloadRequest,
    responses((status = 200, description = "ZIP打包", body = String)),
    tag = "样本导出"
)]
pub async fn batch_download_samples(
    State(app_state): State<AppState>,
    Json(req): Json<BatchDownloadRequest>,
) -> Result<axum::response::Response, AppError> {
    let db = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库不可用"))?;
    let storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储不可用"))?;

    let repo = SampleRepository::new(db.clone());
    if req.ids.is_empty() {
        return Err(AppError::bad_request("ids 不能为空"));
    }

    let use_encryption =
        req.encrypt.unwrap_or(false) && req.password.as_deref().unwrap_or("").len() > 0;

    let mut buffer: Vec<u8> = Vec::new();
    let cursor = std::io::Cursor::new(&mut buffer);
    let mut zip_writer = zip::ZipWriter::new(cursor);
    let mut options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    if use_encryption {
        options = options.with_aes_encryption(zip::AesMode::Aes256, req.password.as_ref().unwrap());
    }

    for id in req.ids {
        match repo.find_by_id(id).await? {
            Some(sample) => {
                let dir = format!(
                    "{}-{}",
                    sample.file_name,
                    sample.id.to_string()[..8].to_string()
                );
                let zip_path = format!("{}/{}", dir, sample.file_name);
                match storage.download("samplefarm", &sample.storage_path).await {
                    Ok(data) => {
                        info!(sample_id=%sample.id, key=%sample.storage_path, "批量下载: 读取成功");
                        zip_writer
                            .start_file(zip_path, options)
                            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                        use std::io::Write;
                        zip_writer
                            .write_all(&data)
                            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                    }
                    Err(e) => {
                        error!(sample_id=%sample.id, key=%sample.storage_path, err=%e, "批量下载: 读取失败，写入错误说明");
                        let err_path = format!("{}/ERROR.txt", dir);
                        zip_writer
                            .start_file(err_path, options)
                            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                        use std::io::Write;
                        let msg = format!("下载失败: {}", e);
                        zip_writer
                            .write_all(msg.as_bytes())
                            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                    }
                }
            }
            None => {
                error!(%id, "批量下载: 样本不存在");
                // 写入一个提示文件
                let dir = format!("missing-{}", id.to_string()[..8].to_string());
                let err_path = format!("{}/ERROR.txt", dir);
                zip_writer
                    .start_file(err_path, options)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                use std::io::Write;
                let msg = format!("样本不存在: {}", id);
                zip_writer
                    .write_all(msg.as_bytes())
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
            }
        }
    }
    zip_writer
        .finish()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let resp = axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "application/zip")
        .header(
            "Content-Disposition",
            if use_encryption {
                "attachment; filename=samples_batch_encrypted.zip"
            } else {
                "attachment; filename=samples_batch.zip"
            },
        )
        .body(axum::body::Body::from(buffer))
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(resp)
}
