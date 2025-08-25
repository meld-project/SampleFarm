use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod cape;
pub mod cfg;

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub minio: MinioConfig,
    pub file: FileConfig,
    pub cape: Option<cape::CapeConfig>,
    pub cfg: Option<cfg::CfgConfig>,
    pub startup_recovery: StartupRecoveryConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

/// MinIO配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinioConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

/// 文件处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub max_size: u64,
    pub temp_dir: String,
}

/// 启动恢复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupRecoveryConfig {
    pub enabled: bool,
    pub initial_delay_secs: u64,
    pub scan_interval_secs: u64,
    pub batch_size: u32,
    pub global_concurrency: u32,
    pub stuck_submitting_threshold_secs: u64,
}

impl Default for StartupRecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay_secs: 10,
            scan_interval_secs: 300,
            batch_size: 20,
            global_concurrency: 8,
            stuck_submitting_threshold_secs: 300,
        }
    }
}

impl StartupRecoveryConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.initial_delay_secs > 300 {
            return Err("初始延迟不应超过5分钟".into());
        }
        if self.scan_interval_secs < 60 {
            return Err("扫描间隔不应少于1分钟".into());
        }
        if self.batch_size == 0 || self.batch_size > 100 {
            return Err("批量大小应在1-100之间".into());
        }
        if self.global_concurrency == 0 || self.global_concurrency > 50 {
            return Err("全局并发数应在1-50之间".into());
        }
        if self.stuck_submitting_threshold_secs < 120 {
            return Err("僵死判定阈值不应少于2分钟".into());
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                url: "postgresql://samplefarm_user:samplefarm_password@localhost/samplefarm"
                    .to_string(),
                max_connections: 20,
            },
            minio: MinioConfig {
                endpoint: "http://localhost:9000".to_string(),
                access_key: "minioadmin".to_string(),
                secret_key: "minioadmin".to_string(),
                bucket: "samplefarm".to_string(),
            },
            file: FileConfig {
                max_size: 1024 * 1024 * 1024, // 1GB
                temp_dir: "/tmp/samplefarm".to_string(),
            },
            cape: Some(cape::CapeConfig::default()),
            cfg: Some(cfg::CfgConfig::default()),
            startup_recovery: StartupRecoveryConfig::default(),
        }
    }
}

impl Config {
    /// 从配置文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| AppError::config(format!("解析配置文件失败: {}", e)))?;

        // 验证配置
        config.validate()?;

        Ok(config)
    }

    /// 验证配置有效性
    pub fn validate(&self) -> AppResult<()> {
        if self.server.port == 0 {
            return Err(AppError::config("服务器端口不能为0"));
        }

        if self.database.url.is_empty() {
            return Err(AppError::config("数据库URL不能为空"));
        }

        if self.database.max_connections == 0 {
            return Err(AppError::config("数据库最大连接数不能为0"));
        }

        if self.minio.endpoint.is_empty() {
            return Err(AppError::config("MinIO endpoint不能为空"));
        }

        if self.minio.bucket.is_empty() {
            return Err(AppError::config("MinIO bucket不能为空"));
        }

        if self.file.max_size == 0 {
            return Err(AppError::config("文件最大大小不能为0"));
        }

        // 验证CAPE配置（如果启用）
        if let Some(cape_config) = &self.cape {
            if let Err(e) = cape_config.validate() {
                return Err(AppError::config(format!("CAPE配置无效: {}", e)));
            }
        }

        // 验证CFG配置（如果启用）
        if let Some(cfg_config) = &self.cfg {
            if let Err(e) = cfg_config.validate() {
                return Err(AppError::config(format!("CFG配置无效: {}", e)));
            }
        }

        // 验证启动恢复配置
        if let Err(e) = self.startup_recovery.validate() {
            return Err(AppError::config(format!("启动恢复配置无效: {}", e)));
        }

        Ok(())
    }

    /// 获取服务器监听地址
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> AppResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| AppError::config(format!("序列化配置失败: {}", e)))?;

        std::fs::write(path.as_ref(), content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.max_connections, 20);
        assert_eq!(config.file.max_size, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_addr() {
        let config = Config::default();
        assert_eq!(config.server_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn test_save_and_load_config() {
        let original_config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();

        // 保存配置
        original_config.save_to_file(temp_file.path()).unwrap();

        // 加载配置
        let loaded_config = Config::from_file(temp_file.path()).unwrap();

        assert_eq!(original_config.server.port, loaded_config.server.port);
        assert_eq!(
            original_config.database.max_connections,
            loaded_config.database.max_connections
        );
    }
}
