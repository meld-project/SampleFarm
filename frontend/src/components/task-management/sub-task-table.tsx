"use client"


import { SubTaskWithSample, SubTaskStatus, PagedResult } from '@/lib/types'
import { formatBytes, formatRelativeTime } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  CheckCircle,
  XCircle,
  Clock,
  FileText,
  Copy,
  Eye,
  ChevronLeft,
  ChevronRight,
  Activity,
} from 'lucide-react'
import { toast } from 'sonner'
import { useI18n } from '@/lib/i18n'
import { TaskStatusBadge } from './task-status-badge'

interface SubTaskTableProps {
  data: PagedResult<SubTaskWithSample>
  loading: boolean
  onPageChange: (page: number) => void
  onViewAnalysisResult?: (analysisId: string, sampleName: string) => void
  onViewRuntimeSnapshot?: (subTaskId: string, sampleName: string) => void
}

const getStatusIcon = (status: SubTaskStatus) => {
  switch (status) {
    case 'completed':
      return <CheckCircle className="h-4 w-4 text-green-500" />
    case 'failed':
      return <XCircle className="h-4 w-4 text-red-500" />
    case 'analyzing':
      return <div className="h-4 w-4 rounded-full border-2 border-blue-500 border-t-transparent animate-spin" />
    default:
      return <Clock className="h-4 w-4 text-gray-400" />
  }
}

const getStatusBadge = (status: SubTaskStatus) => {
  return <TaskStatusBadge status={status} />
}



