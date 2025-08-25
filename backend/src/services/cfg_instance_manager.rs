use chrono::Utc;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        CfgHealthStatus, CfgInstance, CfgInstanceStatus, CreateCfgInstanceRequest,
        UpdateCfgInstanceRequest,
    },
    services::CfgClient,
};

/// CFG实例管理器
///
/// 负责管理多个CFG实例的配置、健康检查和客户端缓存
#[derive(Debug, Clone)]
pub struct CfgInstanceManager {
    /// 数据库连接池
    db_pool: PgPool,
    /// 内存中的实例缓存
    instances: Arc<RwLock<HashMap<Uuid, CfgInstance>>>,
    /// CFG客户端缓存
    clients: Arc<RwLock<HashMap<Uuid, CfgClient>>>,
    /// 健康检查任务句柄
    health_checkers: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
}

impl CfgInstanceManager {
    /// 创建新的CFG实例管理器
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

        info!("CFG实例管理器初始化完成");
        Ok(manager)
    }

    /// 从数据库加载所有CFG实例
    pub async fn load_instances(&self) -> Result<(), AppError> {
        let rows = sqlx::query("SELECT * FROM cfg_instances ORDER BY created_at")
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("加载CFG实例失败: {}", e)))?;

        let mut instances = self.instances.write().await;
        let mut clients = self.clients.write().await;

        instances.clear();
        clients.clear();

        for row in rows {
            let instance = CfgInstance {
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
                    .unwrap_or(CfgInstanceStatus::Unknown),
                last_health_check: row.get("last_health_check"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };

            // 创建客户端
            match CfgClient::new_with_base_url(instance.base_url.clone()) {
                Ok(client) => {
                    instances.insert(instance.id, instance.clone());
                    clients.insert(instance.id, client);
                    debug!("加载CFG实例: {} ({})", instance.name, instance.id);
                }
                Err(e) => {
                    warn!("创建CFG客户端失败: {} - {}", instance.name, e);
                    continue;
                }
            }
        }

        info!("成功加载 {} 个CFG实例", instances.len());
        Ok(())
    }

    /// 获取指定实例
    pub async fn get_instance(&self, id: Uuid) -> Option<CfgInstance> {
        let instances = self.instances.read().await;
        instances.get(&id).cloned()
    }

    /// 获取所有可用的实例
    pub async fn get_available_instances(&self) -> Vec<CfgInstance> {
        let instances = self.instances.read().await;
        instances
            .values()
            .filter(|instance| instance.is_available())
            .cloned()
            .collect()
    }

    /// 获取所有实例
    pub async fn get_all_instances(&self) -> Vec<CfgInstance> {
        let instances = self.instances.read().await;
        instances.values().cloned().collect()
    }

    /// 获取CFG客户端
    pub async fn get_client(&self, instance_id: Uuid) -> Option<CfgClient> {
        let clients = self.clients.read().await;
        clients.get(&instance_id).cloned()
    }

    /// 获取默认实例（第一个可用的启用实例）
    pub async fn get_default_instance(&self) -> Option<CfgInstance> {
        let instances = self.instances.read().await;
        instances
            .values()
            .find(|instance| instance.is_available())
            .cloned()
    }

    /// 获取默认客户端（兼容旧接口）
    pub async fn client(&self) -> Option<CfgClient> {
        if let Some(instance) = self.get_default_instance().await {
            self.get_client(instance.id).await
        } else {
            None
        }
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool, AppError> {
        let instances = self.get_available_instances().await;
        Ok(!instances.is_empty())
    }

    /// 启动健康监控
    pub async fn start_health_monitoring(&self) {
        let instances = self.instances.read().await;
        let mut health_checkers = self.health_checkers.write().await;

        for (id, instance) in instances.iter() {
            if instance.enabled {
                let instance_clone = instance.clone();
                let db_pool = self.db_pool.clone();
                let clients = self.clients.clone();

                let handle = tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(
                            instance_clone.health_check_interval as u64,
                        ))
                        .await;

                        // 执行健康检查
                        let status = if let Some(client) = {
                            let clients_guard = clients.read().await;
                            clients_guard.get(&instance_clone.id).cloned()
                        } {
                            match client.get_system_status().await {
                                Ok(_) => {
                                    debug!("CFG实例 {} 健康检查通过", instance_clone.name);
                                    CfgInstanceStatus::Healthy
                                }
                                Err(e) => {
                                    warn!("CFG实例 {} 健康检查失败: {}", instance_clone.name, e);
                                    CfgInstanceStatus::Unhealthy
                                }
                            }
                        } else {
                            CfgInstanceStatus::Unhealthy
                        };

                        // 更新数据库中的健康状态
                        let now = Utc::now();
                        if let Err(e) = sqlx::query(
                            "UPDATE cfg_instances SET status = $1, last_health_check = $2 WHERE id = $3"
                        )
                        .bind(status.to_string())
                        .bind(now)
                        .bind(instance_clone.id)
                        .execute(&db_pool)
                        .await {
                            warn!("更新CFG实例健康状态失败: {}", e);
                        }
                    }
                });

                health_checkers.insert(*id, handle);
            }
        }

        info!("启动CFG实例健康监控");
    }

    /// 创建新的CFG实例
    pub async fn create_instance(
        &self,
        request: CreateCfgInstanceRequest,
    ) -> Result<CfgInstance, AppError> {
        let instance_id = Uuid::new_v4();
        let now = Utc::now();

        let instance = CfgInstance {
            id: instance_id,
            name: request.name,
            base_url: request.base_url,
            description: request.description,
            enabled: true, // 新创建的实例默认启用
            timeout_seconds: request.timeout_seconds.unwrap_or(300),
            max_concurrent_tasks: request.max_concurrent_tasks.unwrap_or(5),
            health_check_interval: request.health_check_interval.unwrap_or(60),
            status: CfgInstanceStatus::Unknown,
            last_health_check: None,
            created_at: now,
            updated_at: now,
        };

        // 插入到数据库
        sqlx::query(
            r#"INSERT INTO cfg_instances 
               (id, name, base_url, description, enabled, timeout_seconds, max_concurrent_tasks, health_check_interval, status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#
        )
        .bind(instance.id)
        .bind(&instance.name)
        .bind(&instance.base_url)
        .bind(instance.description.as_ref())
        .bind(instance.enabled)
        .bind(instance.timeout_seconds)
        .bind(instance.max_concurrent_tasks)
        .bind(instance.health_check_interval)
        .bind(instance.status.to_string())
        .bind(instance.created_at)
        .bind(instance.updated_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("创建CFG实例失败: {}", e)))?;

        // 重新加载实例到内存缓存
        self.load_instances().await?;

        info!("成功创建CFG实例: {} ({})", instance.name, instance.id);
        Ok(instance)
    }

    /// 更新CFG实例
    pub async fn update_instance(
        &self,
        id: Uuid,
        request: UpdateCfgInstanceRequest,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        let _row = sqlx::query(
            r#"UPDATE cfg_instances SET
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
        .map_err(|e| AppError::service_unavailable(format!("更新CFG实例失败: {}", e)))?;

        // 重新加载实例到内存缓存
        self.load_instances().await?;

        info!("成功更新CFG实例: {}", id);
        Ok(())
    }

    /// 删除CFG实例
    pub async fn delete_instance(&self, id: Uuid) -> Result<(), AppError> {
        // 检查是否有正在使用的任务
        let active_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sub_tasks WHERE cfg_instance_id = $1 AND status IN ('pending', 'submitting', 'submitted', 'analyzing')"
        )
        .bind(id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::service_unavailable(format!("检查活跃任务失败: {}", e)))?;

        if active_count.0 > 0 {
            return Err(AppError::bad_request(format!(
                "无法删除实例，还有 {} 个任务正在使用该实例",
                active_count.0
            )));
        }

        // 检查是否有历史任务记录
        let total_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sub_tasks WHERE cfg_instance_id = $1")
                .bind(id)
                .fetch_one(&self.db_pool)
                .await
                .map_err(|e| AppError::service_unavailable(format!("检查历史任务失败: {}", e)))?;

        if total_count.0 > 0 {
            return Err(AppError::bad_request(format!(
                "无法删除实例，该实例关联了 {} 个历史任务记录。如需删除，请先清理相关任务数据，或联系管理员处理数据迁移",
                total_count.0
            )));
        }

        // 停止健康检查任务
        {
            let mut health_checkers = self.health_checkers.write().await;
            if let Some(handle) = health_checkers.remove(&id) {
                handle.abort();
            }
        }

        // 从数据库删除
        sqlx::query("DELETE FROM cfg_instances WHERE id = $1")
            .bind(id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("删除CFG实例失败: {}", e)))?;

        // 从内存缓存移除
        {
            let mut instances = self.instances.write().await;
            let mut clients = self.clients.write().await;
            instances.remove(&id);
            clients.remove(&id);
        }

        info!("成功删除CFG实例: {}", id);
        Ok(())
    }

    /// 获取实例健康状态
    pub async fn get_health_status(&self, id: Uuid) -> Result<CfgHealthStatus, AppError> {
        if let Some(instance) = self.get_instance(id).await {
            if let Some(client) = self.get_client(id).await {
                match client.get_system_status().await {
                    Ok(_) => Ok(CfgHealthStatus {
                        instance_id: id,
                        instance_name: instance.name.clone(),
                        status: CfgInstanceStatus::Healthy,
                        response_time_ms: None, // TODO: 计算响应时间
                        checked_at: Utc::now(),
                        error_message: None,
                    }),
                    Err(e) => Ok(CfgHealthStatus {
                        instance_id: id,
                        instance_name: instance.name.clone(),
                        status: CfgInstanceStatus::Unhealthy,
                        response_time_ms: None,
                        checked_at: Utc::now(),
                        error_message: Some(e.to_string()),
                    }),
                }
            } else {
                Ok(CfgHealthStatus {
                    instance_id: id,
                    instance_name: instance.name.clone(),
                    status: CfgInstanceStatus::Unhealthy,
                    response_time_ms: None,
                    checked_at: Utc::now(),
                    error_message: Some("客户端不可用".to_string()),
                })
            }
        } else {
            Err(AppError::not_found("CFG实例不存在"))
        }
    }

    /// 获取所有实例的健康状态
    pub async fn get_all_health_status(&self) -> Vec<CfgHealthStatus> {
        let instances = self.get_all_instances().await;
        let mut health_statuses = Vec::new();

        for instance in instances {
            if let Ok(health) = self.get_health_status(instance.id).await {
                health_statuses.push(health);
            }
        }

        health_statuses
    }
}
