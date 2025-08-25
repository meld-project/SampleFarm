'use client'

import { useState } from 'react'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { CapeStatusBadge } from './cape-status-badge'
import { 
  MoreHorizontal, 
  Edit, 
  Trash2, 
  Activity, 
  BarChart3,
  Play,
  Pause,
  RefreshCw
} from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { CapeInstance, PagedResult } from '@/lib/types'
import { useI18n } from '@/lib/i18n'
import { formatDistanceToNow } from 'date-fns'
import { zhCN } from 'date-fns/locale'

interface CapeInstancesTableProps {
  data?: PagedResult<CapeInstance>
  loading?: boolean
  onEdit?: (instance: CapeInstance) => void
  onDelete?: (instance: CapeInstance) => void
  onHealthCheck?: (instance: CapeInstance) => void
  onViewStats?: (instance: CapeInstance) => void
  onToggleEnabled?: (instance: CapeInstance, enabled: boolean) => void
  onPageChange?: (page: number) => void
}

export function CapeInstancesTable({
  data,
  loading = false,
  onEdit,
  onDelete,
  onHealthCheck,
  onViewStats,
  onToggleEnabled,
  onPageChange
}: CapeInstancesTableProps) {
  const { t } = useI18n()
  const [processingId, setProcessingId] = useState<string | null>(null)

  const handleHealthCheck = async (instance: CapeInstance) => {
    if (onHealthCheck) {
      setProcessingId(instance.id)
      try {
        await onHealthCheck(instance)
      } finally {
        setProcessingId(null)
      }
    }
  }

  const formatTimeAgo = (dateString: string) => {
    return formatDistanceToNow(new Date(dateString), { 
      addSuffix: true, 
      locale: zhCN 
    })
  }

  const formatResponseTime = (ms?: number) => {
    if (!ms) return '--'
    return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center space-x-2 text-muted-foreground">
          <RefreshCw className="h-4 w-4 animate-spin" />
          <span>{t('capeTable.loading')}</span>
        </div>
      </div>
    )
  }

  if (!data?.items.length) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
        <Activity className="h-12 w-12 mb-4" />
        <p className="text-lg font-medium">{t('capeTable.noInstances')}</p>
        <p className="text-sm">{t('capeTable.createFirst')}</p>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{t('capeTable.name')}</TableHead>
            <TableHead>{t('capeTable.status')}</TableHead>
            <TableHead>{t('capeTable.address')}</TableHead>
            <TableHead>{t('capeTable.concurrency')}</TableHead>
            <TableHead>{t('capeTable.timeout')}</TableHead>
            <TableHead>{t('capeTable.responseTime')}</TableHead>
            <TableHead>最后检查</TableHead>
            <TableHead>启用状态</TableHead>
            <TableHead className="text-right">操作</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {data.items.map((instance) => (
            <TableRow key={instance.id}>
              <TableCell>
                <div>
                  <div className="font-medium">{instance.name}</div>
                  {instance.description && (
                    <div className="text-sm text-muted-foreground">
                      {instance.description}
                    </div>
                  )}
                </div>
              </TableCell>
              <TableCell>
                <CapeStatusBadge status={instance.status} />
              </TableCell>
              <TableCell>
                <code className="text-xs bg-muted px-2 py-1 rounded">
                  {instance.base_url}
                </code>
              </TableCell>
              <TableCell>{instance.max_concurrent_tasks}</TableCell>
              <TableCell>{instance.timeout_seconds}s</TableCell>
              <TableCell>
                {formatResponseTime(
                  // 这里可以从health status获取响应时间
                  instance.status === 'healthy' ? 100 : undefined
                )}
              </TableCell>
              <TableCell>
                {instance.last_health_check 
                  ? formatTimeAgo(instance.last_health_check)
                  : '--'
                }
              </TableCell>
              <TableCell>
                <Badge variant={instance.enabled ? 'default' : 'secondary'}>
                  {instance.enabled ? '启用' : '禁用'}
                </Badge>
              </TableCell>
              <TableCell className="text-right">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" className="h-8 w-8 p-0">
                      <MoreHorizontal className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      onClick={() => handleHealthCheck(instance)}
                      disabled={processingId === instance.id}
                    >
                      <Activity className="mr-2 h-4 w-4" />
                      {processingId === instance.id ? (
                        <>
                          <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                          检查中...
                        </>
                      ) : (
                        '健康检查'
                      )}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => onViewStats?.(instance)}
                    >
                      <BarChart3 className="mr-2 h-4 w-4" />
                      查看统计
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={() => onToggleEnabled?.(instance, !instance.enabled)}
                    >
                      {instance.enabled ? (
                        <>
                          <Pause className="mr-2 h-4 w-4" />
                          禁用
                        </>
                      ) : (
                        <>
                          <Play className="mr-2 h-4 w-4" />
                          启用
                        </>
                      )}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => onEdit?.(instance)}
                    >
                      <Edit className="mr-2 h-4 w-4" />
                      编辑
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={() => onDelete?.(instance)}
                      className="text-destructive"
                    >
                      <Trash2 className="mr-2 h-4 w-4" />
                      删除
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      {/* 分页控件 */}
      {data && data.total_pages > 1 && (
        <div className="flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            共 {data.total} 个实例，第 {data.page} 页 / 共 {data.total_pages} 页
          </div>
          <div className="flex items-center space-x-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPageChange?.(data.page - 1)}
              disabled={data.page <= 1}
            >
              上一页
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPageChange?.(data.page + 1)}
              disabled={data.page >= data.total_pages}
            >
              下一页
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
