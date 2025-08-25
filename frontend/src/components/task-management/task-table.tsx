"use client"

import { useState } from 'react'
import { useMutation, useQueryClient, useQueries } from '@tanstack/react-query'
import { useRouter } from 'next/navigation'
import { toast } from 'sonner'
import { MasterTask, TaskRuntimeStatus } from '@/lib/types'
import { formatRelativeTime } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Checkbox } from '@/components/ui/checkbox'
import { Button } from '@/components/ui/button'
import { tasksApi } from '@/lib/api'
import { 
  ChevronLeft, 
  ChevronRight,
  Info,
  Download,
  Trash2,
  Play,
  Pause,
  PlayCircle
} from 'lucide-react'
import { 
  TaskStatusBadge, 
  AnalyzerBadge, 
  TaskProgress 
} from '@/components/task-management'
import { TaskDetailDialog } from './task-detail-dialog'

interface TaskTableProps {
  data: MasterTask[]
  total: number
  page: number
  pageSize: number
  loading?: boolean
  error?: Error | null
  onPageChange: (page: number) => void
  onPageSizeChange: (pageSize: number) => void
}

export function TaskTable({ 
  data, 
  total, 
  page, 
  pageSize, 
  loading, 
  error, 
  onPageChange, 
  onPageSizeChange 
}: TaskTableProps) {
  const { t } = useI18n()
  const [selectedTasks, setSelectedTasks] = useState<string[]>([])
  const [detailDialogOpen, setDetailDialogOpen] = useState(false)
  const [selectedTask, setSelectedTask] = useState<MasterTask | null>(null)
  const queryClient = useQueryClient()
  const router = useRouter()

  const totalPages = Math.ceil(total / pageSize)

  // 并发获取每个任务的运行时状态：
  // - 首次进入页面：所有任务都拉取一次，保证计数口径一致（失败=failed+cancelled）
  // - 后续轮询：仅对运行中/等待中的任务继续轮询，减少不必要的请求
  const runtimeQueries = useQueries({
    queries: data.map((task) => ({
      queryKey: ['task-runtime-status', task.id],
      queryFn: () => tasksApi.getRuntimeStatus(task.id),
      // 始终启用以便首次进入页面即可获取一次最新状态；活跃任务才轮询
      enabled: true,
      staleTime: 25000,
      refetchInterval: (task.status === 'running' || task.status === 'pending') ? 30000 : false,
    }))
  })
  const runtimeStatuses: Map<string, TaskRuntimeStatus> = new Map()
  runtimeQueries.forEach((q, idx) => { if (q.data) runtimeStatuses.set(data[idx].id, q.data as TaskRuntimeStatus) })

  // runtimeStatuses 已由 useQueries 汇总到 Map

  // 删除任务Mutation
  const deleteMutation = useMutation({
    mutationFn: async (taskId: string) => {
      await tasksApi.deleteTask(taskId)
    },
    onSuccess: () => {
      toast.success(t('taskTable.deleteSuccess'))
      queryClient.invalidateQueries({ queryKey: ['tasks'] })
      queryClient.invalidateQueries({ queryKey: ['tasks-stats'] })
      setSelectedTasks([]) // 清除选择
    },
    onError: (error) => {
      toast.error(t('taskTable.deleteError', { message: error.message }))
    }
  })

  // 批量删除Mutation
  const batchDeleteMutation = useMutation({
    mutationFn: async (taskIds: string[]) => {
      // 并行删除所有选中任务
      await Promise.all(taskIds.map(id => tasksApi.deleteTask(id)))
    },
    onSuccess: () => {
      toast.success(t('taskTable.batchDeleteSuccess', { count: selectedTasks.length }))
      queryClient.invalidateQueries({ queryKey: ['tasks'] })
      queryClient.invalidateQueries({ queryKey: ['tasks-stats'] })
      setSelectedTasks([])
    },
    onError: (error) => {
      toast.error(t('taskTable.batchDeleteError', { message: error.message }))
    }
  })

  const handleSelectAll = (checked: boolean) => {
    if (checked) {
      setSelectedTasks(data.map(task => task.id))
    } else {
      setSelectedTasks([])
    }
  }

  const handleTaskSelect = (taskId: string, checked: boolean) => {
    if (checked) {
      setSelectedTasks([...selectedTasks, taskId])
    } else {
      setSelectedTasks(selectedTasks.filter(id => id !== taskId))
    }
  }

  const handleTaskDetail = (task: MasterTask) => {
    setSelectedTask(task)
    setDetailDialogOpen(true)
  }

  const handleTaskNavigate = (taskId: string) => {
    router.push(`/tasks/${taskId}`)
  }

  const handleBatchDelete = () => {
    if (selectedTasks.length === 0) return
    
    if (confirm(t('taskTable.confirmBatchDelete', { count: selectedTasks.length }))) {
      batchDeleteMutation.mutate(selectedTasks)
    }
  }

  if (loading) {
    return (
      <div className="space-y-4">
        <div className="border rounded-lg p-8">
          <div className="flex items-center justify-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
            <span className="ml-2">{t('taskTable.loading')}</span>
          </div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="space-y-4">
        <div className="border rounded-lg p-8">
          <div className="text-center text-destructive">
            <p>{t('taskTable.loadError', { message: error.message })}</p>
            <Button 
              variant="outline" 
              className="mt-2" 
              onClick={() => window.location.reload()}
            >
              {t('taskTable.retry')}
            </Button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* 批量操作栏 */}
      {selectedTasks.length > 0 && (
        <div className="flex items-center justify-between bg-muted p-3 rounded-lg">
          <span className="text-sm">{t('taskTable.selected', { count: selectedTasks.length })}</span>
          <Button 
            variant="destructive" 
            size="sm" 
            onClick={handleBatchDelete}
            disabled={batchDeleteMutation.isPending}
          >
            <Trash2 className="h-4 w-4 mr-1" />
            {batchDeleteMutation.isPending ? t('common.loading') : t('common.delete')}
          </Button>
        </div>
      )}

      {/* 任务列表表格 */}
      <div className="border rounded-lg">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-12">
                <Checkbox
                  checked={selectedTasks.length === data.length && data.length > 0}
                  onCheckedChange={handleSelectAll}
                />
              </TableHead>
              <TableHead>{t('tasks.taskName')}</TableHead>
              <TableHead>{t('tasks.analyzer')}</TableHead>
              <TableHead>{t('common.status')}</TableHead>
              <TableHead>{t('tasks.progress')}</TableHead>
              <TableHead>{t('tasks.samples')}</TableHead>
              <TableHead>{t('tasks.createdAt')}</TableHead>
              <TableHead className="text-right">{t('common.actions')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">
                  暂无任务数据
                </TableCell>
              </TableRow>
            ) : (
              data.map((task) => (
                <TableRow key={task.id} className="hover:bg-muted/50">
                  <TableCell>
                    <Checkbox
                      checked={selectedTasks.includes(task.id)}
                      onCheckedChange={(checked) => handleTaskSelect(task.id, !!checked)}
                    />
                  </TableCell>
                  <TableCell>
                    <div className="space-y-1">
                      <div 
                        className="font-medium cursor-pointer hover:text-primary"
                        onClick={() => handleTaskNavigate(task.id)}
                      >
                        {task.task_name}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        ID: {task.id.slice(0, 8)}...
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <AnalyzerBadge type={task.analyzer_type} />
                  </TableCell>
                  <TableCell>
                    <TaskStatusBadge status={task.status} />
                  </TableCell>
                  <TableCell>
                    {(() => {
                      const rt = runtimeStatuses?.get(task.id)
                      const progress = rt?.progress_percentage ?? task.progress
                      return (
                        <>
                          <TaskProgress 
                            progress={progress}
                            size="sm"
                            status={task.status}
                          />
                          <div className="text-xs text-muted-foreground mt-1">
                            {progress.toFixed ? progress.toFixed(1) : progress}%
                          </div>
                        </>
                      )
                    })()}
                  </TableCell>
                  <TableCell>
                    {(() => {
                      const rt = runtimeStatuses?.get(task.id)
                      const totalSamples = rt?.total ?? task.total_samples
                      const completedSamples = rt?.counts?.completed ?? task.completed_samples
                      const failedSamples = rt?.counts ? (rt.counts.failed + rt.counts.cancelled) : task.failed_samples
                      
                      return (
                        <div className="text-sm space-y-1">
                          <div>总计: <span className="font-medium">{totalSamples}</span></div>
                          <div className="text-xs text-muted-foreground">
                            完成: {completedSamples} | 失败: {failedSamples}
                          </div>
                        </div>
                      )
                    })()}
                  </TableCell>
                  <TableCell>
                    <div className="text-sm">
                      {formatRelativeTime(task.created_at)}
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-2">
                      {/* 暂停/恢复按钮 */}
                      {(task.status === 'running' || task.status === 'pending') && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={async () => {
                            if (!confirm(`确定要暂停任务"${task.task_name}"吗？\n\n正在执行的子任务将完成后停止，等待中的子任务将暂停提交。`)) {
                              return
                            }
                            try {
                              await tasksApi.pauseTask(task.id, '用户手动暂停')
                              toast.success('任务已暂停')
                              queryClient.invalidateQueries({ queryKey: ['tasks'] })
                            } catch (error) {
                              const errorMessage = error instanceof Error ? error.message : '暂停任务失败'
                              toast.error(errorMessage)
                            }
                          }}
                          title="暂停任务"
                        >
                          <Pause className="h-4 w-4" />
                        </Button>
                      )}
                      {task.status === 'paused' && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={async () => {
                            try {
                              await tasksApi.resumeTask(task.id)
                              toast.success('任务已恢复')
                              queryClient.invalidateQueries({ queryKey: ['tasks'] })
                            } catch (error) {
                              const errorMessage = error instanceof Error ? error.message : '恢复任务失败'
                              toast.error(errorMessage)
                            }
                          }}
                          title="恢复任务"
                        >
                          <PlayCircle className="h-4 w-4" />
                        </Button>
                      )}

                      {/* 详情按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleTaskDetail(task)}
                        title="查看详情"
                      >
                        <Info className="h-4 w-4" />
                      </Button>

                      {/* 导航到详情页按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleTaskNavigate(task.id)}
                        title="进入任务详情页"
                      >
                        <Play className="h-4 w-4" />
                      </Button>

                      {/* CFG 任务：下载CSV/ZIP (仅已完成) */}
                      {task.status === 'completed' && (
                        <>
                          <Button
                            variant="ghost"
                            size="sm"
                            title="下载CSV"
                            onClick={async () => {
                              try {
                                const blob = await tasksApi.downloadCsv(task.id)
                                const url = window.URL.createObjectURL(blob)
                                const a = document.createElement('a')
                                a.href = url
                                a.download = `task_${task.id}_results.csv`
                                a.click()
                                window.URL.revokeObjectURL(url)
                              } catch {
                                toast.error('下载CSV失败')
                              }
                            }}
                          >
                            <Download className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            title="下载ZIP"
                            onClick={async () => {
                              try {
                                const blob = await tasksApi.downloadZip(task.id)
                                const url = window.URL.createObjectURL(blob)
                                const a = document.createElement('a')
                                a.href = url
                                a.download = `task_${task.id}_results.zip`
                                a.click()
                                window.URL.revokeObjectURL(url)
                              } catch {
                                toast.error('下载ZIP失败')
                              }
                            }}
                          >
                            <Download className="h-4 w-4" />
                          </Button>
                        </>
                      )}

                      {/* 删除按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          if (confirm(`确定要删除任务"${task.task_name}"吗？\n\n此操作无法撤销，任务下的所有子任务和分析记录也将被删除。`)) {
                            deleteMutation.mutate(task.id)
                          }
                        }}
                        disabled={deleteMutation.isPending}
                        title="删除任务"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* 分页控制 */}
      <div className="flex items-center justify-between">
        <div className="text-sm text-muted-foreground">
          第 {page} 页，共 {totalPages} 页 | 每页 {pageSize} 条 | 共 {total} 条记录
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(page - 1)}
            disabled={page <= 1}
          >
            <ChevronLeft className="h-4 w-4" />
            上一页
          </Button>
          
          {/* 页码显示 */}
          <div className="flex items-center gap-1">
            {totalPages <= 7 ? (
              // 总页数少于7页，显示所有页码
              Array.from({ length: totalPages }, (_, i) => i + 1).map((pageNum) => (
                <Button
                  key={pageNum}
                  variant={pageNum === page ? "default" : "outline"}
                  size="sm"
                  onClick={() => onPageChange(pageNum)}
                  className="w-8 h-8 p-0"
                >
                  {pageNum}
                </Button>
              ))
            ) : (
              // 总页数大于7页，显示省略号
              <>
                {page > 3 && (
                  <>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => onPageChange(1)}
                      className="w-8 h-8 p-0"
                    >
                      1
                    </Button>
                    {page > 4 && <span className="px-2">...</span>}
                  </>
                )}
                
                {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
                  const startPage = Math.max(1, Math.min(page - 2, totalPages - 4))
                  return startPage + i
                }).map((pageNum) => (
                  pageNum <= totalPages && (
                    <Button
                      key={pageNum}
                      variant={pageNum === page ? "default" : "outline"}
                      size="sm"
                      onClick={() => onPageChange(pageNum)}
                      className="w-8 h-8 p-0"
                    >
                      {pageNum}
                    </Button>
                  )
                ))}
                
                {page < totalPages - 2 && (
                  <>
                    {page < totalPages - 3 && <span className="px-2">...</span>}
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => onPageChange(totalPages)}
                      className="w-8 h-8 p-0"
                    >
                      {totalPages}
                    </Button>
                  </>
                )}
              </>
            )}
          </div>

          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(page + 1)}
            disabled={page >= totalPages}
          >
            下一页
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* 每页大小控制 */}
      <div className="flex items-center justify-end gap-2">
        <span className="text-sm text-muted-foreground">每页显示:</span>
        {[10, 20, 50, 100].map((size) => (
          <Button
            key={size}
            variant={pageSize === size ? "default" : "outline"}
            size="sm"
            onClick={() => onPageSizeChange(size)}
            className="w-12 h-8 p-0"
          >
            {size}
          </Button>
        ))}
      </div>

      {/* 任务详情对话框 */}
      <TaskDetailDialog
        open={detailDialogOpen}
        onOpenChange={setDetailDialogOpen}
        task={selectedTask}
      />
    </div>
  )
}