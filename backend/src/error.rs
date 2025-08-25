use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::response::{ApiResponse, ResponseCode};

/// 应用程序错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("文件处理错误: {0}")]
    FileProcessing(String),

    #[error("存储错误: {0}")]
    Storage(String),

    #[error("文件过大: 最大允许大小 {max_size} 字节")]
    FileTooLarge { max_size: u64 },

    #[error("不支持的文件类型: {file_type}")]
    UnsupportedFileType { file_type: String },

    #[error("文件未找到: {path}")]
    FileNotFound { path: String },

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("请求参数错误: {0}")]
    BadRequest(String),

    #[error("资源不存在: {resource}")]
    NotFound { resource: String },
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (code, message) = match &self {
            AppError::Database(_) => (ResponseCode::DATABASE_ERROR, self.to_string()),
            AppError::Serialization(_) => {
                (ResponseCode::INTERNAL_ERROR, "数据序列化错误".to_string())
            }
            AppError::Io(_) => (ResponseCode::INTERNAL_ERROR, "文件IO错误".to_string()),
            AppError::Config(_) => (ResponseCode::INTERNAL_ERROR, "配置错误".to_string()),
            AppError::Validation(msg) => (ResponseCode::BAD_REQUEST, msg.clone()),
            AppError::FileProcessing(_) => (ResponseCode::FILE_PROCESSING_ERROR, self.to_string()),
            AppError::Storage(_) => (ResponseCode::STORAGE_ERROR, self.to_string()),
            AppError::FileTooLarge { max_size } => (
                ResponseCode::FILE_TOO_LARGE,
                format!("文件过大，最大允许大小: {} MB", max_size / 1024 / 1024),
            ),
            AppError::UnsupportedFileType { file_type } => (
                ResponseCode::UNSUPPORTED_FILE_TYPE,
                format!("不支持的文件类型: {}", file_type),
            ),
            AppError::FileNotFound { path } => {
                (ResponseCode::NOT_FOUND, format!("文件未找到: {}", path))
            }
            AppError::Internal(_) => (ResponseCode::INTERNAL_ERROR, "服务器内部错误".to_string()),
            AppError::BadRequest(msg) => (ResponseCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound { resource } => {
                (ResponseCode::NOT_FOUND, format!("资源不存在: {}", resource))
            }
        };

        // 记录错误日志
        tracing::error!("应用错误: {}", self);

        ApiResponse::<()>::error(code, message).into_response()
    }
}

/// 应用程序Result类型别名
pub type AppResult<T> = Result<T, AppError>;

/// 错误构造辅助函数
impl AppError {
    pub fn validation<T: Into<String>>(msg: T) -> Self {
        Self::Validation(msg.into())
    }

    pub fn bad_request<T: Into<String>>(msg: T) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn not_found<T: Into<String>>(resource: T) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    pub fn file_not_found<T: Into<String>>(path: T) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    pub fn service_unavailable<T: Into<String>>(msg: T) -> Self {
        Self::Internal(anyhow::anyhow!(msg.into()))
    }

    pub fn file_too_large(max_size: u64) -> Self {
        Self::FileTooLarge { max_size }
    }

    pub fn unsupported_file_type<T: Into<String>>(file_type: T) -> Self {
        Self::UnsupportedFileType {
            file_type: file_type.into(),
        }
    }

    pub fn file_processing<T: Into<String>>(msg: T) -> Self {
        Self::FileProcessing(msg.into())
    }

    pub fn storage<T: Into<String>>(msg: T) -> Self {
        Self::Storage(msg.into())
    }

    pub fn config<T: Into<String>>(msg: T) -> Self {
        Self::Config(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = AppError::validation("测试验证错误");
        assert!(matches!(err, AppError::Validation(_)));
        assert_eq!(err.to_string(), "验证错误: 测试验证错误");
    }

    #[test]
    fn test_file_too_large_error() {
        let err = AppError::file_too_large(1024 * 1024); // 1MB
        assert!(matches!(err, AppError::FileTooLarge { .. }));
    }

    #[test]
    fn test_not_found_error() {
        let err = AppError::not_found("用户");
        assert!(matches!(err, AppError::NotFound { .. }));
    }
}
