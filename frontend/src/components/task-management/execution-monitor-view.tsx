"use client"

import { useState, useEffect } from 'react'
import { MasterTask } from '@/lib/types'
import { capeApi, cfgApi } from '@/lib/api'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Alert, AlertDescription } from '@/components/ui/alert'
import {
  TaskStatusBadge,
  AnalyzerBadge,
  TaskProgress
} from '@/components/task-management'
import {
  Activity,
  Clock,
  Play,
  Pause,
  AlertTriangle,
  CheckCircle,
  Zap,
  RefreshCw
} from 'lucide-react'
import { formatRelativeTime } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'

interface ExecutionMonitorViewProps {
  tasks: MasterTask[]
}

export function ExecutionMonitorView({ tasks }: ExecutionMonitorViewProps) {
  const { t } = useI18n()
  const [autoRefresh, setAutoRefresh] = useState(true)
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null)

  // 筛选出活跃任务（运行中或等待中）
  const activeTasks = tasks.filter(task => 
    task.status === 'running' || task.status === 'pending'
  )

  // 获取执行状态（按分析器区分）
  const { data: executionStatus, refetch: refetchStatus } = useQuery({
    queryKey: ['execution-status', selectedTaskId],
    queryFn: async () => {
      const t = tasks.find(x => x.id === selectedTaskId)
      if (!t) return null
      if (t.analyzer_type === 'CFG') {
        return cfgApi.getTaskStatus(t.id)
      }
      return capeApi.getExecutionStatus(t.id)
    },
    enabled: !!selectedTaskId,
    refetchInterval: autoRefresh ? 5000 : false,
  })

  // 获取选中任务的执行状态 (暂时禁用，等待后端实现)
  // const { data: executionStatus, isLoading: executionLoading } = useQuery({
  //   queryKey: ['execution-status', selectedTaskId],
  //   queryFn: () => capeApi.getExecutionStatus(selectedTaskId!),
  //   enabled: !!selectedTaskId,
  //   refetchInterval: autoRefresh ? 5000 : false, // 5秒自动刷新
  // })

  // 自动选择第一个活跃任务
  useEffect(() => {
    if (!selectedTaskId && activeTasks.length > 0) {
      setSelectedTaskId(activeTasks[0].id)
    }
  }, [activeTasks, selectedTaskId])

  const renderOverviewCards = () => (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {/* 活跃任务数 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('monitor.activeTasks')}</CardTitle>
          <Activity className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold text-blue-600">{activeTasks.length}</div>
          <p className="text-xs text-muted-foreground">
            {t('taskFilters.running')}: {tasks.filter(t => t.status === 'running').length} | 
            {t('taskStatus.pending')}: {tasks.filter(t => t.status === 'pending').length}
          </p>
        </CardContent>
      </Card>

      {/* 完成任务数 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('monitor.completedTasks')}</CardTitle>
          <CheckCircle className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold text-green-600">
            {tasks.filter(t => t.status === 'completed').length}
          </div>
          <p className="text-xs text-muted-foreground">
            {t('taskStats.completionRate')}: {tasks.length > 0 ? 
              ((tasks.filter(t => t.status === 'completed').length / tasks.length) * 100).toFixed(1) 
              : 0}%
          </p>
        </CardContent>
      </Card>

      {/* 失败任务数 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{t('monitor.failedTasks')}</CardTitle>
          <AlertTriangle className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold text-red-600">
            {tasks.filter(t => t.status === 'failed').length}
          </div>
          <p className="text-xs text-muted-foreground">
            {t('taskFilters.failed')}
          </p>
        </CardContent>
      </Card>

      {/* 平均处理时间 */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Average Processing Time</CardTitle>
          <Clock className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            Calculating...
          </div>
          <p className="text-xs text-muted-foreground">
            Average analysis time per sample
          </p>
        </CardContent>
      </Card>
    </div>
  )

  const renderActiveTasksList = () => (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Play className="h-4 w-4" />
              {t('monitor.activeTasks')}
            </CardTitle>
            <CardDescription>{t('monitor.realTimeMonitor')}</CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setAutoRefresh(!autoRefresh)}
              className={autoRefresh ? 'bg-green-50 border-green-200' : ''}
            >
              <RefreshCw className={`h-4 w-4 mr-2 ${autoRefresh ? 'animate-spin' : ''}`} />
              {autoRefresh ? t('monitor.autoRefresh') : t('monitor.refresh')}
            </Button>
            {!autoRefresh && (
              <Button variant="outline" size="sm" onClick={() => refetchStatus()}>
                <RefreshCw className="h-4 w-4 mr-2" />{t('monitor.refresh')}
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {activeTasks.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <Pause className="h-12 w-12 mx-auto mb-4 opacity-50" />
            <p className="text-lg font-medium">{t('monitor.noActiveTasks')}</p>
            <p className="text-sm">{t('monitor.selectTask')}</p>
          </div>
        ) : (
          <div className="space-y-4">
            {activeTasks.map((task) => (
              <div
                key={task.id}
                className={`p-4 border rounded-lg cursor-pointer transition-all ${
                  selectedTaskId === task.id
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:border-primary/50'
                }`}
                onClick={() => setSelectedTaskId(task.id)}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <TaskStatusBadge status={task.status} />
                    <div>
                      <div className="font-medium">{task.task_name}</div>
                      <div className="text-sm text-muted-foreground flex items-center gap-2">
                        <AnalyzerBadge type={task.analyzer_type} />
                        <span>•</span>
                        <span>{t('monitor.createdAt')} {formatRelativeTime(task.created_at)}</span>
                      </div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-sm font-medium">
                      {task.completed_samples}/{task.total_samples} {t('common.samples') || 'samples'}
                    </div>
                    <div className="w-32">
                      <TaskProgress progress={task.progress} size="sm" />
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )

  const renderTaskDetails = () => {
    const selectedTask = tasks.find(t => t.id === selectedTaskId)
    if (!selectedTask) {
      return (
        <Card>
          <CardContent className="py-8">
            <div className="text-center text-muted-foreground">
              <Activity className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>{t('monitor.selectTask')}</p>
            </div>
          </CardContent>
        </Card>
      )
    }

    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
                            <Activity className="h-4 w-4" />
            {t('monitor.taskDetails')}
          </CardTitle>
          <CardDescription>
            {selectedTask.task_name} - {selectedTask.id}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* 基本信息 */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <div className="text-sm text-muted-foreground">{t('common.status') || 'Status'}</div>
              <div className="mt-1">
                <TaskStatusBadge status={selectedTask.status} />
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">{t('common.analyzer') || 'Analyzer'}</div>
              <div className="mt-1">
                <AnalyzerBadge type={selectedTask.analyzer_type} />
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">{t('monitor.totalProgress')}</div>
              <div className="mt-1 font-medium">{selectedTask.progress}%</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">{t('monitor.progress')}</div>
              <div className="mt-1 font-medium">
                {selectedTask.completed_samples}/{selectedTask.total_samples}
              </div>
            </div>
          </div>

          {/* 进度条 */}
          <div>
            <div className="flex justify-between text-sm mb-2">
              <span>{t('monitor.totalProgress')}</span>
              <span>{executionStatus?.progress_percentage ?? selectedTask.progress}%</span>
            </div>
            <TaskProgress progress={executionStatus?.progress_percentage ?? selectedTask.progress} size="lg" />
          </div>

          {/* 任务时间信息 */}
          {selectedTask.status === 'running' && (
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <h4 className="font-medium text-blue-800 mb-3 flex items-center gap-2">
                <Zap className="h-4 w-4" />
                {t('monitor.executionStatus')}
              </h4>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <div className="text-blue-600">{t('monitor.createdAt')}</div>
                  <div className="font-medium">{formatRelativeTime(selectedTask.created_at)}</div>
                </div>
                <div>
                  <div className="text-blue-600">Updated at</div>
                  <div className="font-medium">{formatRelativeTime(selectedTask.updated_at)}</div>
                </div>
              </div>
              <div className="mt-3 text-sm">
                <span className="text-blue-600">Status:</span>
                <span className="font-medium">Processing sample analysis</span>
              </div>
            </div>
          )}

          {/* 错误信息 */}
          {selectedTask.error_message && (
            <Alert>
              <AlertTriangle className="h-4 w-4" />
              <AlertDescription>
                <strong>Error:</strong>{selectedTask.error_message}
              </AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>
    )
  }



  return (
    <div className="space-y-6">
      {/* 概览卡片 */}
      {renderOverviewCards()}

      <Tabs defaultValue="monitor" className="w-full">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="monitor">{t('monitor.realTimeMonitor')}</TabsTrigger>
          <TabsTrigger value="history">{t('monitor.historyRecord')}</TabsTrigger>
        </TabsList>

        <TabsContent value="monitor" className="space-y-6">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {renderActiveTasksList()}
            {renderTaskDetails()}
          </div>
        </TabsContent>

        <TabsContent value="history" className="space-y-6">
          <Card>
            <CardContent className="py-8">
              <div className="text-center text-muted-foreground">
                <Clock className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p className="text-sm">{t('monitor.historyFeature')}</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}