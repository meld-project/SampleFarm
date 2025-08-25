use crate::{
    handlers::{
        analysis_result::{AnalysisResultQuery, AnalysisResultStats, SampleAnalysisHistory},
        cape_executor::{
            BatchExecuteRequest, BatchExecuteResponse, CapeTaskConfigRequest,
            PerformanceStatsResponse, TaskExecutionStatusResponse,
        },
        sample_full::{SampleQueryParams, UploadMetadata, UploadResponse},
        task_management::CreateTaskResponse,
    },
    models::{
        PagedResult, Sample, SampleStats, SampleStatsExtended, SampleType, UpdateSampleRequest,
        analyzer::AnalyzerType,
        cape_result::{
            CapeAnalysisResult, CapeResultSummary, TaskPreviewRequest, TaskPreviewResponse,
        },
        task::{
            CreateMasterTaskRequest, CreateSubTaskRequest, MasterTask, MasterTaskStatus, SubTask,
            SubTaskStatus, TaskFilter, TaskStats, UpdateMasterTaskRequest,
            UpdateSubTaskStatusRequest,
        },
    },
    response::ApiResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        // 样本管理API
        crate::handlers::sample_full::upload_file_full,
        crate::handlers::sample_full::list_samples,
        crate::handlers::sample_full::get_sample,
        crate::handlers::sample_full::update_sample,
        crate::handlers::sample_full::delete_sample,
        crate::handlers::sample_full::get_sample_stats,
        crate::handlers::sample_full::download_sample,
        // 任务管理API
        crate::handlers::task_preview::preview_task,
        crate::handlers::task_management::create_task,
        crate::handlers::task_management::list_tasks,
        crate::handlers::task_management::get_task,
        crate::handlers::task_management::update_task,
        crate::handlers::task_management::delete_task,
        crate::handlers::task_management::list_sub_tasks,
        crate::handlers::task_management::update_sub_task_status,
        crate::handlers::task_management::get_task_stats,
        // CAPE执行API
        crate::handlers::cape_executor::execute_cape_batch,
        crate::handlers::cape_executor::get_task_execution_status,
        crate::handlers::cape_executor::get_performance_stats,
        // CFG 实例管理API
        crate::handlers::cfg_instance::list_cfg_instances,
        crate::handlers::cfg_instance::get_cfg_instance,
        crate::handlers::cfg_instance::create_cfg_instance,
        crate::handlers::cfg_instance::update_cfg_instance,
        crate::handlers::cfg_instance::delete_cfg_instance,
        crate::handlers::cfg_instance::health_check_cfg_instance,
        crate::handlers::cfg_instance::get_cfg_instance_stats,
        // CFG 分析执行与查询 API
        crate::handlers::cfg_executor::execute_cfg_batch,
        crate::handlers::cfg_executor::get_cfg_task_status,
        crate::handlers::cfg_executor::get_cfg_analysis_detail,
        // 分析结果API
        crate::handlers::analysis_result::get_task_results,
        crate::handlers::analysis_result::get_sample_analysis_history,
        crate::handlers::analysis_result::get_cape_analysis_detail,
    ),
    components(
        schemas(
            // 样本相关模型
            Sample,
            SampleType,
            SampleStats,
            SampleQueryParams,
            UploadMetadata,
            UploadResponse,
            UpdateSampleRequest,
            // 任务相关模型
            MasterTask,
            SubTask,
            MasterTaskStatus,
            SubTaskStatus,
            AnalyzerType,
            CreateMasterTaskRequest,
            UpdateMasterTaskRequest,
            TaskFilter,
            CreateSubTaskRequest,
            UpdateSubTaskStatusRequest,
            TaskStats,
            TaskPreviewRequest,
            TaskPreviewResponse,
            // CAPE执行相关模型
            BatchExecuteRequest,
            BatchExecuteResponse,
            CapeTaskConfigRequest,
            TaskExecutionStatusResponse,
            PerformanceStatsResponse,
            // CFG 实例与执行相关模型
            crate::models::cfg_instance::CfgInstance,
            crate::models::cfg_instance::CreateCfgInstanceRequest,
            crate::models::cfg_instance::UpdateCfgInstanceRequest,
            crate::models::cfg_instance::CfgHealthStatus,
            crate::handlers::cfg_instance::CfgInstanceStatsResponse,
            crate::handlers::cfg_executor::CfgBatchExecuteRequest,
            crate::handlers::cfg_executor::CfgBatchExecuteResponse,
            crate::handlers::cfg_executor::CfgTaskStatusResponse,
            crate::handlers::cfg_executor::CfgAnalysisDetailResponse,
            // 任务管理相关模型
            CreateTaskResponse,
            // 分析结果相关模型
            CapeAnalysisResult,
            CapeResultSummary,
            AnalysisResultQuery,
            AnalysisResultStats,
            SampleAnalysisHistory,
            // 通用响应模型
            ApiResponse<Sample>,
            ApiResponse<SampleStats>,
            ApiResponse<SampleStatsExtended>,
            ApiResponse<UploadResponse>,
            ApiResponse<PagedResult<Sample>>,
            ApiResponse<PagedResult<MasterTask>>,
            ApiResponse<PagedResult<SubTask>>,
            ApiResponse<MasterTask>,
            ApiResponse<SubTask>,
            ApiResponse<TaskStats>,
            ApiResponse<TaskPreviewResponse>,
            ApiResponse<CreateTaskResponse>,
            ApiResponse<BatchExecuteResponse>,
            ApiResponse<TaskExecutionStatusResponse>,
            ApiResponse<PerformanceStatsResponse>,
            ApiResponse<String>,
            ApiResponse<PagedResult<crate::models::cfg_instance::CfgInstance>>,
            ApiResponse<crate::models::cfg_instance::CfgInstance>,
            ApiResponse<crate::models::cfg_instance::CfgHealthStatus>,
            ApiResponse<crate::handlers::cfg_instance::CfgInstanceStatsResponse>,
            ApiResponse<crate::handlers::cfg_executor::CfgBatchExecuteResponse>,
            ApiResponse<crate::handlers::cfg_executor::CfgTaskStatusResponse>,
            ApiResponse<crate::handlers::cfg_executor::CfgAnalysisDetailResponse>,
            ApiResponse<Vec<CapeAnalysisResult>>,
            ApiResponse<SampleAnalysisHistory>,
            ApiResponse<CapeAnalysisResult>,
            PagedResult<Sample>,
            PagedResult<MasterTask>,
            PagedResult<SubTask>,
            PagedResult<crate::models::cfg_instance::CfgInstance>,
        )
    ),
    tags(
        (name = "样本管理", description = "样本文件的上传、查询、管理和下载功能"),
        (name = "任务管理", description = "分析任务的创建、监控和管理功能"),
        (name = "CAPE分析", description = "CAPE沙箱分析的执行和监控功能"),
        (name = "CFG管理", description = "CFG实例的管理、健康检查与统计"),
        (name = "CFG分析", description = "CFG分析任务的提交、状态与结果查询"),
        (name = "分析结果管理", description = "分析结果的查询和展示功能"),
        (name = "系统监控", description = "系统健康状态和统计信息")
    ),
    info(
        title = "SampleFarm API",
        version = "1.0.0",
        description = "SampleFarm 样本管理系统 REST API 文档",
        contact(
            name = "SampleFarm Team",
            email = "contact@example.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "开发环境")
    )
)]
pub struct ApiDoc;
