import axios from 'axios'
import { 
  ApiResponse, ApiError, Sample, SampleFilters, Pagination, PagedResult, SampleStats, SampleStatsExtended, UploadMetadata, UploadResult, SystemStatus, SystemInfo,
  // 任务管理相关类型
  MasterTask, SubTask, SubTaskWithSample, TaskStats, TaskRuntimeStatus, TaskFilters,
  CreateMasterTaskRequest, CreateTaskByFilterRequest, CreateTaskResponse, UpdateMasterTaskRequest, UpdateSubTaskStatusRequest,
  TaskPreviewRequest, TaskPreviewResponse,
  // CAPE相关类型
  BatchExecuteRequest, BatchExecuteResponse, TaskExecutionStatusResponse, PerformanceStatsResponse,
  CapeAnalysisResult, CapeRuntimeSnapshot, AnalysisResultQuery, SampleAnalysisHistory,
  // CAPE实例管理相关类型
  CapeInstance, CreateCapeInstanceRequest, UpdateCapeInstanceRequest, 
  CapeHealthStatus, CapeInstanceStats, CapeInstanceQueryParams,
  // CFG 相关类型
  CfgInstance, CreateCfgInstanceRequest as CreateCfgInstanceReq, UpdateCfgInstanceRequest as UpdateCfgInstanceReq,
  CfgHealthStatus, CfgInstanceStats, CfgInstanceQueryParams,
  CfgBatchExecuteRequest, CfgBatchExecuteResponse, CfgTaskStatusResponse, CfgAnalysisDetailResponse
} from './types'
import { loadConfig, getConfig, getBackendURL } from './config'

// API 客户端配置
// 直接使用后端URL
const apiClient = axios.create({
  baseURL: getBackendURL(), // 直接使用后端URL
  timeout: 300000 // 5分钟超时，适应大批量任务创建
})

// 动态更新超时时间（基于配置）
loadConfig().then(config => {
  apiClient.defaults.timeout = config.backend.timeout
  console.log('📡 API客户端配置已更新:', {
    timeout: config.backend.timeout,
    retries: config.backend.retries
  })
}).catch(err => {
  console.warn('⚠️ API客户端配置加载失败，使用默认值:', err)
})

// 请求拦截器
apiClient.interceptors.request.use(
  (config) => {
    // 添加认证token (如果需要)
    // const token = getAuthToken()
    // if (token) config.headers.Authorization = `Bearer ${token}`
    
    // 对于multipart/form-data请求，不要设置Content-Type
    if (config.data instanceof FormData && config.headers) {
      delete config.headers['Content-Type']
    }
    
    return config
  },
  (error) => Promise.reject(error)
)

// 响应拦截器（包含重试逻辑）
apiClient.interceptors.response.use(
  (response) => {
    // 二进制/文件下载直通，不做ApiResponse解包
    const ct = (response.headers?.['content-type'] || '').toString().toLowerCase()
    const isBlob = response.request?.responseType === 'blob'
    const isBinary = ct.includes('application/octet-stream') || ct.includes('application/zip') || ct.includes('text/csv')
    if (isBlob || isBinary) {
      return response
    }

    // 统一处理 ApiResponse<T>
    const data = response.data as ApiResponse<unknown>
    if (data && typeof data === 'object' && 'code' in data) {
      if (data.code !== 200) {
        throw new ApiError(data.code, data.msg)
      }
      return { ...response, data: data.data }
    }

    // 非标准结构，直接返回
    return response
  },
  async (error) => {
    const config = getConfig()
    const originalRequest = error.config
    
    // 重试逻辑
    if (!originalRequest._retry && (originalRequest._retryCount || 0) < config.backend.retries) {
      originalRequest._retry = true
      originalRequest._retryCount = (originalRequest._retryCount || 0) + 1
      
      console.log(`🔄 API请求重试 ${originalRequest._retryCount}/${config.backend.retries}: ${originalRequest.url}`)
      
      // 延迟重试（指数退避）
      await new Promise(resolve => setTimeout(resolve, 1000 * originalRequest._retryCount))
      
      return apiClient(originalRequest)
    }
    
    // 统一错误处理
    if (error.response?.data?.msg) {
      throw new ApiError(error.response.data.code, error.response.data.msg)
    }
    throw error
  }
)

