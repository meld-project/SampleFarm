use crate::{error::AppError, handlers::sample_full::AppState, response::ApiResponse};
use axum::{
    extract::{Multipart, State},
    response::Json,
};
use serde::{Deserialize, Serialize};

/// 简化的样本上传响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleUploadResponse {
    pub message: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_type: String,
    pub md5: String,
    pub sha256: String,
}

/// 简单的文件上传处理器
pub async fn upload_file_simple(
    State(app_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<SimpleUploadResponse>>, AppError> {
    // 检查文件处理器是否可用
    let file_processor = app_state
        .file_processor
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("文件处理服务不可用"))?;

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

    // 解析multipart数据
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        let error_msg = format!("{}", e);
        if error_msg.contains("body longer than") || error_msg.contains("body is too large") {
            AppError::bad_request("上传文件过大，请选择小于1GB的文件")
        } else if error_msg.contains("multipart") {
            AppError::bad_request("文件上传格式不正确，请确保选择了有效的文件")
        } else {
            AppError::bad_request(format!("文件上传失败: {}", e))
        }
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" {
            filename = field.file_name().map(|s| s.to_string());
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        let error_msg = format!("{}", e);
                        if error_msg.contains("body longer than")
                            || error_msg.contains("body is too large")
                        {
                            AppError::bad_request("上传文件过大，请选择小于1GB的文件")
                        } else {
                            AppError::bad_request(format!("读取文件数据失败: {}", e))
                        }
                    })?
                    .to_vec(),
            );
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::bad_request("缺少文件数据"))?;
    let filename = filename.ok_or_else(|| AppError::bad_request("缺少文件名"))?;

    // 处理文件
    let file_info = file_processor.process_file(&file_data, &filename).await?;

    tracing::info!(
        "成功处理文件: {} (大小: {} 字节)",
        filename,
        file_data.len()
    );

    Ok(Json(ApiResponse::success(SimpleUploadResponse {
        message: "文件上传成功".to_string(),
        file_name: filename,
        file_size: file_info.file_info.size,
        file_type: file_info.file_info.mime_type,
        md5: file_info.hashes.md5,
        sha256: file_info.hashes.sha256,
    })))
}

/// 系统状态检查
pub async fn system_status(
    State(app_state): State<AppState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mut status = std::collections::HashMap::new();

    status.insert(
        "database",
        if app_state.database.is_some() {
            "available"
        } else {
            "unavailable"
        },
    );
    status.insert(
        "storage",
        if app_state.storage.is_some() {
            "available"
        } else {
            "unavailable"
        },
    );
    status.insert(
        "file_processor",
        if app_state.file_processor.is_some() {
            "available"
        } else {
            "unavailable"
        },
    );

    Json(ApiResponse::success(serde_json::json!(status)))
}
