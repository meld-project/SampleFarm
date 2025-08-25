"use client"


import { useQuery } from '@tanstack/react-query'

import { analysisApi } from '@/lib/api'
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
import {
  Shield,
  AlertTriangle,
  FileText,
  Settings,
  Download
} from 'lucide-react'
import { formatRelativeTime } from '@/lib/utils'
import { toast } from 'sonner'
import { VirtualJsonViewer } from '@/components/virtual-json-viewer'

interface AnalysisResultDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  analysisId: string | null
  sampleName?: string
}

export function AnalysisResultDialog({ 
  open, 
  onOpenChange, 
  analysisId,
  sampleName 
}: AnalysisResultDialogProps) {

  // 获取分析结果详情
  const { data: analysis, isLoading, error } = useQuery({
    queryKey: ['cape-analysis-detail', analysisId],
    queryFn: () => analysisApi.getCapeAnalysisDetail(analysisId!),
    enabled: !!analysisId && open,
  })



  const handleDownloadReport = () => {
    if (analysis?.full_report) {
      const blob = new Blob([JSON.stringify(analysis.full_report, null, 2)], {
        type: 'application/json'
      })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `cape_analysis_${analysis.cape_task_id}.json`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      toast.success('分析报告下载成功')
    }
  }



  const renderJsonReportTab = () => {
    if (!analysis?.full_report) return (
      <div className="text-center py-8 text-muted-foreground">
        <FileText className="h-12 w-12 mx-auto mb-4 opacity-50" />
        <p>没有找到完整的JSON报告</p>
      </div>
    )

    // 计算JSON大小
    const reportText = JSON.stringify(analysis.full_report)
    const sizeInBytes = new Blob([reportText]).size
    const sizeInKB = (sizeInBytes / 1024).toFixed(2)
    const sizeInMB = (sizeInBytes / (1024 * 1024)).toFixed(2)
    const sizeDisplay = sizeInBytes > 1024 * 1024 ? `${sizeInMB} MB` : `${sizeInKB} KB`

    return (
      <div className="space-y-4">
        {/* JSON报告信息 */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <FileText className="h-4 w-4" />
              报告信息
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-4 text-sm">
              <div>
                <div className="text-muted-foreground">报告大小</div>
                <div className="font-medium">{sizeDisplay}</div>
              </div>
              <div>
                <div className="text-muted-foreground">字节数</div>
                <div className="font-medium">{sizeInBytes.toLocaleString()}</div>
              </div>
              <div>
                <div className="text-muted-foreground">任务ID</div>
                <div className="font-medium">#{analysis.cape_task_id}</div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* JSON查看器 */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Settings className="h-4 w-4" />
              完整JSON报告
              </CardTitle>
            </CardHeader>
            <CardContent>
            <VirtualJsonViewer 
              value={analysis.full_report} 
              collapsed={true} 
              searchable={true}
            />
            </CardContent>
          </Card>
      </div>
    )
  }

  if (!open) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            CAPE分析结果详情
            {sampleName && ` - ${sampleName}`}
          </DialogTitle>
          <DialogDescription>
            {analysis ? '查看详细的CAPE沙箱分析结果' : '查看详细的CAPE沙箱分析结果'}
          </DialogDescription>
        </DialogHeader>

        {analysis && (
          <div className="flex items-center gap-4 text-sm px-6 -mt-2">
            <span>任务ID: #{analysis.cape_task_id}</span>
            {analysis.analysis_started_at && (
              <>
                <span>•</span>
                <span>分析时间: {formatRelativeTime(analysis.analysis_started_at)}</span>
              </>
            )}
          </div>
        )}

        <div className="flex-1 overflow-hidden">
          {isLoading ? (
            <div className="flex items-center justify-center h-64">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
              <span className="ml-3">加载分析结果中...</span>
            </div>
          ) : error ? (
            <Alert>
              <AlertTriangle className="h-4 w-4" />
              <AlertDescription>
                加载分析结果失败，请稍后重试。
              </AlertDescription>
            </Alert>
          ) : analysis ? (
            <div className="h-full overflow-y-auto">
              {renderJsonReportTab()}
              </div>
          ) : null}
        </div>

        <div className="flex items-center justify-between pt-4 border-t">
          <div className="text-sm text-muted-foreground">
            {analysis && (
              <span>更新时间: {formatRelativeTime(analysis.updated_at)}</span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {analysis?.full_report && (
              <Button variant="outline" onClick={handleDownloadReport}>
                <Download className="h-4 w-4 mr-2" />
                下载完整报告
              </Button>
            )}
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              关闭
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}