// 样本API服务
export class SamplesService {
  async list(filters: SampleFilters = {}, pagination: Pagination = { page: 1, page_size: 20 }): Promise<PagedResult<Sample>> {
    const params = new URLSearchParams()
    
    // 分页参数
    params.append('page', pagination.page.toString())
    params.append('page_size', pagination.page_size.toString())
    
    // 筛选参数
    if (filters.sample_type) params.append('sample_type', filters.sample_type)
    if (filters.source) params.append('source', filters.source)
    if (filters.filename) params.append('filename', filters.filename)
    if (filters.md5) params.append('md5', filters.md5)
    if (filters.sha1) params.append('sha1', filters.sha1)
    if (filters.sha256) params.append('sha256', filters.sha256)
    if (filters.is_container !== undefined) params.append('is_container', filters.is_container.toString())
    if (filters.parent_id) params.append('parent_id', filters.parent_id)
    if (filters.labels) params.append('labels', filters.labels)
    if (filters.start_time) params.append('start_time', filters.start_time)
    if (filters.end_time) params.append('end_time', filters.end_time)
    
    const response = await apiClient.get(`/api/samples?${params}`)
    return response.data
  }

  async getById(id: string): Promise<Sample> {
    const response = await apiClient.get(`/api/samples/${id}`)
    return response.data
  }

  async upload(file: File, metadata: UploadMetadata, onProgress?: (progress: number) => void): Promise<UploadResult> {
    const formData = new FormData()
    formData.append('file', file)
    formData.append('metadata', JSON.stringify(metadata))
    
    // 确保不设置Content-Type，让浏览器自动设置multipart/form-data边界
    const config = {
      headers: {
        'Content-Type': undefined // 明确删除Content-Type让axios自动设置
      },
      onUploadProgress: (progressEvent: { loaded: number; total?: number }) => {
        if (onProgress && progressEvent.total) {
          const progress = Math.round((progressEvent.loaded * 100) / progressEvent.total)
          onProgress(progress)
        }
      }
    };
    
    const response = await apiClient.post('/api/samples/upload', formData, config)
    
    return response.data
  }

  async update(id: string, data: Partial<Pick<Sample, 'sample_type' | 'source' | 'labels' | 'custom_metadata' | 'zip_password' | 'run_filename'>>): Promise<Sample> {
    const response = await apiClient.put(`/api/samples/${id}`, data)
    return response.data
  }

  async delete(id: string): Promise<void> {
    await apiClient.delete(`/api/samples/${id}`)
  }

  async deleteBatch(ids: string[]): Promise<{ total: number; deleted: string[]; failed: { 0: string; 1: string }[] }> {
    const response = await apiClient.delete('/api/samples/batch', { data: { ids } })
    return response.data
  }

  async download(id: string): Promise<Blob> {
    const response = await apiClient.get(`/api/samples/${id}/download`, {
      responseType: 'blob'
    })
    return response.data
  }

  async downloadBatch(ids: string[], encrypt?: boolean, password?: string): Promise<Blob> {
    const response = await apiClient.post('/api/samples/batch/download', { ids, encrypt, password }, { responseType: 'blob' })
    return response.data
  }

  async getStats(): Promise<SampleStats> {
    const response = await apiClient.get('/api/samples/stats')
    return response.data
  }

  async getStatsExtended(): Promise<SampleStatsExtended> {
    const response = await apiClient.get('/api/samples/stats/extended')
    return response.data
  }
}

// 系统API服务
export class SystemService {
  async getHealth(): Promise<{ status: string; version?: string; timestamp?: string }> {
    const response = await apiClient.get('/health')
    return response.data
  }

  async getStatus(): Promise<SystemStatus> {
    const response = await apiClient.get('/api/status')
    return response.data
  }

  async getInfo(): Promise<SystemInfo> {
    const response = await apiClient.get('/api/system/info')
    return response.data
  }
}

// 任务管理API服务
export class TasksService {
  /**
   * 任务预览 - 根据筛选条件获取统计信息
   */
  async preview(request: TaskPreviewRequest): Promise<TaskPreviewResponse> {
    const params = new URLSearchParams()
    params.append('analyzer_type', request.analyzer_type)
    
    // 添加样本筛选条件（现在平铺，因为后端使用了 #[serde(flatten)]）
    const filters = request.sample_filter
    if (filters.sample_type) params.append('sample_type', filters.sample_type)
    if (filters.source) params.append('source', filters.source)
    if (filters.filename) params.append('file_name', filters.filename) // 注意：后端字段名是 file_name
    if (filters.md5) params.append('file_hash_md5', filters.md5) // 注意：后端字段名是 file_hash_md5
    if (filters.sha1) params.append('file_hash_sha1', filters.sha1) // 注意：后端字段名是 file_hash_sha1
    if (filters.sha256) params.append('file_hash_sha256', filters.sha256) // 注意：后端字段名是 file_hash_sha256
    if (filters.is_container !== undefined) params.append('is_container', filters.is_container.toString())
    if (filters.parent_id) params.append('parent_id', filters.parent_id)
    if (filters.labels) params.append('labels', filters.labels)
    if (filters.start_time) params.append('start_time', filters.start_time)
    if (filters.end_time) params.append('end_time', filters.end_time)

    const response = await apiClient.get(`/api/tasks/preview?${params}`)
    return response.data
  }

