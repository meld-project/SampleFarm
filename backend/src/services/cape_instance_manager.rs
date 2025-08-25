use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        CapeHealthStatus, CapeInstance, CapeInstanceStatus, CreateCapeInstanceRequest,
        UpdateCapeInstanceRequest,
    },
    services::CapeClient,
};

/// CAPE实例管理器
///
/// 负责管理多个CAPE实例的配置、健康检查和客户端缓存
#[derive(Debug, Clone)]
pub struct CapeInstanceManager {
    /// 数据库连接池
    db_pool: PgPool,
    /// 内存中的实例缓存
    instances: Arc<RwLock<HashMap<Uuid, CapeInstance>>>,
    /// CAPE客户端缓存
    clients: Arc<RwLock<HashMap<Uuid, CapeClient>>>,
    /// 健康检查任务句柄
    health_checkers: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
}

impl CapeInstanceManager {
    /// 创建新的CAPE实例管理器
    pub async fn new(db_pool: PgPool) -> Result<Self, AppError> {
        let manager = Self {
            db_pool,
            instances: Arc::new(RwLock::new(HashMap::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            health_checkers: Arc::new(RwLock::new(HashMap::new())),
        };

        // 加载现有实例
        manager.load_instances().await?;

        // 启动健康监控
        manager.start_health_monitoring().await;

        info!("CAPE实例管理器初始化完成");
        Ok(manager)
    }

    /// 从数据库加载所有CAPE实例
    pub async fn load_instances(&self) -> Result<(), AppError> {
        let rows = sqlx::query("SELECT * FROM cape_instances ORDER BY created_at")
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("加载CAPE实例失败: {}", e)))?;

        let mut instances = self.instances.write().await;
        let mut clients = self.clients.write().await;

        instances.clear();
        clients.clear();

        for row in rows {
            let instance = CapeInstance {
                id: row.get("id"),
                name: row.get("name"),
                base_url: row.get("base_url"),
                description: row.get("description"),
                enabled: row.get("enabled"),
                timeout_seconds: row.get("timeout_seconds"),
                max_concurrent_tasks: row.get("max_concurrent_tasks"),
                health_check_interval: row.get("health_check_interval"),
                status: row
                    .get::<String, _>("status")
                    .parse()
                    .unwrap_or(CapeInstanceStatus::Unknown),
                last_health_check: row.get("last_health_check"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };

            // 创建客户端
            // 实例级超时不再注入客户端，统一由任务级参数控制
            let client = CapeClient::new(instance.base_url.clone());

            instances.insert(instance.id, instance.clone());
            clients.insert(instance.id, client);

            debug!("加载CAPE实例: {} ({})", instance.name, instance.id);
        }

        info!("成功加载 {} 个CAPE实例", instances.len());
        Ok(())
    }

    /// 获取指定实例
    pub async fn get_instance(&self, id: Uuid) -> Option<CapeInstance> {
        let instances = self.instances.read().await;
        instances.get(&id).cloned()
    }

    /// 获取所有可用的实例
    pub async fn get_available_instances(&self) -> Vec<CapeInstance> {
        let instances = self.instances.read().await;
        instances
            .values()
            .filter(|instance| instance.is_available())
            .cloned()
            .collect()
    }

    /// 获取所有实例
    pub async fn get_all_instances(&self) -> Vec<CapeInstance> {
        let instances = self.instances.read().await;
        instances.values().cloned().collect()
    }

    /// 获取CAPE客户端
    pub async fn get_client(&self, instance_id: Uuid) -> Option<CapeClient> {
        let clients = self.clients.read().await;
        clients.get(&instance_id).cloned()
    }

    /// 获取默认实例（如果没有指定实例ID）
    pub async fn get_default_instance(&self) -> Option<CapeInstance> {
        // 返回第一个可用的实例
        let instances = self.instances.read().await;
        instances
            .values()
            .find(|instance| instance.is_available())
            .cloned()
    }

    /// 创建新的CAPE实例
    pub async fn create_instance(
        &self,
        request: CreateCapeInstanceRequest,
    ) -> Result<CapeInstance, AppError> {
        // 验证请求
        request.validate().map_err(|e| AppError::bad_request(e))?;

        let instance_id = Uuid::new_v4();
        let now = Utc::now();

        let timeout_seconds = request.timeout_seconds.unwrap_or(300);
        let max_concurrent_tasks = request.max_concurrent_tasks.unwrap_or(5);
        let health_check_interval = request.health_check_interval.unwrap_or(60);

        // 插入到数据库
        let row = sqlx::query(
            r#"
            INSERT INTO cape_instances 
            (id, name, base_url, description, enabled, timeout_seconds, max_concurrent_tasks, health_check_interval, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, true, $5, $6, $7, 'unknown', $8, $8)
            RETURNING *
            "#
        )
        .bind(instance_id)
        .bind(&request.name)
        .bind(&request.base_url)
        .bind(&request.description)
        .bind(timeout_seconds)
        .bind(max_concurrent_tasks)
        .bind(health_check_interval)
        .bind(now)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("创建CAPE实例失败: {}", e)))?;

