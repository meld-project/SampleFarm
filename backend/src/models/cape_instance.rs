use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// CAPE实例状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CapeInstanceStatus {
    /// 健康
    Healthy,
    /// 不健康
    Unhealthy,
    /// 未知状态
    Unknown,
}

impl std::fmt::Display for CapeInstanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapeInstanceStatus::Healthy => write!(f, "healthy"),
            CapeInstanceStatus::Unhealthy => write!(f, "unhealthy"),
            CapeInstanceStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for CapeInstanceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "healthy" => Ok(CapeInstanceStatus::Healthy),
            "unhealthy" => Ok(CapeInstanceStatus::Unhealthy),
            "unknown" => Ok(CapeInstanceStatus::Unknown),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

/// CAPE实例模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CapeInstance {
    /// 实例ID
    pub id: Uuid,
    /// 实例名称
    pub name: String,
    /// CAPE API基础URL
    pub base_url: String,
    /// 描述信息
    pub description: Option<String>,
    /// 是否启用
    pub enabled: bool,
    /// 超时时间（秒）
    pub timeout_seconds: i32,
    /// 最大并发任务数
    pub max_concurrent_tasks: i32,
    /// 健康检查间隔（秒）
    pub health_check_interval: i32,
    /// 健康状态
    #[sqlx(try_from = "String")]
    pub status: CapeInstanceStatus,
    /// 最后健康检查时间
    pub last_health_check: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 创建CAPE实例请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCapeInstanceRequest {
    /// 实例名称
    pub name: String,
    /// CAPE API基础URL
    pub base_url: String,
    /// 描述信息
    pub description: Option<String>,
    /// 超时时间（秒），默认300
    pub timeout_seconds: Option<i32>,
    /// 最大并发任务数，默认5
    pub max_concurrent_tasks: Option<i32>,
    /// 健康检查间隔（秒），默认60
    pub health_check_interval: Option<i32>,
}

/// 更新CAPE实例请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateCapeInstanceRequest {
    /// 实例名称
    pub name: Option<String>,
    /// CAPE API基础URL
    pub base_url: Option<String>,
    /// 描述信息
    pub description: Option<String>,
    /// 是否启用
    pub enabled: Option<bool>,
    /// 超时时间（秒）
    pub timeout_seconds: Option<i32>,
    /// 最大并发任务数
    pub max_concurrent_tasks: Option<i32>,
    /// 健康检查间隔（秒）
    pub health_check_interval: Option<i32>,
}

/// CAPE实例健康状态响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CapeHealthStatus {
    /// 实例ID
    pub instance_id: Uuid,
    /// 实例名称
    pub instance_name: String,
    /// 健康状态
    pub status: CapeInstanceStatus,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u64>,
    /// 检查时间
    pub checked_at: DateTime<Utc>,
    /// 错误消息（如果不健康）
    pub error_message: Option<String>,
}

/// CAPE实例统计信息
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CapeInstanceStats {
    /// 实例ID
    pub instance_id: Uuid,
    /// 总任务数
    pub total_tasks: i64,
    /// 成功任务数
    pub successful_tasks: i64,
    /// 失败任务数
    pub failed_tasks: i64,
    /// 平均处理时间（秒）
    pub average_processing_time: Option<f64>,
    /// 成功率
    pub success_rate: f64,
    /// 统计时间范围开始
    pub period_start: DateTime<Utc>,
    /// 统计时间范围结束
    pub period_end: DateTime<Utc>,
}

/// CAPE实例列表响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CapeInstanceListResponse {
    /// CAPE实例列表
    pub instances: Vec<CapeInstance>,
    /// 总数
    pub total: usize,
}

// 为数据库查询实现转换
impl sqlx::decode::Decode<'_, sqlx::Postgres> for CapeInstanceStatus {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <String as sqlx::decode::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                as Box<dyn std::error::Error + 'static + Send + Sync>
        })
    }
}

impl sqlx::encode::Encode<'_, sqlx::Postgres> for CapeInstanceStatus {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {
        <String as sqlx::encode::Encode<sqlx::Postgres>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl sqlx::Type<sqlx::Postgres> for CapeInstanceStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl TryFrom<String> for CapeInstanceStatus {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl CapeInstance {
    /// 检查实例是否可用
    pub fn is_available(&self) -> bool {
        self.enabled && self.status == CapeInstanceStatus::Healthy
    }

    /// 检查是否需要健康检查
    pub fn needs_health_check(&self) -> bool {
        match self.last_health_check {
            None => true,
            Some(last_check) => {
                let interval = chrono::Duration::seconds(self.health_check_interval as i64);
                Utc::now() - last_check > interval
            }
        }
    }
}

impl CreateCapeInstanceRequest {
    /// 验证请求参数
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("实例名称不能为空".to_string());
        }

        if self.base_url.trim().is_empty() {
            return Err("CAPE API URL不能为空".to_string());
        }

        // 验证URL格式
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err("CAPE API URL必须以http://或https://开头".to_string());
        }

        if let Some(timeout) = self.timeout_seconds {
            if timeout <= 0 {
                return Err("超时时间必须大于0".to_string());
            }
        }

        if let Some(max_concurrent) = self.max_concurrent_tasks {
            if max_concurrent <= 0 {
                return Err("最大并发任务数必须大于0".to_string());
            }
        }

        if let Some(interval) = self.health_check_interval {
            if interval <= 0 {
                return Err("健康检查间隔必须大于0".to_string());
            }
        }

        Ok(())
    }
}