  /**
   * 创建任务
   */
  async create(request: CreateMasterTaskRequest): Promise<CreateTaskResponse> {
    const response = await apiClient.post('/api/tasks', request)
    return response.data
  }

  /**
   * 按筛选创建任务（避免传全量 sample_ids）
   */
  async createByFilter(request: CreateTaskByFilterRequest): Promise<CreateTaskResponse> {
    const response = await apiClient.post('/api/tasks/by-filter', request)
    return response.data
  }

  /**
   * 获取任务列表
   */
  async list(filters: TaskFilters = {}, pagination: Pagination = { page: 1, page_size: 20 }): Promise<PagedResult<MasterTask>> {
    const params = new URLSearchParams()
    
    // 分页参数
    params.append('page', pagination.page.toString())
    params.append('page_size', pagination.page_size.toString())
    
    // 筛选参数
    if (filters.analyzer_type) params.append('analyzer_type', filters.analyzer_type)
    if (filters.task_type) params.append('task_type', filters.task_type)
    if (filters.status) params.append('status', filters.status)
    if (filters.start_time) params.append('start_time', filters.start_time)
    if (filters.end_time) params.append('end_time', filters.end_time)

    const response = await apiClient.get(`/api/tasks?${params}`)
    return response.data
  }

  /**
   * 获取任务详情
   */
  async getTask(id: string): Promise<MasterTask> {
    const response = await apiClient.get(`/api/tasks/${id}`)
    return response.data
  }

  /**
   * 更新任务
   */
  async updateTask(id: string, data: UpdateMasterTaskRequest): Promise<MasterTask> {
    const response = await apiClient.put(`/api/tasks/${id}`, data)
    return response.data
  }

  /**
   * 删除任务
   */
  async deleteTask(id: string): Promise<void> {
    await apiClient.delete(`/api/tasks/${id}`)
  }

  /**
   * 获取子任务列表
   */
  async getSubTasks(
    masterTaskId: string, 
    pagination: Pagination = { page: 1, page_size: 20 },
    filters?: { status?: string; keyword?: string }
  ): Promise<PagedResult<SubTaskWithSample>> {
    const params = new URLSearchParams()
    params.append('page', pagination.page.toString())
    params.append('page_size', pagination.page_size.toString())
    
    if (filters?.status) {
      params.append('status', filters.status)
    }
    if (filters?.keyword) {
      params.append('keyword', filters.keyword)
    }

    const response = await apiClient.get(`/api/tasks/${masterTaskId}/sub-tasks?${params}`)
    return response.data
  }

  async downloadCsv(masterTaskId: string): Promise<Blob> {
    const response = await apiClient.get(`/api/tasks/${masterTaskId}/export.csv`, { responseType: 'blob' })
    return response.data
  }

  async downloadZip(masterTaskId: string): Promise<Blob> {
    const response = await apiClient.get(`/api/tasks/${masterTaskId}/results.zip`, { responseType: 'blob' })
    return response.data
  }

  /**
   * 更新子任务状态
   */
  async updateSubTaskStatus(id: string, data: UpdateSubTaskStatusRequest): Promise<SubTask> {
    const response = await apiClient.put(`/api/sub-tasks/${id}`, data)
    return response.data
  }

  /**
   * 获取任务统计信息
   */
  async getStats(): Promise<TaskStats> {
    const response = await apiClient.get('/api/tasks/stats')
    return response.data
  }

  /**
   * 获取任务运行时状态统计
   */
  async getRuntimeStatus(taskId: string): Promise<TaskRuntimeStatus> {
    const response = await apiClient.get(`/api/tasks/${taskId}/status`)
    return response.data
  }

  /**
   * 暂停任务
   */
  async pauseTask(taskId: string, reason?: string): Promise<MasterTask> {
    const response = await apiClient.post(`/api/tasks/${taskId}/pause`, { 
      mode: 'soft',
      reason 
    })
    return response.data
  }