        let instance = CapeInstance {
            id: row.get("id"),
            name: row.get("name"),
            base_url: row.get("base_url"),
            description: row.get("description"),
            enabled: row.get("enabled"),
            timeout_seconds: row.get("timeout_seconds"),
            max_concurrent_tasks: row.get("max_concurrent_tasks"),
            health_check_interval: row.get("health_check_interval"),
            status: row
                .get::<String, _>("status")
                .parse()
                .unwrap_or(CapeInstanceStatus::Unknown),
            last_health_check: row.get("last_health_check"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        // 创建客户端
        // 实例级超时不再注入客户端，统一由任务级参数控制
        let client = CapeClient::new(instance.base_url.clone());

        // 更新内存缓存
        {
            let mut instances = self.instances.write().await;
            let mut clients = self.clients.write().await;
            instances.insert(instance.id, instance.clone());
            clients.insert(instance.id, client);
        }

        // 立即进行健康检查
        tokio::spawn({
            let manager = self.clone();
            let instance_id = instance.id;
            async move {
                if let Err(e) = manager.health_check_instance(instance_id).await {
                    warn!("新实例 {} 健康检查失败: {}", instance_id, e);
                }
            }
        });

        info!("成功创建CAPE实例: {} ({})", instance.name, instance.id);
        Ok(instance)
    }

    /// 更新CAPE实例
    pub async fn update_instance(
        &self,
        id: Uuid,
        request: UpdateCapeInstanceRequest,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        // 使用COALESCE的方式更新，类似CFG实例的实现
        let _row = sqlx::query(
            r#"UPDATE cape_instances SET
                name = COALESCE($2, name),
                base_url = COALESCE($3, base_url),
                description = COALESCE($4, description),
                enabled = COALESCE($5, enabled),
                timeout_seconds = COALESCE($6, timeout_seconds),
                max_concurrent_tasks = COALESCE($7, max_concurrent_tasks),
                health_check_interval = COALESCE($8, health_check_interval),
                updated_at = $9
            WHERE id = $1"#,
        )
        .bind(id)
        .bind(request.name.as_ref())
        .bind(request.base_url.as_ref())
        .bind(request.description.as_ref())
        .bind(request.enabled)
        .bind(request.timeout_seconds)
        .bind(request.max_concurrent_tasks)
        .bind(request.health_check_interval)
        .bind(now)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("更新CAPE实例失败: {}", e)))?;

        // 重新加载实例到内存缓存
        self.load_instances().await?;

