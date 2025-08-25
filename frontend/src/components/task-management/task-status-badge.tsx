"use client"

import { Badge } from '@/components/ui/badge'
import { MasterTaskStatus, SubTaskStatus } from '@/lib/types'
import { 
  Clock, 
  Play, 
  CheckCircle, 
  AlertCircle, 
  X, 
  Upload, 
  Loader2, 
  Search,
  Pause
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'

interface TaskStatusBadgeProps {
  status: MasterTaskStatus | SubTaskStatus
  className?: string
}

// 状态配置映射
function getStatusConfig(t: (key: string) => string) {
  return {
    // 主任务状态
    'pending': {
      variant: 'outline' as const,
      icon: Clock,
      label: t('taskStatus.pending'),
      color: 'text-gray-600',
      bgColor: 'bg-gray-100'
    },
    'running': {
      variant: 'outline' as const,
      icon: Play,
      label: t('taskStatus.running'),
      color: 'text-blue-600',
      bgColor: 'bg-blue-100'
    },
    'paused': {
      variant: 'outline' as const,
      icon: Pause,
      label: t('taskStatus.paused'),
      color: 'text-yellow-600',
      bgColor: 'bg-yellow-100'
    },
    'completed': {
      variant: 'outline' as const,
      icon: CheckCircle,
      label: t('taskStatus.completed'),
      color: 'text-green-600',
      bgColor: 'bg-green-100'
    },
    'failed': {
      variant: 'outline' as const,
      icon: AlertCircle,
      label: t('taskStatus.failed'),
      color: 'text-red-600',
      bgColor: 'bg-red-100'
    },
    'cancelled': {
      variant: 'outline' as const,
      icon: X,
      label: t('taskStatus.cancelled'),
      color: 'text-gray-500',
      bgColor: 'bg-gray-50'
    },
    // 子任务专用状态
    'submitting': {
      variant: 'outline' as const,
      icon: Upload,
      label: t('taskStatus.submitting'),
      color: 'text-orange-600',
      bgColor: 'bg-orange-100'
    },
    'submitted': {
      variant: 'outline' as const,
      icon: Loader2,
      label: t('taskStatus.submitted'),
      color: 'text-blue-500',
      bgColor: 'bg-blue-50'
    },
    'analyzing': {
      variant: 'outline' as const,
      icon: Search,
      label: t('taskStatus.analyzing'),
      color: 'text-purple-600',
      bgColor: 'bg-purple-100'
    },
  }
}

export function TaskStatusBadge({ status, className }: TaskStatusBadgeProps) {
  const { t } = useI18n()
  const statusConfig = getStatusConfig(t)
  const config = statusConfig[status]
  
  if (!config) {
    console.warn(`Unknown task status: ${status}`)
    return (
      <Badge variant="outline" className={className}>
        <AlertCircle className="h-3 w-3 mr-1" />
        {t('taskStatus.unknown')}
      </Badge>
    )
  }

  const Icon = config.icon

  return (
    <Badge 
      variant={config.variant} 
      className={cn(
        "inline-flex items-center gap-1",
        config.color,
        className
      )}
    >
      <Icon className={cn(
        "h-3 w-3",
        // 添加动画效果
        status === 'running' && "animate-pulse",
        status === 'submitting' && "animate-bounce",
        status === 'submitted' && "animate-spin",
        status === 'analyzing' && "animate-pulse"
      )} />
      {config.label}
    </Badge>
  )
}

// 获取状态颜色的工具函数
export function getTaskStatusColor(status: MasterTaskStatus | SubTaskStatus, t: (key: string) => string): string {
  const config = getStatusConfig(t)[status]
  return config?.color || 'text-gray-600'
}

// 获取状态背景色的工具函数
export function getTaskStatusBgColor(status: MasterTaskStatus | SubTaskStatus, t: (key: string) => string): string {
  const config = getStatusConfig(t)[status]
  return config?.bgColor || 'bg-gray-100'
}

// 检查状态是否为活跃状态（运行中/进行中）
export function isActiveStatus(status: MasterTaskStatus | SubTaskStatus): boolean {
  return ['running', 'submitting', 'submitted', 'analyzing'].includes(status)
}

// 检查状态是否为完成状态
export function isCompleteStatus(status: MasterTaskStatus | SubTaskStatus): boolean {
  return ['completed', 'failed', 'cancelled'].includes(status)
}