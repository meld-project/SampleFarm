"use client"

import { useState } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import type { TaskFilters, AnalyzerType, MasterTaskStatus, TaskType } from '@/lib/types'
import { Search, X, Filter } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'

interface TaskFiltersProps {
  filters: TaskFilters
  onFiltersChange: (filters: TaskFilters) => void
  className?: string
}

export function TaskFilters({ filters, onFiltersChange, className }: TaskFiltersProps) {
  const { t } = useI18n()
  const [searchQuery, setSearchQuery] = useState('')
  const [activeFilters, setActiveFilters] = useState<TaskFilters>(filters)

  // 防抖搜索 - 暂时注释掉，因为任务没有name搜索字段
  // const debouncedSearch = debounce((query: string) => {
  //   onFiltersChange({
  //     ...activeFilters,
  //     task_name: query || undefined
  //   })
  // }, 500)

  // useEffect(() => {
  //   debouncedSearch(searchQuery)
  // }, [searchQuery, debouncedSearch])

  const handleFilterChange = (key: keyof TaskFilters, value: unknown) => {
    const newFilters = {
      ...activeFilters,
      [key]: value
    }
    setActiveFilters(newFilters)
    onFiltersChange(newFilters)
  }

  const clearFilters = () => {
    setSearchQuery('')
    setActiveFilters({})
    onFiltersChange({})
  }

  const hasActiveFilters = Object.keys(activeFilters).some(key => 
    activeFilters[key as keyof TaskFilters] !== undefined
  ) || searchQuery

  return (
    <div className={cn("space-y-4", className)}>
      {/* 主搜索栏 */}
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground h-4 w-4" />
          <Input
            placeholder={t('taskFilters.searchPlaceholder')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-10"
          />
        </div>
        
        {/* 快速筛选按钮 */}
        <div className="flex items-center gap-2">
          {/* 分析器类型筛选 */}
          <Button
            variant={activeFilters.analyzer_type === 'CAPE' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleFilterChange('analyzer_type', 
              activeFilters.analyzer_type === 'CAPE' ? undefined : 'CAPE' as AnalyzerType
            )}
          >
            CAPE
          </Button>
          <Button
            variant={activeFilters.analyzer_type === 'CFG' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleFilterChange('analyzer_type', 
              activeFilters.analyzer_type === 'CFG' ? undefined : 'CFG' as AnalyzerType
            )}
          >
            CFG
          </Button>

          {/* 任务状态筛选 */}
          <Button
            variant={activeFilters.status === 'running' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleFilterChange('status', 
              activeFilters.status === 'running' ? undefined : 'running' as MasterTaskStatus
            )}
          >
            {t('taskFilters.running')}
          </Button>
          
          <Button
            variant={activeFilters.status === 'completed' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleFilterChange('status', 
              activeFilters.status === 'completed' ? undefined : 'completed' as MasterTaskStatus
            )}
          >
            {t('taskFilters.completed')}
          </Button>
          
          <Button
            variant={activeFilters.status === 'failed' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleFilterChange('status', 
              activeFilters.status === 'failed' ? undefined : 'failed' as MasterTaskStatus
            )}
          >
            {t('taskFilters.failed')}
          </Button>

          {/* 任务类型筛选 */}
          <Button
            variant={activeFilters.task_type === 'batch' ? 'default' : 'outline'}
            size="sm"
            onClick={() => handleFilterChange('task_type', 
              activeFilters.task_type === 'batch' ? undefined : 'batch' as TaskType
            )}
          >
            {t('taskFilters.batchTasks')}
          </Button>
        </div>

        {hasActiveFilters && (
          <Button variant="ghost" size="sm" onClick={clearFilters}>
            <X className="h-4 w-4 mr-1" />
            {t('taskFilters.clearAll')}
          </Button>
        )}
      </div>

      {/* 激活的筛选条件显示 */}
      {hasActiveFilters && (
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm text-muted-foreground flex items-center gap-1">
            <Filter className="h-3 w-3" />
            {t('common.filter')}:
          </span>
          
          {searchQuery && (
            <Badge variant="secondary">
              {t('taskFilters.searchPlaceholder')}: {searchQuery}
              <X 
                className="h-3 w-3 ml-1 cursor-pointer" 
                onClick={() => setSearchQuery('')}
              />
            </Badge>
          )}
          
          {activeFilters.analyzer_type && (
            <Badge variant="default">
              分析器: {activeFilters.analyzer_type}
              <X 
                className="h-3 w-3 ml-1 cursor-pointer" 
                onClick={() => handleFilterChange('analyzer_type', undefined)}
              />
            </Badge>
          )}
          
          {activeFilters.status && (
            <Badge variant={getStatusBadgeVariant(activeFilters.status)}>
              状态: {getStatusLabel(activeFilters.status)}
              <X 
                className="h-3 w-3 ml-1 cursor-pointer" 
                onClick={() => handleFilterChange('status', undefined)}
              />
            </Badge>
          )}
          
          {activeFilters.task_type && (
            <Badge variant="outline">
              类型: {getTaskTypeLabel(activeFilters.task_type)}
              <X 
                className="h-3 w-3 ml-1 cursor-pointer" 
                onClick={() => handleFilterChange('task_type', undefined)}
              />
            </Badge>
          )}
          
          {(activeFilters.start_time || activeFilters.end_time) && (
            <Badge variant="outline">
              时间范围
              <X 
                className="h-3 w-3 ml-1 cursor-pointer" 
                onClick={() => {
                  handleFilterChange('start_time', undefined)
                  handleFilterChange('end_time', undefined)
                }}
              />
            </Badge>
          )}
        </div>
      )}
    </div>
  )
}

// 获取状态标签
function getStatusLabel(status: MasterTaskStatus): string {
  const labels = {
    'pending': '等待中',
    'running': '执行中',
    'paused': '已暂停',
    'completed': '已完成',
    'failed': '失败',
    'cancelled': '已取消'
  }
  return labels[status] || status
}

// 获取任务类型标签
function getTaskTypeLabel(type: TaskType): string {
  const labels = {
    'batch': '批量任务',
    'single': '单个任务'
  }
  return labels[type] || type
}

// 获取状态Badge变体
function getStatusBadgeVariant(status: MasterTaskStatus): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case 'completed': return 'default'
    case 'failed': return 'destructive'
    case 'running': return 'secondary'
    default: return 'outline'
  }
}