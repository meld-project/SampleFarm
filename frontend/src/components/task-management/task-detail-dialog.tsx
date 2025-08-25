"use client"

import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { MasterTask, TaskRuntimeStatus } from '@/lib/types'
import { formatRelativeTime, formatBytes } from '@/lib/utils'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { tasksApi } from '@/lib/api'
import {
  TaskStatusBadge,
  AnalyzerBadge,
  TaskProgress,
  TaskStatusCountsDisplay
} from '@/components/task-management'
import {
  FileText,
  AlertCircle,
  CheckCircle,
  Activity,
  Download,
  ExternalLink
} from 'lucide-react'

interface TaskDetailDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  task: MasterTask | null
}

export function TaskDetailDialog({ open, onOpenChange, task }: TaskDetailDialogProps) {
  const [activeTab, setActiveTab] = useState('overview')

  // 统一实时运行状态（与列表/详情页一致）
  const { data: runtimeStatus } = useQuery<TaskRuntimeStatus>({
    queryKey: ['task-runtime-status', task?.id],
    queryFn: () => tasksApi.getRuntimeStatus(task!.id),
    enabled: !!task,
    refetchInterval: 30000,
    staleTime: 25000,
  })

  // 获取子任务列表
  const { data: subTasksData, isLoading: subTasksLoading } = useQuery({
    queryKey: ['sub-tasks', task?.id],
    queryFn: () => tasksApi.getSubTasks(task!.id, { page: 1, page_size: 100 }),
    enabled: !!task && activeTab === 'sub-tasks'
  })

  if (!task) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-6xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            {task.task_name}
          </DialogTitle>
          <DialogDescription>
            任务ID: {task.id} • 创建于 {formatRelativeTime(task.created_at)}
          </DialogDescription>
        </DialogHeader>

        {/* 实时进度卡片 (仅运行中任务显示) */}
        {task.status === 'running' && runtimeStatus && (
            <Card className="border-blue-200 bg-blue-50">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-blue-800">
                实时执行状态
              </CardTitle>
            </CardHeader>
            <CardContent>
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span>进度</span>
                    <span className="font-medium">{(runtimeStatus?.progress_percentage ?? task.progress).toFixed(1)}%</span>
                  </div>
                  <TaskProgress progress={runtimeStatus?.progress_percentage ?? task.progress} size="sm" status={task.status} />
                </div>

                {runtimeStatus?.counts && (
                  <div className="mt-3">
                    <TaskStatusCountsDisplay counts={runtimeStatus.counts} total={runtimeStatus.total} />
                  </div>
                )}
            </CardContent>
          </Card>
        )}

        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="overview">概览</TabsTrigger>
            <TabsTrigger value="sub-tasks">
              子任务 ({runtimeStatus?.total ?? task.total_samples})
            </TabsTrigger>
            <TabsTrigger value="results">分析结果</TabsTrigger>
          </TabsList>

          {/* 概览标签页 */}
          <TabsContent value="overview" className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* 任务基本信息 */}
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <FileText className="h-4 w-4" />
                    任务信息
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <div className="text-muted-foreground">任务名称</div>
                      <div className="font-medium">{task.task_name}</div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">分析器</div>
                      <div><AnalyzerBadge type={task.analyzer_type} showDescription /></div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">任务类型</div>
                      <div className="font-medium">{task.task_type}</div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">当前状态</div>
                      <div><TaskStatusBadge status={task.status} /></div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">创建时间</div>
                      <div className="font-medium">{formatRelativeTime(task.created_at)}</div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">更新时间</div>
                      <div className="font-medium">{formatRelativeTime(task.updated_at)}</div>
                    </div>
                  </div>

                  {task.error_message && (
                    <div className="p-3 rounded-lg bg-red-50 border border-red-200">
                      <div className="flex items-center gap-2 text-red-800 text-sm font-medium">
                        <AlertCircle className="h-4 w-4" />
                        错误信息
                      </div>
                      <div className="text-red-700 text-sm mt-1">
                        {task.error_message}
                      </div>
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* 执行统计 */}
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Activity className="h-4 w-4" />
                    执行统计
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span>完成进度</span>
                      <span className="font-medium">{(runtimeStatus?.progress_percentage ?? task.progress).toFixed(1)}%</span>
                    </div>
                    <TaskProgress progress={runtimeStatus?.progress_percentage ?? task.progress} status={task.status} size="lg" />
                  </div>

                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <div className="text-muted-foreground">样本总数</div>
                      <div className="font-medium flex items-center gap-1">
                        <FileText className="h-3 w-3" />
                        {runtimeStatus?.total ?? task.total_samples}
                      </div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">已完成</div>
                      <div className="font-medium flex items-center gap-1 text-green-600">
                        <CheckCircle className="h-3 w-3" />
                        {runtimeStatus?.counts?.completed ?? task.completed_samples}
                      </div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">失败数</div>
                      <div className="font-medium flex items-center gap-1 text-red-600">
                        <AlertCircle className="h-3 w-3" />
                        {(runtimeStatus ? (runtimeStatus.counts.failed + runtimeStatus.counts.cancelled) : task.failed_samples) ?? 0}
                      </div>
                    </div>
                    <div>
                      <div className="text-muted-foreground">成功率</div>
                      <div className="font-medium">
                        {(() => {
                          const total = runtimeStatus?.total ?? task.total_samples
                          const completed = runtimeStatus?.counts?.completed ?? task.completed_samples
                          return total > 0 ? `${((completed / total) * 100).toFixed(1)}%` : '0%'
                        })()}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </div>

            {/* 结果摘要 */}
            {task.result_summary && (
              <Card>
                <CardHeader>
                  <CardTitle>结果摘要</CardTitle>
                </CardHeader>
                <CardContent>
                  <pre className="text-sm bg-muted p-3 rounded-lg whitespace-pre-wrap">
                    {JSON.stringify(task.result_summary, null, 2)}
                  </pre>
                </CardContent>
              </Card>
            )}
          </TabsContent>

          {/* 子任务标签页 */}
          <TabsContent value="sub-tasks">
            <Card>
              <CardHeader>
                <CardTitle>子任务列表</CardTitle>
                <CardDescription>
                  显示当前任务的所有子任务执行状态
                </CardDescription>
              </CardHeader>
              <CardContent>
                {subTasksLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
                    <span className="ml-2">加载子任务中...</span>
                  </div>
                ) : subTasksData?.items?.length ? (
                  <div className="space-y-2 max-h-96 overflow-y-auto">
                    {subTasksData.items.map((subTask, index) => (
                      <div 
                        key={subTask.id} 
                        className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50"
                      >
                        <div className="flex items-center gap-3">
                          <div className="text-sm font-mono text-muted-foreground">
                            #{String(index + 1).padStart(3, '0')}
                          </div>
                          <div>
                            <div className="font-medium">{subTask.sample_name}</div>
                            <div className="text-xs text-muted-foreground">
                              {formatBytes(subTask.file_size)}
                            </div>
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          <TaskStatusBadge status={subTask.status} />
                          {subTask.external_task_id && (
                            <Badge variant="outline" className="text-xs">
                              ID: {subTask.external_task_id}
                            </Badge>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-8 text-muted-foreground">
                    暂无子任务数据
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          {/* 分析结果标签页（仅在有接口时显示） */}
          <TabsContent value="results">
            <div className="text-center py-8 text-muted-foreground">
              <p>分析结果查看将在结果接口开放后提供</p>
            </div>
          </TabsContent>
        </Tabs>

        {/* 操作按钮 */}
        <div className="flex justify-between pt-4 border-t">
          <div className="flex gap-2">
            {task.status === 'completed' && (
              <Button variant="outline" disabled>
                <Download className="h-4 w-4 mr-2" />
                下载结果
              </Button>
            )}
            <Button variant="outline" onClick={() => window.open(`/tasks/${task.id}`, '_blank')}>
              <ExternalLink className="h-4 w-4 mr-2" />
              打开详情页
            </Button>
          </div>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}