  /**
   * 恢复任务
   */
  async resumeTask(taskId: string): Promise<MasterTask> {
    const response = await apiClient.post(`/api/tasks/${taskId}/resume`)
    return response.data
  }
}

// CAPE分析API服务
export class CapeService {
  /**
   * 批量执行CAPE分析
   */
  async executeBatch(request: BatchExecuteRequest): Promise<BatchExecuteResponse> {
    const response = await apiClient.post('/api/cape/execute', request)
    return response.data
  }

  /**
   * 获取任务执行状态
   */
  async getExecutionStatus(masterTaskId: string): Promise<TaskExecutionStatusResponse> {
    const response = await apiClient.get(`/api/cape/status/${masterTaskId}`)
    return response.data
  }

  /**
   * 获取性能统计
   */
  async getPerformanceStats(days: number = 7): Promise<PerformanceStatsResponse> {
    const params = new URLSearchParams()
    params.append('period_days', days.toString())
    
    const response = await apiClient.get(`/api/cape/performance?${params}`)
    return response.data
  }
}

// 分析结果API服务
export class AnalysisService {
  /**
   * 获取任务的分析结果
   */
  async getTaskResults(taskId: string, pagination: Pagination = { page: 1, page_size: 20 }): Promise<PagedResult<CapeAnalysisResult>> {
    const params = new URLSearchParams()
    params.append('page', pagination.page.toString())
    params.append('page_size', pagination.page_size.toString())

    const response = await apiClient.get(`/api/tasks/${taskId}/results?${params}`)
    return response.data
  }

  /**
   * 获取样本的分析历史
   */
  async getSampleAnalysisHistory(sampleId: string): Promise<SampleAnalysisHistory> {
    const response = await apiClient.get(`/api/samples/${sampleId}/analysis`)
    return response.data
  }

  /**
   * 获取CAPE分析详情
   */
  async getCapeAnalysisDetail(analysisId: string): Promise<CapeAnalysisResult> {
    const response = await apiClient.get(`/api/analysis/cape/${analysisId}`)
    return response.data
  }

  /**
   * 获取CAPE任务运行时快照
   */
  async getCapeRuntimeSnapshot(subTaskId: string): Promise<CapeRuntimeSnapshot> {
    const response = await apiClient.get(`/api/analysis/cape/${subTaskId}/runtime`)
    return response.data
  }

  /**
   * 查询分析结果
   */
  async queryResults(query: AnalysisResultQuery, pagination: Pagination = { page: 1, page_size: 20 }): Promise<PagedResult<CapeAnalysisResult>> {
    const params = new URLSearchParams()
    
    // 分页参数
    params.append('page', pagination.page.toString())
    params.append('page_size', pagination.page_size.toString())
    
    // 查询参数
    if (query.task_id) params.append('task_id', query.task_id)
    if (query.sample_id) params.append('sample_id', query.sample_id)
    if (query.min_score !== undefined) params.append('min_score', query.min_score.toString())
    if (query.max_score !== undefined) params.append('max_score', query.max_score.toString())
    if (query.severity) params.append('severity', query.severity)
    if (query.verdict) params.append('verdict', query.verdict)
    if (query.start_date) params.append('start_date', query.start_date)
    if (query.end_date) params.append('end_date', query.end_date)

    const response = await apiClient.get(`/api/analysis/results?${params}`)
    return response.data
  }
}

// CAPE实例管理服务
export class CapeInstancesService {
  // 获取CAPE实例列表
  async list(params?: CapeInstanceQueryParams): Promise<PagedResult<CapeInstance>> {
    const searchParams = new URLSearchParams()
    
    // 设置默认分页参数
    searchParams.append('page', (params?.page || 1).toString())
    searchParams.append('page_size', (params?.page_size || 20).toString())
    
    if (params?.enabled_only !== undefined) {
      searchParams.append('enabled_only', params.enabled_only.toString())
    }
    if (params?.status) {
      searchParams.append('status', params.status)
    }

    const response = await apiClient.get(`/api/cape-instances?${searchParams}`)
    return response.data
  }

  // 获取指定CAPE实例详情
  async get(id: string): Promise<CapeInstance> {
    const response = await apiClient.get(`/api/cape-instances/${id}`)
    return response.data
  }

  // 创建CAPE实例
  async create(request: CreateCapeInstanceRequest): Promise<CapeInstance> {
    const response = await apiClient.post('/api/cape-instances', request)
    return response.data
  }

