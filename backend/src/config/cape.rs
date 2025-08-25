use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 是否启用重试
    pub enabled: bool,
    /// 最大重试次数
    pub max_attempts: u32,
    /// 初始退避时间（秒）
    pub initial_backoff_secs: u64,
    /// 最大退避时间（秒）
    pub max_backoff_secs: u64,
    /// 退避倍率
    pub backoff_multiplier: f64,
    /// 添加随机抖动以避免惊群效应
    pub jitter: bool,
    /// 可重试的错误类型（正则表达式模式）
    pub retryable_error_patterns: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            initial_backoff_secs: 5,
            max_backoff_secs: 300, // 5分钟
            backoff_multiplier: 2.0,
            jitter: true,
            retryable_error_patterns: vec![
                "connection".to_string(),
                "timeout".to_string(),
                "network".to_string(),
                "提交文件到CAPE失败".to_string(),
                "service_unavailable".to_string(),
                "error sending request".to_string(),
            ],
        }
    }
}

/// CAPE List轮询配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapeListPollConfig {
    /// 最小limit值，确保覆盖足够的任务
    pub limit_floor: u32,
    /// 最大翻页数，防止无限扫描
    pub max_pages: u32,
    /// 轮询间隔（秒）
    pub interval_secs: u64,
}

impl Default for CapeListPollConfig {
    fn default() -> Self {
        Self {
            limit_floor: 200,
            max_pages: 5,
            interval_secs: 30,
        }
    }
}

/// CAPE Sandbox 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapeConfig {
    /// CAPE API 基础URL
    pub base_url: String,
    /// 状态检查间隔（秒）
    pub status_check_interval_seconds: u64,
    /// 默认虚拟机名称
    pub default_machine: Option<String>,
    /// 是否启用CAPE集成
    pub enabled: bool,
    /// 最大并发任务数
    pub max_concurrent_tasks: u32,
    /// 重试次数
    pub max_retries: u32,
    /// 额外的默认参数
    pub default_options: HashMap<String, String>,
    /// 状态同步策略
    pub status_strategy: Option<String>,
    /// List轮询配置
    pub list_poll: Option<CapeListPollConfig>,
}

impl Default for CapeConfig {
    fn default() -> Self {
        Self {
            base_url: "http://192.168.2.186:8000/apiv2".to_string(),
            status_check_interval_seconds: 30,
            default_machine: None,
            enabled: false, // 默认禁用，需要手动配置启用
            max_concurrent_tasks: 5,
            max_retries: 3,
            default_options: HashMap::new(),
            status_strategy: Some("auto".to_string()),
            list_poll: Some(CapeListPollConfig::default()),
        }
    }
}

impl CapeConfig {
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("CAPE base_url 不能为空".to_string());
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err("CAPE base_url 必须以 http:// 或 https:// 开头".to_string());
        }

        if self.status_check_interval_seconds == 0 {
            return Err("状态检查间隔必须大于0".to_string());
        }

        if self.max_concurrent_tasks == 0 {
            return Err("最大并发任务数必须大于0".to_string());
        }

        Ok(())
    }

    /// 获取完整的API URL
    pub fn get_api_url(&self, endpoint: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let endpoint = endpoint.trim_start_matches('/');
        format!("{}/{}", base, endpoint)
    }

    /// 检查是否启用CAPE功能
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取默认的任务配置
    pub fn get_default_task_config(&self) -> CapeTaskConfig {
        CapeTaskConfig {
            machine: self.default_machine.clone(),
            options: Some(self.default_options.clone()),
            retry: Some(RetryConfig::default()),
        }
    }

    /// 获取状态同步策略
    pub fn get_status_strategy(&self) -> &str {
        self.status_strategy.as_deref().unwrap_or("auto")
    }

    /// 获取List轮询配置
    pub fn get_list_poll_config(&self) -> CapeListPollConfig {
        self.list_poll.clone().unwrap_or_default()
    }
}

/// CAPE 任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapeTaskConfig {
    pub machine: Option<String>,
    pub options: Option<HashMap<String, String>>,
    pub retry: Option<RetryConfig>,
}

