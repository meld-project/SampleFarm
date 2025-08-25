use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// CAPE分析结果数据模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CapeAnalysisResult {
    pub id: Uuid,
    /// 子任务ID
    pub sub_task_id: Uuid,
    /// 样本ID
    pub sample_id: Uuid,
    /// CAPE系统中的任务ID
    pub cape_task_id: i32,

    // 基础信息
    /// 分析开始时间
    pub analysis_started_at: Option<DateTime<Utc>>,
    /// 分析完成时间
    pub analysis_completed_at: Option<DateTime<Utc>>,
    /// 分析耗时（秒）
    pub analysis_duration: Option<i32>,

    // 分析结果摘要
    /// 恶意评分 (0-10)
    pub score: Option<f32>,
    /// 严重程度：low/medium/high/critical
    pub severity: Option<String>,
    /// 分析判定：clean/suspicious/malicious
    pub verdict: Option<String>,

    // 检测信息（JSONB格式）
    /// 命中的特征签名
    pub signatures: Option<JsonValue>,
    /// 行为摘要
    pub behavior_summary: Option<JsonValue>,

    // 完整报告
    /// CAPE完整JSON报告
    pub full_report: Option<JsonValue>,
    /// 报告摘要文本
    pub report_summary: Option<String>,

    /// 子任务错误信息（来自 sub_tasks.error_message，便于前端展示失败原因）
    pub error_message: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建CAPE分析结果的请求模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCapeResultRequest {
    pub sub_task_id: Uuid,
    pub sample_id: Uuid,
    pub cape_task_id: i32,
    pub full_report: JsonValue,
}

/// CAPE分析结果摘要（用于列表显示）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CapeResultSummary {
    pub id: Uuid,
    pub sub_task_id: Uuid,
    pub sample_id: Uuid,
    pub cape_task_id: i32,
    pub score: Option<f32>,
    pub severity: Option<String>,
    pub verdict: Option<String>,
    pub analysis_completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 样本查询参数 (从 sample_full.rs 复制过来的定义)
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct SampleQueryParams {
    /// 文件名模糊搜索
    pub file_name: Option<String>,
    /// 文件类型
    pub file_type: Option<String>,
    /// 样本类型
    pub sample_type: Option<super::SampleType>,
    /// MD5哈希
    pub file_hash_md5: Option<String>,
    /// SHA1哈希
    pub file_hash_sha1: Option<String>,
    /// SHA256哈希
    pub file_hash_sha256: Option<String>,
    /// 最小文件大小
    pub min_size: Option<i64>,
    /// 最大文件大小
    pub max_size: Option<i64>,
    /// 上传者
    pub uploader: Option<String>,
    /// 来源
    pub source: Option<String>,
    /// 标签
    pub labels: Option<Vec<String>>,
    /// 是否为容器文件
    pub is_container: Option<bool>,
    /// 父样本ID
    pub parent_id: Option<Uuid>,
    /// 开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 页码
    pub page: Option<u32>,
    /// 每页数量
    pub page_size: Option<u32>,
    /// 排序字段
    pub sort_by: Option<String>,
    /// 排序方向
    pub sort_order: Option<String>,
}

/// 任务预览请求模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct TaskPreviewRequest {
    /// 分析器类型
    pub analyzer_type: super::AnalyzerType,
    /// 文件名模糊搜索
    pub file_name: Option<String>,
    /// 文件类型
    pub file_type: Option<String>,
    /// 样本类型
    pub sample_type: Option<super::SampleType>,
    /// MD5哈希
    pub file_hash_md5: Option<String>,
    /// SHA1哈希
    pub file_hash_sha1: Option<String>,
    /// SHA256哈希
    pub file_hash_sha256: Option<String>,
    /// 最小文件大小
    pub min_size: Option<i64>,
    /// 最大文件大小
    pub max_size: Option<i64>,
    /// 上传者
    pub uploader: Option<String>,
    /// 来源
    pub source: Option<String>,
    /// 标签
    pub labels: Option<Vec<String>>,
    /// 是否为容器文件
    pub is_container: Option<bool>,
    /// 父样本ID
    pub parent_id: Option<Uuid>,
    /// 开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,
}

/// 任务预览响应模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskPreviewResponse {
    /// 匹配的样本总数
    pub total_samples: i64,
    /// 样本总大小（字节）
    pub total_size: i64,
    /// 文件类型分布
    pub file_type_distribution: Vec<FileTypeCount>,
    /// 样本类型分布
    pub sample_type_distribution: Vec<SampleTypeCount>,
    /// 来源分布
    pub source_distribution: Vec<SourceCount>,
    /// 预计分析时间（分钟）
    pub estimated_duration_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileTypeCount {
    pub file_type: String,
    pub count: i64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SampleTypeCount {
    pub sample_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceCount {
    pub source: String,
    pub count: i64,
}

impl CapeAnalysisResult {
    /// 从CAPE API响应解析分析结果
    pub fn from_cape_report(
        sub_task_id: Uuid,
        sample_id: Uuid,
        cape_task_id: i32,
        report: JsonValue,
    ) -> Self {
        // 解析基础信息
        let score = report["info"]["score"].as_f64().map(|s| s as f32);

        let severity = if let Some(score) = score {
            if score >= 8.0 {
                Some("critical".to_string())
            } else if score >= 6.0 {
                Some("high".to_string())
            } else if score >= 4.0 {
                Some("medium".to_string())
            } else {
                Some("low".to_string())
            }
        } else {
            None
        };

        let verdict = if let Some(score) = score {
            if score >= 7.0 {
                Some("malicious".to_string())
            } else if score >= 3.0 {
                Some("suspicious".to_string())
            } else {
                Some("clean".to_string())
            }
        } else {
            None
        };

        // 解析分析时间
        let analysis_started_at = report["info"]["started"]
            .as_str()
            .and_then(|s| DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
            .map(|dt| dt.with_timezone(&Utc));

        let analysis_completed_at = report["info"]["ended"]
            .as_str()
            .and_then(|s| DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
            .map(|dt| dt.with_timezone(&Utc));

        let analysis_duration = report["info"]["duration"].as_i64().map(|d| d as i32);

        // 解析检测信息（保存为JSONB）
        let signatures = report.get("signatures").cloned();
        let behavior_summary = report.get("behavior").cloned();

        // 生成报告摘要（基于关键指标）
        let mut summary_parts = vec![];
        if let Some(score) = score {
            summary_parts.push(format!("恶意评分: {:.1}/10", score));
        }
        if let Some(sig_count) = signatures
            .as_ref()
            .and_then(|s| s.as_array())
            .map(|a| a.len())
        {
            summary_parts.push(format!("命中{}个特征签名", sig_count));
        }
        // 从网络活动中提取域名数量用于摘要
        if let Some(domains) = report["network"]["domains"].as_array() {
            let domain_count = domains.len();
            if domain_count > 0 {
                summary_parts.push(format!("访问{}个域名", domain_count));
            }
        }
        let report_summary = if summary_parts.is_empty() {
            None
        } else {
            Some(summary_parts.join(", "))
        };

        Self {
            id: Uuid::new_v4(),
            sub_task_id,
            sample_id,
            cape_task_id,
            analysis_started_at,
            analysis_completed_at,
            analysis_duration,
            score,
            severity,
            verdict,
            signatures,
            behavior_summary,
            full_report: Some(report),
            report_summary,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
