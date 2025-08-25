// API响应格式
export interface ApiResponse<T> {
  code: number
  msg: string
  data?: T
}

// 样本类型
export type SampleType = "Benign" | "Malicious"

// 样本模型
export interface Sample {
  id: string
  file_name: string
  file_size: number // 后端是i64，前端用number处理
  file_hash_md5: string
  file_hash_sha1: string
  file_hash_sha256: string
  file_type: string
  file_extension?: string
  sample_type: SampleType
  source?: string
  storage_path: string
  is_container: boolean
  parent_id?: string
  file_path_in_zip?: string
  has_custom_metadata: boolean
  labels?: string[]
  custom_metadata?: Record<string, unknown>
  zip_password?: string
  run_filename?: string
  created_at: string // ISO格式字符串
  updated_at: string // ISO格式字符串
}

// 分页结果
export interface PagedResult<T> {
  items: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

// 样本统计
export interface SampleStats {
  total_samples: number
  benign_samples: number
  malicious_samples: number
  container_files: number
  total_size: number
}

// 扩展样本统计
export interface SampleStatsExtended {
  basic_stats: SampleStats
  file_type_distribution: FileTypeDistribution[]
  file_size_distribution: FileSizeDistribution[]
  source_distribution: SourceDistribution[]
  recent_upload_trend: DailyUploadCount[]
}

// 文件类型分布
export interface FileTypeDistribution {
  file_type: string
  count: number
  size: number
  percentage: number
}

// 文件大小分布
export interface FileSizeDistribution {
  size_range: string
  count: number
  total_size: number
  percentage: number
}

// 来源分布
export interface SourceDistribution {
  source: string
  count: number
  percentage: number
}

// 每日上传数量
export interface DailyUploadCount {
  date: string
  count: number
  size: number
}

// 样本筛选条件
export interface SampleFilters {
  sample_type?: SampleType
  source?: string
  filename?: string
  md5?: string
  sha1?: string
  sha256?: string
  is_container?: boolean
  parent_id?: string
  labels?: string
  start_time?: string
  end_time?: string
}

// 分页参数
export interface Pagination {
  page: number
  page_size: number
}

// 上传元数据
export interface UploadMetadata {
  sample_type: SampleType
  labels?: string[]
  source?: string
  custom_metadata?: Record<string, unknown>
  passwords?: string[]
}

// 上传结果
export interface UploadResult {
  sample_id: string
  filename: string
  file_size: number
  file_type: string
  md5: string
  sha256: string
  is_duplicate: boolean
  duplicate_sample_id?: string
  sub_files_count?: number
}

// 系统状态
export interface SystemStatus {
  database: boolean
  storage: boolean
  file_processor: boolean
}

// 系统信息
export interface SystemInfo {
  name: string
  version: string
  build_time: string
}

// 响应码
export const ResponseCodes = {
  SUCCESS: 200,
  BAD_REQUEST: 400,
  NOT_FOUND: 404,
  DUPLICATE_FILE: 409,
  FILE_TOO_LARGE: 413,
  UNSUPPORTED_FILE_TYPE: 415,
  INTERNAL_ERROR: 500,
  DATABASE_ERROR: 501,
  STORAGE_ERROR: 502,
  FILE_PROCESSING_ERROR: 503,
} as const

// 文件树节点
export interface FileTreeNode {
  id: string
  sample: Sample
  children: FileTreeNode[]
  level: number
  isExpanded: boolean
  parent?: FileTreeNode
}

// ==================== 任务管理相关类型 ====================

// 任务状态枚举 (与后端MasterTaskStatus一致)
export type MasterTaskStatus = 'pending' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled'

// 子任务状态枚举 (与后端SubTaskStatus一致)  
export type SubTaskStatus = 'pending' | 'submitting' | 'submitted' | 'analyzing' | 'paused' | 'completed' | 'failed' | 'cancelled'

// 分析器类型枚举 (与后端AnalyzerType一致)
export type AnalyzerType = 'CAPE' | 'CFG'

// 任务类型枚举 (与后端TaskType一致)
export type TaskType = 'batch' | 'single'

// 主任务模型 (与后端MasterTask一致)
export interface MasterTask {
  id: string
  task_name: string
  analyzer_type: AnalyzerType
  task_type: string
  total_samples: number
  completed_samples: number
  failed_samples: number
  status: MasterTaskStatus
  progress: number
  error_message?: string
  result_summary?: Record<string, unknown>
  sample_filter?: Record<string, unknown>
  paused_at?: string
  pause_reason?: string
  created_by?: string
  created_at: string
  updated_at: string
}

// 子任务模型 (与后端SubTask一致)
export interface SubTask {
  id: string
  master_task_id: string
  sample_id: string
  analyzer_type: AnalyzerType
  external_task_id?: string
  status: SubTaskStatus
  priority: number
  parameters?: Record<string, unknown>
  error_message?: string
  retry_count: number
  created_at: string
  started_at?: string
  completed_at?: string
}

// 子任务与样本信息 (与后端SubTaskWithSample一致)
export interface SubTaskWithSample {
  id: string
  master_task_id: string
  sample_id: string
  analysis_system: string
  cape_instance_id?: string // CAPE实例ID
  cfg_instance_id?: string // CFG实例ID
  cape_instance_name?: string // CAPE实例名称
  cfg_instance_name?: string // CFG实例名称
  status: SubTaskStatus
  priority: number
  parameters?: Record<string, unknown>
  error_message?: string
  retry_count: number
  created_at: string
  started_at?: string
  completed_at?: string
  sample_name: string
  sample_type: SampleType
  file_size: number
  file_hash_md5: string
  file_hash_sha1: string
  file_hash_sha256: string
  labels?: string[] // JSON数组
  source?: string
  external_task_id?: string
}

// 任务统计信息 (与后端TaskStats一致)
export interface TaskStats {
  total_tasks: number
  pending_tasks: number
  running_tasks: number
  completed_tasks: number
  failed_tasks: number
  total_sub_tasks: number
  pending_sub_tasks: number
  running_sub_tasks: number
  completed_sub_tasks: number
  failed_sub_tasks: number
}

// 创建主任务请求 (与后端CreateMasterTaskRequest一致)
export interface CreateMasterTaskRequest {
  task_name: string
  analyzer_type: AnalyzerType
  task_type: TaskType
  sample_ids: string[]
  cape_instance_id?: string // CAPE实例ID（向后兼容）
  cape_instance_ids?: string[] // CAPE实例ID列表（优先使用）
  cfg_instance_ids?: string[] // CFG实例ID列表（优先使用）
  parameters?: Record<string, unknown>
}

// 按筛选创建主任务请求（与后端CreateTaskByFilterRequest一致）
export interface CreateTaskByFilterRequest {
  task_name: string
  analyzer_type: AnalyzerType
  task_type: TaskType
  cape_instance_ids?: string[]
  cfg_instance_ids?: string[]
  parameters?: Record<string, unknown>
  // 平铺的筛选条件
  file_name?: string
  file_type?: string
  sample_type?: SampleType
  file_hash_md5?: string
  file_hash_sha1?: string
  file_hash_sha256?: string
  min_size?: number
  max_size?: number
  uploader?: string
  source?: string
  labels?: string[]
  is_container?: boolean
  parent_id?: string
  start_time?: string
  end_time?: string
}

// 更新主任务请求 (与后端UpdateMasterTaskRequest一致)
export interface UpdateMasterTaskRequest {
  status?: MasterTaskStatus
  progress?: number
  completed_samples?: number
  failed_samples?: number
  error_message?: string
  result_summary?: Record<string, unknown>
}

// 更新子任务状态请求 (与后端UpdateSubTaskStatusRequest一致)
export interface UpdateSubTaskStatusRequest {
  status?: SubTaskStatus
  external_task_id?: string
  error_message?: string
  started_at?: string
  completed_at?: string
}

// 任务筛选条件 (与后端TaskFilter一致)
export interface TaskFilters {
  analyzer_type?: AnalyzerType
  task_type?: TaskType
  status?: MasterTaskStatus
  start_time?: string
  end_time?: string
}

// 子任务筛选条件 (与后端SubTaskFilter一致)
export interface SubTaskFilters {
  master_task_id?: string
  sample_id?: string
  analyzer_type?: AnalyzerType
  status?: SubTaskStatus
  start_time?: string
  end_time?: string
}

// 任务预览请求 (与后端TaskPreviewRequest一致)
export interface TaskPreviewRequest {
  analyzer_type: AnalyzerType
  sample_filter: SampleFilters
}

// 文件类型统计
export interface FileTypeCount {
  file_type: string
  count: number
  size: number
}

// 样本类型统计
export interface SampleTypeCount {
  sample_type: string
  count: number
}

// 来源统计
export interface SourceCount {
  source: string
  count: number
}

// 任务预览响应 (与后端TaskPreviewResponse一致)
export interface TaskPreviewResponse {
  total_samples: number
  total_size: number
  file_type_distribution: FileTypeCount[]
  sample_type_distribution: SampleTypeCount[]
  source_distribution: SourceCount[]
  estimated_duration_minutes?: number
}

// 创建任务响应
export interface CreateTaskResponse {
  master_task: MasterTask
  sub_tasks: SubTask[]
}

// CAPE任务配置请求 (与后端CapeTaskConfigRequest一致)
export interface CapeTaskConfigRequest {
  machine?: string
  // 已移除CAPE超时配置，保留接口兼容但不再使用
  options?: Record<string, string>
}

// 批量执行请求 (与后端BatchExecuteRequest一致)
export interface BatchExecuteRequest {
  master_task_id: string
  config?: CapeTaskConfigRequest
  submit_interval_ms?: number
}

// 批量执行响应 (与后端BatchExecuteResponse一致)
export interface BatchExecuteResponse {
  master_task_id: string
  submitted_tasks: number
  estimated_completion_time?: string
}

// 任务执行状态响应 (与后端TaskExecutionStatusResponse一致)
export interface TaskExecutionStatusResponse {
  master_task_id: string
  total_tasks: number
  pending_tasks: number
  running_tasks: number
  completed_tasks: number
  failed_tasks: number
  progress_percentage: number
  estimated_remaining_time?: string
  average_task_duration?: string
  current_throughput_mbps?: number
}

// 性能统计响应 (与后端PerformanceStatsResponse一致)
export interface PerformanceStatsResponse {
  period_days: number
  total_tasks: number
  success_rate: number
  average_analysis_duration?: string
  average_submit_duration?: string
  average_throughput_mbps?: number
  recommendations: string[]
}

// CAPE分析结果 (与后端CapeAnalysisResult一致)
export interface CapeAnalysisResult {
  id: string
  sub_task_id: string
  sample_id: string
  cape_task_id: number
  analysis_started_at?: string
  analysis_completed_at?: string
  analysis_duration?: number
  score?: number
  severity?: string
  verdict?: string
  signatures?: Array<{
    name?: string;
    description?: string;
    alert?: string;
    severity?: string;
    categories?: string[];
    confidence?: number;
    families?: string[];
    weight?: number;
    references?: string[];
    data?: Record<string, unknown>;
    new_data?: Record<string, unknown>;
  } | string>
  behavior_summary?: Record<string, unknown>
  network_domains?: string[]
  network_ips?: string[]
  network_protocols?: string[]
  files_created?: string[]
  files_deleted?: string[]
  files_modified?: string[]
  files_dropped?: Record<string, unknown>
  processes_created?: string[]
  processes_terminated?: string[]
  full_report?: Record<string, unknown>
  report_summary?: string
  created_at: string
  updated_at: string
}

// CAPE运行时快照 (与后端CapeRuntimeSnapshot一致)
export interface CapeRuntimeSnapshot {
  status: string
  snapshot: Record<string, unknown>
  updated_at: string
}

// 任务状态计数 (与后端TaskStatusCounts一致)
export interface TaskStatusCounts {
  pending: number
  submitting: number
  submitted: number
  analyzing: number
  paused: number
  completed: number
  failed: number
  cancelled: number
}

// 任务运行时状态 (与后端TaskRuntimeStatus一致)
export interface TaskRuntimeStatus {
  master_task_id: string
  total: number
  counts: TaskStatusCounts
  progress_percentage: number
  started_at?: string
  completed_at?: string
  duration_seconds?: number
}

// 分析结果查询参数
export interface AnalysisResultQuery {
  task_id?: string
  sample_id?: string
  min_score?: number
  max_score?: number
  severity?: string
  verdict?: string
  start_date?: string
  end_date?: string
}

// 样本分析历史
export interface SampleAnalysisHistory {
  analysis_results: CapeAnalysisResult[]
  total_analyses: number
  latest_analysis?: CapeAnalysisResult
}

// ==================== CAPE实例管理相关类型 ====================

// CAPE实例状态枚举
export type CapeInstanceStatus = 'healthy' | 'unhealthy' | 'unknown'

// CAPE实例模型 (与后端CapeInstance一致)
export interface CapeInstance {
  id: string
  name: string
  base_url: string
  description?: string
  enabled: boolean
  timeout_seconds: number
  max_concurrent_tasks: number
  health_check_interval: number
  status: CapeInstanceStatus
  last_health_check?: string
  created_at: string
  updated_at: string
}

// 创建CAPE实例请求 (与后端CreateCapeInstanceRequest一致)
export interface CreateCapeInstanceRequest {
  name: string
  base_url: string
  description?: string
  timeout_seconds?: number
  max_concurrent_tasks?: number
  health_check_interval?: number
}

// 更新CAPE实例请求 (与后端UpdateCapeInstanceRequest一致)
export interface UpdateCapeInstanceRequest {
  name?: string
  base_url?: string
  description?: string
  enabled?: boolean
  timeout_seconds?: number
  max_concurrent_tasks?: number
  health_check_interval?: number
}

// CAPE实例健康状态响应 (与后端CapeHealthStatus一致)
export interface CapeHealthStatus {
  instance_id: string
  instance_name: string
  status: CapeInstanceStatus
  response_time_ms?: number
  checked_at: string
  error_message?: string
}

// CAPE实例统计信息 (与后端CapeInstanceStats一致)
export interface CapeInstanceStats {
  instance_id: string
  total_tasks: number
  successful_tasks: number
  failed_tasks: number
  average_processing_time?: number
  success_rate: number
  period_start: string
  period_end: string
}

// CAPE实例查询参数
export interface CapeInstanceQueryParams {
  enabled_only?: boolean
  status?: string
  page?: number
  page_size?: number
}

// CAPE实例列表响应
export interface CapeInstanceListResponse {
  instances: CapeInstance[]
  total: number
}

// ==================== CFG 实例与执行相关类型 ====================

export type CfgInstanceStatus = 'healthy' | 'unhealthy' | 'unknown'

export interface CfgInstance {
  id: string
  name: string
  base_url: string
  description?: string
  enabled: boolean
  timeout_seconds: number
  max_concurrent_tasks: number
  health_check_interval: number
  status: CfgInstanceStatus
  last_health_check?: string
  created_at: string
  updated_at: string
}

export interface CreateCfgInstanceRequest {
  name: string
  base_url: string
  description?: string
  timeout_seconds?: number
  max_concurrent_tasks?: number
  health_check_interval?: number
}

export interface UpdateCfgInstanceRequest {
  name?: string
  base_url?: string
  description?: string
  enabled?: boolean
  timeout_seconds?: number
  max_concurrent_tasks?: number
  health_check_interval?: number
}

export interface CfgHealthStatus {
  instance_id: string
  instance_name: string
  status: CfgInstanceStatus
  response_time_ms?: number
  checked_at: string
  error_message?: string
}

export interface CfgInstanceStats {
  instance_id: string
  total_tasks: number
  successful_tasks: number
  failed_tasks: number
  average_processing_time?: number
  success_rate: number
  period_start: string
  period_end: string
}

export interface CfgInstanceQueryParams {
  enabled_only?: boolean
  status?: string
  page?: number
  page_size?: number
}

// ==================== 重试配置相关类型 ====================

// 重试配置接口
export interface RetryConfig {
  enabled?: boolean
  max_attempts?: number
  initial_backoff_secs?: number
  max_backoff_secs?: number
  backoff_multiplier?: number
  jitter?: boolean
}

// ==================== CAPE 任务执行相关类型 ====================

// CAPE 任务配置请求
export interface CapeTaskConfigRequest {
  machine?: string
  options?: Record<string, string>
  retry?: RetryConfig
}

// CAPE 批量执行请求
export interface BatchExecuteRequest {
  master_task_id: string
  config?: CapeTaskConfigRequest
  submit_interval_ms?: number
  concurrency?: number
}

// CAPE 批量执行响应
export interface BatchExecuteResponse {
  master_task_id: string
  submitted_tasks: number
  estimated_completion_time?: string
}

// CAPE 任务执行状态响应
export interface TaskExecutionStatusResponse {
  master_task_id: string
  total_tasks: number
  pending_tasks: number
  running_tasks: number
  completed_tasks: number
  failed_tasks: number
  progress_percentage: number
  estimated_remaining_time?: string
  average_task_duration?: string
  current_throughput_mbps?: number
}

// CAPE 性能统计响应
export interface PerformanceStatsResponse {
  period_days: number
  total_tasks: number
  success_rate: number
  average_analysis_duration?: string
  average_submit_duration?: string
  average_throughput_mbps?: number
  recommendations: string[]
}

// ==================== CFG 任务执行相关类型 ====================

// CFG 任务配置请求
export interface CfgTaskConfigRequest {
  poll_interval_secs?: number
  max_wait_secs?: number
  label?: number
  retry?: RetryConfig
}

export interface CfgBatchExecuteRequest {
  master_task_id: string
  label: number
  poll_interval_secs?: number
  max_wait_secs?: number
  submit_interval_ms?: number
  config?: CfgTaskConfigRequest
}

export interface CfgBatchExecuteResponse {
  master_task_id: string
  submitted_tasks: number
}

export interface CfgTaskStatusResponse {
  master_task_id: string
  total_tasks: number
  pending_tasks: number
  running_tasks: number
  completed_tasks: number
  failed_tasks: number
  progress_percentage: number
}

export interface CfgAnalysisDetailResponse {
  id: string
  sub_task_id: string
  sample_id: string
  message?: string
  result_files?: Record<string, string>
  full_report?: Record<string, unknown>
}

// 错误类型
export class ApiError extends Error {
  constructor(
    public code: number,
    message: string
  ) {
    super(message)
    this.name = 'ApiError'
  }
}