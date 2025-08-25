pub mod analyzer;
pub mod cape_instance;
pub mod cape_result;
pub mod cfg_instance;
pub mod cfg_result;
pub mod sample;
pub mod task;

pub use analyzer::*;
pub use cape_instance::*;
pub use cape_result::*;
pub use cfg_instance::*;
pub use cfg_result::*;
pub use sample::*;
pub use task::*;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 分页参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    #[serde(deserialize_with = "deserialize_string_to_u32")]
    pub page: u32,
    #[serde(deserialize_with = "deserialize_string_to_u32")]
    pub page_size: u32,
}

fn deserialize_string_to_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringOrU32Visitor;

    impl<'de> Visitor<'de> for StringOrU32Visitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or u32")
        }

        fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::try_from(value).map_err(|_| E::custom(format!("u32 overflow: {}", value)))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(E::custom)
        }
    }

    deserializer.deserialize_any(StringOrU32Visitor)
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

/// 分页结果
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

impl<T> PagedResult<T> {
    pub fn new(items: Vec<T>, total: i64, page: u32, page_size: u32) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            ((total as f64) / (page_size as f64)).ceil() as u32
        };

        Self {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

/// 时间戳字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 实体通用trait
pub trait Entity {
    type Id;
    fn id(&self) -> Self::Id;
}
