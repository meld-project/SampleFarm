use crate::{
    database::Database,
    error::AppError,
    file_processing::FileProcessor,
    models::{
        CreateSampleRequest, PagedResult, Pagination, Sample, SampleFilter, SampleStats,
        SampleStatsExtended, SampleType, UpdateSampleRequest,
    },
    repositories::SampleRepository,
    response::{ApiResponse, ResponseCode},
    storage::{MinioStorage, Storage},
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// 应用状态
#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Option<Database>,
    pub storage: Option<MinioStorage>,
    pub file_processor: Option<FileProcessor>,
    pub cape_manager: Option<crate::services::CapeManager>,
    pub config: crate::config::Config,
}

/// 文件上传请求元数据
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadMetadata {
    /// 样本类型（必填）：Benign（安全）或 Malicious（恶意）
    pub sample_type: SampleType,
    /// 标签列表，用于标记样本特征（如：trojan、ransomware、backdoor等）
    pub labels: Option<Vec<String>>,
    /// 样本来源，记录样本的获取渠道（如：VirusTotal、用户上传、蜜罐等）
    pub source: Option<String>,
    /// 自定义元数据，可存储任意JSON格式的结构化数据
    pub custom_metadata: Option<serde_json::Value>,
    /// ZIP文件密码列表，系统会依次尝试这些密码来解压加密的ZIP文件
    pub passwords: Option<Vec<String>>,
}

/// 文件上传响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadResponse {
    /// 新创建或已存在样本的ID
    pub sample_id: Uuid,
    /// 上传的文件名
    pub filename: String,
    /// 文件大小（字节）
    pub file_size: i64,
    /// 文件MIME类型
    pub file_type: String,
    /// 文件的MD5哈希值
    pub md5: String,
    /// 文件的SHA256哈希值
    pub sha256: String,
    /// 是否为重复文件（基于SHA256判断）
    pub is_duplicate: bool,
    /// 如果是重复文件，返回已存在样本的ID
    pub duplicate_sample_id: Option<Uuid>,
    /// 如果是容器文件（如ZIP），返回其中包含的文件数量
    pub sub_files_count: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchDeleteRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchDeleteResponse {
    pub total: usize,
    pub deleted: Vec<Uuid>,
    pub failed: Vec<(Uuid, String)>,
}

/// 样本查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct SampleQueryParams {
    // 分页参数
    /// 页码，从1开始，默认为1
    pub page: Option<u32>,
    /// 每页数量，默认20，最大100
    pub page_size: Option<u32>,

    // 过滤参数
    /// 样本类型筛选：Benign（安全）或 Malicious（恶意）
    pub sample_type: Option<SampleType>,
    /// 样本来源筛选，精确匹配
    pub source: Option<String>,
    /// 文件名模糊查询，不区分大小写
    pub filename: Option<String>,
    /// MD5哈希值精确查询（32位十六进制字符串）
    pub md5: Option<String>,
    /// SHA1哈希值精确查询（40位十六进制字符串）
    pub sha1: Option<String>,
    /// SHA256哈希值精确查询（64位十六进制字符串）
    pub sha256: Option<String>,
    /// 是否为容器文件（如ZIP、RAR等）
    pub is_container: Option<bool>,
    /// 父样本ID（用于查询从特定ZIP文件中提取的文件）
    pub parent_id: Option<Uuid>,
    /// 标签筛选，多个标签用逗号分隔，匹配任一标签即可
    pub labels: Option<String>,
    /// 创建时间范围开始，ISO 8601格式（如：2024-01-01T00:00:00Z）
    pub start_time: Option<String>,
    /// 创建时间范围结束，ISO 8601格式
    pub end_time: Option<String>,
}

