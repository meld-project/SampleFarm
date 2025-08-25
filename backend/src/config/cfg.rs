use crate::config::cape::RetryConfig;
use serde::{Deserialize, Serialize};

/// CFG Sandbox 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgConfig {
    /// CFG API 基础URL
    pub base_url: String,
    /// 是否启用CFG集成
    pub enabled: bool,
    /// 最大并发任务数
    pub max_concurrent_tasks: u32,
    /// 默认轮询间隔（秒）
    pub default_poll_interval_secs: u64,
    /// 默认最大等待时间（秒）- 已废弃，保留用于兼容性
    #[deprecated(note = "超时机制已移除，此字段仅保留用于向后兼容")]
    pub default_max_wait_secs: u64,
    /// 默认标签
    pub default_label: i32,
    /// CFG结果存储桶名称
    pub result_bucket: String,
}

impl Default for CfgConfig {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            base_url: "http://localhost:17777".to_string(),
            enabled: false,
            max_concurrent_tasks: 2,
            default_poll_interval_secs: 10,
            #[allow(deprecated)]
            default_max_wait_secs: 1800, // 保留用于兼容性
            default_label: 0,
            result_bucket: "cfg-results".to_string(),
        }
    }
}

impl CfgConfig {
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("CFG base_url 不能为空".to_string());
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err("CFG base_url 必须以 http:// 或 https:// 开头".to_string());
        }

        if self.max_concurrent_tasks == 0 {
            return Err("最大并发任务数必须大于0".to_string());
        }

        if self.default_poll_interval_secs == 0 {
            return Err("轮询间隔必须大于0".to_string());
        }

        // 超时验证已移除，保留字段仅用于兼容性
        // if self.default_max_wait_secs == 0 {
        //     return Err("最大等待时间必须大于0".to_string());
        // }

        if self.result_bucket.is_empty() {
            return Err("CFG result_bucket 不能为空".to_string());
        }

        Ok(())
    }

    /// 获取完整的API URL
    pub fn get_api_url(&self, endpoint: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let endpoint = endpoint.trim_start_matches('/');
        format!("{}/{}", base, endpoint)
    }

    /// 检查是否启用CFG功能
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取默认的任务配置
    #[allow(deprecated)]
    pub fn get_default_task_config(&self) -> CfgTaskConfig {
        CfgTaskConfig {
            poll_interval_secs: self.default_poll_interval_secs,
            max_wait_secs: self.default_max_wait_secs, // 保留用于兼容性，实际不使用
            label: self.default_label,
            retry: Some(RetryConfig::default()),
        }
    }
}

/// CFG 任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgTaskConfig {
    /// 轮询间隔（秒）
    pub poll_interval_secs: u64,
    /// 最大等待时间（秒）- 已废弃，保留用于兼容性
    #[deprecated(note = "超时机制已移除，此字段仅保留用于向后兼容")]
    pub max_wait_secs: u64,
    /// 标签
    pub label: i32,
    /// 重试配置
    pub retry: Option<RetryConfig>,
}

impl Default for CfgTaskConfig {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            poll_interval_secs: 10,
            max_wait_secs: 0, // 超时机制已移除，设为0表示无超时
            label: 0,
            retry: Some(RetryConfig::default()),
        }
    }
}

/// CFG 任务配置请求
#[derive(Debug, Deserialize, Clone)]
pub struct CfgTaskConfigRequest {
    /// 轮询间隔（秒）
    pub poll_interval_secs: Option<u64>,
    /// 最大等待时间（秒）- 已废弃，保留用于兼容性
    #[deprecated(note = "超时机制已移除，此字段仅保留用于向后兼容")]
    pub max_wait_secs: Option<u64>,
    /// 标签
    pub label: Option<i32>,
    /// 重试配置
    pub retry: Option<RetryConfigRequest>,
}

/// 重试配置请求（CFG版本）
#[derive(Debug, Deserialize, Clone)]
pub struct RetryConfigRequest {
    /// 是否启用重试
    pub enabled: Option<bool>,
    /// 最大重试次数
    pub max_attempts: Option<u32>,
    /// 初始退避时间（秒）
    pub initial_backoff_secs: Option<u64>,
    /// 最大退避时间（秒）
    pub max_backoff_secs: Option<u64>,
    /// 退避倍率
    pub backoff_multiplier: Option<f64>,
    /// 是否添加随机抖动
    pub jitter: Option<bool>,
}

impl From<CfgTaskConfigRequest> for CfgTaskConfig {
    #[allow(deprecated)]
    fn from(req: CfgTaskConfigRequest) -> Self {
        let retry_config = req.retry.map(|retry_req| {
            let mut retry_config = RetryConfig::default();
            if let Some(enabled) = retry_req.enabled {
                retry_config.enabled = enabled;
            }
            if let Some(max_attempts) = retry_req.max_attempts {
                retry_config.max_attempts = max_attempts;
            }
            if let Some(initial_backoff_secs) = retry_req.initial_backoff_secs {
                retry_config.initial_backoff_secs = initial_backoff_secs;
            }
            if let Some(max_backoff_secs) = retry_req.max_backoff_secs {
                retry_config.max_backoff_secs = max_backoff_secs;
            }
            if let Some(backoff_multiplier) = retry_req.backoff_multiplier {
                retry_config.backoff_multiplier = backoff_multiplier;
            }
            if let Some(jitter) = retry_req.jitter {
                retry_config.jitter = jitter;
            }
            retry_config
        });

        Self {
            poll_interval_secs: req.poll_interval_secs.unwrap_or(10),
            max_wait_secs: req.max_wait_secs.unwrap_or(0), // 超时机制已移除，默认设为0
            label: req.label.unwrap_or(0),
            retry: retry_config,
        }
    }
}
