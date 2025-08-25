use crate::{
    error::AppError,
    models::{
        AnalyzerType, CreateMasterTaskRequest, CreateTaskByFilterRequest, MasterTask, PagedResult,
        Pagination, SampleQueryParams, SubTask, SubTaskFilter, SubTaskWithSample, TaskFilter,
        UpdateMasterTaskRequest, UpdateSubTaskStatusRequest,
    },
};
use chrono::Utc;
use sqlx::QueryBuilder;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// 简化的任务存储库（避免复杂的 sqlx 宏）
#[derive(Debug, Clone)]
pub struct TaskRepository {
    pub(crate) pool: PgPool,
}

impl TaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取数据库连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 创建主任务（简化版本）
    pub async fn create_master_task(
        &self,
        request: &CreateMasterTaskRequest,
    ) -> Result<MasterTask, AppError> {
        let task_id = Uuid::new_v4();
        let now = Utc::now();

        let task_type_str = match request.task_type {
            crate::models::TaskType::Batch => "batch",
            crate::models::TaskType::Single => "single",
        };

        let query = r#"
            INSERT INTO master_tasks 
            (id, task_name, analyzer_type, task_type, total_samples, completed_samples, failed_samples, status, progress, sample_filter, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 0, 0, 'pending', 0, $6, $7, $7)
            RETURNING id, task_name, analyzer_type, task_type, total_samples, completed_samples, failed_samples, status, progress, error_message, result_summary, sample_filter, paused_at, pause_reason, created_by, created_at, updated_at
        "#;

        let row = sqlx::query(query)
            .bind(task_id)
            .bind(&request.task_name)
            .bind(request.analyzer_type)
            .bind(task_type_str)
            .bind(request.sample_ids.len() as i32)
            .bind(&request.parameters)
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("创建主任务失败: {}", e)))?;

        Ok(MasterTask {
            id: row.get("id"),
            task_name: row.get("task_name"),
            analyzer_type: row.get("analyzer_type"),
            task_type: row.get("task_type"),
            total_samples: row.get("total_samples"),
            completed_samples: row.get("completed_samples"),
            failed_samples: row.get("failed_samples"),
            status: row.get("status"),
            progress: row.get("progress"),
            error_message: row.get("error_message"),
            result_summary: row.get("result_summary"),
            sample_filter: row.get("sample_filter"),
            paused_at: row.get("paused_at"),
            pause_reason: row.get("pause_reason"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// 批量创建子任务（简化版本）
    pub async fn create_sub_tasks(
        &self,
        master_task_id: Uuid,
        sample_ids: &[Uuid],
        analyzer_type: AnalyzerType,
        cape_instance_ids: Option<Vec<Uuid>>, // 改为列表
        cfg_instance_ids: Option<Vec<Uuid>>,  // 新增
        _priority: Option<i32>,
        parameters: Option<serde_json::Value>,
    ) -> Result<Vec<SubTask>, AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::service_unavailable(format!("开始事务失败: {}", e)))?;

        let mut sub_tasks = Vec::new();
        let now = Utc::now();

        for (idx, sample_id) in sample_ids.iter().enumerate() {
            let sub_task_id = Uuid::new_v4();
            // 轮询分配实例
            let cape_instance_id = cape_instance_ids.as_ref().and_then(|ids| {
                if !ids.is_empty() {
                    Some(ids[idx % ids.len()])
                } else {
                    None
                }
            });
            let cfg_instance_id = cfg_instance_ids.as_ref().and_then(|ids| {
                if !ids.is_empty() {
                    Some(ids[idx % ids.len()])
                } else {
                    None
                }
            });

            let query = r#"
                INSERT INTO sub_tasks 
                (id, master_task_id, sample_id, analyzer_type, cape_instance_id, cfg_instance_id, status, priority, parameters, retry_count, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, 'pending', 0, $7, 0, $8, $8)
                RETURNING id, master_task_id, sample_id, analyzer_type, cape_instance_id, cfg_instance_id, external_task_id, status, priority, parameters, error_message, retry_count, created_at, started_at, completed_at, updated_at
            "#;

            let row = sqlx::query(query)
                .bind(sub_task_id)
                .bind(master_task_id)
                .bind(sample_id)
                .bind(analyzer_type)
                .bind(cape_instance_id)
                .bind(cfg_instance_id)
                .bind(&parameters)
                .bind(now)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| AppError::service_unavailable(format!("创建子任务失败: {}", e)))?;

            sub_tasks.push(SubTask {
                id: row.get("id"),
                master_task_id: row.get("master_task_id"),
                sample_id: row.get("sample_id"),
                analyzer_type: row.get("analyzer_type"),
                cape_instance_id: row.get("cape_instance_id"),
                cfg_instance_id: row.get("cfg_instance_id"),
                external_task_id: row.get("external_task_id"),
                status: row.get("status"),
                priority: row.get("priority"),
                parameters: row.get("parameters"),
                error_message: row.get("error_message"),
                retry_count: row.get("retry_count"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                updated_at: row.get("updated_at"),
            });
        }

        tx.commit()
            .await
            .map_err(|e| AppError::service_unavailable(format!("提交事务失败: {}", e)))?;

        Ok(sub_tasks)
    }

    /// 按筛选条件列出匹配的样本ID
    pub async fn list_sample_ids_by_query(
        &self,
        filter: &SampleQueryParams,
    ) -> Result<Vec<uuid::Uuid>, AppError> {
        let mut qb = QueryBuilder::new("SELECT id FROM samples");

        // 复用与预览一致的过滤逻辑
        Self::apply_sample_query_params(&mut qb, filter);

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询样本ID失败: {}", e)))?;

        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            ids.push(id);
        }
        Ok(ids)
    }

    /// 创建主任务（基于筛选条件）并批量创建子任务
    pub async fn create_master_task_by_filter(
        &self,
        request: &CreateTaskByFilterRequest,
    ) -> Result<(MasterTask, Vec<SubTask>), AppError> {
        // 先获取匹配的所有样本ID
        let sample_ids = self.list_sample_ids_by_query(&request.filter).await?;

        if sample_ids.is_empty() {
            return Err(AppError::bad_request("没有匹配的样本，无法创建任务"));
        }

        // 构造主任务记录（正确写入 sample_filter）
        let task_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();
        let task_type_str = match request.task_type {
            crate::models::TaskType::Batch => "batch",
            crate::models::TaskType::Single => "single",
        };

        let sample_filter_json =
            serde_json::to_value(&request.filter).unwrap_or(serde_json::Value::Null);

        let insert_query = r#"
            INSERT INTO master_tasks 
            (id, task_name, analyzer_type, task_type, total_samples, completed_samples, failed_samples, status, progress, sample_filter, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 0, 0, 'pending', 0, $6, $7, $7)
            RETURNING id, task_name, analyzer_type, task_type, total_samples, completed_samples, failed_samples, status, progress, error_message, result_summary, sample_filter, paused_at, pause_reason, created_by, created_at, updated_at
        "#;

        let row = sqlx::query(insert_query)
            .bind(task_id)
            .bind(&request.task_name)
            .bind(request.analyzer_type)
            .bind(task_type_str)
            .bind(sample_ids.len() as i32)
            .bind(&sample_filter_json)
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("创建主任务失败: {}", e)))?;

        let master_task = MasterTask {
            id: row.get("id"),
            task_name: row.get("task_name"),
            analyzer_type: row.get("analyzer_type"),
            task_type: row.get("task_type"),
            total_samples: row.get("total_samples"),
            completed_samples: row.get("completed_samples"),
            failed_samples: row.get("failed_samples"),
            status: row.get("status"),
            progress: row.get("progress"),
            error_message: row.get("error_message"),
            result_summary: row.get("result_summary"),
            sample_filter: row.get("sample_filter"),
            paused_at: row.get("paused_at"),
            pause_reason: row.get("pause_reason"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        // 基于已有的创建子任务逻辑（支持轮询分配实例）
        let sub_tasks = self
            .create_sub_tasks(
                master_task.id,
                &sample_ids,
                request.analyzer_type,
                request.cape_instance_ids.clone(),
                request.cfg_instance_ids.clone(),
                None,
                request.parameters.clone(),
            )
            .await?;

        Ok((master_task, sub_tasks))
    }

    /// 将 SampleQueryParams 转为 WHERE 条件
    fn apply_sample_query_params<'a>(
        qb: &mut QueryBuilder<'a, sqlx::Postgres>,
        f: &'a SampleQueryParams,
    ) {
        let mut first = true;
        let mut push_cond = |qb: &mut QueryBuilder<'a, sqlx::Postgres>| {
            if first {
                qb.push(" WHERE ");
                first = false;
            } else {
                qb.push(" AND ");
            }
        };

        if let Some(v) = &f.file_name {
            push_cond(qb);
            qb.push("file_name ILIKE ").push_bind(format!("%{}%", v));
        }
        if let Some(v) = &f.file_type {
            push_cond(qb);
            qb.push("file_type = ").push_bind(v);
        }
        if let Some(v) = &f.sample_type {
            push_cond(qb);
            qb.push("sample_type = ").push_bind(v);
        }
        if let Some(v) = &f.file_hash_md5 {
            push_cond(qb);
            qb.push("file_hash_md5 = ").push_bind(v);
        }
        if let Some(v) = &f.file_hash_sha1 {
            push_cond(qb);
            qb.push("file_hash_sha1 = ").push_bind(v);
        }
        if let Some(v) = &f.file_hash_sha256 {
            push_cond(qb);
            qb.push("file_hash_sha256 = ").push_bind(v);
        }
        if let Some(v) = f.min_size {
            push_cond(qb);
            qb.push("file_size >= ").push_bind(v);
        }
        if let Some(v) = f.max_size {
            push_cond(qb);
            qb.push("file_size <= ").push_bind(v);
        }
        if let Some(v) = &f.source {
            push_cond(qb);
            qb.push("source ILIKE ").push_bind(format!("%{}%", v));
        }
        if let Some(v) = &f.labels {
            if !v.is_empty() {
                push_cond(qb);
                qb.push("labels && ").push_bind(v);
            }
        }
        if let Some(v) = f.is_container {
            push_cond(qb);
            qb.push("is_container = ").push_bind(v);
        }
        if let Some(v) = f.parent_id {
            push_cond(qb);
            qb.push("parent_id = ").push_bind(v);
        }
        if let Some(v) = f.start_time {
            push_cond(qb);
            qb.push("created_at >= ").push_bind(v);
        }
        if let Some(v) = f.end_time {
            push_cond(qb);
            qb.push("created_at <= ").push_bind(v);
        }
    }

    /// 获取主任务列表（简化版本）
    pub async fn list_master_tasks(
        &self,
        _filter: &TaskFilter,
        pagination: &Pagination,
    ) -> Result<PagedResult<MasterTask>, AppError> {
        // 简化版本：只支持基本分页，不支持复杂过滤
        let count_query = "SELECT COUNT(*) as count FROM master_tasks";
        let count_row = sqlx::query(count_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询任务数量失败: {}", e)))?;

        let total: i64 = count_row.get("count");

        let offset = (pagination.page - 1) * pagination.page_size;
        let data_query = "SELECT * FROM master_tasks ORDER BY created_at DESC LIMIT $1 OFFSET $2";

        let rows = sqlx::query(data_query)
            .bind(pagination.page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询任务列表失败: {}", e)))?;

        let tasks: Vec<MasterTask> = rows
            .into_iter()
            .map(|row| MasterTask {
                id: row.get("id"),
                task_name: row.get("task_name"),
                analyzer_type: row.get("analyzer_type"),
                task_type: row.get("task_type"),
                total_samples: row.get("total_samples"),
                completed_samples: row.get("completed_samples"),
                failed_samples: row.get("failed_samples"),
                status: row.get("status"),
                progress: row.get("progress"),
                error_message: row.get("error_message"),
                result_summary: row.get("result_summary"),
                sample_filter: row.get("sample_filter"),
                paused_at: row.get("paused_at"),
                pause_reason: row.get("pause_reason"),
                created_by: row.get("created_by"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(PagedResult {
            items: tasks,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages: (total as f64 / pagination.page_size as f64).ceil() as u32,
        })
    }

    /// 根据ID获取主任务
    pub async fn get_master_task_by_id(
        &self,
        task_id: Uuid,
    ) -> Result<Option<MasterTask>, AppError> {
        let query = "SELECT * FROM master_tasks WHERE id = $1";
        let row = sqlx::query(query)
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询主任务失败: {}", e)))?;

        Ok(row.map(|r| MasterTask {
            id: r.get("id"),
            task_name: r.get("task_name"),
            analyzer_type: r.get("analyzer_type"),
            task_type: r.get("task_type"),
            total_samples: r.get("total_samples"),
            completed_samples: r.get("completed_samples"),
            failed_samples: r.get("failed_samples"),
            status: r.get("status"),
            progress: r.get("progress"),
            error_message: r.get("error_message"),
            result_summary: r.get("result_summary"),
            sample_filter: r.get("sample_filter"),
            paused_at: r.get("paused_at"),
            pause_reason: r.get("pause_reason"),
            created_by: r.get("created_by"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// 更新主任务（简化版本）
    pub async fn update_master_task(
        &self,
        task_id: Uuid,
        request: &UpdateMasterTaskRequest,
    ) -> Result<MasterTask, AppError> {
        let now = Utc::now();

        // 简化的更新逻辑，只更新非None字段
        let mut query_parts = vec!["updated_at = $1".to_string()];
        let mut bind_index = 2;

        if request.status.is_some() {
            query_parts.push(format!("status = ${}", bind_index));
            bind_index += 1;
        }
        if request.progress.is_some() {
            query_parts.push(format!("progress = ${}", bind_index));
            bind_index += 1;
        }
        if request.completed_samples.is_some() {
            query_parts.push(format!("completed_samples = ${}", bind_index));
            bind_index += 1;
        }
        if request.failed_samples.is_some() {
            query_parts.push(format!("failed_samples = ${}", bind_index));
            bind_index += 1;
        }
        if request.error_message.is_some() {
            query_parts.push(format!("error_message = ${}", bind_index));
            bind_index += 1;
        }
        if request.result_summary.is_some() {
            query_parts.push(format!("result_summary = ${}", bind_index));
            bind_index += 1;
        }

        let query = format!(
            "UPDATE master_tasks SET {} WHERE id = ${} RETURNING *",
            query_parts.join(", "),
            bind_index
        );

        let mut query_builder = sqlx::query(&query).bind(now);

        if let Some(status) = &request.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(progress) = request.progress {
            query_builder = query_builder.bind(progress);
        }
        if let Some(completed_samples) = request.completed_samples {
            query_builder = query_builder.bind(completed_samples);
        }
        if let Some(failed_samples) = request.failed_samples {
            query_builder = query_builder.bind(failed_samples);
        }
        if let Some(error_message) = &request.error_message {
            query_builder = query_builder.bind(error_message);
        }
        if let Some(result_summary) = &request.result_summary {
            query_builder = query_builder.bind(result_summary);
        }

        query_builder = query_builder.bind(task_id);

        let row = query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("更新主任务失败: {}", e)))?;

        Ok(MasterTask {
            id: row.get("id"),
            task_name: row.get("task_name"),
            analyzer_type: row.get("analyzer_type"),
            task_type: row.get("task_type"),
            total_samples: row.get("total_samples"),
            completed_samples: row.get("completed_samples"),
            failed_samples: row.get("failed_samples"),
            status: row.get("status"),
            progress: row.get("progress"),
            error_message: row.get("error_message"),
            result_summary: row.get("result_summary"),
            sample_filter: row.get("sample_filter"),
            paused_at: row.get("paused_at"),
            pause_reason: row.get("pause_reason"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// 获取子任务列表（简化版本）
    pub async fn list_sub_tasks(
        &self,
        filter: &SubTaskFilter,
        pagination: &Pagination,
    ) -> Result<PagedResult<SubTask>, AppError> {
        let mut where_clause = String::new();
        let mut bind_values = Vec::new();

        if let Some(master_task_id) = filter.master_task_id {
            where_clause = " WHERE master_task_id = $1".to_string();
            bind_values.push(master_task_id);
        }

        let count_query = format!("SELECT COUNT(*) as count FROM sub_tasks{}", where_clause);
        let mut count_query_builder = sqlx::query(&count_query);
        for value in &bind_values {
            count_query_builder = count_query_builder.bind(value);
        }

        let count_row = count_query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询子任务数量失败: {}", e)))?;

        let total: i64 = count_row.get("count");

        let offset = (pagination.page - 1) * pagination.page_size;
        let data_query = format!(
            "SELECT * FROM sub_tasks{} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            bind_values.len() + 1,
            bind_values.len() + 2
        );

        let mut data_query_builder = sqlx::query(&data_query);
        for value in &bind_values {
            data_query_builder = data_query_builder.bind(value);
        }
        data_query_builder = data_query_builder
            .bind(pagination.page_size as i64)
            .bind(offset as i64);

        let rows = data_query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询子任务列表失败: {}", e)))?;

        let tasks: Vec<SubTask> = rows
            .into_iter()
            .map(|row| SubTask {
                id: row.get("id"),
                master_task_id: row.get("master_task_id"),
                sample_id: row.get("sample_id"),
                analyzer_type: row.get("analyzer_type"),
                cape_instance_id: row.get("cape_instance_id"),
                cfg_instance_id: row.get("cfg_instance_id"),
                external_task_id: row.get("external_task_id"),
                status: row.get("status"),
                priority: row.get("priority"),
                parameters: row.get("parameters"),
                error_message: row.get("error_message"),
                retry_count: row.get("retry_count"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(PagedResult {
            items: tasks,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages: (total as f64 / pagination.page_size as f64).ceil() as u32,
        })
    }

    /// 更新子任务状态
    pub async fn update_sub_task_status(
        &self,
        sub_task_id: Uuid,
        request: &UpdateSubTaskStatusRequest,
    ) -> Result<SubTask, AppError> {
        // 简化的更新逻辑
        let query = r#"
            UPDATE sub_tasks 
            SET 
                status = COALESCE($2, status),
                external_task_id = COALESCE($3, external_task_id),
                error_message = COALESCE($4, error_message),
                started_at = COALESCE($5, started_at),
                completed_at = COALESCE($6, completed_at)
            WHERE id = $1
            RETURNING *
        "#;

        let row = sqlx::query(query)
            .bind(sub_task_id)
            .bind(request.status)
            .bind(&request.external_task_id)
            .bind(&request.error_message)
            .bind(request.started_at)
            .bind(request.completed_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("更新子任务状态失败: {}", e)))?;

        Ok(SubTask {
            id: row.get("id"),
            master_task_id: row.get("master_task_id"),
            sample_id: row.get("sample_id"),
            analyzer_type: row.get("analyzer_type"),
            cape_instance_id: row.get("cape_instance_id"),
            cfg_instance_id: row.get("cfg_instance_id"),
            external_task_id: row.get("external_task_id"),
            status: row.get("status"),
            priority: row.get("priority"),
            parameters: row.get("parameters"),
            error_message: row.get("error_message"),
            retry_count: row.get("retry_count"),
            created_at: row.get("created_at"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// 根据主任务ID列出子任务
    pub async fn list_sub_tasks_by_master_task(
        &self,
        master_task_id: Uuid,
    ) -> Result<Vec<SubTask>, AppError> {
        let query = "SELECT * FROM sub_tasks WHERE master_task_id = $1 ORDER BY created_at";

        let rows = sqlx::query(query)
            .bind(master_task_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询子任务失败: {}", e)))?;

        let mut sub_tasks = Vec::new();
        for row in rows {
            sub_tasks.push(SubTask {
                id: row.get("id"),
                master_task_id: row.get("master_task_id"),
                sample_id: row.get("sample_id"),
                analyzer_type: row.get("analyzer_type"),
                cape_instance_id: row.get("cape_instance_id"),
                cfg_instance_id: row.get("cfg_instance_id"),
                external_task_id: row.get("external_task_id"),
                status: row.get("status"),
                priority: row.get("priority"),
                parameters: row.get("parameters"),
                error_message: row.get("error_message"),
                retry_count: row.get("retry_count"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(sub_tasks)
    }

    /// 删除主任务（级联删除子任务）
    pub async fn delete_master_task(&self, task_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM master_tasks WHERE id = $1")
            .bind(task_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("删除主任务失败: {}", e)))?;

        Ok(())
    }

    /// 获取子任务列表（包含样本信息）
    pub async fn list_sub_tasks_with_sample(
        &self,
        filter: SubTaskFilter,
        pagination: Pagination,
    ) -> Result<PagedResult<SubTaskWithSample>, AppError> {
        // 基础查询SQL（计数包含快照表关联）
        let mut count_query = sqlx::QueryBuilder::new("SELECT COUNT(*) as count ");
        count_query.push(
            r#"
            FROM sub_tasks st 
            JOIN samples s ON st.sample_id = s.id
            LEFT JOIN cape_instances ci ON st.cape_instance_id = ci.id
            LEFT JOIN cfg_instances cfi ON st.cfg_instance_id = cfi.id
            WHERE 1=1
            "#,
        );

        if let Some(master_task_id) = filter.master_task_id {
            count_query.push(" AND st.master_task_id = ");
            count_query.push_bind(master_task_id);
        }

        if let Some(sample_id) = filter.sample_id {
            count_query.push(" AND st.sample_id = ");
            count_query.push_bind(sample_id);
        }

        if let Some(status) = filter.status {
            count_query.push(" AND st.status = ");
            count_query.push_bind(status);
        }

        // 执行计数查询
        let count_row = count_query
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询子任务数量失败: {}", e)))?;

        let total: i64 = count_row.get("count");

        // 构建数据查询
        let mut data_query = sqlx::QueryBuilder::new(
            r#"SELECT 
                st.id,
                st.master_task_id,
                st.sample_id,
                st.analyzer_type,
                st.cape_instance_id,
                st.cfg_instance_id,
                ci.name as cape_instance_name,
                cfi.name as cfg_instance_name,
                st.external_task_id,
                st.status,
                st.priority,
                st.parameters,
                st.error_message,
                st.retry_count,
                st.created_at,
                st.started_at,
                st.completed_at,
                st.updated_at,
                s.file_name as sample_name,
                s.sample_type,
                s.file_size,
                s.file_hash_md5,
                s.file_hash_sha1,
                s.file_hash_sha256,
                s.labels,
                s.source "#,
        );
        data_query.push(
            r#"
            FROM sub_tasks st 
            JOIN samples s ON st.sample_id = s.id
            LEFT JOIN cape_instances ci ON st.cape_instance_id = ci.id
            LEFT JOIN cfg_instances cfi ON st.cfg_instance_id = cfi.id
            WHERE 1=1
            "#,
        );

        if let Some(master_task_id) = filter.master_task_id {
            data_query.push(" AND st.master_task_id = ");
            data_query.push_bind(master_task_id);
        }

        if let Some(sample_id) = filter.sample_id {
            data_query.push(" AND st.sample_id = ");
            data_query.push_bind(sample_id);
        }

        if let Some(status) = filter.status {
            data_query.push(" AND st.status = ");
            data_query.push_bind(status);
        }

        // 添加排序、分页
        let offset = (pagination.page - 1) * pagination.page_size;
        data_query.push(" ORDER BY st.created_at DESC LIMIT ");
        data_query.push_bind(pagination.page_size as i64);
        data_query.push(" OFFSET ");
        data_query.push_bind(offset as i64);

        // 执行数据查询
        let rows = data_query
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询子任务列表失败: {}", e)))?;

        let tasks: Vec<SubTaskWithSample> = rows
            .into_iter()
            .map(|row| {
                SubTaskWithSample {
                    id: row.get("id"),
                    master_task_id: row.get("master_task_id"),
                    sample_id: row.get("sample_id"),
                    analysis_system: format!("{:?}", row.get::<AnalyzerType, _>("analyzer_type")), // 映射到analysis_system字段
                    cape_instance_id: row.get("cape_instance_id"),
                    cfg_instance_id: row.get("cfg_instance_id"),
                    cape_instance_name: row.get("cape_instance_name"),
                    cfg_instance_name: row.get("cfg_instance_name"),
                    external_task_id: row.get("external_task_id"),
                    status: row.get("status"),
                    priority: row.get("priority"),
                    parameters: row.get("parameters"),
                    error_message: row.get("error_message"),
                    retry_count: row.get("retry_count"),
                    created_at: row.get("created_at"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    updated_at: row.get("updated_at"),
                    sample_name: row.get("sample_name"),
                    sample_type: row.get("sample_type"),
                    file_size: row.get("file_size"),
                    file_hash_md5: row.get("file_hash_md5"),
                    file_hash_sha1: row.get("file_hash_sha1"),
                    file_hash_sha256: row.get("file_hash_sha256"),
                    labels: row.get("labels"),
                    source: row.get("source"),
                }
            })
            .collect();

        Ok(PagedResult {
            items: tasks,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages: (total as f64 / pagination.page_size as f64).ceil() as u32,
        })
    }

    /// 获取子任务列表（包含样本信息和关键词搜索）
    pub async fn list_sub_tasks_with_sample_and_keyword(
        &self,
        filter: SubTaskFilter,
        pagination: Pagination,
        keyword: Option<String>,
    ) -> Result<PagedResult<SubTaskWithSample>, AppError> {
        // 基础查询SQL（已移除快照表依赖）
        let base_sql = r#"
            FROM sub_tasks st 
            JOIN samples s ON st.sample_id = s.id
            LEFT JOIN cape_instances ci ON st.cape_instance_id = ci.id
            LEFT JOIN cfg_instances cfi ON st.cfg_instance_id = cfi.id
        "#;

        // 构建计数查询（关键词版本）
        let mut count_query = sqlx::QueryBuilder::new("SELECT COUNT(*) as count ");
        count_query.push(
            r#"
            FROM sub_tasks st 
            JOIN samples s ON st.sample_id = s.id
            WHERE 1=1
            "#,
        );

        if let Some(master_task_id) = filter.master_task_id {
            count_query.push(" AND st.master_task_id = ");
            count_query.push_bind(master_task_id);
        }

        if let Some(sample_id) = filter.sample_id {
            count_query.push(" AND st.sample_id = ");
            count_query.push_bind(sample_id);
        }

        if let Some(status) = filter.status {
            count_query.push(" AND st.status = ");
            count_query.push_bind(status);
        }

        // 添加关键词搜索条件
        let search_pattern = if let Some(kw) = keyword.as_ref() {
            if !kw.trim().is_empty() {
                count_query.push(" AND ");

                let pattern = format!("%{}%", kw.trim());
                count_query.push("(s.file_name ILIKE ");
                count_query.push_bind(pattern.clone());
                count_query.push(" OR s.file_hash_md5 = ");
                count_query.push_bind(kw.trim());
                count_query.push(" OR s.file_hash_sha1 = ");
                count_query.push_bind(kw.trim());
                count_query.push(" OR s.file_hash_sha256 = ");
                count_query.push_bind(kw.trim());
                count_query.push(")");
                Some(pattern)
            } else {
                None
            }
        } else {
            None
        };

        // 执行计数查询
        let count_row = count_query
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询子任务数量失败: {}", e)))?;

        let total: i64 = count_row.get("count");

        // 构建数据查询
        let mut data_query = sqlx::QueryBuilder::new(
            r#"SELECT 
                st.id,
                st.master_task_id,
                st.sample_id,
                st.analyzer_type,
                st.cape_instance_id,
                st.cfg_instance_id,
                ci.name as cape_instance_name,
                cfi.name as cfg_instance_name,
                st.external_task_id,
                st.status,
                st.priority,
                st.parameters,
                st.error_message,
                st.retry_count,
                st.created_at,
                st.started_at,
                st.completed_at,
                st.updated_at,
                false as runtime_snapshot_available,
                s.file_name as sample_name,
                s.sample_type,
                s.file_size,
                s.file_hash_md5,
                s.file_hash_sha1,
                s.file_hash_sha256,
                s.labels,
                s.source "#,
        );
        data_query.push(base_sql);

        // 重复添加WHERE条件
        let mut has_conditions = false;
        if let Some(master_task_id) = filter.master_task_id {
            if !has_conditions {
                data_query.push(" WHERE ");
                has_conditions = true;
            } else {
                data_query.push(" AND ");
            }
            data_query.push("st.master_task_id = ");
            data_query.push_bind(master_task_id);
        }

        if let Some(sample_id) = filter.sample_id {
            if !has_conditions {
                data_query.push(" WHERE ");
                has_conditions = true;
            } else {
                data_query.push(" AND ");
            }
            data_query.push("st.sample_id = ");
            data_query.push_bind(sample_id);
        }

        if let Some(status) = filter.status {
            if !has_conditions {
                data_query.push(" WHERE ");
                has_conditions = true;
            } else {
                data_query.push(" AND ");
            }
            data_query.push("st.status = ");
            data_query.push_bind(status);
        }

        // 再次添加关键词搜索条件
        if let Some(pattern) = search_pattern.as_ref() {
            if let Some(kw) = keyword.as_ref() {
                if !has_conditions {
                    data_query.push(" WHERE ");
                } else {
                    data_query.push(" AND ");
                }

                data_query.push("(s.file_name ILIKE ");
                data_query.push_bind(pattern);
                data_query.push(" OR s.file_hash_md5 = ");
                data_query.push_bind(kw.trim());
                data_query.push(" OR s.file_hash_sha1 = ");
                data_query.push_bind(kw.trim());
                data_query.push(" OR s.file_hash_sha256 = ");
                data_query.push_bind(kw.trim());
                data_query.push(")");
            }
        }

        // 添加排序、分页
        let offset = (pagination.page - 1) * pagination.page_size;
        data_query.push(" ORDER BY st.created_at DESC LIMIT ");
        data_query.push_bind(pagination.page_size as i64);
        data_query.push(" OFFSET ");
        data_query.push_bind(offset as i64);

        // 执行数据查询
        let rows = data_query
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("查询子任务列表失败: {}", e)))?;

        let tasks: Vec<SubTaskWithSample> = rows
            .into_iter()
            .map(|row| SubTaskWithSample {
                id: row.get("id"),
                master_task_id: row.get("master_task_id"),
                sample_id: row.get("sample_id"),
                analysis_system: format!("{:?}", row.get::<AnalyzerType, _>("analyzer_type")),
                cape_instance_id: row.get("cape_instance_id"),
                cfg_instance_id: row.get("cfg_instance_id"),
                cape_instance_name: row.get("cape_instance_name"),
                cfg_instance_name: row.get("cfg_instance_name"),
                external_task_id: row.get("external_task_id"),
                status: row.get("status"),
                priority: row.get("priority"),
                parameters: row.get("parameters"),
                error_message: row.get("error_message"),
                retry_count: row.get("retry_count"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                updated_at: row.get("updated_at"),
                sample_name: row.get("sample_name"),
                sample_type: row.get("sample_type"),
                file_size: row.get("file_size"),
                file_hash_md5: row.get("file_hash_md5"),
                file_hash_sha1: row.get("file_hash_sha1"),
                file_hash_sha256: row.get("file_hash_sha256"),
                labels: row.get("labels"),
                source: row.get("source"),
            })
            .collect();

        Ok(PagedResult {
            items: tasks,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages: (total as f64 / pagination.page_size as f64).ceil() as u32,
        })
    }

    /// 暂停主任务
    pub async fn pause_master_task(
        &self,
        master_task_id: Uuid,
        reason: Option<String>,
    ) -> Result<MasterTask, AppError> {
        let now = Utc::now();

        let query = r#"
            UPDATE master_tasks
            SET status = 'paused', paused_at = $2, pause_reason = $3, updated_at = $4
            WHERE id = $1 AND status IN ('pending', 'running')
            RETURNING *
        "#;

        let row = sqlx::query(query)
            .bind(master_task_id)
            .bind(now)
            .bind(reason)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("暂停主任务失败: {}", e)))?;

        match row {
            Some(r) => Ok(MasterTask {
                id: r.get("id"),
                task_name: r.get("task_name"),
                analyzer_type: r.get("analyzer_type"),
                task_type: r.get("task_type"),
                total_samples: r.get("total_samples"),
                completed_samples: r.get("completed_samples"),
                failed_samples: r.get("failed_samples"),
                status: r.get("status"),
                progress: r.get("progress"),
                error_message: r.get("error_message"),
                result_summary: r.get("result_summary"),
                sample_filter: r.get("sample_filter"),
                paused_at: r.get("paused_at"),
                pause_reason: r.get("pause_reason"),
                created_by: r.get("created_by"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }),
            None => Err(AppError::bad_request("任务状态不允许暂停或任务不存在")),
        }
    }

    /// 恢复主任务
    pub async fn resume_master_task(&self, master_task_id: Uuid) -> Result<MasterTask, AppError> {
        let now = Utc::now();

        let query = r#"
            UPDATE master_tasks
            SET status = 'running', updated_at = $2, pause_reason = NULL, paused_at = NULL
            WHERE id = $1 AND status = 'paused'
            RETURNING *
        "#;

        let row = sqlx::query(query)
            .bind(master_task_id)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("恢复主任务失败: {}", e)))?;

        // 恢复所有已暂停的子任务为 pending，确保恢复动作完整
        // 若恢复子任务失败，直接返回错误，避免主任务已running但子任务仍停留在paused
        if let Err(e) = self.resume_paused_sub_tasks(master_task_id).await {
            return Err(AppError::service_unavailable(format!(
                "恢复子任务失败: {}",
                e
            )));
        }

        match row {
            Some(r) => Ok(MasterTask {
                id: r.get("id"),
                task_name: r.get("task_name"),
                analyzer_type: r.get("analyzer_type"),
                task_type: r.get("task_type"),
                total_samples: r.get("total_samples"),
                completed_samples: r.get("completed_samples"),
                failed_samples: r.get("failed_samples"),
                status: r.get("status"),
                progress: r.get("progress"),
                error_message: r.get("error_message"),
                result_summary: r.get("result_summary"),
                sample_filter: r.get("sample_filter"),
                paused_at: r.get("paused_at"),
                pause_reason: r.get("pause_reason"),
                created_by: r.get("created_by"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }),
            None => Err(AppError::bad_request("任务未处于暂停状态或任务不存在")),
        }
    }

    /// 暂停待调度的子任务
    pub async fn pause_pending_sub_tasks(&self, master_task_id: Uuid) -> Result<u64, AppError> {
        let now = Utc::now();

        let query = r#"
            UPDATE sub_tasks
            SET status = 'paused', updated_at = $2
            WHERE master_task_id = $1 AND status IN ('pending', 'submitting')
        "#;

        let result = sqlx::query(query)
            .bind(master_task_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("暂停子任务失败: {}", e)))?;

        Ok(result.rows_affected())
    }

    /// 恢复暂停的子任务
    pub async fn resume_paused_sub_tasks(&self, master_task_id: Uuid) -> Result<u64, AppError> {
        let now = Utc::now();

        let query = r#"
            UPDATE sub_tasks
            SET status = 'pending', error_message = NULL, updated_at = $2
            WHERE master_task_id = $1 AND status = 'paused'
        "#;

        let result = sqlx::query(query)
            .bind(master_task_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::service_unavailable(format!("恢复子任务失败: {}", e)))?;

        Ok(result.rows_affected())
    }
}