/// 完整的文件上传处理器
///
/// 支持上传单个文件或ZIP压缩包，自动解析文件类型、计算哈希值、提取ZIP内容。
/// 如果是ZIP文件，会递归解析其中的文件结构。
#[utoipa::path(
    post,
    path = "/api/samples/upload",
    tag = "samples",
    request_body = UploadMetadata,
    responses(
        (status = 200, description = "上传成功", body = UploadResponse),
        (status = 400, description = "请求参数错误（缺少必填字段、格式错误或文件超限）"),
        (status = 409, description = "文件已存在（基于SHA256哈希值判断）"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn upload_file_full(
    State(app_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<UploadResponse>>, AppError> {
    // 检查必要的服务是否可用
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;
    let storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储服务不可用"))?;
    let file_processor = app_state
        .file_processor
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("文件处理服务不可用"))?;
    let config = &app_state.config;

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut upload_metadata: Option<UploadMetadata> = None;

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

        match field_name.as_str() {
            "file" => {
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
            "metadata" => {
                let metadata_json = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("读取元数据失败: {}", e)))?;
                upload_metadata =
                    Some(serde_json::from_str(&metadata_json).map_err(|e| {
                        AppError::bad_request(format!("解析元数据JSON失败: {}", e))
                    })?);
            }
            _ => {
                // 忽略未知字段
            }
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::bad_request("缺少文件数据"))?;
    let filename = filename.ok_or_else(|| AppError::bad_request("缺少文件名"))?;
    let upload_metadata = upload_metadata.ok_or_else(|| AppError::bad_request("缺少元数据"))?;

    // 检查文件大小
    let max_size = config.file.max_size as usize;
    if file_data.len() > max_size {
        let file_size_mb = file_data.len() / (1024 * 1024);
        let max_size_mb = max_size / (1024 * 1024);
        return Err(AppError::bad_request(format!(
            "文件过大: {} MB，最大允许 {} MB",
            file_size_mb, max_size_mb
        )));
    }

    // 处理文件
    let file_info = file_processor
        .process_file_with_passwords(
            &file_data,
            &filename,
            upload_metadata.passwords.as_deref().unwrap_or(&[]),
        )
        .await?;

    // 检查文件是否已存在（去重）
    let repo = SampleRepository::new(database.clone());
    let existing_sample = repo
        .find_by_hash(&file_info.hashes.md5, &file_info.hashes.sha256)
        .await?;

    if let Some(existing) = existing_sample {
        return Ok(Json(ApiResponse::error_with_data(
            ResponseCode::DUPLICATE_FILE,
            "文件已存在".to_string(),
            UploadResponse {
                sample_id: existing.id,
                filename: existing.file_name,
                file_size: existing.file_size,
                file_type: existing.file_type,
                md5: existing.file_hash_md5,
                sha256: existing.file_hash_sha256,
                is_duplicate: true,
                duplicate_sample_id: Some(existing.id),
                sub_files_count: None,
            },
        )));
    }

    // 生成存储路径
    let sample_id = Uuid::new_v4();
    let storage_path = format!(
        "samples/{}/{}",
        chrono::Utc::now().format("%Y/%m/%d"),
        sample_id
    );

    // 上传到MinIO
    storage
        .upload(
            "samplefarm",
            &storage_path,
            &file_data,
            Some(&file_info.file_info.mime_type),
        )
        .await?;

    // 创建样本记录
    let create_request = CreateSampleRequest {
        file_name: filename.clone(),
        file_size: file_info.file_info.size as i64,
        file_hash_md5: file_info.hashes.md5.clone(),
        file_hash_sha1: file_info.hashes.sha1.clone(),
        file_hash_sha256: file_info.hashes.sha256.clone(),
        file_type: file_info.file_info.mime_type.clone(),
        file_extension: file_info.file_info.extension.clone(),
        sample_type: upload_metadata.sample_type,
        source: upload_metadata.source.clone(),
        storage_path,
        is_container: file_info.file_info.is_container,
        parent_id: None,
        file_path_in_zip: None,
        has_custom_metadata: upload_metadata.custom_metadata.is_some(),
        labels: upload_metadata.labels.clone(),
        custom_metadata: upload_metadata.custom_metadata.clone(),
        zip_password: upload_metadata
            .passwords
            .as_ref()
            .and_then(|p| p.first().cloned()),
        run_filename: None, // 可以从ZIP文件内容中检测或由用户指定
    };

    let sample = repo.create(create_request).await?;
    let mut sub_files_count = 0;

    // 如果是容器文件，处理子文件
    if let Some(sub_files) = file_info.sub_files {
        sub_files_count = sub_files.len();

        for sub_file in sub_files {
            let sub_sample_id = Uuid::new_v4();
            let sub_storage_path = format!(
                "samples/{}/sub_files/{}",
                chrono::Utc::now().format("%Y/%m/%d"),
                sub_sample_id
            );

            // 上传子文件到MinIO
            storage
                .upload(
                    "samplefarm",
                    &sub_storage_path,
                    &sub_file.data,
                    Some(&sub_file.mime_type),
                )
                .await?;

            // 创建子文件记录
            let sub_create_request = CreateSampleRequest {
                file_name: sub_file.filename.clone(),
                file_size: sub_file.uncompressed_size as i64,
                file_hash_md5: sub_file.hashes.md5.clone(),
                file_hash_sha1: sub_file.hashes.sha1.clone(),
                file_hash_sha256: sub_file.hashes.sha256.clone(),
                file_type: sub_file.mime_type.clone(),
                file_extension: sub_file.extension.clone(),
                sample_type: upload_metadata.sample_type,
                source: upload_metadata
                    .source
                    .clone()
                    .map(|s| format!("{} (来自ZIP: {})", s, filename)),
                storage_path: sub_storage_path,
                is_container: false,
                parent_id: Some(sample.id),
                file_path_in_zip: Some(sub_file.path_in_zip),
                has_custom_metadata: false,
                labels: upload_metadata.labels.clone(),
                custom_metadata: None,
                zip_password: None,
                run_filename: None,
            };

            repo.create(sub_create_request).await?;
        }
    }

    tracing::info!(
        "成功上传样本: {} (ID: {}, 子文件数: {})",
        filename,
        sample_id,
        sub_files_count
    );

    Ok(Json(ApiResponse::success(UploadResponse {
        sample_id: sample.id,
        filename,
        file_size: file_info.file_info.size as i64,
        file_type: file_info.file_info.mime_type,
        md5: file_info.hashes.md5,
        sha256: file_info.hashes.sha256,
        is_duplicate: false,
        duplicate_sample_id: None,
        sub_files_count: if sub_files_count > 0 {
            Some(sub_files_count)
        } else {
            None
        },
    })))
}

/// 查询样本列表
///
/// 支持分页和多条件筛选的样本查询接口。返回样本列表和分页信息。
#[utoipa::path(
    get,
    path = "/api/samples",
    tag = "samples",
    params(
        ("page" = Option<u32>, Query, description = "页码，从1开始，默认为1"),
        ("page_size" = Option<u32>, Query, description = "每页数量，默认20，最大100"),
        ("sample_type" = Option<String>, Query, description = "样本类型筛选：Benign（安全）或 Malicious（恶意）"),
        ("source" = Option<String>, Query, description = "样本来源筛选，精确匹配"),
        ("filename" = Option<String>, Query, description = "文件名模糊查询，不区分大小写"),
        ("md5" = Option<String>, Query, description = "MD5哈希值精确查询（32位十六进制字符串）"),
        ("sha256" = Option<String>, Query, description = "SHA256哈希值精确查询（64位十六进制字符串）"),
        ("is_container" = Option<bool>, Query, description = "是否为容器文件（如ZIP、RAR等）"),
        ("labels" = Option<String>, Query, description = "标签筛选，多个标签用逗号分隔，匹配任一标签即可"),
        ("start_time" = Option<String>, Query, description = "创建时间范围开始，ISO 8601格式（如：2024-01-01T00:00:00Z）"),
        ("end_time" = Option<String>, Query, description = "创建时间范围结束，ISO 8601格式")
    ),
    responses(
        (status = 200, description = "查询成功"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn list_samples(
    State(app_state): State<AppState>,
    Query(params): Query<SampleQueryParams>,
) -> Result<Json<ApiResponse<PagedResult<Sample>>>, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;

    let repo = SampleRepository::new(database.clone());

    // 构建分页参数
    let pagination = Pagination {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20).min(100), // 限制最大页面大小
    };

    // 构建过滤条件
    let mut filter = SampleFilter::default();
    filter.sample_type = params.sample_type;
    filter.source = params.source;
    filter.filename = params.filename;
    filter.md5 = params.md5;
    filter.sha1 = params.sha1;
    filter.sha256 = params.sha256;
    filter.is_container = params.is_container;
    filter.parent_id = params.parent_id;

    // 解析标签
    if let Some(labels_str) = params.labels {
        filter.labels = Some(
            labels_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        );
    }

    // 解析时间范围
    if let Some(start_time_str) = params.start_time {
        filter.start_time = Some(
            chrono::DateTime::parse_from_rfc3339(&start_time_str)
                .map_err(|e| AppError::bad_request(format!("无效的开始时间格式: {}", e)))?
                .with_timezone(&chrono::Utc),
        );
    }

    if let Some(end_time_str) = params.end_time {
        filter.end_time = Some(
            chrono::DateTime::parse_from_rfc3339(&end_time_str)
                .map_err(|e| AppError::bad_request(format!("无效的结束时间格式: {}", e)))?
                .with_timezone(&chrono::Utc),
        );
    }

    let result = repo.list(filter, &pagination).await?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取样本详情
///
/// 根据样本ID获取完整的样本信息，包括文件属性、哈希值、标签、元数据等。
#[utoipa::path(
    get,
    path = "/api/samples/{id}",
    tag = "samples",
    params(
        ("id" = Uuid, Path, description = "样本唯一标识符（UUID格式）")
    ),
    responses(
        (status = 200, description = "查询成功", body = Sample),
        (status = 404, description = "样本不存在"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn get_sample(
    State(app_state): State<AppState>,
    Path(sample_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Sample>>, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;

    let repo = SampleRepository::new(database.clone());
    let sample = repo
        .find_by_id(sample_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("样本 {} 不存在", sample_id)))?;

    Ok(Json(ApiResponse::success(sample)))
}

/// 更新样本
///
/// 根据样本ID更新样本的元数据信息，包括类型、标签、来源等。
#[utoipa::path(
    put,
    path = "/api/samples/{id}",
    tag = "samples",
    params(
        ("id" = Uuid, Path, description = "样本唯一标识符（UUID格式）")
    ),
    request_body(
        content = UpdateSampleRequest,
        description = "要更新的样本信息",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "更新成功", body = Sample),
        (status = 404, description = "样本不存在"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn update_sample(
    State(app_state): State<AppState>,
    Path(sample_id): Path<Uuid>,
    Json(update_request): Json<UpdateSampleRequest>,
) -> Result<Json<ApiResponse<Sample>>, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;

    let repo = SampleRepository::new(database.clone());

    // 检查样本是否存在
    let _existing = repo
        .find_by_id(sample_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("样本 {} 不存在", sample_id)))?;

    let updated_sample = repo.update(sample_id, update_request).await?;

    tracing::info!("成功更新样本: {}", sample_id);

    Ok(Json(ApiResponse::success(updated_sample)))
}

/// 删除样本
///
/// 根据样本ID删除样本，同时删除存储中的文件和数据库记录。
#[utoipa::path(
    delete,
    path = "/api/samples/{id}",
    tag = "samples",
    params(
        ("id" = Uuid, Path, description = "样本唯一标识符（UUID格式）")
    ),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "样本不存在"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn delete_sample(
    State(app_state): State<AppState>,
    Path(sample_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;
    let storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储服务不可用"))?;

    let repo = SampleRepository::new(database.clone());

    // 获取样本信息
    let sample = repo
        .find_by_id(sample_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("样本 {} 不存在", sample_id)))?;

    // 删除存储中的文件
    if let Err(e) = storage.delete("samplefarm", &sample.storage_path).await {
        tracing::warn!("删除存储文件失败: {}", e);
        // 继续删除数据库记录，即使存储删除失败
    }

    // 删除数据库记录
    repo.delete(sample_id).await?;

    tracing::info!("成功删除样本: {}", sample_id);

    Ok(Json(ApiResponse::success(())))
}

/// 批量删除样本
#[utoipa::path(
    delete,
    path = "/api/samples/batch",
    request_body = BatchDeleteRequest,
    responses(
        (status = 200, description = "批量删除完成", body = ApiResponse<BatchDeleteResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "samples"
)]
pub async fn batch_delete_samples(
    State(app_state): State<AppState>,
    Json(req): Json<BatchDeleteRequest>,
) -> Result<Json<ApiResponse<BatchDeleteResponse>>, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;
    let storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储服务不可用"))?;

    let repo = SampleRepository::new(database.clone());
    let mut deleted = Vec::new();
    let mut failed = Vec::new();

    for id in req.ids {
        match repo.find_by_id(id).await {
            Ok(Some(sample)) => {
                if let Err(e) = storage.delete("samplefarm", &sample.storage_path).await {
                    tracing::warn!(sample_id=%id, err=%e, "删除存储文件失败，继续删除数据库记录");
                }
                match repo.delete(id).await {
                    Ok(_) => deleted.push(id),
                    Err(e) => failed.push((id, format!("删除数据库记录失败: {}", e))),
                }
            }
            Ok(None) => failed.push((id, "样本不存在".to_string())),
            Err(e) => failed.push((id, format!("查询样本失败: {}", e))),
        }
    }

    Ok(Json(ApiResponse::success(BatchDeleteResponse {
        total: deleted.len() + failed.len(),
        deleted,
        failed,
    })))
}

/// 获取样本统计信息
///
/// 获取系统中所有样本的统计信息，包括总数量、类型分布、容器文件数量和总存储大小等。
#[utoipa::path(
    get,
    path = "/api/samples/stats",
    tag = "samples",
    responses(
        (status = 200, description = "查询成功", body = SampleStats),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn get_sample_stats(
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<SampleStats>>, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;

    let repo = SampleRepository::new(database.clone());
    let stats = repo.get_stats().await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 获取扩展样本统计信息
///
/// 获取详细的样本统计信息，包括文件类型分布、大小分布、来源分布和上传趋势等。
#[utoipa::path(
    get,
    path = "/api/samples/stats/extended",
    tag = "samples",
    responses(
        (status = 200, description = "查询成功", body = SampleStatsExtended),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn get_sample_stats_extended(
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<SampleStatsExtended>>, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;

    let repo = SampleRepository::new(database.clone());
    let stats = repo.get_stats_extended().await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 下载样本文件
///
/// 根据样本ID下载原始文件。返回文件的二进制数据流。
#[utoipa::path(
    get,
    path = "/api/samples/{id}/download",
    tag = "samples",
    params(
        ("id" = Uuid, Path, description = "样本唯一标识符（UUID格式）")
    ),
    responses(
        (status = 200, description = "下载成功", content_type = "application/octet-stream"),
        (status = 404, description = "样本不存在"),
        (status = 500, description = "服务器内部错误")
    )
)]
pub async fn download_sample(
    State(app_state): State<AppState>,
    Path(sample_id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    let database = app_state
        .database
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("数据库服务不可用"))?;
    let storage = app_state
        .storage
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("存储服务不可用"))?;

    let repo = SampleRepository::new(database.clone());
    let sample = repo
        .find_by_id(sample_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("样本 {} 不存在", sample_id)))?;

    // 从存储中下载文件
    let file_data = storage.download("samplefarm", &sample.storage_path).await?;

    // 构建下载响应
    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", sample.file_name),
        )
        .header("Content-Length", file_data.len().to_string())
        .body(axum::body::Body::from(file_data))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("构建响应失败: {}", e)))?;

    tracing::info!("下载样本文件: {} ({})", sample.file_name, sample_id);

    Ok(response)
}
