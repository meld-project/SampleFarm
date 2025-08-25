"use client"

import { TaskStatusCounts } from '@/lib/types'
import { Badge } from '@/components/ui/badge'

interface TaskStatusCountsProps {
  counts: TaskStatusCounts
  total: number
  className?: string
}

export function TaskStatusCountsDisplay({ counts, total, className }: TaskStatusCountsProps) {
  // 合并相近的状态以简化显示
  const processing = counts.submitting + counts.submitted + counts.analyzing


  return (
    <div className={`flex items-center gap-1 flex-wrap ${className}`}>
      {counts.pending > 0 && (
        <Badge variant="secondary" className="text-xs">
          等待: {counts.pending}
        </Badge>
      )}
      {processing > 0 && (
        <Badge variant="default" className="text-xs bg-blue-100 text-blue-800 hover:bg-blue-200">
          处理中: {processing}
        </Badge>
      )}
      {counts.paused > 0 && (
        <Badge variant="default" className="text-xs bg-yellow-100 text-yellow-800 hover:bg-yellow-200">
          暂停: {counts.paused}
        </Badge>
      )}
      {counts.completed > 0 && (
        <Badge variant="default" className="text-xs bg-green-100 text-green-800 hover:bg-green-200">
          完成: {counts.completed}
        </Badge>
      )}
      {(counts.failed + counts.cancelled) > 0 && (
        <Badge variant="destructive" className="text-xs">
          失败: {counts.failed + counts.cancelled}
        </Badge>
      )}
      <Badge variant="outline" className="text-xs">
        总计: {total}
      </Badge>
    </div>
  )
}
