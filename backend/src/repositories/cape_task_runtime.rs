use crate::error::AppError;
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;
use serde_json::Value as JsonValue;
use chrono::{DateTime, Utc};

/// CAPE任务运行时快照仓库（已弃用：当前极简模式默认不使用快照表）
#[derive(Debug, Clone)]
pub struct CapeTaskRuntimeRepository {
    pool: Pool<Postgres>,
}

/// CAPE任务状态快照（已弃用）
#[derive(Debug, Clone)]
pub struct CapeTaskStatusSnapshot {
    pub id: Uuid,
    pub sub_task_id: Uuid,
    pub cape_instance_id: Uuid,
    pub cape_task_id: i32,
    pub status: String,
    pub snapshot: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CapeTaskRuntimeRepository {
    /// 创建新的仓库实例
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// 获取数据库连接池引用
    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    /// 插入或更新任务状态快照（已弃用）
    pub async fn upsert_snapshot(
        &self,
        sub_task_id: Uuid,
        cape_instance_id: Uuid,
        cape_task_id: i32,
        status: &str,
        snapshot_json: &JsonValue,
    ) -> Result<(), AppError> {
        let query = r#"
            INSERT INTO cape_task_status_snapshots (
                id, sub_task_id, cape_instance_id, cape_task_id, status, snapshot
            ) VALUES (
                $1, $2, $3, $4, $5, $6
            )
            ON CONFLICT (sub_task_id) 
            DO UPDATE SET
                cape_instance_id = EXCLUDED.cape_instance_id,
                cape_task_id = EXCLUDED.cape_task_id,
                status = EXCLUDED.status,
                snapshot = EXCLUDED.snapshot,
                updated_at = CURRENT_TIMESTAMP
        "#;

        sqlx::query(query)
            .bind(Uuid::new_v4()) // 新记录的ID，如果是更新则不会使用
            .bind(sub_task_id)
            .bind(cape_instance_id)
            .bind(cape_task_id)
            .bind(status)
            .bind(snapshot_json)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("插入/更新快照失败: {}", e)))?;

        Ok(())
    }

    /// 根据子任务ID获取最新的状态快照（已弃用）
    pub async fn get_snapshot_by_sub_task(
        &self,
        sub_task_id: Uuid,
    ) -> Result<Option<CapeTaskStatusSnapshot>, AppError> {
        let query = r#"
            SELECT id, sub_task_id, cape_instance_id, cape_task_id, status, snapshot, created_at, updated_at
            FROM cape_task_status_snapshots
            WHERE sub_task_id = $1
        "#;

        let row = sqlx::query(query)
            .bind(sub_task_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询快照失败: {}", e)))?;

        if let Some(row) = row {
            Ok(Some(CapeTaskStatusSnapshot {
                id: row.get("id"),
                sub_task_id: row.get("sub_task_id"),
                cape_instance_id: row.get("cape_instance_id"),
                cape_task_id: row.get("cape_task_id"),
                status: row.get("status"),
                snapshot: row.get("snapshot"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// 根据CAPE实例ID获取所有进行中任务的快照（已弃用）
    pub async fn get_snapshots_by_instance(
        &self,
        cape_instance_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<CapeTaskStatusSnapshot>, AppError> {
        let query = r#"
            SELECT id, sub_task_id, cape_instance_id, cape_task_id, status, snapshot, created_at, updated_at
            FROM cape_task_status_snapshots
            WHERE cape_instance_id = $1
            ORDER BY updated_at DESC
            LIMIT $2
        "#;

        let limit = limit.unwrap_or(100);
        let rows = sqlx::query(query)
            .bind(cape_instance_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询实例快照失败: {}", e)))?;

        let snapshots = rows
            .into_iter()
            .map(|row| CapeTaskStatusSnapshot {
                id: row.get("id"),
                sub_task_id: row.get("sub_task_id"),
                cape_instance_id: row.get("cape_instance_id"),
                cape_task_id: row.get("cape_task_id"),
                status: row.get("status"),
                snapshot: row.get("snapshot"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(snapshots)
    }

    /// 清理旧的快照记录（已弃用）
    pub async fn cleanup_old_snapshots(&self, days_to_keep: i32) -> Result<u64, AppError> {
        let query = r#"
            DELETE FROM cape_task_status_snapshots
            WHERE updated_at < NOW() - INTERVAL '%d days'
            AND status IN ('completed', 'reported', 'failed', 'failed_analysis')
        "#;

        let query = query.replace("%d", &days_to_keep.to_string());
        
        let result = sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("清理旧快照失败: {}", e)))?;

        Ok(result.rows_affected())
    }

    /// 批量插入快照（已弃用）
    pub async fn batch_upsert_snapshots(
        &self,
        snapshots: Vec<(Uuid, Uuid, i32, String, JsonValue)>, // (sub_task_id, cape_instance_id, cape_task_id, status, snapshot)
    ) -> Result<(), AppError> {
        if snapshots.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await
            .map_err(|e| AppError::service_unavailable(format!("开始事务失败: {}", e)))?;

        let query = r#"
            INSERT INTO cape_task_status_snapshots (
                id, sub_task_id, cape_instance_id, cape_task_id, status, snapshot
            ) VALUES (
                $1, $2, $3, $4, $5, $6
            )
            ON CONFLICT (sub_task_id) 
            DO UPDATE SET
                cape_instance_id = EXCLUDED.cape_instance_id,
                cape_task_id = EXCLUDED.cape_task_id,
                status = EXCLUDED.status,
                snapshot = EXCLUDED.snapshot,
                updated_at = CURRENT_TIMESTAMP
        "#;

        for (sub_task_id, cape_instance_id, cape_task_id, status, snapshot) in snapshots {
            sqlx::query(query)
                .bind(Uuid::new_v4())
                .bind(sub_task_id)
                .bind(cape_instance_id)
                .bind(cape_task_id)
                .bind(&status)
                .bind(&snapshot)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::service_unavailable(format!("批量插入快照失败: {}", e)))?;
        }

        tx.commit().await
            .map_err(|e| AppError::service_unavailable(format!("提交事务失败: {}", e)))?;

        Ok(())
    }
}