        info!("成功更新CAPE实例: {}", id);
        Ok(())
    }

    /// 删除CAPE实例
    pub async fn delete_instance(&self, id: Uuid) -> Result<(), AppError> {
        // 检查是否有正在使用的任务
        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sub_tasks WHERE cape_instance_id = $1 AND status IN ('pending', 'submitting', 'submitted', 'analyzing')"
        )
        .bind(id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("检查任务使用情况失败: {}", e)))?;

        if active_count > 0 {
            return Err(AppError::bad_request(format!(
                "无法删除实例，还有 {} 个任务正在使用该实例",
                active_count
            )));
        }

        // 检查是否有历史任务记录
        let total_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sub_tasks WHERE cape_instance_id = $1")
                .bind(id)
                .fetch_one(&self.db_pool)
                .await
                .map_err(|e| AppError::service_unavailable(format!("检查历史任务失败: {}", e)))?;

        if total_count > 0 {
            return Err(AppError::bad_request(format!(
                "无法删除实例，该实例关联了 {} 个历史任务记录。如需删除，请先清理相关任务数据，或联系管理员处理数据迁移",
                total_count
            )));
        }

        // 从数据库删除
        let affected = sqlx::query("DELETE FROM cape_instances WHERE id = $1")
            .bind(id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("删除CAPE实例失败: {}", e)))?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::not_found("CAPE实例不存在"));
        }

        // 从内存缓存中移除
        {
            let mut instances = self.instances.write().await;
            let mut clients = self.clients.write().await;
            let mut health_checkers = self.health_checkers.write().await;

            instances.remove(&id);
            clients.remove(&id);

            // 停止健康检查任务
            if let Some(handle) = health_checkers.remove(&id) {
                handle.abort();
            }
        }

        info!("成功删除CAPE实例: {}", id);
        Ok(())
    }

    /// 对指定实例进行健康检查
    pub async fn health_check_instance(&self, id: Uuid) -> Result<CapeHealthStatus, AppError> {
        let instance = self
            .get_instance(id)
            .await
            .ok_or_else(|| AppError::not_found("CAPE实例不存在"))?;

        let client = self
            .get_client(id)
            .await
            .ok_or_else(|| AppError::service_unavailable("CAPE客户端不可用"))?;

        let start_time = std::time::Instant::now();
        let checked_at = Utc::now();

        let (status, response_time_ms, error_message) = match client.health_check().await {
            Ok(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                (CapeInstanceStatus::Healthy, Some(response_time), None)
            }
            Err(e) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                (
                    CapeInstanceStatus::Unhealthy,
                    Some(response_time),
                    Some(e.to_string()),
                )
            }
        };

        // 更新数据库中的健康状态
        sqlx::query("UPDATE cape_instances SET status = $1, last_health_check = $2 WHERE id = $3")
            .bind(status.to_string())
            .bind(checked_at)
            .bind(id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("更新健康状态失败: {}", e)))?;

        // 更新内存缓存
        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(&id) {
                instance.status = status;
                instance.last_health_check = Some(checked_at);
            }
        }

        Ok(CapeHealthStatus {
            instance_id: id,
            instance_name: instance.name,
            status,
            response_time_ms,
            checked_at,
            error_message,
        })
    }

    /// 启动健康监控
    pub async fn start_health_monitoring(&self) {
        info!("启动CAPE实例健康监控");

        let instances = self.get_all_instances().await;
        let mut health_checkers = self.health_checkers.write().await;

        for instance in instances {
            if instance.enabled {
                let manager = self.clone();
                let instance_id = instance.id;
                let interval = Duration::seconds(instance.health_check_interval as i64);

                let handle = tokio::spawn(async move {
                    let mut interval_timer = tokio::time::interval(
                        interval
                            .to_std()
                            .unwrap_or(std::time::Duration::from_secs(60)),
                    );

                    loop {
                        interval_timer.tick().await;

                        if let Err(e) = manager.health_check_instance(instance_id).await {
                            warn!("实例 {} 健康检查失败: {}", instance_id, e);
                        }
                    }
                });

                health_checkers.insert(instance_id, handle);
                debug!("为实例 {} 启动健康检查任务", instance_id);
            }
        }
    }

    /// 停止健康监控
    pub async fn stop_health_monitoring(&self) {
        let mut health_checkers = self.health_checkers.write().await;

        for (instance_id, handle) in health_checkers.drain() {
            handle.abort();
            debug!("停止实例 {} 的健康检查任务", instance_id);
        }

        info!("已停止所有CAPE实例健康监控");
    }
}

impl Drop for CapeInstanceManager {
    fn drop(&mut self) {
        // 在manager被销毁时停止所有健康检查任务
        // 注意：这里不能使用async，所以只能abort任务
        if let Ok(mut health_checkers) = self.health_checkers.try_write() {
            for (_, handle) in health_checkers.drain() {
                handle.abort();
            }
        }
    }
}
