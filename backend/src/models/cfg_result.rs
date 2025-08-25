use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CfgAnalysisResult {
    pub id: Uuid,
    pub sub_task_id: Uuid,
    pub sample_id: Uuid,
    pub message: Option<String>,
    pub result_files: Option<serde_json::Value>,
    pub full_report: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
