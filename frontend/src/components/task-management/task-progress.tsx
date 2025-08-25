"use client"

import { cn } from '@/lib/utils'
import { MasterTaskStatus } from '@/lib/types'

interface TaskProgressProps {
  progress: number // 0-100的进度百分比
  total?: number   // 总样本数
  completed?: number // 已完成样本数
  failed?: number    // 失败样本数
  status?: MasterTaskStatus
  showDetails?: boolean // 是否显示详细信息
  size?: 'sm' | 'md' | 'lg'
  className?: string
}

// 根据进度值获取颜色
function getProgressColor(progress: number, status?: MasterTaskStatus): string {
  if (status === 'failed') return 'bg-red-500'
  if (status === 'completed') return 'bg-green-500'
  if (status === 'cancelled') return 'bg-gray-400'
  
  // 根据进度动态调整颜色
  if (progress >= 80) return 'bg-green-500'
  if (progress >= 60) return 'bg-blue-500'
  if (progress >= 40) return 'bg-yellow-500'
  if (progress >= 20) return 'bg-orange-500'
  return 'bg-gray-400'
}

// 获取进度条高度
function getProgressHeight(size: 'sm' | 'md' | 'lg'): string {
  switch (size) {
    case 'sm': return 'h-1'
    case 'md': return 'h-2'
    case 'lg': return 'h-3'
    default: return 'h-2'
  }
}

export function TaskProgress({ 
  progress, 
  total, 
  completed, 
  failed, 
  status,
  showDetails = false,
  size = 'md',
  className 
}: TaskProgressProps) {
  // 确保进度值在0-100范围内
  const clampedProgress = Math.max(0, Math.min(100, progress))
  
  const progressColor = getProgressColor(clampedProgress, status)
  const heightClass = getProgressHeight(size)

  return (
    <div className={cn("w-full", className)}>
      {/* 进度条 */}
      <div className={cn(
        "relative w-full bg-gray-200 rounded-full overflow-hidden",
        heightClass
      )}>
        <div
          className={cn(
            "h-full transition-all duration-300 ease-out rounded-full",
            progressColor,
            // 添加动画效果
            status === 'running' && "animate-pulse"
          )}
          style={{ width: `${clampedProgress}%` }}
        />
        
        {/* 进度值文字 (仅在md和lg尺寸显示) */}
        {size !== 'sm' && (
          <div className="absolute inset-0 flex items-center justify-center">
            <span className={cn(
              "text-xs font-medium",
              clampedProgress > 50 ? "text-white" : "text-gray-700"
            )}>
              {clampedProgress}%
            </span>
          </div>
        )}
      </div>

      {/* 详细信息 */}
      {showDetails && (total !== undefined || completed !== undefined) && (
        <div className="mt-1 flex items-center justify-between text-xs text-gray-600">
          <div className="flex items-center gap-2">
            {completed !== undefined && total !== undefined && (
              <span>
                已完成: {completed}/{total}
              </span>
            )}
            {failed !== undefined && failed > 0 && (
              <span className="text-red-600">
                失败: {failed}
              </span>
            )}
          </div>
          
          {status && (
            <span className={cn(
              "capitalize",
              status === 'completed' && "text-green-600",
              status === 'failed' && "text-red-600",
              status === 'running' && "text-blue-600",
              status === 'pending' && "text-gray-500"
            )}>
              {getStatusLabel(status)}
            </span>
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

// 简化版本的进度条（只显示百分比）
export function SimpleProgress({ 
  progress, 
  className 
}: { 
  progress: number
  className?: string 
}) {
  return (
    <TaskProgress 
      progress={progress}
      size="sm"
      showDetails={false}
      className={className}
    />
  )
}

// 带详情的进度条
export function DetailedProgress({
  progress,
  total,
  completed,
  failed,
  status,
  className
}: {
  progress: number
  total?: number
  completed?: number
  failed?: number
  status?: MasterTaskStatus
  className?: string
}) {
  return (
    <TaskProgress
      progress={progress}
      total={total}
      completed={completed}
      failed={failed}
      status={status}
      size="lg"
      showDetails={true}
      className={className}
    />
  )
}