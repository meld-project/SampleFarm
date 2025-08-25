use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 统一API响应格式
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// 业务响应码
    pub code: i32,
    /// 响应消息（中文）
    pub msg: String,
    /// 响应数据
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: ResponseCode::SUCCESS,
            msg: "操作成功".to_string(),
            data: Some(data),
        }
    }

    /// 创建成功响应（自定义消息）
    pub fn success_with_message(data: T, msg: String) -> Self {
        Self {
            code: ResponseCode::SUCCESS,
            msg,
            data: Some(data),
        }
    }

    /// 创建成功响应（无数据）
    pub fn success_empty() -> ApiResponse<()> {
        ApiResponse {
            code: ResponseCode::SUCCESS,
            msg: "操作成功".to_string(),
            data: None,
        }
    }

    /// 创建错误响应
    pub fn error(code: i32, msg: String) -> ApiResponse<()> {
        ApiResponse {
            code,
            msg,
            data: None,
        }
    }

    /// 创建错误响应（带数据）
    pub fn error_with_data(code: i32, msg: String, data: T) -> Self {
        Self {
            code,
            msg,
            data: Some(data),
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // 根据业务响应码确定HTTP状态码
        let status = match self.code {
            ResponseCode::SUCCESS => StatusCode::OK,
            ResponseCode::BAD_REQUEST => StatusCode::BAD_REQUEST,
            ResponseCode::NOT_FOUND => StatusCode::NOT_FOUND,
            ResponseCode::FILE_TOO_LARGE => StatusCode::PAYLOAD_TOO_LARGE,
            ResponseCode::UNSUPPORTED_FILE_TYPE => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

/// 业务响应码常量
pub struct ResponseCode;

impl ResponseCode {
    /// 成功响应码
    pub const SUCCESS: i32 = 200;

    /// 客户端错误 4xx
    pub const BAD_REQUEST: i32 = 400;
    pub const NOT_FOUND: i32 = 404;
    pub const DUPLICATE_FILE: i32 = 409; // Conflict
    pub const FILE_TOO_LARGE: i32 = 413;
    pub const UNSUPPORTED_FILE_TYPE: i32 = 415;

    /// 服务器错误 5xx
    pub const INTERNAL_ERROR: i32 = 500;
    pub const DATABASE_ERROR: i32 = 501;
    pub const STORAGE_ERROR: i32 = 502;
    pub const FILE_PROCESSING_ERROR: i32 = 503;
}

/// 响应码对应的默认消息
impl ResponseCode {
    pub fn get_message(code: i32) -> &'static str {
        match code {
            Self::SUCCESS => "操作成功",
            Self::BAD_REQUEST => "请求参数错误",
            Self::NOT_FOUND => "资源不存在",
            Self::FILE_TOO_LARGE => "文件过大",
            Self::UNSUPPORTED_FILE_TYPE => "不支持的文件类型",
            Self::INTERNAL_ERROR => "服务器内部错误",
            Self::DATABASE_ERROR => "数据库错误",
            Self::STORAGE_ERROR => "存储服务错误",
            Self::FILE_PROCESSING_ERROR => "文件处理错误",
            _ => "未知错误",
        }
    }
}

/// 便捷的响应构造宏
#[macro_export]
macro_rules! ok_response {
    ($data:expr) => {
        $crate::response::ApiResponse::success($data)
    };
    () => {
        $crate::response::ApiResponse::success_empty()
    };
}

#[macro_export]
macro_rules! err_response {
    ($code:expr) => {
        $crate::response::ApiResponse::error(
            $code,
            $crate::response::ResponseCode::get_message($code).to_string(),
        )
    };
    ($code:expr, $msg:expr) => {
        $crate::response::ApiResponse::error($code, $msg.to_string())
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_success_response() {
        let response = ApiResponse::success(json!({"id": 1, "name": "test"}));
        assert_eq!(response.code, 200);
        assert_eq!(response.msg, "操作成功");
        assert!(response.data.is_some());
    }

    #[test]
    fn test_error_response() {
        let response = ApiResponse::<()>::error(400, "测试错误".to_string());
        assert_eq!(response.code, 400);
        assert_eq!(response.msg, "测试错误");
        assert!(response.data.is_none());
    }

    #[test]
    fn test_response_code_message() {
        assert_eq!(ResponseCode::get_message(200), "操作成功");
        assert_eq!(ResponseCode::get_message(404), "资源不存在");
        assert_eq!(ResponseCode::get_message(999), "未知错误");
    }
}
