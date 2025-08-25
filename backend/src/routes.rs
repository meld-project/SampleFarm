use crate::handlers::sample_full::AppState;
use crate::handlers::{
    batch_download_samples,
    create_cape_instance,
    create_cfg_instance,
    create_task,
    create_task_by_filter,
    delete_cape_instance,
    delete_cfg_instance,
    delete_sample,
    delete_task,
    download_sample,
    download_task_results_zip,
    execute_cape_batch,
    execute_cfg_batch,
    export_task_results_csv,
    get_all_health_status,
    get_cape_analysis_detail,
    get_cape_instance,
    get_cape_instance_stats,
    get_cape_runtime_snapshot,
    get_cfg_analysis_detail,
    get_cfg_instance,
    get_cfg_instance_stats as get_cfg_stats,
    get_cfg_task_status,
    get_performance_stats,
    get_sample,
    get_sample_analysis_history,
    get_sample_stats,
    get_sample_stats_extended,
    get_task,
    get_task_execution_status,
    // cfg handlers will be added
    get_task_results,
    get_task_runtime_status,
    get_task_stats,
    health_check_cape_instance,
    health_check_cfg_instance,
    list_cape_instances,
    // CFG 实例管理 handlers
    list_cfg_instances,
    list_samples,
    list_sub_tasks,
    list_tasks,
    pause_task,
    preview_task,
    resume_task,
    system_status,
    update_cape_instance,
    update_cfg_instance,
    update_sample,
    update_sub_task_status,
    update_task,
    upload_file_full,
    upload_file_simple,
};
use axum::{
    Router,
    routing::{delete as axum_delete, get, post, put},
};

/// 创建API路由
pub fn create_api_routes() -> Router<AppState> {
    Router::new()
        // 系统状态（在API路径下）
        .route("/api/status", get(system_status))
        // 样本管理API
        .route("/api/samples/upload", post(upload_file_full)) // 完整上传功能
        .route("/api/samples/upload-simple", post(upload_file_simple)) // 简单上传（用于测试）
        .route("/api/samples", get(list_samples)) // 查询样本列表
        .route("/api/samples/stats", get(get_sample_stats)) // 样本统计
        .route(
            "/api/samples/stats/extended",
            get(get_sample_stats_extended),
        ) // 扩展样本统计
        .route("/api/samples/{id}", get(get_sample)) // 获取单个样本
        .route("/api/samples/{id}", put(update_sample)) // 更新样本
        .route("/api/samples/{id}", axum_delete(delete_sample)) // 删除样本
        .route("/api/samples/{id}/download", get(download_sample)) // 下载样本文件
        .route("/api/samples/batch/download", post(batch_download_samples)) // 批量下载ZIP
        .route(
            "/api/samples/batch",
            axum_delete(crate::handlers::sample_full::batch_delete_samples),
        ) // 批量删除
        // 任务管理API
        .route("/api/tasks/preview", get(preview_task)) // 任务预览（根据筛选条件获取统计）
        .route("/api/tasks/stats", get(get_task_stats)) // 任务统计信息
        .route("/api/tasks", post(create_task)) // 创建任务
        .route("/api/tasks/by-filter", post(create_task_by_filter)) // 按筛选创建任务
        .route("/api/tasks", get(list_tasks)) // 任务列表
        .route("/api/tasks/{id}", get(get_task)) // 获取任务详情
        .route("/api/tasks/{id}", put(update_task)) // 更新任务状态
        .route("/api/tasks/{id}", axum_delete(delete_task)) // 删除任务
        .route("/api/tasks/{id}/status", get(get_task_runtime_status)) // 实时任务状态统计
        .route("/api/tasks/{id}/pause", post(pause_task)) // 暂停任务
        .route("/api/tasks/{id}/resume", post(resume_task)) // 恢复任务
        .route("/api/tasks/{id}/sub-tasks", get(list_sub_tasks)) // 子任务列表
        .route("/api/sub-tasks/{id}", put(update_sub_task_status)) // 更新子任务状态
        // CAPE实例管理API
        .route("/api/cape-instances", get(list_cape_instances)) // 获取CAPE实例列表
        .route("/api/cape-instances", post(create_cape_instance)) // 创建CAPE实例
        .route("/api/cape-instances/health", get(get_all_health_status)) // 获取所有实例健康状态
        .route("/api/cape-instances/{id}", get(get_cape_instance)) // 获取CAPE实例详情
        .route("/api/cape-instances/{id}", put(update_cape_instance)) // 更新CAPE实例
        .route(
            "/api/cape-instances/{id}",
            axum_delete(delete_cape_instance),
        ) // 删除CAPE实例
        .route(
            "/api/cape-instances/{id}/health-check",
            post(health_check_cape_instance),
        ) // 健康检查
        .route(
            "/api/cape-instances/{id}/stats",
            get(get_cape_instance_stats),
        ) // 实例统计
        // CAPE分析执行API
        .route("/api/cape/execute", post(execute_cape_batch)) // 批量执行CAPE分析
        .route("/api/cape/status/{id}", get(get_task_execution_status)) // 任务执行状态
        .route("/api/cape/performance", get(get_performance_stats)) // 性能统计
        // CFG 实例管理API
        .route("/api/cfg-instances", get(list_cfg_instances))
        .route("/api/cfg-instances", post(create_cfg_instance))
        .route("/api/cfg-instances/{id}", get(get_cfg_instance))
        .route("/api/cfg-instances/{id}", put(update_cfg_instance))
        .route("/api/cfg-instances/{id}", axum_delete(delete_cfg_instance))
        .route(
            "/api/cfg-instances/{id}/health-check",
            post(health_check_cfg_instance),
        )
        .route("/api/cfg-instances/{id}/stats", get(get_cfg_stats))
        // CFG 分析执行API
        .route("/api/cfg/execute", post(execute_cfg_batch))
        .route("/api/cfg/status/{id}", get(get_cfg_task_status))
        .route("/api/analysis/cfg/{id}", get(get_cfg_analysis_detail))
        // 分析结果查询API
        .route("/api/tasks/{id}/results", get(get_task_results)) // 获取任务的所有分析结果
        .route(
            "/api/samples/{id}/analysis",
            get(get_sample_analysis_history),
        ) // 获取样本的分析历史
        .route("/api/analysis/cape/{id}", get(get_cape_analysis_detail)) // 获取CAPE分析详情
        .route(
            "/api/analysis/cape/{sub_task_id}/runtime",
            get(get_cape_runtime_snapshot),
        ) // 获取CAPE运行时快照
        // 统一任务导出
        .route("/api/tasks/{id}/export.csv", get(export_task_results_csv))
        .route(
            "/api/tasks/{id}/results.zip",
            get(download_task_results_zip),
        )

    // TODO: 后续在 handlers 中增加 cfg 对应路由（实例管理/execute/status/detail/query）
}
