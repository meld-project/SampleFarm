"use client"

import React, { useState } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"

import { Loader2, Activity, Clock, RefreshCw } from "lucide-react"
import { JsonViewer } from "@/components/json-viewer"
import { formatRelativeTime } from "@/lib/utils"
import { analysisApi } from "@/lib/api"
import { toast } from "sonner"
import { useQuery } from "@tanstack/react-query"

interface CapeRuntimeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  subTaskId: string
  sampleName: string
}

export function CapeRuntimeDialog({
  open,
  onOpenChange,
  subTaskId,
  sampleName,
}: CapeRuntimeDialogProps) {
  const [isRefreshing, setIsRefreshing] = useState(false)

  // 查询运行时快照
  const { data: snapshot, isLoading, error, refetch } = useQuery({
    queryKey: ['cape-runtime-snapshot', subTaskId],
    queryFn: () => analysisApi.getCapeRuntimeSnapshot(subTaskId),
    enabled: open,
    refetchInterval: 30000, // 30秒自动刷新
    retry: 1,
  })

  const handleRefresh = async () => {
    setIsRefreshing(true)
    try {
      await refetch()
      toast.success('快照已刷新')
    } catch {
      toast.error('刷新失败')
    } finally {
      setIsRefreshing(false)
    }
  }

  const getStatusBadge = (status: string) => {
    const statusConfig = {
      pending: { variant: 'outline' as const, label: '等待中' },
      starting: { variant: 'outline' as const, label: '启动中' }, 
      running: { variant: 'default' as const, label: '运行中' },
      processing: { variant: 'default' as const, label: '处理中' },
      reporting: { variant: 'secondary' as const, label: '生成报告' },
      completed: { variant: 'default' as const, label: '已完成' },
      reported: { variant: 'default' as const, label: '已报告' },
      failed: { variant: 'destructive' as const, label: '失败' },
      failed_analysis: { variant: 'destructive' as const, label: '分析失败' },
    }

    const config = statusConfig[status as keyof typeof statusConfig] || {
      variant: 'outline' as const,
      label: status
    }

    return (
      <Badge variant={config.variant}>
        {config.label}
      </Badge>
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[80vh] overflow-hidden">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Activity className="h-4 w-4" />
            CAPE运行时快照 - {sampleName}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin mr-2" />
              <span>正在加载快照...</span>
            </div>
          )}

          {error && (
            <Card>
              <CardContent className="p-4">
                <div className="text-center text-muted-foreground">
                  <Activity className="h-8 w-8 mx-auto mb-2 opacity-50" />
                  <p className="text-sm">
                    {error.message || '该任务暂无运行时快照'}
                  </p>
                  <p className="text-xs text-muted-foreground mt-1">
                    快照数据需要任务提交到CAPE系统后，通过后台同步器定期获取。请稍后刷新或检查任务是否已成功提交。
                  </p>
                  <Button
                    variant="outline"
                    size="sm"
                    className="mt-3"
                    onClick={handleRefresh}
                    disabled={isRefreshing}
                  >
                    {isRefreshing ? (
                      <Loader2 className="h-3 w-3 animate-spin mr-1" />
                    ) : (
                      <RefreshCw className="h-3 w-3 mr-1" />
                    )}
                    刷新
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          {snapshot && (
            <>
              {/* 快照概览 */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Clock className="h-4 w-4" />
                      快照状态
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleRefresh}
                      disabled={isRefreshing}
                    >
                      {isRefreshing ? (
                        <Loader2 className="h-3 w-3 animate-spin mr-1" />
                      ) : (
                        <RefreshCw className="h-3 w-3 mr-1" />
                      )}
                      刷新
                    </Button>
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-muted-foreground">状态:</span>
                    {getStatusBadge(snapshot.status)}
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-muted-foreground">更新时间:</span>
                    <span className="text-sm">
                      {formatRelativeTime(snapshot.updated_at)}
                    </span>
                  </div>
                  <div className="text-xs text-muted-foreground mt-2">
                    ℹ️ 快照每30秒自动刷新，显示CAPE系统中的最新状态信息
                  </div>
                </CardContent>
              </Card>

              {/* 快照详细内容 */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm">详细信息</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="max-h-[55vh] overflow-auto rounded-md border">
                    <JsonViewer value={snapshot.snapshot} collapsed={false} />
                  </div>
                </CardContent>
              </Card>
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