  // 更新CAPE实例
  async update(id: string, request: UpdateCapeInstanceRequest): Promise<string> {
    const response = await apiClient.put(`/api/cape-instances/${id}`, request)
    return response.data
  }

  // 删除CAPE实例
  async delete(id: string): Promise<string> {
    const response = await apiClient.delete(`/api/cape-instances/${id}`)
    return response.data
  }

  // 测试CAPE实例健康状态
  async healthCheck(id: string): Promise<CapeHealthStatus> {
    const response = await apiClient.post(`/api/cape-instances/${id}/health-check`)
    return response.data
  }

  // 获取所有CAPE实例的健康状态
  async getAllHealthStatus(): Promise<CapeHealthStatus[]> {
    const response = await apiClient.get('/api/cape-instances/health')
    return response.data
  }

  // 获取CAPE实例统计信息
  async getStats(id: string, days?: number): Promise<CapeInstanceStats> {
    const params = days ? `?days=${days}` : ''
    const response = await apiClient.get(`/api/cape-instances/${id}/stats${params}`)
    return response.data
  }
}

// 导出API实例
export const samplesApi = new SamplesService()
export const systemApi = new SystemService()
export const tasksApi = new TasksService()
export const capeApi = new CapeService()
export const analysisApi = new AnalysisService()
export const capeInstancesApi = new CapeInstancesService()

// ==================== CFG实例管理服务 ====================
export class CfgInstancesService {
  async list(params?: CfgInstanceQueryParams): Promise<PagedResult<CfgInstance>> {
    const sp = new URLSearchParams()
    sp.append('page', (params?.page || 1).toString())
    sp.append('page_size', (params?.page_size || 20).toString())
    if (params?.enabled_only !== undefined) sp.append('enabled_only', params.enabled_only.toString())
    if (params?.status) sp.append('status', params.status)
    const resp = await apiClient.get(`/api/cfg-instances?${sp}`)
    return resp.data
  }

  async get(id: string): Promise<CfgInstance> {
    const resp = await apiClient.get(`/api/cfg-instances/${id}`)
    return resp.data
  }

  async create(req: CreateCfgInstanceReq): Promise<CfgInstance> {
    const resp = await apiClient.post('/api/cfg-instances', req)
    return resp.data
  }

  async update(id: string, req: UpdateCfgInstanceReq): Promise<string> {
    const resp = await apiClient.put(`/api/cfg-instances/${id}`, req)
    return resp.data
  }

  async delete(id: string): Promise<string> {
    const resp = await apiClient.delete(`/api/cfg-instances/${id}`)
    return resp.data
  }

  async healthCheck(id: string): Promise<CfgHealthStatus> {
    const resp = await apiClient.post(`/api/cfg-instances/${id}/health-check`)
    return resp.data
  }

  async getStats(id: string, days?: number): Promise<CfgInstanceStats> {
    const params = days ? `?days=${days}` : ''
    const resp = await apiClient.get(`/api/cfg-instances/${id}/stats${params}`)
    return resp.data
  }
}

// ==================== CFG 执行与结果 ====================
export class CfgService {
  async executeBatch(req: CfgBatchExecuteRequest): Promise<CfgBatchExecuteResponse> {
    const resp = await apiClient.post('/api/cfg/execute', req)
    return resp.data
  }

  async getTaskStatus(masterTaskId: string): Promise<CfgTaskStatusResponse> {
    const resp = await apiClient.get(`/api/cfg/status/${masterTaskId}`)
    return resp.data
  }

  async downloadCsv(masterTaskId: string): Promise<Blob> {
    const resp = await apiClient.get(`/api/cfg/tasks/${masterTaskId}/export.csv`, { responseType: 'blob' })
    return resp.data
  }

  async downloadZip(masterTaskId: string): Promise<Blob> {
    const resp = await apiClient.get(`/api/cfg/tasks/${masterTaskId}/results.zip`, { responseType: 'blob' })
    return resp.data
  }
}

export class CfgAnalysisService {
  async getAnalysisDetail(id: string): Promise<CfgAnalysisDetailResponse> {
    const resp = await apiClient.get(`/api/analysis/cfg/${id}`)
    return resp.data
  }
}

export const cfgInstancesApi = new CfgInstancesService()
export const cfgApi = new CfgService()
export const cfgAnalysisApi = new CfgAnalysisService()

// 导出axios实例供其他地方使用
export { apiClient }