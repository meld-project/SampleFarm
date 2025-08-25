use super::{FileMetadata, Storage};
use crate::{
    config::MinioConfig,
    error::{AppError, AppResult},
};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{
    Client,
    config::Credentials,
    primitives::ByteStream,
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};
use std::sync::Arc;

/// MinIO存储实现
#[derive(Debug, Clone)]
pub struct MinioStorage {
    client: Arc<Client>,
    #[allow(dead_code)]
    config: MinioConfig,
}

impl MinioStorage {
    /// 创建新的MinIO存储实例
    pub async fn new(config: MinioConfig) -> AppResult<Self> {
        // 创建自定义凭证
        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,    // session token
            None,    // expiration
            "minio", // provider name
        );

        // 构建S3配置
        let s3_config = aws_sdk_s3::Config::builder()
            .endpoint_url(&config.endpoint)
            .credentials_provider(credentials)
            .region(Region::new("us-east-1")) // MinIO默认区域
            .force_path_style(true) // MinIO需要路径样式
            .behavior_version(BehaviorVersion::latest())
            .build();

        let client = Client::from_conf(s3_config);

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    /// 确保bucket存在
    pub async fn ensure_bucket(&self, bucket: &str) -> AppResult<()> {
        match self.client.head_bucket().bucket(bucket).send().await {
            Ok(_) => {
                tracing::debug!("Bucket '{}' 已存在", bucket);
                Ok(())
            }
            Err(_) => {
                tracing::info!("Bucket '{}' 不存在，正在创建", bucket);
                self.create_bucket(bucket).await
            }
        }
    }

    /// 创建bucket
    async fn create_bucket(&self, bucket: &str) -> AppResult<()> {
        let create_bucket_config = CreateBucketConfiguration::builder()
            .location_constraint(BucketLocationConstraint::UsEast2)
            .build();

        self.client
            .create_bucket()
            .bucket(bucket)
            .create_bucket_configuration(create_bucket_config)
            .send()
            .await
            .map_err(|e| AppError::Storage(format!("创建bucket失败: {}", e)))?;

        tracing::info!("成功创建bucket: {}", bucket);
        Ok(())
    }

    /// 健康检查
    pub async fn health_check(&self) -> AppResult<bool> {
        match self.client.list_buckets().send().await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("MinIO健康检查失败: {}", e);
                Ok(false)
            }
        }
    }
}

#[async_trait::async_trait]
impl Storage for MinioStorage {
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<&str>,
    ) -> AppResult<String> {
        // 确保bucket存在
        self.ensure_bucket(bucket).await?;

        let mut request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(data.to_vec()));

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        let result = request
            .send()
            .await
            .map_err(|e| AppError::Storage(format!("上传文件失败: {}", e)))?;

        let etag = result.e_tag().unwrap_or("").to_string();
        tracing::info!("成功上传文件到MinIO: {}/{}, ETag: {}", bucket, key, etag);

        Ok(etag)
    }

    async fn download(&self, bucket: &str, key: &str) -> AppResult<Vec<u8>> {
        let result = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::Storage(format!("下载文件失败: {}", e)))?;

        let data = result
            .body
            .collect()
            .await
            .map_err(|e| AppError::Storage(format!("读取文件数据失败: {}", e)))?;

        Ok(data.to_vec())
    }

    async fn delete(&self, bucket: &str, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::Storage(format!("删除文件失败: {}", e)))?;

        tracing::info!("成功删除文件: {}/{}", bucket, key);
        Ok(())
    }

    async fn exists(&self, bucket: &str, key: &str) -> AppResult<bool> {
        match self
            .client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(err) => {
                let service_err = err.into_service_error();
                if service_err.is_not_found() {
                    Ok(false)
                } else {
                    Err(AppError::Storage(format!(
                        "检查文件是否存在失败: {}",
                        service_err
                    )))
                }
            }
        }
    }

    async fn get_metadata(&self, bucket: &str, key: &str) -> AppResult<FileMetadata> {
        let result = self
            .client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::Storage(format!("获取文件元数据失败: {}", e)))?;

        Ok(FileMetadata {
            size: result.content_length().unwrap_or(0) as u64,
            content_type: result.content_type().map(|s| s.to_string()),
            last_modified: result
                .last_modified()
                .map(|dt| chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos()).unwrap())
                .unwrap_or_else(chrono::Utc::now),
            etag: result.e_tag().map(|s| s.to_string()),
        })
    }

    async fn presigned_url(
        &self,
        bucket: &str,
        key: &str,
        expires_in_secs: u64,
    ) -> AppResult<String> {
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(
            std::time::Duration::from_secs(expires_in_secs),
        )
        .map_err(|e| AppError::Storage(format!("预签名配置错误: {}", e)))?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| AppError::Storage(format!("生成预签名URL失败: {}", e)))?;

        Ok(presigned_request.uri().to_string())
    }
}