export function SubTaskTable({ 
  data, 
  loading, 
  onPageChange, 
  onViewAnalysisResult,
  onViewRuntimeSnapshot
}: SubTaskTableProps) {
  const { t } = useI18n()

  const copyToClipboard = async (text: string, label: string) => {
    try {
      // 检查剪贴板API是否可用
      if (!navigator.clipboard) {
        throw new Error(t('subTasks.copyError'))
      }
      await navigator.clipboard.writeText(text)
      toast.success(t('subTasks.copySuccess', { label }))
    } catch (error) {
      console.error('复制失败:', error)
      toast.error(t('common.error'))
    }
  }

  const handlePageChange = (newPage: number) => {
    onPageChange(newPage)
  }

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileText className="h-4 w-4" />
            {t('subTasks.title')}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
            <span className="ml-2">{t('subTasks.loading')}</span>
          </div>
        </CardContent>
      </Card>
    )
  }

  if (!data?.items?.length) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileText className="h-4 w-4" />
            {t('subTasks.title')}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8 text-muted-foreground">
            {t('subTasks.noData')}
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4" />
            {t('subTasks.title')}
          </div>
          <div className="text-sm text-muted-foreground">
            {t('subTasks.totalRecords', { total: data.total, page: data.page, totalPages: data.total_pages })}
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-12">{t('subTasks.status')}</TableHead>
                <TableHead>{t('subTasks.sampleName')}</TableHead>
                <TableHead>{t('subTasks.type')}</TableHead>
                <TableHead>{t('subTasks.instance')}</TableHead>
                <TableHead>{t('subTasks.size')}</TableHead>
                <TableHead>{t('subTasks.md5Hash')}</TableHead>
                <TableHead>{t('subTasks.labels')}</TableHead>
                <TableHead>{t('subTasks.source')}</TableHead>
                <TableHead>{t('subTasks.retryCount')}</TableHead>
                 <TableHead>{t('subTasks.externalTaskId')}</TableHead>
                  <TableHead>{t('subTasks.createdAt')}</TableHead>
                 <TableHead className="w-28">{t('subTasks.actions')}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.items.map((subTask) => (
                <TableRow key={subTask.id}>
                  <TableCell>
                    <div className="flex items-center justify-center">
                      {getStatusIcon(subTask.status)}
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="space-y-1">
                      <div className="font-medium text-sm">
                        {subTask.sample_name}
                      </div>
                      <div className="flex items-center gap-1">
                        {getStatusBadge(subTask.status)}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="text-xs">
                      {subTask.sample_type}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="space-y-1">
                      {subTask.cape_instance_name && (
                        <div className="flex items-center gap-1">
                          <Badge variant="secondary" className="text-xs">
                            CAPE: {subTask.cape_instance_name}
                          </Badge>
                        </div>
                      )}
                      {subTask.cfg_instance_name && (
                        <div className="flex items-center gap-1">
                          <Badge variant="secondary" className="text-xs">
                            CFG: {subTask.cfg_instance_name}
                          </Badge>
                        </div>
                      )}
                      {!subTask.cape_instance_name && !subTask.cfg_instance_name && (
                        <span className="text-xs text-muted-foreground">-</span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className="text-sm">{formatBytes(subTask.file_size)}</span>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <code className="text-xs bg-muted px-1 py-0.5 rounded">
                        {subTask.file_hash_md5.substring(0, 8)}...
                      </code>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 w-6 p-0"
                        onClick={() => copyToClipboard(subTask.file_hash_md5, t('subTasks.md5Hash'))}
                      >
                        <Copy className="h-3 w-3" />
                      </Button>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {subTask.labels?.map((label, index) => (
                        <Badge key={index} variant="secondary" className="text-xs">
                          {label}
                        </Badge>
                      )) || (
                        <span className="text-xs text-muted-foreground">{t('subTasks.none')}</span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className="text-sm">
                      {subTask.source || '-'}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="text-sm">
                      {subTask.retry_count}
                    </span>
                  </TableCell>
                  <TableCell>
                    {subTask.external_task_id ? (
                      <code className="text-xs bg-muted px-1 py-0.5 rounded">
                        {subTask.external_task_id}
                      </code>
                    ) : (
                      <span className="text-xs text-muted-foreground">-</span>
                    )}
                    {subTask.error_message && (
                      <div className="text-[11px] mt-1 text-red-600 break-words max-w-[220px]" title={subTask.error_message}>
                        {t('subTasks.error', { message: subTask.error_message })}
                      </div>
                    )}
                  </TableCell>
                  <TableCell>
                    <span className="text-sm">
                      {formatRelativeTime(subTask.created_at)}
                    </span>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      {onViewAnalysisResult && subTask.status === 'completed' && (
                        <Button
                          variant="outline"
                          size="sm"
                          className="h-7 px-2"
                          title={subTask.analysis_system === 'CFG' ? t('subTasks.viewCfgDetails') : t('subTasks.viewCapeDetails')}
                          onClick={() => onViewAnalysisResult(subTask.id, subTask.sample_name)}
                        >
                          <Eye className="h-3 w-3 mr-1" />
                          {t('subTasks.viewDetails')}
                        </Button>
                      )}
                      {onViewRuntimeSnapshot && (
                        (() => {
                          // 只有CAPE任务才有运行快照功能
                          if (!subTask.cape_instance_name) {
                            return null
                          }

                          // 已完成、失败、取消的任务不需要运行快照
                          if (['completed', 'failed', 'cancelled'].includes(subTask.status)) {
                            return null
                          }

                          // 只有已提交的任务才可能有快照
                          if (!subTask.external_task_id || !['submitted', 'analyzing'].includes(subTask.status)) {
                            return (
                              <span className="text-xs text-muted-foreground px-2 py-1">
                                暂无快照
                              </span>
                            )
                          }

                          // 检查是否有可用的快照数据
                          const hasSnapshot = false
                          
                          return (
                            <Button
                              variant="outline"
                              size="sm"
                              className="h-7 px-2"
                              title={hasSnapshot ? '查看CAPE运行时快照' : '暂无快照'}
                              onClick={() => hasSnapshot && onViewRuntimeSnapshot(subTask.id, subTask.sample_name)}
                              disabled={!hasSnapshot}
                            >
                              <Activity className="h-3 w-3 mr-1" />
                              {hasSnapshot ? '运行快照' : '暂无快照'}
                            </Button>
                          )
                        })()
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>

        {/* 分页控件 */}
        {data.total_pages > 1 && (
          <div className="flex items-center justify-between mt-4">
            <div className="text-sm text-muted-foreground">
              显示第 {(data.page - 1) * data.page_size + 1} - {Math.min(data.page * data.page_size, data.total)} 条，共 {data.total} 条记录
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => handlePageChange(data.page - 1)}
                disabled={data.page <= 1}
              >
                <ChevronLeft className="h-4 w-4" />
                上一页
              </Button>
              <div className="flex items-center gap-1">
                {Array.from({ length: Math.min(5, data.total_pages) }, (_, i) => {
                  const page = Math.max(1, Math.min(data.total_pages - 4, data.page - 2)) + i
                  return (
                    <Button
                      key={page}
                      variant={page === data.page ? "default" : "outline"}
                      size="sm"
                      className="h-8 w-8 p-0"
                      onClick={() => handlePageChange(page)}
                    >
                      {page}
                    </Button>
                  )
                })}
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handlePageChange(data.page + 1)}
                disabled={data.page >= data.total_pages}
              >
                下一页
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
