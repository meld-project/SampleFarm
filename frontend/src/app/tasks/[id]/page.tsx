"use client"

import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { useParams, useRouter } from 'next/navigation'
import { tasksApi, analysisApi } from '@/lib/api'
import { Pagination } from '@/lib/types'
import {
  TaskStatusBadge,
  AnalyzerBadge,
  TaskProgress
} from '@/components/task-management'
import { SubTaskTable } from '@/components/task-management/sub-task-table'
import { AnalysisResultDialog } from '@/components/task-management/analysis-result-dialog'
import { CfgAnalysisResultDialog } from '@/components/task-management/cfg-analysis-result-dialog'
import { CapeRuntimeDialog } from '@/components/task-management/cape-runtime-dialog'
import { TaskStatusCountsDisplay } from '@/components/task-management/task-status-counts'
import { SubTaskFilters } from '@/components/task-management/sub-task-filters'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

import { Alert, AlertDescription } from '@/components/ui/alert'
import { useI18n } from '@/lib/i18n'
import {
  ArrowLeft,
  RefreshCw,
  Download,
  Eye,
  Clock,
  AlertTriangle,
  Settings,
  BarChart3,
  Activity,
  Play,
  Pause,
  PlayCircle
} from 'lucide-react'
import { ExecutionParamsDialog } from '@/components/task-management/execution-params-dialog'
import { formatRelativeTime } from '@/lib/utils'

