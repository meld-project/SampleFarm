use super::{AnalyzerType, Entity, SampleQueryParams};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// 主任务状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "master_task_status_enum", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MasterTaskStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 子任务状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "sub_task_status_enum", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubTaskStatus {
    /// 等待提交
    Pending,
    /// 正在提交
    Submitting,
    /// 已提交
    Submitted,
    /// 分析中
    Analyzing,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    /// 批量分析
    Batch,
    /// 单文件分析
    Single,
}

/// 主任务数据模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MasterTask {
    pub id: Uuid,
    /// 任务名称
    pub task_name: String,
    /// 分析器类型
    pub analyzer_type: AnalyzerType,
    /// 任务类型
    pub task_type: String, // 使用String存储，因为不是数据库枚举
    /// 样本总数
    pub total_samples: i32,
    /// 已完成样本数
    pub completed_samples: i32,
    /// 失败样本数
    pub failed_samples: i32,
    /// 任务状态
    pub status: MasterTaskStatus,
    /// 进度百分比 (0-100)
    pub progress: i32,
    /// 错误信息
    pub error_message: Option<String>,
    /// 结果摘要
    pub result_summary: Option<JsonValue>,
    /// 样本筛选条件
    pub sample_filter: Option<JsonValue>,
    /// 暂停时间
    pub paused_at: Option<DateTime<Utc>>,
    /// 暂停原因
    pub pause_reason: Option<String>,
    /// 创建者ID
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for MasterTask {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }
}

/// 子任务数据模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SubTask {
    pub id: Uuid,
    /// 主任务ID
    pub master_task_id: Uuid,
    /// 样本ID
    pub sample_id: Uuid,
    /// 分析器类型
    pub analyzer_type: AnalyzerType,
    /// CAPE实例ID（可选，NULL表示使用默认实例）
    pub cape_instance_id: Option<Uuid>,
    /// CFG实例ID（可选，NULL表示使用默认实例）
    pub cfg_instance_id: Option<Uuid>,
    /// 外部任务ID (如CAPE task_id)
    pub external_task_id: Option<String>,
    /// 子任务状态
    pub status: SubTaskStatus,
    /// 优先级
    pub priority: i32,
    /// 任务参数
    pub parameters: Option<JsonValue>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 重试次数
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for SubTask {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }
}

/// 创建主任务的请求模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMasterTaskRequest {
    /// 任务名称
    pub task_name: String,
    /// 分析器类型
    pub analyzer_type: AnalyzerType,
    /// 任务类型
    pub task_type: TaskType,
    /// 样本ID列表
    pub sample_ids: Vec<Uuid>,
    /// CAPE实例ID（可选，NULL表示使用默认实例）
    pub cape_instance_id: Option<Uuid>,
    /// CAPE实例ID列表（优先于单个ID）
    pub cape_instance_ids: Option<Vec<Uuid>>,
    /// CFG实例ID列表
    pub cfg_instance_ids: Option<Vec<Uuid>>,
    /// 分析参数
    pub parameters: Option<JsonValue>,
}

/// 按筛选条件创建主任务的请求模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaskByFilterRequest {
    /// 任务名称
    pub task_name: String,
    /// 分析器类型
    pub analyzer_type: AnalyzerType,
    /// 任务类型
    pub task_type: TaskType,
    /// CAPE实例ID列表
    pub cape_instance_ids: Option<Vec<uuid::Uuid>>,
    /// CFG实例ID列表
    pub cfg_instance_ids: Option<Vec<uuid::Uuid>>,
    /// 分析参数
    pub parameters: Option<serde_json::Value>,
    /// 样本筛选条件（平铺字段，与预览接口一致）
    #[serde(flatten)]
    pub filter: SampleQueryParams,
}

/// 创建子任务的请求模型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSubTaskRequest {
    pub master_task_id: Uuid,
    pub sample_id: Uuid,
    pub analyzer_type: AnalyzerType,
    pub cape_instance_id: Option<Uuid>,
    pub priority: Option<i32>,
    pub parameters: Option<JsonValue>,
}

/// 更新主任务的请求模型
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateMasterTaskRequest {
    pub status: Option<MasterTaskStatus>,
    pub progress: Option<i32>,
    pub completed_samples: Option<i32>,
    pub failed_samples: Option<i32>,
    pub error_message: Option<String>,
    pub result_summary: Option<JsonValue>,
}

/// 更新子任务的请求模型
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateSubTaskStatusRequest {
    pub status: Option<SubTaskStatus>,
    pub external_task_id: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 任务查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct TaskFilter {
    pub analyzer_type: Option<AnalyzerType>,
    pub task_type: Option<TaskType>,
    pub status: Option<MasterTaskStatus>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 子任务查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct SubTaskFilter {
    pub master_task_id: Option<Uuid>,
    pub sample_id: Option<Uuid>,
    pub analyzer_type: Option<AnalyzerType>,
    pub status: Option<SubTaskStatus>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 任务详情（包含子任务）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    pub master_task: MasterTask,
    pub sub_tasks: Vec<SubTaskWithSample>,
}

/// 子任务与样本信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SubTaskWithSample {
    // SubTask字段
    pub id: Uuid,
    pub master_task_id: Uuid,
    pub sample_id: Uuid,
    pub analysis_system: String,
    pub cape_instance_id: Option<Uuid>,
    pub cfg_instance_id: Option<Uuid>,
    /// CAPE实例名称
    pub cape_instance_name: Option<String>,
    /// CFG实例名称
    pub cfg_instance_name: Option<String>,
    pub external_task_id: Option<String>,
    pub status: SubTaskStatus,
    pub priority: i32,
    pub parameters: Option<JsonValue>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    // Sample字段
    pub sample_name: String,
    pub sample_type: super::SampleType,
    pub file_size: i64,
    pub file_hash_md5: String,
    pub file_hash_sha1: String,
    pub file_hash_sha256: String,
    pub labels: Option<JsonValue>,
    pub source: Option<String>,
}

/// 任务统计信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaskStats {
    pub total_tasks: i64,
    pub pending_tasks: i64,
    pub running_tasks: i64,
    pub completed_tasks: i64,
    pub failed_tasks: i64,
    pub total_sub_tasks: i64,
    pub pending_sub_tasks: i64,
    pub running_sub_tasks: i64,
    pub completed_sub_tasks: i64,
    pub failed_sub_tasks: i64,
}

impl TaskFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_filters(&self) -> bool {
        self.analyzer_type.is_some()
            || self.task_type.is_some()
            || self.status.is_some()
            || self.start_time.is_some()
            || self.end_time.is_some()
    }
}

impl SubTaskFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_filters(&self) -> bool {
        self.master_task_id.is_some()
            || self.sample_id.is_some()
            || self.analyzer_type.is_some()
            || self.status.is_some()
            || self.start_time.is_some()
            || self.end_time.is_some()
    }
}
