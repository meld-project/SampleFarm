"use client"

import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { cfgInstancesApi } from '@/lib/api'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Card, CardContent } from '@/components/ui/card'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  instanceId?: string | null
  days?: number
}

export function CfgInstanceStatsDialog({ open, onOpenChange, instanceId, days = 7 }: Props) {
  const { data, refetch, isFetching } = useQuery({
    queryKey: ['cfg-instance-stats', instanceId, days],
    queryFn: () => cfgInstancesApi.getStats(instanceId!, days),
    enabled: open && !!instanceId,
  })

  useEffect(() => {
    if (open && instanceId) {
      refetch()
    }
  }, [open, instanceId, refetch])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>实例统计（{days}天）</DialogTitle>
          <DialogDescription>展示近 {days} 天该实例的任务统计概览</DialogDescription>
        </DialogHeader>
        <div className="grid grid-cols-2 gap-3">
          <Card>
            <CardContent className="p-4">
              <div className="text-xs text-muted-foreground">总任务</div>
              <div className="text-2xl font-semibold">{data?.total_tasks ?? (isFetching ? '...' : 0)}</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="text-xs text-muted-foreground">成功</div>
              <div className="text-2xl font-semibold text-green-600">{data?.successful_tasks ?? (isFetching ? '...' : 0)}</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="text-xs text-muted-foreground">失败</div>
              <div className="text-2xl font-semibold text-red-600">{data?.failed_tasks ?? (isFetching ? '...' : 0)}</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="text-xs text-muted-foreground">成功率</div>
              <div className="text-2xl font-semibold">
                {data ? `${Math.round((data.success_rate || 0) * 100) / 100}%` : (isFetching ? '...' : '0%')}
              </div>
            </CardContent>
          </Card>
        </div>
        {data && (
          <div className="text-xs text-muted-foreground mt-2">
            统计区间：{data.period_start} ~ {data.period_end}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}


