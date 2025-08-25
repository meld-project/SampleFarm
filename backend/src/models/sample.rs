use super::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// 样本类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "sample_type_enum", rename_all = "PascalCase")]
pub enum SampleType {
    /// 安全样本
    Benign,
    /// 恶意样本
    Malicious,
}

/// 样本模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Sample {
    /// 样本唯一标识符（UUID v4）
    pub id: Uuid,
    /// 文件名（包含扩展名）
    pub file_name: String,
    /// 文件大小（字节）
    pub file_size: i64,
    /// MD5哈希值（32位十六进制字符串）
    pub file_hash_md5: String,
    /// SHA1哈希值（40位十六进制字符串）
    pub file_hash_sha1: String,
    /// SHA256哈希值（64位十六进制字符串）
    pub file_hash_sha256: String,
    /// 文件MIME类型（如：application/x-dosexec）
    pub file_type: String,
    /// 文件扩展名（如：exe、dll、zip）
    pub file_extension: Option<String>,
    /// 样本类型（安全/恶意）
    pub sample_type: SampleType,
    /// 样本来源（如：VirusTotal、用户上传、蜜罐等）
    pub source: Option<String>,
    /// 在对象存储中的路径
    pub storage_path: String,
    /// 是否为容器文件（ZIP、RAR等）
    pub is_container: bool,
    /// 父样本ID（如果是从ZIP中提取的文件）
    pub parent_id: Option<Uuid>,
    /// 在ZIP中的文件路径（如果是从ZIP中提取的）
    pub file_path_in_zip: Option<String>,
    /// 是否包含自定义元数据
    pub has_custom_metadata: bool,
    /// 标签列表（如：trojan、ransomware、backdoor等）
    #[sqlx(json)]
    pub labels: Option<Vec<String>>,
    /// 自定义元数据（JSON格式，可存储任意结构化数据）
    pub custom_metadata: Option<serde_json::Value>,
    /// ZIP文件密码（如果是加密的ZIP文件）
    pub zip_password: Option<String>,
    /// 运行时文件名（用于动态分析）
    pub run_filename: Option<String>,
    /// 创建时间（UTC时间）
    pub created_at: DateTime<Utc>,
    /// 最后更新时间（UTC时间）
    pub updated_at: DateTime<Utc>,
}

impl Entity for Sample {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }
}

/// 创建样本请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSampleRequest {
    pub file_name: String,
    pub file_size: i64,
    pub file_hash_md5: String,
    pub file_hash_sha1: String,
    pub file_hash_sha256: String,
    pub file_type: String,
    pub file_extension: Option<String>,
    pub sample_type: SampleType,
    pub source: Option<String>,
    pub storage_path: String,
    pub is_container: bool,
    pub parent_id: Option<Uuid>,
    pub file_path_in_zip: Option<String>,
    pub has_custom_metadata: bool,
    pub labels: Option<Vec<String>>,
    pub custom_metadata: Option<serde_json::Value>,
    pub zip_password: Option<String>,
    pub run_filename: Option<String>,
}

/// 更新样本请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSampleRequest {
    pub sample_type: Option<SampleType>,
    pub source: Option<String>,
    pub labels: Option<Vec<String>>,
    pub custom_metadata: Option<serde_json::Value>,
    pub zip_password: Option<String>,
    pub run_filename: Option<String>,
}

/// 样本查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleFilter {
    pub sample_type: Option<SampleType>,
    pub source: Option<String>,
    pub filename: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub is_container: Option<bool>,
    pub parent_id: Option<Uuid>,
    pub labels: Option<Vec<String>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl Default for SampleFilter {
    fn default() -> Self {
        Self {
            sample_type: None,
            source: None,
            filename: None,
            md5: None,
            sha1: None,
            sha256: None,
            is_container: None,
            parent_id: None,
            labels: None,
            start_time: None,
            end_time: None,
        }
    }
}

impl SampleFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_filters(&self) -> bool {
        self.sample_type.is_some()
            || self.source.is_some()
            || self.filename.is_some()
            || self.md5.is_some()
            || self.sha1.is_some()
            || self.sha256.is_some()
            || self.is_container.is_some()
            || self.parent_id.is_some()
            || self.labels.is_some()
            || self.start_time.is_some()
            || self.end_time.is_some()
    }
}

/// 样本统计信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SampleStats {
    /// 样本总数量
    pub total_samples: i64,
    /// 安全样本数量
    pub benign_samples: i64,
    /// 恶意样本数量
    pub malicious_samples: i64,
    /// 容器文件数量（如ZIP、RAR等）
    pub container_files: i64,
    /// 所有样本的总大小（字节）
    pub total_size: i64,
}

/// 扩展样本统计信息（包含分布数据）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SampleStatsExtended {
    /// 基础统计信息
    pub basic_stats: SampleStats,
    /// 文件类型分布
    pub file_type_distribution: Vec<FileTypeDistribution>,
    /// 文件大小分布
    pub file_size_distribution: Vec<FileSizeDistribution>,
    /// 来源分布
    pub source_distribution: Vec<SourceDistribution>,
    /// 最近上传趋势（按天）
    pub recent_upload_trend: Vec<DailyUploadCount>,
}

/// 文件类型分布
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileTypeDistribution {
    pub file_type: String,
    pub count: i64,
    pub size: i64,
    pub percentage: f64,
}

/// 文件大小分布
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileSizeDistribution {
    pub size_range: String,
    pub count: i64,
    pub total_size: i64,
    pub percentage: f64,
}

/// 来源分布
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceDistribution {
    pub source: String,
    pub count: i64,
    pub percentage: f64,
}

/// 每日上传数量
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DailyUploadCount {
    pub date: String,
    pub count: i64,
    pub size: i64,
}

/// 样本树结构（显示父子关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleTree {
    pub sample: Sample,
    pub children: Vec<SampleTree>,
}

impl SampleTree {
    pub fn new(sample: Sample) -> Self {
        Self {
            sample,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: SampleTree) {
        self.children.push(child);
    }
}