impl Default for CapeTaskConfig {
    fn default() -> Self {
        Self {
            machine: None,
            options: Some(HashMap::new()),
            retry: Some(RetryConfig::default()),
        }
    }
}

/// CAPE 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapePerformanceConfig {
    /// 性能统计保留天数
    pub stats_retention_days: u32,
    /// 性能报告生成间隔（小时）
    pub report_generation_interval_hours: u32,
    /// 低性能阈值（成功率）
    pub low_performance_threshold: f64,
    /// 慢任务阈值（秒）
    pub slow_task_threshold_seconds: u64,
}

impl Default for CapePerformanceConfig {
    fn default() -> Self {
        Self {
            stats_retention_days: 30,
            report_generation_interval_hours: 24,
            low_performance_threshold: 0.9,
            slow_task_threshold_seconds: 600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cape_config_default() {
        let config = CapeConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_concurrent_tasks, 5);
    }

    #[test]
    fn test_cape_config_validation() {
        let mut config = CapeConfig::default();

        // 有效配置
        assert!(config.validate().is_ok());

        // 无效的base_url
        config.base_url = "".to_string();
        assert!(config.validate().is_err());

        config.base_url = "invalid-url".to_string();
        assert!(config.validate().is_err());

        // 重置为有效URL
        config.base_url = "http://example.com".to_string();
        assert!(config.validate().is_ok());

        // 无效的超时时间
        // 超时相关校验已移除
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_backoff_secs, 5);
        assert_eq!(config.max_backoff_secs, 300);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.jitter);
        assert!(!config.retryable_error_patterns.is_empty());
    }

    #[test]
    fn test_get_api_url() {
        let config = CapeConfig::default();

        assert_eq!(
            config.get_api_url("/tasks/create/file/"),
            "http://192.168.2.186:8000/apiv2/tasks/create/file/"
        );

        assert_eq!(
            config.get_api_url("tasks/status/123/"),
            "http://192.168.2.186:8000/apiv2/tasks/status/123/"
        );
    }
}

/// 通用的重试执行器
pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// 执行带重试的异步操作
    pub async fn execute_with_retry<F, Fut, T, E>(
        &self,
        operation: F,
        task_id: &str,
    ) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        if !self.config.enabled {
            return operation().await;
        }

        let mut attempt = 1;
        let mut backoff_secs = self.config.initial_backoff_secs;

        loop {
            info!(
                "任务 {} 第 {}/{} 次尝试",
                task_id, attempt, self.config.max_attempts
            );

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("任务 {} 在第 {} 次尝试后成功", task_id, attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    if attempt >= self.config.max_attempts {
                        error!(
                            "任务 {} 达到最大重试次数 {}，最终失败: {}",
                            task_id, self.config.max_attempts, error
                        );
                        return Err(error);
                    }

                    if !self.is_retryable_error(&error.to_string()) {
                        error!("任务 {} 遇到不可重试错误: {}", task_id, error);
                        return Err(error);
                    }

                    let delay_secs = if self.config.jitter {
                        self.add_jitter(backoff_secs)
                    } else {
                        backoff_secs
                    };

                    warn!(
                        "任务 {} 第 {} 次尝试失败: {}，{}秒后重试",
                        task_id, attempt, error, delay_secs
                    );

                    sleep(Duration::from_secs(delay_secs)).await;

                    // 指数退避
                    backoff_secs = (backoff_secs as f64 * self.config.backoff_multiplier) as u64;
                    backoff_secs = backoff_secs.min(self.config.max_backoff_secs);

                    attempt += 1;
                }
            }
        }
    }

    fn is_retryable_error(&self, error_msg: &str) -> bool {
        self.config
            .retryable_error_patterns
            .iter()
            .any(|pattern| error_msg.contains(pattern))
    }

    fn add_jitter(&self, base_secs: u64) -> u64 {
        let jitter_range = (base_secs as f64 * 0.1) as u64; // 10% 抖动
        let jitter = rand::random::<f64>() * jitter_range.max(1) as f64;
        base_secs + jitter as u64
    }
}
