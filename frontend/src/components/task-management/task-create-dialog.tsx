"use client"

import { useState } from 'react'
import { useMutation, useQueryClient, useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import { 
  TaskPreviewRequest, 
  TaskPreviewResponse, 
  CreateTaskByFilterRequest,
  AnalyzerType,
  TaskType,
  SampleFilters 
} from '@/lib/types'
import { tasksApi, capeInstancesApi, cfgInstancesApi, cfgApi, capeApi } from '@/lib/api'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Progress } from '@/components/ui/progress'
import { Checkbox } from '@/components/ui/checkbox'
import { AnalyzerBadge } from '@/components/task-management'
import {
  ChevronRight,
  ChevronLeft,
  FileText,
  Filter,
  BarChart3,
  Settings,
  AlertCircle,
  CheckCircle,
  Loader2,
  Shield
} from 'lucide-react'

interface TaskCreateDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

type CreateStep = 'config' | 'filters' | 'preview' | 'creating'

export function TaskCreateDialog({ open, onOpenChange }: TaskCreateDialogProps) {
  const [currentStep, setCurrentStep] = useState<CreateStep>('config')
  const [taskConfig, setTaskConfig] = useState<{
    task_name: string
    description?: string
    analyzer_type: AnalyzerType
    task_type: TaskType
    priority: number
    cape_instance_ids: string[]
    cfg_instance_ids: string[]
    parameters?: Record<string, unknown>
    // 执行控制参数（去除CAPE单文件超时）
    poll_interval_secs?: number
    max_wait_secs?: number
    submit_interval_ms?: number
    // 重试配置（任务执行时重试）
    retry_enabled: boolean
    retry_max_attempts: number
    retry_initial_backoff_secs: number
    retry_max_backoff_secs: number
    retry_backoff_multiplier: number
    retry_jitter: boolean
    // CAPE失败任务重试配置
    cape_failed_retry_enabled: boolean
    cape_failed_retry_interval_hours: number
    cape_failed_retry_max_attempts: number
    cape_failed_retry_initial_delay_minutes: number
  }>({
    task_name: '',
    description: '',
    analyzer_type: 'CAPE' as AnalyzerType,
    task_type: 'batch' as TaskType,
    priority: 5,
    cape_instance_ids: [],
    cfg_instance_ids: [],
    parameters: {},
    poll_interval_secs: 10,
    max_wait_secs: 1200,
    submit_interval_ms: 1000,
    // 重试配置默认值
    retry_enabled: true,
    retry_max_attempts: 3,
    retry_initial_backoff_secs: 5,
    retry_max_backoff_secs: 300,
    retry_backoff_multiplier: 2.0,
    retry_jitter: true,
    // CAPE失败任务重试默认值
    cape_failed_retry_enabled: true,
    cape_failed_retry_interval_hours: 1,
    cape_failed_retry_max_attempts: 0, // 0表示无限重试
    cape_failed_retry_initial_delay_minutes: 30
  })
  
  const [sampleFilters, setSampleFilters] = useState<SampleFilters>({})
  const [previewData, setPreviewData] = useState<TaskPreviewResponse | null>(null)
  const [executeImmediately, setExecuteImmediately] = useState<boolean>(true)

  const queryClient = useQueryClient()

  // 获取可用的CAPE实例列表
  const { data: capeInstancesData } = useQuery({
    queryKey: ['cape-instances', 'enabled'],
    queryFn: () => capeInstancesApi.list({ enabled_only: true, page_size: 100 }),
    enabled: open && taskConfig.analyzer_type === 'CAPE'
  })

  // 获取可用的CFG实例列表
  const { data: cfgInstancesData } = useQuery({
    queryKey: ['cfg-instances', 'enabled'],
    queryFn: () => cfgInstancesApi.list({ enabled_only: true, page_size: 100 }),
    enabled: open && taskConfig.analyzer_type === 'CFG'
  })

  // 预览查询
  const previewMutation = useMutation({
    mutationFn: (request: TaskPreviewRequest) => tasksApi.preview(request),
    onSuccess: (data) => {
      setPreviewData(data)
      setCurrentStep('preview')
    },
    onError: (error: Error) => {
      toast.error(`预览失败: ${error.message}`)
    }
  })

  // 创建任务（按筛选，避免前端拉取全量sample_ids）
  const createMutation = useMutation({
    mutationFn: (request: CreateTaskByFilterRequest) => tasksApi.createByFilter(request),
    onSuccess: async (data) => {
      toast.success(`任务创建成功: ${data.master_task.task_name}`)
      // 创建成功后，按开关触发执行
      if (executeImmediately) {
        try {
          if (taskConfig.analyzer_type === 'CFG') {
            await cfgApi.executeBatch({
              master_task_id: data.master_task.id,
              label: 0,
              poll_interval_secs: taskConfig.poll_interval_secs,
              max_wait_secs: taskConfig.max_wait_secs,
              submit_interval_ms: taskConfig.submit_interval_ms,
              config: {
                poll_interval_secs: taskConfig.poll_interval_secs,
                max_wait_secs: taskConfig.max_wait_secs,
                label: 0,
                retry: taskConfig.retry_enabled ? {
                  enabled: taskConfig.retry_enabled,
                  max_attempts: taskConfig.retry_max_attempts,
                  initial_backoff_secs: taskConfig.retry_initial_backoff_secs,
                  max_backoff_secs: taskConfig.retry_max_backoff_secs,
                  backoff_multiplier: taskConfig.retry_backoff_multiplier,
                  jitter: taskConfig.retry_jitter
                } : undefined
              }
            })
          } else {
            await capeApi.executeBatch({
              master_task_id: data.master_task.id,
              submit_interval_ms: taskConfig.submit_interval_ms,
              config: {
                retry: taskConfig.retry_enabled ? {
                  enabled: taskConfig.retry_enabled,
                  max_attempts: taskConfig.retry_max_attempts,
                  initial_backoff_secs: taskConfig.retry_initial_backoff_secs,
                  max_backoff_secs: taskConfig.retry_max_backoff_secs,
                  backoff_multiplier: taskConfig.retry_backoff_multiplier,
                  jitter: taskConfig.retry_jitter
                } : undefined
              }
            })
          }
          toast.success('已触发执行')
        } catch (e) {
          const err = e as Error
          toast.error(`触发执行失败: ${err.message}`)
        }
      }
      queryClient.invalidateQueries({ queryKey: ['tasks'] })
      queryClient.invalidateQueries({ queryKey: ['tasks-stats'] })
      onOpenChange(false)
      resetForm()
    },
    onError: (error: Error) => {
      toast.error(`任务创建失败: ${error.message}`)
      setCurrentStep('preview')
    }
  })

  const fetchSampleIds = () => {
    // 使用后端按筛选创建：直接构造请求并调用
    const labelsArray = sampleFilters.labels
      ? sampleFilters.labels.split(',').map(s => s.trim()).filter(Boolean)
      : undefined

    const req: CreateTaskByFilterRequest = {
      task_name: taskConfig.task_name,
      analyzer_type: taskConfig.analyzer_type,
      task_type: taskConfig.task_type,
      cape_instance_ids: taskConfig.cape_instance_ids.length > 0 ? taskConfig.cape_instance_ids : undefined,
      cfg_instance_ids: taskConfig.cfg_instance_ids.length > 0 ? taskConfig.cfg_instance_ids : undefined,
      parameters: {
        ...taskConfig.parameters,
        cape_submit_interval_ms: taskConfig.submit_interval_ms,
        cfg_poll_interval_secs: taskConfig.poll_interval_secs,
        cfg_max_wait_secs: taskConfig.max_wait_secs,
        cfg_submit_interval_ms: taskConfig.submit_interval_ms,
        ...(taskConfig.analyzer_type === 'CAPE' && taskConfig.cape_failed_retry_enabled ? {
          cape_config: {
            failed_task_retry: {
              enabled: taskConfig.cape_failed_retry_enabled,
              retry_interval_secs: taskConfig.cape_failed_retry_interval_hours * 3600,
              max_retry_attempts: taskConfig.cape_failed_retry_max_attempts,
              initial_delay_secs: taskConfig.cape_failed_retry_initial_delay_minutes * 60
            }
          }
        } : {})
      },
      file_name: sampleFilters.filename || undefined,
      sample_type: sampleFilters.sample_type,
      source: sampleFilters.source || undefined,
      file_hash_md5: sampleFilters.md5 || undefined,
      file_hash_sha1: sampleFilters.sha1 || undefined,
      file_hash_sha256: sampleFilters.sha256 || undefined,
      is_container: sampleFilters.is_container,
      parent_id: sampleFilters.parent_id,
      labels: labelsArray,
      start_time: sampleFilters.start_time,
      end_time: sampleFilters.end_time,
    }

    createMutation.mutate(req)
  }

  const resetForm = () => {
    setCurrentStep('config')
    setTaskConfig({
      task_name: '',
      description: '',
      analyzer_type: 'CAPE' as AnalyzerType,
      task_type: 'batch' as TaskType,
      priority: 5,
      cape_instance_ids: [],
      cfg_instance_ids: [],
      parameters: {},
      poll_interval_secs: 10,
      max_wait_secs: 1200,
      submit_interval_ms: 1000,
      // 重试配置默认值
      retry_enabled: true,
      retry_max_attempts: 3,
      retry_initial_backoff_secs: 5,
      retry_max_backoff_secs: 300,
      retry_backoff_multiplier: 2.0,
      retry_jitter: true,
      // CAPE失败任务重试默认值
      cape_failed_retry_enabled: true,
      cape_failed_retry_interval_hours: 1,
      cape_failed_retry_max_attempts: 0,
      cape_failed_retry_initial_delay_minutes: 30
    })
    setSampleFilters({})
    setPreviewData(null)
  }

  const handleNext = () => {
    switch (currentStep) {
      case 'config':
        if (!taskConfig.task_name?.trim()) {
          toast.error('请输入任务名称')
          return
        }
        setCurrentStep('filters')
        break
      case 'filters':
        // 执行预览
        const previewRequest: TaskPreviewRequest = {
          analyzer_type: taskConfig.analyzer_type!,
          sample_filter: sampleFilters
        }
        previewMutation.mutate(previewRequest)
        break
      case 'preview':
        if (!previewData || previewData.total_samples === 0) {
          toast.error('没有符合条件的样本，无法创建任务')
          return
        }
        setCurrentStep('creating')
        
        // 先查询符合条件的样本ID
        fetchSampleIds()
        break
    }
  }

  const handleBack = () => {
    switch (currentStep) {
      case 'filters':
        setCurrentStep('config')
        break
      case 'preview':
        setCurrentStep('filters')
        break
      case 'creating':
        setCurrentStep('preview')
        break
    }
  }

  const renderStepContent = () => {
    switch (currentStep) {
      case 'config':
        return (
          <div className="space-y-6">
            <div className="space-y-4">
              <div>
                <Label htmlFor="task-name">任务名称 *</Label>
                <Input
                  id="task-name"
                  placeholder="输入任务名称"
                  value={taskConfig.task_name || ''}
                  onChange={(e) => setTaskConfig(prev => ({ ...prev, task_name: e.target.value }))}
                />
              </div>

              <div>
                <Label htmlFor="description">任务描述</Label>
                <Textarea
                  id="description"
                  placeholder="输入任务描述（可选）"
                  value={taskConfig.description || ''}
                  onChange={(e) => setTaskConfig(prev => ({ ...prev, description: e.target.value }))}
                  rows={3}
                />
              </div>

              <div>
                <Label>分析器类型</Label>
                <RadioGroup 
                  value={taskConfig.analyzer_type} 
                  onValueChange={(value) => setTaskConfig(prev => ({ ...prev, analyzer_type: value as AnalyzerType }))}
                  className="mt-2"
                >
                  <div className="flex items-center space-x-2">
                    <RadioGroupItem value="CAPE" id="cape" />
                    <Label htmlFor="cape" className="flex items-center gap-2 text-foreground font-normal">
                      <Shield className="h-4 w-4 text-blue-600" />
                      <span>CAPE 沙箱分析</span>
                      <span className="text-xs text-muted-foreground">(动态行为分析)</span>
                    </Label>
                  </div>
                  <div className="flex items-center space-x-2">
                    <RadioGroupItem value="CFG" id="cfg" />
                    <Label htmlFor="cfg" className="flex items-center gap-2 text-foreground font-normal">
                      <Shield className="h-4 w-4 text-violet-600" />
                      <span>CFG 分析</span>
                      <span className="text-xs text-muted-foreground">(控制流图/特征向量)</span>
                    </Label>
                  </div>
                </RadioGroup>
              </div>

              {/* CAPE实例选择器 - 多选 */}
              {taskConfig.analyzer_type === 'CAPE' && (
                <div>
                  <Label>CAPE实例（多选）</Label>
                  <div className="mt-2 space-y-2 border rounded-lg p-3 max-h-48 overflow-y-auto">
                    {capeInstancesData?.items.length === 0 ? (
                      <p className="text-sm text-muted-foreground">暂无可用的CAPE实例</p>
                    ) : (
                      capeInstancesData?.items.map((instance) => (
                        <div key={instance.id} className="flex items-center space-x-2">
                          <Checkbox
                            id={`cape-${instance.id}`}
                            checked={taskConfig.cape_instance_ids.includes(instance.id)}
                            onCheckedChange={(checked) => {
                              setTaskConfig(prev => ({
                                ...prev,
                                cape_instance_ids: checked
                                  ? [...prev.cape_instance_ids, instance.id]
                                  : prev.cape_instance_ids.filter(id => id !== instance.id)
                              }))
                            }}
                          />
                          <label 
                            htmlFor={`cape-${instance.id}`}
                            className="flex-1 flex items-center gap-2 cursor-pointer"
                          >
                            <Shield className={`h-4 w-4 ${
                              instance.status === 'healthy' ? 'text-green-500' : 
                              instance.status === 'unhealthy' ? 'text-red-500' : 'text-gray-500'
                            }`} />
                            <span className="text-sm">{instance.name}</span>
                            <Badge variant={instance.status === 'healthy' ? 'default' : 'secondary'} className="text-xs">
                              {instance.status === 'healthy' ? '健康' : 
                               instance.status === 'unhealthy' ? '异常' : '未知'}
                            </Badge>
                          </label>
                        </div>
                      ))
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    选择要使用的CAPE实例，未选择时将使用所有可用实例进行轮询负载均衡
                  </p>
                </div>
              )}

              {/* CFG实例选择器 - 多选 */}
              {taskConfig.analyzer_type === 'CFG' && (
                <div>
                  <Label>CFG实例（多选）</Label>
                  <div className="mt-2 space-y-2 border rounded-lg p-3 max-h-48 overflow-y-auto">
                    {cfgInstancesData?.items.length === 0 ? (
                      <p className="text-sm text-muted-foreground">暂无可用的CFG实例</p>
                    ) : (
                      cfgInstancesData?.items.map((instance) => (
                        <div key={instance.id} className="flex items-center space-x-2">
                          <Checkbox
                            id={`cfg-${instance.id}`}
                            checked={taskConfig.cfg_instance_ids.includes(instance.id)}
                            onCheckedChange={(checked) => {
                              setTaskConfig(prev => ({
                                ...prev,
                                cfg_instance_ids: checked
                                  ? [...prev.cfg_instance_ids, instance.id]
                                  : prev.cfg_instance_ids.filter(id => id !== instance.id)
                              }))
                            }}
                          />
                          <label 
                            htmlFor={`cfg-${instance.id}`}
                            className="flex-1 flex items-center gap-2 cursor-pointer"
                          >
                            <Shield className={`h-4 w-4 ${
                              instance.status === 'healthy' ? 'text-green-500' : 
                              instance.status === 'unhealthy' ? 'text-red-500' : 'text-gray-500'
                            }`} />
                            <span className="text-sm">{instance.name}</span>
                            <Badge variant={instance.status === 'healthy' ? 'default' : 'secondary'} className="text-xs">
                              {instance.status === 'healthy' ? '健康' : 
                               instance.status === 'unhealthy' ? '异常' : '未知'}
                            </Badge>
                          </label>
                        </div>
                      ))
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    选择要使用的CFG实例，未选择时将使用所有可用实例进行轮询负载均衡
                  </p>
                </div>
              )}

              <div>
                <Label>任务类型</Label>
                <Select 
                  value={taskConfig.task_type} 
                  onValueChange={(value) => setTaskConfig(prev => ({ ...prev, task_type: value as TaskType }))}
                >
                  <SelectTrigger className="mt-2">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="batch">批量分析</SelectItem>
                    <SelectItem value="single">单个分析</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* 统一执行参数 */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {/* CAPE 单文件超时输入已移除 */}
                <div>
                  <Label htmlFor="submit-interval">提交间隔(ms)</Label>
                  <Input id="submit-interval" type="number" value={taskConfig.submit_interval_ms ?? 1000}
                    onChange={e => setTaskConfig(prev => ({ ...prev, submit_interval_ms: parseInt(e.target.value)||0 }))}
                    className="mt-2" />
                </div>
                {/* 并发数输入已移除，改为仅依赖提交间隔进行流控 */}
                {taskConfig.analyzer_type === 'CFG' && (
                  <>
                    <div>
                      <Label htmlFor="poll-secs">轮询间隔(秒)</Label>
                      <Input id="poll-secs" type="number" value={taskConfig.poll_interval_secs ?? 10}
                        onChange={e => setTaskConfig(prev => ({ ...prev, poll_interval_secs: parseInt(e.target.value)||0 }))}
                        className="mt-2" />
                    </div>
                    <div>
                      <Label htmlFor="max-wait">最大等待(秒)</Label>
                      <Input id="max-wait" type="number" value={taskConfig.max_wait_secs ?? 1200}
                        onChange={e => setTaskConfig(prev => ({ ...prev, max_wait_secs: parseInt(e.target.value)||0 }))}
                        className="mt-2" />
                    </div>
                  </>
                )}
              </div>

              {/* 重试配置 */}
              <div className="border rounded-lg p-4 space-y-4">
                <div className="flex items-center justify-between">
                  <Label className="text-base font-medium">重试配置</Label>
                  <Checkbox
                    checked={taskConfig.retry_enabled}
                    onCheckedChange={(checked) => setTaskConfig(prev => ({ ...prev, retry_enabled: checked === true }))}
                  />
                </div>
                
                {taskConfig.retry_enabled && (
                  <div className="space-y-4 pl-4 border-l-2 border-gray-200">
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div>
                        <Label htmlFor="retry-attempts">最大重试次数</Label>
                        <Input
                          id="retry-attempts"
                          type="number"
                          min="1"
                          max="10"
                          value={taskConfig.retry_max_attempts}
                          onChange={e => setTaskConfig(prev => ({ ...prev, retry_max_attempts: parseInt(e.target.value) || 3 }))}
                          className="mt-2"
                        />
                      </div>
                      <div>
                        <Label htmlFor="initial-backoff">初始退避时间(秒)</Label>
                        <Input
                          id="initial-backoff"
                          type="number"
                          min="1"
                          max="60"
                          value={taskConfig.retry_initial_backoff_secs}
                          onChange={e => setTaskConfig(prev => ({ ...prev, retry_initial_backoff_secs: parseInt(e.target.value) || 5 }))}
                          className="mt-2"
                        />
                      </div>
                    </div>
                    
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div>
                        <Label htmlFor="max-backoff">最大退避时间(秒)</Label>
                        <Input
                          id="max-backoff"
                          type="number"
                          min="60"
                          max="3600"
                          value={taskConfig.retry_max_backoff_secs}
                          onChange={e => setTaskConfig(prev => ({ ...prev, retry_max_backoff_secs: parseInt(e.target.value) || 300 }))}
                          className="mt-2"
                        />
                      </div>
                      <div>
                        <Label htmlFor="backoff-multiplier">退避倍率</Label>
                        <Input
                          id="backoff-multiplier"
                          type="number"
                          min="1"
                          max="5"
                          step="0.1"
                          value={taskConfig.retry_backoff_multiplier}
                          onChange={e => setTaskConfig(prev => ({ ...prev, retry_backoff_multiplier: parseFloat(e.target.value) || 2.0 }))}
                          className="mt-2"
                        />
                      </div>
                    </div>
                    
                    <div className="flex items-center space-x-2">
                      <Checkbox
                        id="retry-jitter"
                        checked={taskConfig.retry_jitter}
                        onCheckedChange={(checked) => setTaskConfig(prev => ({ ...prev, retry_jitter: checked === true }))}
                      />
                      <Label htmlFor="retry-jitter" className="text-sm">启用随机抖动（避免惊群效应）</Label>
                    </div>
                    
                    <Alert>
                      <AlertCircle className="h-4 w-4" />
                      <AlertDescription className="text-sm">
                        重试功能可以提高任务的成功率，特别是在网络不稳定的情况下。
                        指数退避策略会在每次重试时增加等待时间，避免对服务器造成过大压力。
                      </AlertDescription>
                    </Alert>
                  </div>
                )}
              </div>

              {/* CAPE失败任务重试配置 */}
              {taskConfig.analyzer_type === 'CAPE' && (
                <div className="border rounded-lg p-4 space-y-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <Label className="text-base font-medium">失败任务重试配置</Label>
                      <p className="text-xs text-muted-foreground mt-1">当CAPE任务因临时问题失败时，系统将自动重试</p>
                    </div>
                    <Checkbox
                      checked={taskConfig.cape_failed_retry_enabled}
                      onCheckedChange={(checked) => setTaskConfig(prev => ({ ...prev, cape_failed_retry_enabled: checked === true }))}
                    />
                  </div>
                  
                  {taskConfig.cape_failed_retry_enabled && (
                    <div className="space-y-4 pl-4 border-l-2 border-blue-200">
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                          <Label htmlFor="failed-retry-interval">重试间隔(小时)</Label>
                          <Input
                            id="failed-retry-interval"
                            type="number"
                            min="0.5"
                            max="24"
                            step="0.5"
                            value={taskConfig.cape_failed_retry_interval_hours}
                            onChange={e => setTaskConfig(prev => ({ ...prev, cape_failed_retry_interval_hours: parseFloat(e.target.value) || 1 }))}
                            className="mt-2"
                          />
                          <p className="text-xs text-muted-foreground mt-1">失败任务多久后重试一次</p>
                        </div>
                        <div>
                          <Label htmlFor="failed-retry-attempts">最大重试次数</Label>
                          <Input
                            id="failed-retry-attempts"
                            type="number"
                            min="0"
                            max="100"
                            value={taskConfig.cape_failed_retry_max_attempts}
                            onChange={e => setTaskConfig(prev => ({ ...prev, cape_failed_retry_max_attempts: parseInt(e.target.value) || 0 }))}
                            className="mt-2"
                          />
                          <p className="text-xs text-muted-foreground mt-1">0表示无限重试，直到成功</p>
                        </div>
                      </div>
                      
                      <div>
                        <Label htmlFor="failed-retry-delay">初始延迟(分钟)</Label>
                        <Input
                          id="failed-retry-delay"
                          type="number"
                          min="5"
                          max="120"
                          value={taskConfig.cape_failed_retry_initial_delay_minutes}
                          onChange={e => setTaskConfig(prev => ({ ...prev, cape_failed_retry_initial_delay_minutes: parseInt(e.target.value) || 30 }))}
                          className="mt-2"
                        />
                        <p className="text-xs text-muted-foreground mt-1">任务首次失败后等待多久开始重试</p>
                      </div>
                      
                      <Alert>
                        <Shield className="h-4 w-4" />
                        <AlertDescription className="text-sm">
                          <strong>失败任务自动恢复：</strong>当CAPE任务因网络问题、服务重启等临时原因失败时，
                          系统会在指定时间后自动重新检查任务状态和报告，尝试恢复被误判失败的任务。
                          这可以大幅提高任务的最终成功率。
                        </AlertDescription>
                      </Alert>
                    </div>
                  )}
                </div>
              )}

              <div>
                <Label htmlFor="priority">优先级 (1-10)</Label>
                <Input
                  id="priority"
                  type="number"
                  min="1"
                  max="10"
                  value={taskConfig.priority || 5}
                  onChange={(e) => setTaskConfig(prev => ({ ...prev, priority: parseInt(e.target.value) || 5 }))}
                  className="mt-2"
                />
                <p className="text-xs text-muted-foreground mt-1">数字越大优先级越高</p>
              </div>

              {/* 立即执行开关 */}
              <div className="flex items-center gap-3 border rounded-md p-3">
                <Checkbox id="exec-now" checked={executeImmediately} onCheckedChange={(v) => setExecuteImmediately(!!v)} />
                <div>
                  <Label htmlFor="exec-now">创建完成后立即执行</Label>
                  <div className="text-xs text-muted-foreground">开启后将在任务创建成功后自动调用对应的执行接口（CAPE/CFG）</div>
                </div>
              </div>
            </div>
          </div>
        )

      case 'filters':
        return (
          <div className="space-y-6">
            <div className="text-center mb-4">
              <Filter className="h-8 w-8 mx-auto mb-2 text-primary" />
              <h3 className="text-lg font-semibold">选择样本范围</h3>
              <p className="text-sm text-muted-foreground">设置筛选条件来选择要分析的样本</p>
            </div>

            <div className="space-y-4">
              <div>
                <Label htmlFor="filename">文件名称</Label>
                <Input
                  id="filename"
                  placeholder="搜索文件名称"
                  value={sampleFilters.filename || ''}
                  onChange={(e) => setSampleFilters(prev => ({ ...prev, filename: e.target.value }))}
                />
              </div>

              <div>
                <Label>样本分类</Label>
                <RadioGroup 
                  value={sampleFilters.sample_type || ''} 
                  onValueChange={(value) => setSampleFilters(prev => ({ 
                    ...prev, 
                    sample_type: value ? value as 'Benign' | 'Malicious' : undefined 
                  }))}
                  className="mt-2"
                >
                  <div className="flex items-center space-x-2">
                    <RadioGroupItem value="Benign" id="benign" />
                    <Label htmlFor="benign">良性</Label>
                  </div>
                  <div className="flex items-center space-x-2">
                    <RadioGroupItem value="Malicious" id="malicious" />
                    <Label htmlFor="malicious">恶意</Label>
                  </div>
                  <div className="flex items-center space-x-2">
                    <RadioGroupItem value="" id="all" />
                    <Label htmlFor="all">全部</Label>
                  </div>
                </RadioGroup>
              </div>

              <div>
                <Label htmlFor="source">样本来源</Label>
                <Input
                  id="source"
                  placeholder="筛选样本来源"
                  value={sampleFilters.source || ''}
                  onChange={(e) => setSampleFilters(prev => ({ ...prev, source: e.target.value }))}
                />
              </div>



              <Alert>
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>
                  不设置任何筛选条件将会分析所有样本。请根据需要设置合适的筛选条件。
                </AlertDescription>
              </Alert>
            </div>
          </div>
        )

      case 'preview':
        return (
          <div className="space-y-6">
            <div className="text-center mb-4">
              <BarChart3 className="h-8 w-8 mx-auto mb-2 text-primary" />
              <h3 className="text-lg font-semibold">任务预览</h3>
              <p className="text-sm text-muted-foreground">确认任务配置和样本统计</p>
            </div>

            {previewData && (
              <>
                {/* 任务配置卡片 */}
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                      <Settings className="h-4 w-4" />
                      任务配置
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <div className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <div className="text-muted-foreground">任务名称</div>
                        <div className="font-medium">{taskConfig.task_name}</div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">分析器</div>
                        <div><AnalyzerBadge type={taskConfig.analyzer_type!} /></div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">任务类型</div>
                        <div className="font-medium">{taskConfig.task_type}</div>
                      </div>
                      <div>
                        <div className="text-muted-foreground">优先级</div>
                        <div className="font-medium">{taskConfig.priority}</div>
                      </div>
                    </div>
                    {taskConfig.description && (
                      <div>
                        <div className="text-muted-foreground text-sm">描述</div>
                        <div className="text-sm">{taskConfig.description}</div>
                      </div>
                    )}
                  </CardContent>
                </Card>

                {/* 样本统计卡片 */}
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                      <FileText className="h-4 w-4" />
                      样本统计
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="text-center">
                      <div className="text-3xl font-bold text-primary">{previewData.total_samples}</div>
                      <div className="text-sm text-muted-foreground">符合条件的样本数量</div>
                      <div className="text-xs text-muted-foreground mt-1">
                        总大小: {(previewData.total_size / (1024 * 1024)).toFixed(2)} MB
                      </div>
                    </div>

                    {previewData.file_type_distribution && previewData.file_type_distribution.length > 0 && (
                      <div>
                        <div className="text-sm font-medium mb-2">文件类型分布</div>
                        <div className="flex flex-wrap gap-2">
                          {previewData.file_type_distribution.map((item) => (
                            <Badge key={item.file_type} variant="outline">
                              {item.file_type}: {item.count}
                            </Badge>
                          ))}
                        </div>
                      </div>
                    )}

                    {previewData.sample_type_distribution && previewData.sample_type_distribution.length > 0 && (
                      <div>
                        <div className="text-sm font-medium mb-2">样本类型分布</div>
                        <div className="flex flex-wrap gap-2">
                          {previewData.sample_type_distribution.map((item) => (
                            <Badge key={item.sample_type} variant="outline">
                              {item.sample_type}: {item.count}
                            </Badge>
                          ))}
                        </div>
                      </div>
                    )}

                    {previewData.source_distribution && previewData.source_distribution.length > 0 && (
                      <div>
                        <div className="text-sm font-medium mb-2">来源分布</div>
                        <div className="flex flex-wrap gap-2">
                          {previewData.source_distribution.map((item) => (
                            <Badge key={item.source} variant="outline">
                              {item.source}: {item.count}
                            </Badge>
                          ))}
                        </div>
                      </div>
                    )}
                  </CardContent>
                </Card>

                {previewData.total_samples === 0 && (
                  <Alert>
                    <AlertCircle className="h-4 w-4" />
                    <AlertDescription>
                      没有找到符合筛选条件的样本。请返回上一步调整筛选条件。
                    </AlertDescription>
                  </Alert>
                )}
              </>
            )}
          </div>
        )

      case 'creating':
        return (
          <div className="space-y-6">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto mb-4"></div>
              <h3 className="text-lg font-semibold">正在创建任务...</h3>
              <p className="text-sm text-muted-foreground">请稍候，正在处理您的请求</p>
            </div>
            
            <Progress value={75} className="w-full" />
            
            <div className="text-xs text-center text-muted-foreground">
              正在初始化任务配置和样本队列...
            </div>
          </div>
        )

      default:
        return null
    }
  }

  const getStepTitle = () => {
    switch (currentStep) {
      case 'config': return '任务配置'
      case 'filters': return '样本筛选'
      case 'preview': return '任务预览'
      case 'creating': return '创建任务'
      default: return '创建任务'
    }
  }

  const canGoNext = () => {
    switch (currentStep) {
      case 'config':
        return !!taskConfig.task_name?.trim()
      case 'filters':
        return !previewMutation.isPending
      case 'preview':
        return previewData && previewData.total_samples > 0 && !createMutation.isPending
      case 'creating':
        return false
      default:
        return false
    }
  }

  const canGoBack = () => {
    return currentStep !== 'config' && currentStep !== 'creating'
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{getStepTitle()}</DialogTitle>
          <DialogDescription>
            {currentStep === 'config' && '配置任务的基本信息和参数'}
            {currentStep === 'filters' && '设置样本筛选条件'}
            {currentStep === 'preview' && '确认任务配置和样本统计信息'}
            {currentStep === 'creating' && '正在创建任务，请稍候'}
          </DialogDescription>
        </DialogHeader>

        {/* 步骤指示器 */}
        <div className="flex items-center justify-center space-x-2 py-4">
          {['config', 'filters', 'preview', 'creating'].map((step, index) => (
            <div key={step} className="flex items-center">
              <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium ${
                currentStep === step 
                  ? 'bg-primary text-primary-foreground' 
                  : index < ['config', 'filters', 'preview', 'creating'].indexOf(currentStep)
                    ? 'bg-primary/20 text-primary'
                    : 'bg-muted text-muted-foreground'
              }`}>
                {index + 1}
              </div>
              {index < 3 && (
                <div className={`w-12 h-px ${
                  index < ['config', 'filters', 'preview', 'creating'].indexOf(currentStep)
                    ? 'bg-primary'
                    : 'bg-muted'
                }`} />
              )}
            </div>
          ))}
        </div>

        {/* 步骤内容 */}
        <div className="min-h-[400px]">
          {renderStepContent()}
        </div>

        {/* 操作按钮 */}
        <div className="flex justify-between pt-4 border-t">
          <Button 
            variant="outline" 
            onClick={handleBack}
            disabled={!canGoBack()}
          >
            <ChevronLeft className="h-4 w-4 mr-2" />
            上一步
          </Button>
          
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              取消
            </Button>
            <Button 
              onClick={handleNext}
              disabled={!canGoNext()}
            >
              {currentStep === 'preview' ? (
                <>
                  <CheckCircle className="h-4 w-4 mr-2" />
                  创建任务
                </>
              ) : (
                <>
                  下一步
                  <ChevronRight className="h-4 w-4 ml-2" />
                </>
              )}
              {(previewMutation.isPending || createMutation.isPending) && (
                <Loader2 className="h-4 w-4 ml-2 animate-spin" />
              )}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}