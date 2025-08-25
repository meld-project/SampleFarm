use crate::{
    database::Database,
    error::AppResult,
    models::{
        CreateSampleRequest, DailyUploadCount, FileSizeDistribution, FileTypeDistribution,
        PagedResult, Pagination, Sample, SampleFilter, SampleStats, SampleStatsExtended,
        SourceDistribution, UpdateSampleRequest,
    },
};
use sqlx::{Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// 样本仓库
#[derive(Clone)]
pub struct SampleRepository {
    db: Database,
}

impl SampleRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// 创建样本
    pub async fn create(&self, request: CreateSampleRequest) -> AppResult<Sample> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let sample = sqlx::query_as::<_, Sample>(
            r#"
            INSERT INTO samples (
                id, file_name, file_size, file_hash_md5, file_hash_sha1, file_hash_sha256, 
                file_type, file_extension, sample_type, source, storage_path,
                is_container, parent_id, file_path_in_zip, has_custom_metadata,
                labels, custom_metadata, zip_password, run_filename, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            RETURNING 
                id, file_name, file_size, file_hash_md5, file_hash_sha1, file_hash_sha256,
                file_type, file_extension, sample_type, 
                source, storage_path, is_container, parent_id, file_path_in_zip,
                has_custom_metadata, labels, custom_metadata, zip_password, run_filename, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(request.file_name)
        .bind(request.file_size)
        .bind(request.file_hash_md5)
        .bind(request.file_hash_sha1)
        .bind(request.file_hash_sha256)
        .bind(request.file_type)
        .bind(request.file_extension)
        .bind(request.sample_type)
        .bind(request.source)
        .bind(request.storage_path)
        .bind(request.is_container)
        .bind(request.parent_id)
        .bind(request.file_path_in_zip)
        .bind(request.has_custom_metadata)
        .bind(serde_json::to_value(&request.labels).unwrap_or(serde_json::Value::Null))
        .bind(request.custom_metadata)
        .bind(request.zip_password)
        .bind(request.run_filename)
        .bind(now)
        .bind(now)
        .fetch_one(self.db.pool())
        .await?;

        Ok(sample)
    }

    /// 根据ID查找样本
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Sample>> {
        let sample = sqlx::query_as::<_, Sample>(
            r#"
            SELECT 
                id, file_name, file_size, file_hash_md5, file_hash_sha1, file_hash_sha256,
                file_type, file_extension, sample_type, 
                source, storage_path, is_container, parent_id, file_path_in_zip,
                has_custom_metadata, labels, custom_metadata, zip_password, run_filename, created_at, updated_at
            FROM samples 
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(sample)
    }

    /// 根据MD5查找样本
    pub async fn find_by_md5(&self, md5: &str) -> AppResult<Option<Sample>> {
        let sample = sqlx::query_as::<_, Sample>(
            r#"
            SELECT 
                id, file_name, file_size, file_hash_md5, file_hash_sha1, file_hash_sha256,
                file_type, file_extension, sample_type, 
                source, storage_path, is_container, parent_id, file_path_in_zip,
                has_custom_metadata, labels, custom_metadata, zip_password, run_filename, created_at, updated_at
            FROM samples 
            WHERE file_hash_md5 = $1
            "#
        )
        .bind(md5)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(sample)
    }

    /// 根据哈希查找样本（MD5或SHA256）
    pub async fn find_by_hash(&self, md5: &str, sha256: &str) -> AppResult<Option<Sample>> {
        let sample = sqlx::query_as::<_, Sample>(
            r#"
            SELECT 
                id, file_name, file_size, file_hash_md5, file_hash_sha1, file_hash_sha256,
                file_type, file_extension, sample_type, 
                source, storage_path, is_container, parent_id, file_path_in_zip,
                has_custom_metadata, labels, custom_metadata, zip_password, run_filename, created_at, updated_at
            FROM samples 
            WHERE file_hash_md5 = $1 OR file_hash_sha1 = $2 OR file_hash_sha256 = $3
            "#
        )
        .bind(md5)
        .bind(sha256)
        .bind(sha256)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(sample)
    }

    /// 分页查询样本列表
    pub async fn list(
        &self,
        filter: SampleFilter,
        pagination: &Pagination,
    ) -> AppResult<PagedResult<Sample>> {
        let mut query_builder = QueryBuilder::new(
            r#"
            SELECT 
                id, file_name, file_size, file_hash_md5, file_hash_sha1, file_hash_sha256,
                file_type, file_extension, sample_type, 
                source, storage_path, is_container, parent_id, file_path_in_zip,
                has_custom_metadata, labels, custom_metadata, zip_password, run_filename, created_at, updated_at
            FROM samples
            "#,
        );

        let mut count_builder = QueryBuilder::new("SELECT COUNT(*) FROM samples");

        // 应用过滤条件
        if filter.has_filters() {
            query_builder.push(" WHERE ");
            count_builder.push(" WHERE ");
            self.apply_filter(&mut query_builder, &mut count_builder, &filter);
        }

        // 添加排序
        query_builder.push(" ORDER BY created_at DESC");

        // 添加分页
        let offset = (pagination.page - 1) * pagination.page_size;
        query_builder.push(" LIMIT ");
        query_builder.push_bind(pagination.page_size as i64);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset as i64);

        // 执行查询
        let samples = query_builder
            .build_query_as::<Sample>()
            .fetch_all(self.db.pool())
            .await?;

        let total = count_builder
            .build_query_scalar::<i64>()
            .fetch_one(self.db.pool())
            .await?;

        Ok(PagedResult::new(
            samples,
            total,
            pagination.page,
            pagination.page_size,
        ))
    }

    /// 更新样本
    pub async fn update(&self, id: Uuid, request: UpdateSampleRequest) -> AppResult<Sample> {
        let sample = sqlx::query_as::<_, Sample>(
            r#"
            UPDATE samples SET
                sample_type = COALESCE($2, sample_type),
                source = COALESCE($3, source),
                labels = COALESCE($4, labels),
                custom_metadata = COALESCE($5, custom_metadata),
                zip_password = COALESCE($6, zip_password),
                run_filename = COALESCE($7, run_filename),
                updated_at = $8
            WHERE id = $1
            RETURNING 
                id, file_name, file_size, file_hash_md5, file_hash_sha1, file_hash_sha256,
                file_type, file_extension, sample_type, 
                source, storage_path, is_container, parent_id, file_path_in_zip,
                has_custom_metadata, labels, custom_metadata, zip_password, run_filename, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(request.sample_type)
        .bind(request.source)
        .bind(serde_json::to_value(&request.labels).unwrap_or(serde_json::Value::Null))
        .bind(request.custom_metadata)
        .bind(request.zip_password)
        .bind(request.run_filename)
        .bind(chrono::Utc::now())
        .fetch_one(self.db.pool())
        .await?;

        Ok(sample)
    }

    /// 删除样本
    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let mut tx = self.db.pool().begin().await?;
        // 先删除引用该样本的子任务（其下属的分析结果表已在schema中设置 ON DELETE CASCADE）
        sqlx::query("DELETE FROM sub_tasks WHERE sample_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // 再删除样本本身
        sqlx::query("DELETE FROM samples WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// 获取样本统计信息
    pub async fn get_stats(&self) -> AppResult<SampleStats> {
        let stats = sqlx::query_as::<_, SampleStats>(
            r#"
            SELECT 
                COUNT(*) as total_samples,
                COUNT(*) FILTER (WHERE sample_type = 'Benign') as benign_samples,
                COUNT(*) FILTER (WHERE sample_type = 'Malicious') as malicious_samples,
                COUNT(*) FILTER (WHERE is_container = true) as container_files,
                COALESCE(SUM(file_size)::bigint, 0) as total_size
            FROM samples
            "#,
        )
        .fetch_one(self.db.pool())
        .await?;

        Ok(stats)
    }

    /// 获取扩展样本统计信息（包含分布数据）
    pub async fn get_stats_extended(&self) -> AppResult<SampleStatsExtended> {
        // 1. 基础统计
        let basic_stats = self.get_stats().await?;

        // 2. 文件类型分布
        let file_type_rows = sqlx::query(
            r#"
            SELECT 
                file_type,
                COUNT(*) as count,
                COALESCE(SUM(file_size), 0)::bigint as size
            FROM samples 
            GROUP BY file_type 
            ORDER BY count DESC
            LIMIT 10
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let file_type_distribution: Vec<FileTypeDistribution> = file_type_rows
            .into_iter()
            .map(|row| {
                let count: i64 = row.get("count");
                let percentage = if basic_stats.total_samples > 0 {
                    (count as f64 / basic_stats.total_samples as f64) * 100.0
                } else {
                    0.0
                };
                FileTypeDistribution {
                    file_type: row.get("file_type"),
                    count: row.get("count"),
                    size: row.get("size"),
                    percentage,
                }
            })
            .collect();

        // 3. 文件大小分布
        let file_size_rows = sqlx::query(
            r#"
            SELECT 
                CASE 
                    WHEN file_size < 1024 THEN '< 1KB'
                    WHEN file_size < 1024 * 1024 THEN '1KB - 1MB'
                    WHEN file_size < 10 * 1024 * 1024 THEN '1MB - 10MB'
                    WHEN file_size < 100 * 1024 * 1024 THEN '10MB - 100MB'
                    ELSE '> 100MB'
                END as size_range,
                COUNT(*) as count,
                COALESCE(SUM(file_size), 0)::bigint as total_size
            FROM samples 
            GROUP BY size_range 
            ORDER BY MIN(file_size)
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let file_size_distribution: Vec<FileSizeDistribution> = file_size_rows
            .into_iter()
            .map(|row| {
                let count: i64 = row.get("count");
                let percentage = if basic_stats.total_samples > 0 {
                    (count as f64 / basic_stats.total_samples as f64) * 100.0
                } else {
                    0.0
                };
                FileSizeDistribution {
                    size_range: row.get("size_range"),
                    count: row.get("count"),
                    total_size: row.get("total_size"),
                    percentage,
                }
            })
            .collect();

        // 4. 来源分布
        let source_rows = sqlx::query(
            r#"
            SELECT 
                COALESCE(source, '未知') as source,
                COUNT(*) as count
            FROM samples 
            GROUP BY source 
            ORDER BY count DESC
            LIMIT 10
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let source_distribution: Vec<SourceDistribution> = source_rows
            .into_iter()
            .map(|row| {
                let count: i64 = row.get("count");
                let percentage = if basic_stats.total_samples > 0 {
                    (count as f64 / basic_stats.total_samples as f64) * 100.0
                } else {
                    0.0
                };
                SourceDistribution {
                    source: row.get("source"),
                    count: row.get("count"),
                    percentage,
                }
            })
            .collect();

        // 5. 最近7天上传趋势
        let trend_rows = sqlx::query(
            r#"
            SELECT 
                DATE(created_at) as upload_date,
                COUNT(*) as count,
                COALESCE(SUM(file_size), 0)::bigint as size
            FROM samples 
            WHERE created_at >= CURRENT_DATE - INTERVAL '6 days'
            GROUP BY DATE(created_at)
            ORDER BY upload_date
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let recent_upload_trend: Vec<DailyUploadCount> = trend_rows
            .into_iter()
            .map(|row| DailyUploadCount {
                date: row.get::<chrono::NaiveDate, _>("upload_date").to_string(),
                count: row.get("count"),
                size: row.get("size"),
            })
            .collect();

        Ok(SampleStatsExtended {
            basic_stats,
            file_type_distribution,
            file_size_distribution,
            source_distribution,
            recent_upload_trend,
        })
    }

    /// 应用过滤条件
    fn apply_filter<'a>(
        &self,
        query_builder: &mut QueryBuilder<'a, Postgres>,
        count_builder: &mut QueryBuilder<'a, Postgres>,
        filter: &'a SampleFilter,
    ) {
        let mut first = true;

        if let Some(sample_type) = &filter.sample_type {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("sample_type = ");
            query_builder.push_bind(sample_type);
            count_builder.push("sample_type = ");
            count_builder.push_bind(sample_type);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(source) = &filter.source {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("source ILIKE ");
            query_builder.push_bind(format!("%{}%", source));
            count_builder.push("source ILIKE ");
            count_builder.push_bind(format!("%{}%", source));
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(filename) = &filter.filename {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("file_name ILIKE ");
            query_builder.push_bind(format!("%{}%", filename));
            count_builder.push("file_name ILIKE ");
            count_builder.push_bind(format!("%{}%", filename));
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(md5) = &filter.md5 {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("file_hash_md5 = ");
            query_builder.push_bind(md5);
            count_builder.push("file_hash_md5 = ");
            count_builder.push_bind(md5);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(sha1) = &filter.sha1 {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("file_hash_sha1 = ");
            query_builder.push_bind(sha1);
            count_builder.push("file_hash_sha1 = ");
            count_builder.push_bind(sha1);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(sha256) = &filter.sha256 {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("file_hash_sha256 = ");
            query_builder.push_bind(sha256);
            count_builder.push("file_hash_sha256 = ");
            count_builder.push_bind(sha256);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(is_container) = filter.is_container {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("is_container = ");
            query_builder.push_bind(is_container);
            count_builder.push("is_container = ");
            count_builder.push_bind(is_container);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(parent_id) = filter.parent_id {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("parent_id = ");
            query_builder.push_bind(parent_id);
            count_builder.push("parent_id = ");
            count_builder.push_bind(parent_id);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(labels) = &filter.labels {
            if !labels.is_empty() {
                if !first {
                    query_builder.push(" AND ");
                    count_builder.push(" AND ");
                }
                query_builder.push("labels && ");
                query_builder.push_bind(labels);
                count_builder.push("labels && ");
                count_builder.push_bind(labels);
                #[allow(unused_assignments)]
                {
                    first = false;
                }
            }
        }

        if let Some(start_time) = filter.start_time {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("created_at >= ");
            query_builder.push_bind(start_time);
            count_builder.push("created_at >= ");
            count_builder.push_bind(start_time);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }

        if let Some(end_time) = filter.end_time {
            if !first {
                query_builder.push(" AND ");
                count_builder.push(" AND ");
            }
            query_builder.push("created_at <= ");
            query_builder.push_bind(end_time);
            count_builder.push("created_at <= ");
            count_builder.push_bind(end_time);
            #[allow(unused_assignments)]
            {
                first = false;
            }
        }
    }
}
