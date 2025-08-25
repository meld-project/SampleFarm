pub mod minio;

pub use minio::MinioStorage;

use crate::error::AppResult;

/// 存储抽象接口
#[async_trait::async_trait]
pub trait Storage {
    /// 上传文件
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<&str>,
    ) -> AppResult<String>;

    /// 下载文件
    async fn download(&self, bucket: &str, key: &str) -> AppResult<Vec<u8>>;

    /// 删除文件
    async fn delete(&self, bucket: &str, key: &str) -> AppResult<()>;

    /// 检查文件是否存在
    async fn exists(&self, bucket: &str, key: &str) -> AppResult<bool>;

    /// 获取文件信息
    async fn get_metadata(&self, bucket: &str, key: &str) -> AppResult<FileMetadata>;

    /// 生成预签名URL（用于直接下载）
    async fn presigned_url(
        &self,
        bucket: &str,
        key: &str,
        expires_in_secs: u64,
    ) -> AppResult<String>;
}

/// 文件元数据
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub content_type: Option<String>,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub etag: Option<String>,
}
