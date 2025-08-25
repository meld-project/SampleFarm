"use client"

import { Card } from '@/components/ui/card'
import { TaskStats } from '@/lib/types'
import { 
  FileText, 
  Clock, 
  Play, 
  CheckCircle, 
  AlertTriangle,
  Layers
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'

interface TaskStatsBarProps {
  data?: TaskStats
  loading?: boolean
  className?: string
}

interface StatCardProps {
  title: string
  value: number | string
  icon: React.ElementType
  color: string
  trend?: {
    value: number
    isPositive: boolean
  }
}

function StatCard({ title, value, icon: Icon, color, trend }: StatCardProps) {
  return (
    <Card className="p-4">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <p className="text-sm text-muted-foreground">{title}</p>
          <p className="text-2xl font-bold">{value}</p>
          {trend && (
            <p className={cn(
              "text-xs flex items-center gap-1",
              trend.isPositive ? "text-green-600" : "text-red-600"
            )}>
              <span>{trend.isPositive ? "↗" : "↘"}</span>
              {Math.abs(trend.value)}%
            </p>
          )}
        </div>
        <div className={cn("p-2 rounded-lg", color)}>
          <Icon className="h-6 w-6 text-white" />
        </div>
      </div>
    </Card>
  )
}

export function TaskStatsBar({ data, loading, className }: TaskStatsBarProps) {
  const { t } = useI18n()
  
  if (loading) {
    return (
      <div className={cn("grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4", className)}>
        {Array.from({ length: 6 }).map((_, i) => (
          <Card key={i} className="p-4 animate-pulse">
            <div className="space-y-2">
              <div className="h-4 bg-gray-200 rounded w-3/4" />
              <div className="h-6 bg-gray-200 rounded w-1/2" />
            </div>
          </Card>
        ))}
      </div>
    )
  }

  if (!data) {
    return (
      <div className={cn("p-4 text-center text-muted-foreground", className)}>
        <AlertTriangle className="h-8 w-8 mx-auto mb-2" />
        <p>{t('taskStats.loadError')}</p>
      </div>
    )
  }

  // 计算完成率
  const completionRate = data.total_tasks > 0 
    ? ((data.completed_tasks / data.total_tasks) * 100).toFixed(1)
    : '0.0'

  return (
    <div className={cn("grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4", className)}>
      <StatCard
        title={t('taskStats.totalTasks')}
        value={data.total_tasks}
        icon={FileText}
        color="bg-blue-500"
      />
      
      <StatCard
        title={t('taskStats.pendingTasks')}
        value={data.pending_tasks}
        icon={Clock}
        color="bg-gray-500"
      />
      
      <StatCard
        title={t('taskStats.runningTasks')}
        value={data.running_tasks}
        icon={Play}
        color="bg-orange-500"
      />
      
      <StatCard
        title={t('taskStats.completedTasks')}
        value={data.completed_tasks}
        icon={CheckCircle}
        color="bg-green-500"
      />
      
      <StatCard
        title={t('taskStats.failedTasks')}
        value={data.failed_tasks}
        icon={AlertTriangle}
        color="bg-red-500"
      />
      
      <StatCard
        title={t('taskStats.completionRate')}
        value={`${completionRate}%`}
        icon={Layers}
        color="bg-purple-500"
      />
    </div>
  )
}

// 子任务统计栏（用于任务详情页面）
export function SubTaskStatsBar({ data, loading, className }: TaskStatsBarProps) {
  const { t } = useI18n()
  
  if (loading) {
    return (
      <div className={cn("grid grid-cols-2 md:grid-cols-5 gap-4", className)}>
        {Array.from({ length: 5 }).map((_, i) => (
          <Card key={i} className="p-4 animate-pulse">
            <div className="space-y-2">
              <div className="h-4 bg-gray-200 rounded w-3/4" />
              <div className="h-6 bg-gray-200 rounded w-1/2" />
            </div>
          </Card>
        ))}
      </div>
    )
  }

  if (!data) {
    return null
  }

  return (
    <div className={cn("grid grid-cols-2 md:grid-cols-5 gap-4", className)}>
      <StatCard
        title={t('taskStats.totalSubTasks')}
        value={data.total_sub_tasks}
        icon={Layers}
        color="bg-blue-500"
      />
      
      <StatCard
        title={t('taskStats.pendingSubTasks')}
        value={data.pending_sub_tasks}
        icon={Clock}
        color="bg-gray-500"
      />
      
      <StatCard
        title={t('taskStats.runningSubTasks')}
        value={data.running_sub_tasks}
        icon={Play}
        color="bg-orange-500"
      />
      
      <StatCard
        title={t('taskStats.completedSubTasks')}
        value={data.completed_sub_tasks}
        icon={CheckCircle}
        color="bg-green-500"
      />
      
      <StatCard
        title={t('taskStats.failedSubTasks')}
        value={data.failed_sub_tasks}
        icon={AlertTriangle}
        color="bg-red-500"
      />
    </div>
  )
}