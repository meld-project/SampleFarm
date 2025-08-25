"use client"

// no local state
import { useQuery } from '@tanstack/react-query'
import { cfgAnalysisApi } from '@/lib/api'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Copy, AlertTriangle, FileText } from 'lucide-react'
import { toast } from 'sonner'
import { JsonViewer } from '@/components/json-viewer'

interface CfgAnalysisResultDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  analysisId: string | null
  sampleName?: string
}

export function CfgAnalysisResultDialog({ open, onOpenChange, analysisId, sampleName }: CfgAnalysisResultDialogProps) {
  const { data: detail, isLoading, error } = useQuery({
    queryKey: ['cfg-analysis-detail', analysisId],
    queryFn: () => cfgAnalysisApi.getAnalysisDetail(analysisId!),
    enabled: !!analysisId && open,
  })

  const handleCopy = (text: string, label: string) => {
    navigator.clipboard.writeText(text)
    toast.success(`${label}已复制到剪贴板`)
  }

  if (!open) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            CFG分析结果详情
            {sampleName && ` - ${sampleName}`}
          </DialogTitle>
          <DialogDescription>查看CFG任务的处理结果与原始报告</DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-6 pr-2">
          {isLoading ? (
            <div className="flex items-center justify-center h-64">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
              <span className="ml-3">加载分析结果中...</span>
            </div>
          ) : error ? (
            <Alert>
              <AlertTriangle className="h-4 w-4" />
              <AlertDescription>加载分析结果失败，请稍后重试。</AlertDescription>
            </Alert>
          ) : detail ? (
            <>
              {/* 概览 */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">概览</CardTitle>
                </CardHeader>
                <CardContent className="text-sm space-y-2">
                  <div className="flex flex-wrap items-center gap-3">
                    <span className="text-muted-foreground">结果ID</span>
                    <code className="bg-muted px-2 py-0.5 rounded">{detail.id}</code>
                    <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={() => handleCopy(detail.id, '结果ID')}>
                      <Copy className="h-3 w-3" />
                    </Button>
                  </div>
                  {detail.message && (
                    <div>
                      <div className="text-muted-foreground">消息</div>
                      <div className="mt-1 break-words">{detail.message}</div>
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* 结果文件（MinIO对象键） */}
              {detail.result_files && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">结果文件</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {Object.entries(detail.result_files as Record<string, string>).map(([name, key]) => (
                        <div key={name} className="flex items-center justify-between p-2 border rounded">
                          <div>
                            <div className="text-sm font-medium">{name}</div>
                            <div className="text-xs text-muted-foreground break-all">{key}</div>
                          </div>
                          <Button variant="outline" size="sm" onClick={() => handleCopy(String(key), '对象键')}>
                            <Copy className="h-3 w-3 mr-1" />复制
                          </Button>
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* 完整报告 */}
              {detail.full_report && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">完整报告</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <JsonViewer value={detail.full_report} />
                  </CardContent>
                </Card>
              )}
            </>
          ) : null}
        </div>

        <div className="flex items-center justify-end pt-4 border-t">
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}


