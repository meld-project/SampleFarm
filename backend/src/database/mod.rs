use crate::{config::DatabaseConfig, error::AppResult};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

/// 数据库连接池
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("pool", &"<PgPool>")
            .finish()
    }
}

impl Database {
    /// 创建数据库连接池
    pub async fn new(config: &DatabaseConfig) -> AppResult<Self> {
        tracing::info!("正在连接数据库: {}", mask_database_url(&config.url));

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&config.url)
            .await?;

        // 测试连接
        sqlx::query("SELECT 1").fetch_one(&pool).await?;

        tracing::info!("数据库连接成功，最大连接数: {}", config.max_connections);

        Ok(Self { pool })
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 数据库初始化验证（详细的版本和连接检查）
    pub async fn verify_connection(&self) -> AppResult<()> {
        tracing::info!("正在验证数据库连接和版本...");

        // 检查数据库版本
        let version = sqlx::query_scalar::<_, String>("SELECT version()")
            .fetch_one(&self.pool)
            .await?;

        tracing::info!("数据库版本: {}", version);

        // 可以添加更多初始化检查，比如检查必要的表是否存在
        // let table_exists = sqlx::query_scalar::<_, bool>(
        //     "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'samples')"
        // ).fetch_one(&self.pool).await?;

        tracing::info!("数据库连接验证完成");

        Ok(())
    }

    /// 运行数据库迁移（不需要实现）
    /// 数据库结构通过外部SQL脚本管理
    pub async fn migrate(&self) -> AppResult<()> {
        tracing::info!("数据库结构由外部SQL脚本管理，跳过迁移检查");
        Ok(())
    }

    /// 检查数据库健康状态
    pub async fn health_check(&self) -> AppResult<bool> {
        let result = sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await?;

        Ok(result == 1)
    }

    /// 关闭数据库连接池
    pub async fn close(&self) {
        tracing::info!("正在关闭数据库连接池...");
        self.pool.close().await;
        tracing::info!("数据库连接池已关闭");
    }
}

/// 隐藏数据库URL中的敏感信息
fn mask_database_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        // 查找协议部分后的第一个冒号位置
        if let Some(protocol_end) = url.find("://") {
            let auth_part = &url[protocol_end + 3..at_pos];
            if let Some(colon_pos) = auth_part.find(':') {
                // 找到了用户名:密码格式，需要掩码密码
                let mut masked = url.to_string();
                let password_start = protocol_end + 3 + colon_pos + 1;
                let password_end = at_pos;
                masked.replace_range(password_start..password_end, "***");
                return masked;
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_database_url() {
        let url = "postgresql://user:password@localhost/db";
        let masked = mask_database_url(url);
        assert_eq!(masked, "postgresql://user:***@localhost/db");

        let url_no_password = "postgresql://user@localhost/db";
        let masked = mask_database_url(url_no_password);
        assert_eq!(masked, "postgresql://user@localhost/db");
    }
}
