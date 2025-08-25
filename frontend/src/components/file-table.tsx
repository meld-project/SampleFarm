"use client"

import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Sample } from '@/lib/types'
import { formatBytes, formatRelativeTime, truncateHash } from '@/lib/utils'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Checkbox } from '@/components/ui/checkbox'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { samplesApi } from '@/lib/api'
import { 
  File, 
  Archive, 
  AlertTriangle, 
  Shield, 
  Download, 
  Trash2,
  ChevronLeft, 
  ChevronRight,
  Info
} from 'lucide-react'
import { SampleDetailDialog } from './sample-detail-dialog'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Checkbox as UICheckbox } from '@/components/ui/checkbox'
import { useI18n } from '@/lib/i18n'

interface FileTableProps {
  data: Sample[]
  total: number
  page: number
  pageSize: number
  loading?: boolean
  error?: Error | null
  onPageChange: (page: number) => void
  onPageSizeChange: (pageSize: number) => void
}

function getFileIcon(sample: Sample) {
  if (sample.is_container) {
    return <Archive className="h-4 w-4 text-blue-600" />
  }
  return <File className="h-4 w-4 text-gray-600" />
}

function SampleTypeBadge({ type }: { type: Sample['sample_type'] }) {
  const { t } = useI18n()
  
  return (
    <Badge variant={type === 'Malicious' ? 'malicious' : 'benign'}>
      {type === 'Malicious' ? (
        <>
          <AlertTriangle className="h-3 w-3 mr-1" />
          {t('fileTable.malicious')}
        </>
      ) : (
        <>
          <Shield className="h-3 w-3 mr-1" />
          {t('fileTable.benign')}
        </>
      )}
    </Badge>
  )
}