export default function TaskDetailPage() {
  const { t } = useI18n()
  const params = useParams()
  const router = useRouter()
  const queryClient = useQueryClient()
  const taskId = params.id as string

  const [subTaskPagination, setSubTaskPagination] = useState<Pagination>({ page: 1, page_size: 20 })
  const [subTaskFilters, setSubTaskFilters] = useState<{ status?: string; keyword?: string }>({})
  const [analysisDialogOpen, setAnalysisDialogOpen] = useState(false)
  const [cfgAnalysisDialogOpen, setCfgAnalysisDialogOpen] = useState(false)
  const [runtimeDialogOpen, setRuntimeDialogOpen] = useState(false)
  const [selectedAnalysisId, setSelectedAnalysisId] = useState<string | null>(null)
  const [selectedSubTaskId, setSelectedSubTaskId] = useState<string | null>(null)
  const [selectedSampleName, setSelectedSampleName] = useState<string>('')
  const [execDialogOpen, setExecDialogOpen] = useState(false)

  // 获取任务详情 - 为运行中的任务启用自动轮询
  const { data: task, isLoading: taskLoading, error: taskError, refetch: refetchTask } = useQuery({
    queryKey: ['task', taskId],
    queryFn: () => tasksApi.getTask(taskId),
    enabled: !!taskId,
    refetchInterval: (query) => {
      // 如果任务正在运行或等待中，每5秒刷新一次
      const data = query.state.data
      if (data && (data.status === 'running' || data.status === 'pending')) {
        return 5000
      }
      // 其他状态不自动刷新
      return false
    }
  })

  // 获取子任务列表 - 为活跃任务启用自动轮询
  const { data: subTasksData, isLoading: subTasksLoading, refetch: refetchSubTasks } = useQuery({
    queryKey: ['sub-tasks', taskId, subTaskPagination, subTaskFilters],
    queryFn: () => tasksApi.getSubTasks(taskId, subTaskPagination, subTaskFilters),
    enabled: !!taskId,
    refetchInterval: task && (task.status === 'running' || task.status === 'pending') ? 10000 : false
  })

  // 获取任务分析结果 - 只在任务完成后查询
  const { data: analysisResults, isLoading: resultsLoading } = useQuery({
    queryKey: ['task-results', taskId],
    queryFn: () => analysisApi.getTaskResults(taskId),
    enabled: !!taskId && task?.status === 'completed',
  })

  // 获取实时任务状态统计 - 运行中的任务启用轮询
  const { data: runtimeStatus } = useQuery({
    queryKey: ['task-runtime-status', taskId],
    queryFn: () => tasksApi.getRuntimeStatus(taskId),
    enabled: !!taskId && !!task,
    refetchInterval: task && (task.status === 'running' || task.status === 'pending') ? 30000 : false,
    staleTime: 25000,
  })

  const handleRefresh = () => {
    refetchTask()
    refetchSubTasks()
  }

  // 由执行参数对话框触发执行

  const handleViewAnalysisResult = (analysisId: string, sampleName: string) => {
    setSelectedAnalysisId(analysisId)
    setSelectedSampleName(sampleName)
    if (task?.analyzer_type === 'CFG') {
      setCfgAnalysisDialogOpen(true)
    } else {
      setAnalysisDialogOpen(true)
    }
  }

  const handleViewRuntimeSnapshot = (subTaskId: string, sampleName: string) => {
    setSelectedSubTaskId(subTaskId)
    setSelectedSampleName(sampleName)
    setRuntimeDialogOpen(true)
  }

  const handleSubTaskPageChange = (page: number) => {
    setSubTaskPagination(prev => ({ ...prev, page }))
  }

  const handleSubTaskFiltersChange = (filters: { status?: string; keyword?: string }) => {
    setSubTaskFilters(filters)
    setSubTaskPagination(prev => ({ ...prev, page: 1 })) // 重置到第一页
  }

  if (taskLoading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="flex items-center justify-center min-h-[400px]">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
          <span className="ml-3">{t('taskDetail.loading')}</span>
        </div>
      </div>
    )
  }

  if (taskError || !task) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="flex items-center gap-4 mb-8">
          <Button variant="outline" onClick={() => router.back()}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            {t('taskDetail.back')}
          </Button>
        </div>
        <Alert>
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>
            {t('taskDetail.notFound')}
          </AlertDescription>
        </Alert>
      </div>
    )
  }

  const renderTaskHeader = () => (
    <div className="flex items-center justify-between mb-8">
      <div className="flex items-center gap-4">
        <Button variant="outline" onClick={() => router.back()}>
          <ArrowLeft className="h-4 w-4 mr-2" />
          {t('taskDetail.back')}
        </Button>
        <div>
          <h1 className="text-3xl font-bold">{task.task_name}</h1>
          <p className="text-muted-foreground">{t('taskDetail.taskId', { id: task.id })}</p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Button variant="outline" onClick={handleRefresh}>
          <RefreshCw className="h-4 w-4 mr-2" />
          {t('taskDetail.refresh')}
        </Button>
        
        {/* 暂停/恢复按钮 */}
        {(task.status === 'running' || task.status === 'pending') && (
          <Button
            variant="outline"
            onClick={async () => {
              if (!confirm(t('taskDetail.confirmPause', { name: task.task_name }))) {
                return
              }
              try {
                await tasksApi.pauseTask(task.id, '用户手动暂停')
                toast.success(t('taskDetail.pauseSuccess'))
                queryClient.invalidateQueries({ queryKey: ['task', taskId] })
                queryClient.invalidateQueries({ queryKey: ['task-runtime-status', taskId] })
              } catch (error) {
                const errorMessage = error instanceof Error ? error.message : t('taskDetail.pauseError')
                toast.error(errorMessage)
              }
            }}
          >
            <Pause className="h-4 w-4 mr-2" />
            {t('taskDetail.pauseTask')}
          </Button>
        )}
        {task.status === 'paused' && (
          <Button
            variant="outline"
            onClick={async () => {
              try {
                await tasksApi.resumeTask(task.id)
                toast.success(t('taskDetail.resumeSuccess'))
                queryClient.invalidateQueries({ queryKey: ['task', taskId] })
                queryClient.invalidateQueries({ queryKey: ['task-runtime-status', taskId] })
              } catch (error) {
                const errorMessage = error instanceof Error ? error.message : t('taskDetail.resumeError')
                toast.error(errorMessage)
              }
            }}
          >
            <PlayCircle className="h-4 w-4 mr-2" />
            {t('taskDetail.resumeTask')}
          </Button>
        )}

        {task.status === 'completed' ? (
          <Button>
            <Download className="h-4 w-4 mr-2" />
            {t('taskDetail.exportResults')}
          </Button>
        ) : task.status !== 'paused' ? (
          <Button onClick={() => setExecDialogOpen(true)}>
            <Play className="h-4 w-4 mr-2" />
            执行{task.analyzer_type === 'CFG' ? 'CFG' : 'CAPE'}
          </Button>
        ) : (
          <Button disabled>
            <Play className="h-4 w-4 mr-2" />
            任务已暂停，请先恢复
          </Button>
        )}
      </div>
    </div>
  )

  const renderTaskOverview = () => (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
      {/* 任务状态 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('taskDetail.taskStatus')}</CardTitle>
          <Activity className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <TaskStatusBadge status={task.status} />
          <p className="text-xs text-muted-foreground mt-2">
            {task.status === 'running' ? '正在执行中' :
             task.status === 'completed' ? '已完成' :
             task.status === 'failed' ? '执行失败' :
             task.status === 'pending' ? '等待执行' :
             task.status === 'paused' ? '任务已暂停' :
             task.status === 'cancelled' ? '任务已取消' : '未知状态'}
          </p>
        </CardContent>
      </Card>

      {/* 分析器类型 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('taskDetail.analyzer')}</CardTitle>
          <Settings className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <AnalyzerBadge type={task.analyzer_type} />
          <p className="text-xs text-muted-foreground mt-2">
            {task.task_type} 类型任务
          </p>
        </CardContent>
      </Card>

      {/* 进度信息 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('taskDetail.executionProgress')}</CardTitle>
          <BarChart3 className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold mb-2">
            {runtimeStatus?.progress_percentage?.toFixed(1) ?? task.progress?.toFixed(1) ?? '0.0'}%
          </div>
          <TaskProgress progress={runtimeStatus?.progress_percentage ?? task.progress ?? 0} size="sm" />
          <p className="text-xs text-muted-foreground mt-1">
            {runtimeStatus?.counts ? `${runtimeStatus.counts.completed}/${runtimeStatus.total}` : `${task.completed_samples ?? 0}/${task.total_samples ?? 0}`} 样本完成
          </p>
          {runtimeStatus?.counts && (
            <div className="mt-2">
              <TaskStatusCountsDisplay 
                counts={runtimeStatus.counts} 
                total={runtimeStatus.total}
                className="text-xs"
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* 时间信息 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('taskDetail.timeInfo')}</CardTitle>
          <Clock className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="space-y-1">
            <div className="text-sm">
              <span className="text-muted-foreground">创建：</span>
              <span className="font-medium">{task.created_at ? formatRelativeTime(task.created_at) : '未知'}</span>
            </div>
            <div className="text-sm">
              <span className="text-muted-foreground">更新：</span>
              <span className="font-medium">{task.updated_at ? formatRelativeTime(task.updated_at) : '未知'}</span>
            </div>
            {runtimeStatus?.started_at && (
              <div className="text-sm">
                <span className="text-muted-foreground">开始：</span>
                <span className="font-medium">{formatRelativeTime(runtimeStatus.started_at)}</span>
              </div>
            )}
            {runtimeStatus?.duration_seconds && runtimeStatus.duration_seconds > 0 && (
              <div className="text-sm">
                <span className="text-muted-foreground">运行时间：</span>
                <span className="font-medium">
                  {Math.floor(runtimeStatus.duration_seconds / 60)}分{runtimeStatus.duration_seconds % 60}秒
                </span>
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  )

  const renderSubTasksList = () => {
    try {
      return (
        <div className="space-y-4">
          <SubTaskFilters 
            filters={subTaskFilters}
            onFiltersChange={handleSubTaskFiltersChange}
          />
          <SubTaskTable
            data={subTasksData || { items: [], total: 0, page: 1, page_size: 20, total_pages: 0 }}
            loading={subTasksLoading}
            onPageChange={handleSubTaskPageChange}
            onViewAnalysisResult={handleViewAnalysisResult}
            onViewRuntimeSnapshot={handleViewRuntimeSnapshot}
          />
        </div>
      )
    } catch (error) {
      console.error('Error rendering sub tasks list:', error)
      return (
        <div className="space-y-4">
          <Alert>
            <AlertTriangle className="h-4 w-4" />
            <AlertDescription>
              子任务列表加载失败，请刷新页面重试。错误: {error instanceof Error ? error.message : '未知错误'}
            </AlertDescription>
          </Alert>
        </div>
      )
    }
  }

  const renderTaskParameters = () => (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Settings className="h-4 w-4" />
          任务配置
        </CardTitle>
        <CardDescription>
          任务创建时的配置参数和设置
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <div className="text-sm text-muted-foreground">任务类型</div>
            <div className="font-medium">{task.task_type}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground">分析器</div>
            <div className="font-medium">{task.analyzer_type}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground">样本总数</div>
            <div className="font-medium">{task.total_samples}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground">已完成</div>
            <div className="font-medium">{task.completed_samples}</div>
          </div>
          {task.failed_samples > 0 && (
            <div>
              <div className="text-sm text-muted-foreground">失败数量</div>
              <div className="font-medium text-red-600">{task.failed_samples}</div>
            </div>
          )}
        </div>
        {task.sample_filter && Object.keys(task.sample_filter).length > 0 && (
          <div className="mt-4">
            <div className="text-sm text-muted-foreground mb-2">样本筛选条件</div>
            <pre className="bg-muted p-3 rounded text-xs overflow-x-auto">
              {JSON.stringify(task.sample_filter, null, 2)}
            </pre>
          </div>
        )}
      </CardContent>
    </Card>
  )

  const renderAnalysisResults = () => {
    try {
      return (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BarChart3 className="h-4 w-4" />
              分析结果概览
            </CardTitle>
            <CardDescription>
              任务执行完成后的分析结果和统计信息
            </CardDescription>
          </CardHeader>
          <CardContent>
            {resultsLoading ? (
              <div className="flex items-center justify-center py-8">
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
                <span className="ml-2">加载分析结果中...</span>
              </div>
            ) : task.status === 'completed' ? (
              <div className="space-y-4">
                {task.result_summary && (
                  <div>
                    <div className="text-sm text-muted-foreground mb-2">结果摘要</div>
                    <pre className="bg-muted p-3 rounded text-xs overflow-x-auto">
                      {JSON.stringify(task.result_summary, null, 2)}
                    </pre>
                  </div>
                )}
                {analysisResults && analysisResults.items && analysisResults.items.length > 0 ? (
                  <div>
                    <div className="text-sm text-muted-foreground mb-2">
                      分析结果 ({analysisResults.items.length} 个)
                    </div>
                    <div className="space-y-2">
                      {analysisResults.items.slice(0, 5).map((result) => (
                        <div key={result.id} className="flex items-center justify-between p-3 border rounded-lg hover:bg-accent/50 transition-colors">
                          <div className="flex items-center gap-3">
                            <div>
                              <div className="font-medium text-sm">{result.sample_id}</div>
                              <div className="text-xs text-muted-foreground">
                                CAPE #{result.cape_task_id}
                                {result.score && ` • 评分: ${result.score}`}
                                {result.verdict && ` • ${result.verdict}`}
                              </div>
                            </div>
                          </div>
                          <Button 
                            variant="outline" 
                            size="sm"
                            onClick={() => handleViewAnalysisResult(result.id, result.sample_id)}
                          >
                            <Eye className="h-4 w-4 mr-1" />
                            查看详情
                          </Button>
                        </div>
                      ))}
                      {analysisResults.items.length > 5 && (
                        <div className="text-center">
                          <Button variant="outline" size="sm">
                            查看更多结果 ({analysisResults.items.length - 5})
                          </Button>
                        </div>
                      )}
                    </div>
                  </div>
                ) : (
                  <p className="text-muted-foreground">暂无详细分析结果</p>
                )}
              </div>
            ) : (
              <div className="text-center py-8 text-muted-foreground">
                <BarChart3 className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p>任务完成后将显示分析结果</p>
              </div>
            )}
          </CardContent>
        </Card>
      )
    } catch (error) {
      console.error('Error rendering analysis results:', error)
      return (
        <Alert>
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>
            分析结果加载失败，请刷新页面重试。错误: {error instanceof Error ? error.message : '未知错误'}
          </AlertDescription>
        </Alert>
      )
    }
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-7xl">
      {renderTaskHeader()}
      {renderTaskOverview()}

      {/* 错误信息 */}
      {task.error_message && (
        <Alert className="mb-8">
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>
            <strong>任务执行错误：</strong>{task.error_message}
          </AlertDescription>
        </Alert>
      )}

      <Tabs defaultValue="subtasks" className="w-full">
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="subtasks">{t('taskDetail.tabSubtasks')}</TabsTrigger>
          <TabsTrigger value="config">{t('taskDetail.tabConfig')}</TabsTrigger>
          <TabsTrigger value="results">{t('taskDetail.tabResults')}</TabsTrigger>
        </TabsList>

        <TabsContent value="subtasks" className="space-y-6">
          {renderSubTasksList()}
        </TabsContent>

        <TabsContent value="config" className="space-y-6">
          {renderTaskParameters()}
        </TabsContent>

        <TabsContent value="results" className="space-y-6">
          {renderAnalysisResults()}
        </TabsContent>
      </Tabs>

      {/* 分析结果详情对话框 */}
      <AnalysisResultDialog
        open={analysisDialogOpen}
        onOpenChange={setAnalysisDialogOpen}
        analysisId={selectedAnalysisId}
        sampleName={selectedSampleName}
      />
      <CfgAnalysisResultDialog
        open={cfgAnalysisDialogOpen}
        onOpenChange={setCfgAnalysisDialogOpen}
        analysisId={selectedAnalysisId}
        sampleName={selectedSampleName}
      />
      <CapeRuntimeDialog
        open={runtimeDialogOpen}
        onOpenChange={setRuntimeDialogOpen}
        subTaskId={selectedSubTaskId || ''}
        sampleName={selectedSampleName}
      />
      <ExecutionParamsDialog
        open={execDialogOpen}
        onOpenChange={setExecDialogOpen}
        analyzer={task.analyzer_type}
        masterTaskId={task.id}
        defaults={task.sample_filter as unknown as Record<string, unknown>}
        onExecuted={handleRefresh}
      />
    </div>
  )
}