"use client"

import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { tasksApi } from '@/lib/api'
import { TaskFilters, Pagination } from '@/lib/types'
import { TaskStatsBar } from '@/components/task-management/task-stats-bar'
import { TaskFilters as TaskFiltersComponent } from '@/components/task-management/task-filters'
import { TaskTable } from '@/components/task-management/task-table'
import { TaskCreateDialog } from '@/components/task-management/task-create-dialog'
import { ExecutionMonitorView } from '@/components/task-management/execution-monitor-view'
import { Button } from '@/components/ui/button'
import { Plus, List, Monitor, RefreshCw } from 'lucide-react'
import { useI18n } from '@/lib/i18n'

export default function TasksPage() {
  const { t } = useI18n()
  const queryClient = useQueryClient()
  const [filters, setFilters] = useState<TaskFilters>({})
  const [pagination, setPagination] = useState<Pagination>({ page: 1, page_size: 20 })
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [viewMode, setViewMode] = useState<'list' | 'monitor'>('list')

  // 查询任务列表 - 基于现有的useQuery模式
  const { data: tasksData, isLoading: tasksLoading, error: tasksError } = useQuery({
    queryKey: ['tasks', filters, pagination],
    queryFn: () => tasksApi.list(filters, pagination),
    // refetchInterval: 30000, // 30秒自动刷新 - 暂时禁用以排查分页问题
  })

  // 查询统计信息 - 基于现有的stats查询模式
  const { data: statsData, isLoading: statsLoading } = useQuery({
    queryKey: ['tasks-stats'],
    queryFn: () => tasksApi.getStats(),
    refetchInterval: 30000,
  })

  const handleFiltersChange = (newFilters: TaskFilters) => {
    setFilters(newFilters)
    setPagination(prev => ({ ...prev, page: 1 })) // 重置到第一页
  }

  const handlePageChange = (page: number) => {
    setPagination(prev => ({ ...prev, page }))
  }

  const handlePageSizeChange = (pageSize: number) => {
    setPagination({ page: 1, page_size: pageSize })
  }

  const handleRefresh = () => {
    queryClient.invalidateQueries({ queryKey: ['tasks'] })
    queryClient.invalidateQueries({ queryKey: ['tasks-stats'] })
  }

  return (
    <div className="min-h-screen bg-background">
      <div className="container mx-auto px-4 py-6 space-y-6">
        {/* 页面标题 */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold">{t('pages.tasks.title')}</h1>
            <p className="text-muted-foreground mt-1">{t('pages.tasks.desc')}</p>
          </div>
          <div className="flex items-center space-x-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleRefresh}
              disabled={tasksLoading || statsLoading}
            >
              <RefreshCw className={`mr-2 h-4 w-4 ${tasksLoading || statsLoading ? 'animate-spin' : ''}`} />
              {t('common.refresh')}
            </Button>
            <Button onClick={() => setCreateDialogOpen(true)}>
              <Plus className="w-4 h-4 mr-2" />
              {t('pages.tasks.create')}
            </Button>
          </div>
        </div>
        {/* 统计信息栏 - 基于StatsBar组件模式 */}
        <TaskStatsBar data={statsData} loading={statsLoading} />

        {/* 筛选器和视图切换 - 基于现有模式 */}
        <div className="flex items-center justify-between">
          <TaskFiltersComponent filters={filters} onFiltersChange={handleFiltersChange} />
          <div className="flex items-center gap-2">
            <Button
              variant={viewMode === 'list' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setViewMode('list')}
            >
              <List className="h-4 w-4 mr-2" />
              {t('taskViews.listView')}
            </Button>
            <Button
              variant={viewMode === 'monitor' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setViewMode('monitor')}
            >
              <Monitor className="h-4 w-4 mr-2" />
              {t('taskViews.monitorView')}
            </Button>
          </div>
        </div>

        {/* 任务列表/监控视图 */}
        {viewMode === 'list' ? (
          <TaskTable
            data={tasksData?.items || []}
            total={tasksData?.total || 0}
            page={pagination.page}
            pageSize={pagination.page_size}
            loading={tasksLoading}
            error={tasksError}
            onPageChange={handlePageChange}
            onPageSizeChange={handlePageSizeChange}
          />
        ) : (
          <ExecutionMonitorView tasks={tasksData?.items || []} />
        )}
      </div>

      {/* 任务创建对话框 */}
      <TaskCreateDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
      />
    </div>
  )
}





