"use client"

import { useState } from 'react'
import { useDropzone } from 'react-dropzone'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { UploadMetadata } from '@/lib/types'
import { formatBytes } from '@/lib/utils'
import { samplesApi } from '@/lib/api'
import { useI18n } from '@/lib/i18n'
import { 
  Upload, 
  File, 
  X, 
  CheckCircle, 
  AlertCircle,
  Loader2
} from 'lucide-react'

interface FileUploadDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

interface UploadFile {
  file: File
  status: 'pending' | 'uploading' | 'success' | 'error'
  progress: number
  error?: string
}

export function FileUploadDialog({ open, onOpenChange }: FileUploadDialogProps) {
  const { t } = useI18n()
  const [uploadFiles, setUploadFiles] = useState<UploadFile[]>([])
  const [metadata, setMetadata] = useState<UploadMetadata>({
    sample_type: 'Malicious',
    labels: [],
    source: '',
    passwords: []
  })

  const queryClient = useQueryClient()

  // 上传Mutation
  const uploadMutation = useMutation({
    mutationFn: async (uploadFile: UploadFile) => {
      setUploadFiles(prev => prev.map(f => 
        f === uploadFile 
          ? { ...f, status: 'uploading' as const, progress: 0 }
          : f
      ))

      try {
        const result = await samplesApi.upload(uploadFile.file, metadata, (progress) => {
          setUploadFiles(prev => prev.map(f => 
            f === uploadFile 
              ? { ...f, progress }
              : f
          ))
        })

        setUploadFiles(prev => prev.map(f => 
          f === uploadFile 
            ? { ...f, status: 'success' as const, progress: 100 }
            : f
        ))

        return result
      } catch (error) {
        setUploadFiles(prev => prev.map(f => 
          f === uploadFile 
            ? { ...f, status: 'error' as const, error: (error as Error).message }
            : f
        ))
        throw error
      }
    },
    onSuccess: (result) => {
      toast.success(t('upload.success', { filename: result.filename }))
      
      // 刷新样本列表和统计信息
      queryClient.invalidateQueries({ queryKey: ['samples'] })
      queryClient.invalidateQueries({ queryKey: ['samples-stats'] })
    },
    onError: (error) => {
      toast.error(t('upload.failed', { error: error.message }))
    }
  })

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop: (acceptedFiles, rejectedFiles) => {
      // 处理被拒绝的文件
      rejectedFiles.forEach(rejection => {
        const { file, errors } = rejection
        errors.forEach(error => {
          if (error.code === 'file-too-large') {
            toast.error(t('upload.fileTooLarge', { filename: file.name, size: (file.size / (1024 * 1024)).toFixed(1) }))
          } else {
            toast.error(t('upload.fileRejected', { filename: file.name, error: error.message }))
          }
        })
      })

      // 处理接受的文件
      const newFiles = acceptedFiles.map(file => ({
        file,
        status: 'pending' as const,
        progress: 0
      }))
      setUploadFiles(prev => [...prev, ...newFiles])
    },
    maxSize: 1024 * 1024 * 1024, // 1GB
    multiple: true
  })

  const removeFile = (index: number) => {
    setUploadFiles(prev => prev.filter((_, i) => i !== index))
  }

  const handleUpload = async () => {
    const pendingFiles = uploadFiles.filter(f => f.status === 'pending')
    
    if (pendingFiles.length === 0) {
      toast.warning(t('upload.noFiles'))
      return
    }

    // 逐个上传文件
    for (const uploadFile of pendingFiles) {
      try {
        await uploadMutation.mutateAsync(uploadFile)
      } catch (error) {
        // 错误已在mutation中处理
        console.error('Upload failed:', error)
      }
    }

    // 检查是否所有文件都上传成功
    const allSuccess = uploadFiles.every(f => f.status === 'success' || f.status === 'pending')
    if (allSuccess && !uploadMutation.isPending) {
      setTimeout(() => {
        onOpenChange(false)
        clearAll()
      }, 2000) // 2秒后自动关闭对话框
    }
  }

  const clearAll = () => {
    setUploadFiles([])
    setMetadata({
      sample_type: 'Malicious',
      labels: [],
      source: '',
      passwords: []
    })
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-background rounded-lg shadow-lg w-full max-w-4xl max-h-[95vh] overflow-hidden">
        {/* 头部 */}
        <div className="border-b p-6">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">{t('upload.title')}</h2>
            <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* 内容 */}
        <div className="p-8 space-y-8 overflow-y-auto max-h-[calc(95vh-140px)]">
          {/* 拖拽上传区域 */}
          <div
            {...getRootProps()}
            className={`
              border-2 border-dashed rounded-lg p-8 text-center cursor-pointer transition-colors
              ${isDragActive ? 'border-primary bg-primary/5' : 'border-muted-foreground/25'}
            `}
          >
            <input {...getInputProps()} />
            <Upload className="h-8 w-8 mx-auto mb-4 text-muted-foreground" />
            <p className="text-sm text-muted-foreground mb-2">
              {isDragActive ? t('upload.dragActive') : t('upload.dragDrop')}
            </p>
            <p className="text-xs text-muted-foreground">
              {t('upload.supportedFormats')}
            </p>
          </div>

          {/* 已选择的文件 */}
          {uploadFiles.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h3 className="font-medium">{t('upload.selectedFiles', { count: uploadFiles.length })}</h3>
                <Button variant="ghost" size="sm" onClick={clearAll}>
                  {t('upload.clear')}
                </Button>
              </div>
              
              <div className="space-y-2 max-h-48 overflow-y-auto">
                {uploadFiles.map((uploadFile, index) => (
                  <div key={index} className="flex items-center gap-3 p-3 border rounded-lg">
                    <File className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">{uploadFile.file.name}</p>
                      <p className="text-xs text-muted-foreground">
                        {formatBytes(uploadFile.file.size)}
                      </p>
                    </div>
                    
                    {/* 状态指示器 */}
                    <div className="flex items-center gap-2">
                      {uploadFile.status === 'pending' && (
                        <Badge variant="outline">{t('upload.pending')}</Badge>
                      )}
                      {uploadFile.status === 'uploading' && (
                        <>
                          <Loader2 className="h-4 w-4 animate-spin" />
                          <span className="text-xs">{uploadFile.progress}%</span>
                        </>
                      )}
                      {uploadFile.status === 'success' && (
                        <CheckCircle className="h-4 w-4 text-green-600" />
                      )}
                      {uploadFile.status === 'error' && (
                        <div title={uploadFile.error}>
                          <AlertCircle className="h-4 w-4 text-red-600" />
                        </div>
                      )}
                      
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => removeFile(index)}
                        disabled={uploadFile.status === 'uploading'}
                      >
                        <X className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* 元数据表单 */}
          <div className="space-y-6">
            <h3 className="font-medium">{t('upload.fileInfo')}</h3>
            
            {/* 样本类型 */}
            <div className="space-y-3">
              <label className="text-sm font-medium">{t('upload.sampleTypeRequired')}</label>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant={metadata.sample_type === 'Malicious' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setMetadata(prev => ({ ...prev, sample_type: 'Malicious' }))}
                >
                  {t('upload.malicious')}
                </Button>
                <Button
                  type="button"
                  variant={metadata.sample_type === 'Benign' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setMetadata(prev => ({ ...prev, sample_type: 'Benign' }))}
                >
                  {t('upload.benign')}
                </Button>
              </div>
            </div>

            {/* 来源 */}
            <div className="space-y-3">
              <label className="text-sm font-medium">{t('upload.sourceLabel')}</label>
              <Input
                placeholder={t('upload.sourcePlaceholder')}
                value={metadata.source || ''}
                onChange={(e) => setMetadata(prev => ({ ...prev, source: e.target.value }))}
              />
            </div>

            {/* 标签 */}
            <div className="space-y-3">
              <label className="text-sm font-medium">{t('upload.labelsLabel')}</label>
              <Input
                placeholder={t('upload.labelsPlaceholder')}
                value={metadata.labels?.join(', ') || ''}
                onChange={(e) => setMetadata(prev => ({ 
                  ...prev, 
                  labels: e.target.value.split(',').map(s => s.trim()).filter(Boolean)
                }))}
              />
            </div>

            {/* ZIP密码 */}
            <div className="space-y-3">
              <label className="text-sm font-medium">{t('upload.zipPassword')}</label>
              <Input
                type="password"
                placeholder={t('upload.zipPasswordPlaceholder')}
                value={metadata.passwords?.[0] || ''}
                onChange={(e) => setMetadata(prev => ({ 
                  ...prev, 
                  passwords: e.target.value ? [e.target.value] : []
                }))}
              />
            </div>
          </div>
        </div>

        {/* 底部操作栏 */}
        <div className="border-t p-8">
          <div className="flex items-center justify-end gap-3">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              {t('upload.cancel')}
            </Button>
            <Button 
              onClick={handleUpload}
              disabled={uploadFiles.length === 0 || uploadMutation.isPending}
            >
              {uploadMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  {t('upload.uploading')}
                </>
              ) : (
                t('upload.uploadCount', { count: uploadFiles.length })
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}