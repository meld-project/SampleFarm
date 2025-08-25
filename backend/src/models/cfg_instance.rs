use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// CFG实例状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CfgInstanceStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for CfgInstanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgInstanceStatus::Healthy => write!(f, "healthy"),
            CfgInstanceStatus::Unhealthy => write!(f, "unhealthy"),
            CfgInstanceStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for CfgInstanceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "healthy" => Ok(CfgInstanceStatus::Healthy),
            "unhealthy" => Ok(CfgInstanceStatus::Unhealthy),
            "unknown" => Ok(CfgInstanceStatus::Unknown),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

/// CFG实例模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CfgInstance {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub timeout_seconds: i32,
    pub max_concurrent_tasks: i32,
    pub health_check_interval: i32,
    #[sqlx(try_from = "String")]
    pub status: CfgInstanceStatus,
    pub last_health_check: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建CFG实例请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCfgInstanceRequest {
    pub name: String,
    pub base_url: String,
    pub description: Option<String>,
    pub timeout_seconds: Option<i32>,
    pub max_concurrent_tasks: Option<i32>,
    pub health_check_interval: Option<i32>,
}

/// 更新CFG实例请求
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateCfgInstanceRequest {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub timeout_seconds: Option<i32>,
    pub max_concurrent_tasks: Option<i32>,
    pub health_check_interval: Option<i32>,
}

/// CFG实例健康状态响应
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CfgHealthStatus {
    pub instance_id: Uuid,
    pub instance_name: String,
    pub status: CfgInstanceStatus,
    pub response_time_ms: Option<u64>,
    pub checked_at: DateTime<Utc>,
    pub error_message: Option<String>,
}

impl sqlx::decode::Decode<'_, sqlx::Postgres> for CfgInstanceStatus {
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

impl sqlx::encode::Encode<'_, sqlx::Postgres> for CfgInstanceStatus {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync + 'static>> {
        <String as sqlx::encode::Encode<sqlx::Postgres>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl sqlx::Type<sqlx::Postgres> for CfgInstanceStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl TryFrom<String> for CfgInstanceStatus {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl CfgInstance {
    /// 检查实例是否可用（启用且健康）
    pub fn is_available(&self) -> bool {
        self.enabled
            && matches!(
                self.status,
                CfgInstanceStatus::Healthy | CfgInstanceStatus::Unknown
            )
    }
}

impl CreateCfgInstanceRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("实例名称不能为空".to_string());
        }
        if self.base_url.trim().is_empty() {
            return Err("CFG API URL不能为空".to_string());
        }
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err("CFG API URL必须以http://或https://开头".to_string());
        }
        if let Some(t) = self.timeout_seconds {
            if t <= 0 {
                return Err("超时时间必须大于0".to_string());
            }
        }
        if let Some(c) = self.max_concurrent_tasks {
            if c <= 0 {
                return Err("最大并发任务数必须大于0".to_string());
            }
        }
        if let Some(h) = self.health_check_interval {
            if h <= 0 {
                return Err("健康检查间隔必须大于0".to_string());
            }
        }
        Ok(())
    }
}