export function FileTable({ 
  data, 
  total, 
  page, 
  pageSize, 
  loading, 
  error, 
  onPageChange, 
  onPageSizeChange 
}: FileTableProps) {
  const { t } = useI18n()
  const [selectedFiles, setSelectedFiles] = useState<string[]>([])
  const [detailDialogOpen, setDetailDialogOpen] = useState(false)
  const [selectedSample, setSelectedSample] = useState<Sample | null>(null)
  const [batchDownloadOpen, setBatchDownloadOpen] = useState(false)
  const [encryptZip, setEncryptZip] = useState(false)
  const [zipPassword, setZipPassword] = useState('')
  const queryClient = useQueryClient()

  const totalPages = Math.ceil(total / pageSize)

  // 删除文件Mutation
  const deleteMutation = useMutation({
    mutationFn: async (sampleId: string) => {
      await samplesApi.delete(sampleId)
    },
    onSuccess: () => {
      toast.success(t('fileTable.deleteSuccess'))
      queryClient.invalidateQueries({ queryKey: ['samples'] })
      queryClient.invalidateQueries({ queryKey: ['samples-stats'] })
      setSelectedFiles([]) // 清除选择
    },
    onError: (error) => {
      toast.error(t('fileTable.deleteError', { message: error.message }))
    }
  })

  // 批量删除Mutation
  const batchDeleteMutation = useMutation({
    mutationFn: async (sampleIds: string[]) => {
      return samplesApi.deleteBatch(sampleIds)
    },
    onSuccess: (res) => {
      const deleted = (res?.deleted || []).length
      const failed = (res?.failed || []).length
      if (failed === 0) {
        toast.success(t('fileTable.batchDeleteSuccess', { count: deleted }))
      } else {
        toast.warning(t('fileTable.batchDeletePartial', { success: deleted, failed }))
      }
      queryClient.invalidateQueries({ queryKey: ['samples'] })
      queryClient.invalidateQueries({ queryKey: ['samples-stats'] })
      setSelectedFiles([])
    },
    onError: (error) => {
      toast.error(t('fileTable.batchDeleteError', { message: error.message }))
    }
  })

  const handleSelectAll = (checked: boolean) => {
    setSelectedFiles(checked ? data.map(item => item.id) : [])
  }

  const handleSelectFile = (fileId: string, checked: boolean) => {
    setSelectedFiles(prev => 
      checked 
        ? [...prev, fileId]
        : prev.filter(id => id !== fileId)
    )
  }

  // 下载文件
  const handleDownload = async (sample: Sample) => {
    try {
      const blob = await samplesApi.download(sample.id)
      const url = window.URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = sample.file_name
      document.body.appendChild(a)
      a.click()
      window.URL.revokeObjectURL(url)
      document.body.removeChild(a)
      toast.success(t('fileTable.downloadStart', { filename: sample.file_name }))
    } catch (error) {
      toast.error(t('fileTable.downloadError', { message: (error as Error).message }))
    }
  }

  // 批量下载
  const handleBatchDownload = async () => {
    setBatchDownloadOpen(true)
  }

  // 删除单个文件
  const handleDelete = async (sampleId: string) => {
    if (confirm(t('fileTable.confirmDelete'))) {
      deleteMutation.mutate(sampleId)
    }
  }

  // 批量删除
  const handleBatchDelete = async () => {
    if (confirm(t('fileTable.confirmBatchDelete', { count: selectedFiles.length }))) {
      batchDeleteMutation.mutate(selectedFiles)
    }
  }

  // 显示详情
  const handleShowDetail = (sample: Sample) => {
    setSelectedSample(sample)
    setDetailDialogOpen(true)
  }

  if (loading) {
    return (
      <div className="space-y-4">
        <div className="h-10 bg-muted animate-pulse rounded" />
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="h-16 bg-muted animate-pulse rounded" />
        ))}
      </div>
    )
  }

  if (error) {
    return (
      <div className="text-center py-8">
        <p className="text-red-500">{t('fileTable.loadError', { message: error.message })}</p>
        <Button variant="outline" className="mt-2" onClick={() => window.location.reload()}>
          {t('fileTable.retry')}
        </Button>
      </div>
    )
  }

  if (!data.length) {
    return (
      <div className="text-center py-8">
        <File className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <p className="text-muted-foreground">{t('fileTable.noData')}</p>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* 批量操作栏 */}
      {selectedFiles.length > 0 && (
        <div className="flex items-center gap-2 p-4 bg-muted/50 rounded-lg">
          <span className="text-sm text-muted-foreground">
            {t('fileTable.selected', { count: selectedFiles.length })}
          </span>
          <Button size="sm" variant="outline" onClick={handleBatchDownload} disabled={selectedFiles.length === 0}>
            <Download className="h-4 w-4 mr-2" />
            {t('fileTable.batchDownload')}
          </Button>
          <Button 
            size="sm" 
            variant="destructive"
            onClick={handleBatchDelete}
            disabled={selectedFiles.length === 0 || batchDeleteMutation.isPending}
          >
            <Trash2 className="h-4 w-4 mr-2" />
            {t('fileTable.batchDelete')}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setSelectedFiles([])}>
            {t('fileTable.cancelSelection')}
          </Button>
        </div>
      )}

      {/* 文件表格 */}
      <div className="border rounded-lg">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-12">
                <Checkbox 
                  checked={selectedFiles.length === data.length && data.length > 0}
                  onCheckedChange={handleSelectAll}
                />
              </TableHead>
              <TableHead>{t('fileTable.fileInfo')}</TableHead>
              <TableHead>{t('fileTable.type')}</TableHead>
              <TableHead>{t('fileTable.size')}</TableHead>
              <TableHead>{t('fileTable.labels')}</TableHead>
              <TableHead>{t('fileTable.hash')}</TableHead>
              <TableHead>{t('fileTable.createdAt')}</TableHead>
              <TableHead className="w-32">{t('fileTable.actions')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.map((sample) => (
              <TableRow key={sample.id}>
                <TableCell>
                  <Checkbox 
                    checked={selectedFiles.includes(sample.id)}
                    onCheckedChange={(checked) => handleSelectFile(sample.id, checked as boolean)}
                  />
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-3">
                    {getFileIcon(sample)}
                    <div className="min-w-0 flex-1">
                      <p className="font-medium truncate">{sample.file_name}</p>
                      {sample.file_path_in_zip && (
                        <p className="text-xs text-muted-foreground truncate">
                          📍 {sample.file_path_in_zip}
                        </p>
                      )}
                      {sample.source && (
                        <p className="text-xs text-muted-foreground truncate">
                          {t('fileTable.source')}: {sample.source}
                        </p>
                      )}
                    </div>
                  </div>
                </TableCell>
                <TableCell>
                  <div className="space-y-1">
                    <SampleTypeBadge type={sample.sample_type} />
                    {sample.is_container && (
                      <Badge variant="container" className="block w-fit">
                        <Archive className="h-3 w-3 mr-1" />
                        {t('fileTable.container')}
                      </Badge>
                    )}
                  </div>
                </TableCell>
                <TableCell>
                  <div className="text-sm">
                    {formatBytes(sample.file_size)}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {sample.file_type}
                  </div>
                </TableCell>
                <TableCell>
                  <div className="flex flex-wrap gap-1 max-w-32">
                    {sample.labels && sample.labels.length > 0 ? (
                      sample.labels.slice(0, 3).map((label, index) => (
                        <Badge key={index} variant="secondary" className="text-xs">
                          {label}
                        </Badge>
                      ))
                    ) : (
                      <span className="text-xs text-muted-foreground">{t('fileTable.noLabels')}</span>
                    )}
                    {sample.labels && sample.labels.length > 3 && (
                      <Badge variant="secondary" className="text-xs">
                        +{sample.labels.length - 3}
                      </Badge>
                    )}
                  </div>
                </TableCell>
                <TableCell>
                  <div className="space-y-1 text-xs font-mono">
                    <div title={`MD5: ${sample.file_hash_md5}`}>
                      MD5: {truncateHash(sample.file_hash_md5)}
                    </div>
                    <div title={`SHA256: ${sample.file_hash_sha256}`}>
                      SHA256: {truncateHash(sample.file_hash_sha256)}
                    </div>
                  </div>
                </TableCell>
                <TableCell>
                  <div className="text-sm">
                    {formatRelativeTime(sample.created_at)}
                  </div>
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-1">
                    <Button 
                      size="sm" 
                      variant="ghost"
                      onClick={() => handleShowDetail(sample)}
                      title={t('fileTable.viewDetails')}
                    >
                      <Info className="h-4 w-4" />
                    </Button>
                    <Button 
                      size="sm" 
                      variant="ghost"
                      onClick={() => handleDownload(sample)}
                      title={t('fileTable.downloadFile')}
                    >
                      <Download className="h-4 w-4" />
                    </Button>
                    <Button 
                      size="sm" 
                      variant="ghost"
                      onClick={() => handleDelete(sample.id)}
                      disabled={deleteMutation.isPending}
                      title={t('fileTable.deleteFile')}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* 分页导航 */}
      <div className="flex items-center justify-between">
        <div className="text-sm text-muted-foreground">
          {t('fileTable.pagination', { start: (page - 1) * pageSize + 1, end: Math.min(page * pageSize, total), total })}
        </div>
        <div className="flex items-center gap-4">
          {/* 每页大小控制 */}
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">{t('fileTable.pageSize')}</span>
            {[10, 20, 50, 100].map((size) => (
              <Button
                key={size}
                variant={pageSize === size ? "default" : "outline"}
                size="sm"
                onClick={() => onPageSizeChange(size)}
                className="w-12 h-8 p-0"
              >
                {size}
              </Button>
            ))}
          </div>

          {/* 分页控制 */}
          <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(page - 1)}
            disabled={page <= 1}
          >
            <ChevronLeft className="h-4 w-4" />
            {t('fileTable.previous')}
          </Button>
          
          {/* 页码显示 */}
          <div className="flex items-center gap-1">
            {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
              const pageNum = Math.max(1, Math.min(totalPages - 4, page - 2)) + i
              return (
                <Button
                  key={pageNum}
                  variant={pageNum === page ? "default" : "outline"}
                  size="sm"
                  onClick={() => onPageChange(pageNum)}
                >
                  {pageNum}
                </Button>
              )
            })}
          </div>
          
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(page + 1)}
            disabled={page >= totalPages}
          >
            {t('fileTable.next')}
            <ChevronRight className="h-4 w-4" />
          </Button>
          </div>
        </div>
      </div>

      {/* 详情弹窗 */}
      <SampleDetailDialog 
        sample={selectedSample}
        open={detailDialogOpen}
        onOpenChange={setDetailDialogOpen}
      />

      {/* 批量下载配置弹窗 */}
      <Dialog open={batchDownloadOpen} onOpenChange={setBatchDownloadOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('fileTable.batchDownloadTitle')}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <UICheckbox id="encryptZip" checked={encryptZip} onCheckedChange={(v) => setEncryptZip(!!v)} />
              <label htmlFor="encryptZip" className="text-sm">{t('fileTable.encryptZip')}</label>
            </div>
            {encryptZip && (
              <div>
                <label className="text-sm text-muted-foreground">{t('fileTable.password')}</label>
                <Input type="password" value={zipPassword} onChange={(e) => setZipPassword(e.target.value)} placeholder={t('fileTable.passwordPlaceholder')} />
              </div>
            )}
            <div className="text-xs text-muted-foreground">{t('fileTable.downloadInfo', { count: selectedFiles.length })}</div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setBatchDownloadOpen(false)}>{t('fileTable.cancel')}</Button>
            <Button onClick={async () => {
              try {
                const blob = await samplesApi.downloadBatch(selectedFiles, encryptZip, zipPassword || undefined)
                const url = window.URL.createObjectURL(blob)
                const a = document.createElement('a')
                a.href = url
                a.download = encryptZip ? 'samples_batch_encrypted.zip' : 'samples_batch.zip'
                a.click()
                window.URL.revokeObjectURL(url)
                setBatchDownloadOpen(false)
                                    } catch {
                toast.error(t('fileTable.batchDownloadError'))
              }
            }}>{t('fileTable.startDownload')}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}