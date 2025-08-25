"use client"

import { useMemo, useState } from 'react'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { AlertCircle } from 'lucide-react'
import { toast } from 'sonner'
import { AnalyzerType } from '@/lib/types'
import { capeApi, cfgApi } from '@/lib/api'

interface ExecutionParamsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  analyzer: AnalyzerType
  masterTaskId: string
  defaults?: Record<string, unknown>
  onExecuted?: () => void
}

export function ExecutionParamsDialog({ open, onOpenChange, analyzer, masterTaskId, defaults, onExecuted }: ExecutionParamsDialogProps) {
  const d = useMemo(() => (defaults || {}) as Record<string, unknown>, [defaults])
  const num = (v: unknown, def: number) => (typeof v === 'number' ? v : def)

  // CAPE 单文件超时已移除
  const [submitIntervalMs, setSubmitIntervalMs] = useState<number>(num(d.cape_submit_interval_ms ?? d.cfg_submit_interval_ms, 1000))
  // 并发参数已移除，仅保留提交间隔节流
  const [pollIntervalSecs, setPollIntervalSecs] = useState<number>(num(d.cfg_poll_interval_secs, 10))
  const [maxWaitSecs, setMaxWaitSecs] = useState<number>(num(d.cape_max_wait_seconds ?? d.cfg_max_wait_secs, 1200))
  const [submitting, setSubmitting] = useState(false)
  
  // 重试配置状态
  const [retryEnabled, setRetryEnabled] = useState<boolean>(true)
  const [retryMaxAttempts, setRetryMaxAttempts] = useState<number>(3)
  const [retryInitialBackoffSecs, setRetryInitialBackoffSecs] = useState<number>(5)
  const [retryMaxBackoffSecs, setRetryMaxBackoffSecs] = useState<number>(300)
  const [retryBackoffMultiplier, setRetryBackoffMultiplier] = useState<number>(2.0)
  const [retryJitter, setRetryJitter] = useState<boolean>(true)

  const handleExecute = async () => {
    setSubmitting(true)
    try {
      if (analyzer === 'CFG') {
        await cfgApi.executeBatch({
          master_task_id: masterTaskId,
          label: 0,
          poll_interval_secs: pollIntervalSecs,
          max_wait_secs: maxWaitSecs,
          submit_interval_ms: submitIntervalMs,
          config: {
            poll_interval_secs: pollIntervalSecs,
            max_wait_secs: maxWaitSecs,
            label: 0,
            retry: retryEnabled ? {
              enabled: retryEnabled,
              max_attempts: retryMaxAttempts,
              initial_backoff_secs: retryInitialBackoffSecs,
              max_backoff_secs: retryMaxBackoffSecs,
              backoff_multiplier: retryBackoffMultiplier,
              jitter: retryJitter
            } : undefined
          }
        })
      } else {
        await capeApi.executeBatch({
          master_task_id: masterTaskId,
          submit_interval_ms: submitIntervalMs,
          config: {
            retry: retryEnabled ? {
              enabled: retryEnabled,
              max_attempts: retryMaxAttempts,
              initial_backoff_secs: retryInitialBackoffSecs,
              max_backoff_secs: retryMaxBackoffSecs,
              backoff_multiplier: retryBackoffMultiplier,
              jitter: retryJitter
            } : undefined
          }
        })
      }
      toast.success('已触发执行')
      if (onExecuted) onExecuted()
      onOpenChange(false)
    } catch (e) {
      const err = e as Error
      toast.error(`执行失败: ${err.message}`)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>执行参数</DialogTitle>
          <DialogDescription>为本次执行设置提交间隔与超时参数</DialogDescription>
        </DialogHeader>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <Label>提交间隔(ms)</Label>
            <Input type="number" value={submitIntervalMs} onChange={e => setSubmitIntervalMs(parseInt(e.target.value)||0)} />
          </div>
          {/* 并发数输入已移除 */}
          {/* CAPE 单文件超时输入已移除 */}
          {analyzer === 'CFG' && (
            <>
              <div>
                <Label>轮询间隔(秒)</Label>
                <Input type="number" value={pollIntervalSecs} onChange={e => setPollIntervalSecs(parseInt(e.target.value)||0)} />
              </div>
              <div>
                <Label>最大等待(秒)</Label>
                <Input type="number" value={maxWaitSecs} onChange={e => setMaxWaitSecs(parseInt(e.target.value)||0)} />
              </div>
            </>
          )}
        </div>

        {/* 重试配置 */}
        <div className="border rounded-lg p-4 space-y-4">
          <div className="flex items-center justify-between">
            <Label className="text-base font-medium">重试配置</Label>
            <Checkbox
              checked={retryEnabled}
              onCheckedChange={(checked) => setRetryEnabled(checked === true)}
            />
          </div>
          
          {retryEnabled && (
            <div className="space-y-4 pl-4 border-l-2 border-gray-200">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="retry-attempts">最大重试次数</Label>
                  <Input
                    id="retry-attempts"
                    type="number"
                    min="1"
                    max="10"
                    value={retryMaxAttempts}
                    onChange={e => setRetryMaxAttempts(parseInt(e.target.value) || 3)}
                  />
                </div>
                <div>
                  <Label htmlFor="initial-backoff">初始退避时间(秒)</Label>
                  <Input
                    id="initial-backoff"
                    type="number"
                    min="1"
                    max="60"
                    value={retryInitialBackoffSecs}
                    onChange={e => setRetryInitialBackoffSecs(parseInt(e.target.value) || 5)}
                  />
                </div>
              </div>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="max-backoff">最大退避时间(秒)</Label>
                  <Input
                    id="max-backoff"
                    type="number"
                    min="60"
                    max="3600"
                    value={retryMaxBackoffSecs}
                    onChange={e => setRetryMaxBackoffSecs(parseInt(e.target.value) || 300)}
                  />
                </div>
                <div>
                  <Label htmlFor="backoff-multiplier">退避倍率</Label>
                  <Input
                    id="backoff-multiplier"
                    type="number"
                    min="1"
                    max="5"
                    step="0.1"
                    value={retryBackoffMultiplier}
                    onChange={e => setRetryBackoffMultiplier(parseFloat(e.target.value) || 2.0)}
                  />
                </div>
              </div>
              
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="retry-jitter"
                  checked={retryJitter}
                  onCheckedChange={(checked) => setRetryJitter(checked === true)}
                />
                <Label htmlFor="retry-jitter" className="text-sm">启用随机抖动</Label>
              </div>
              
              <Alert>
                <AlertCircle className="h-4 w-4" />
                <AlertDescription className="text-sm">
                  重试功能可以提高任务的成功率，特别是在网络不稳定的情况下。
                </AlertDescription>
              </Alert>
            </div>
          )}
        </div>

        <div className="flex justify-end gap-2 pt-2">
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleExecute} disabled={submitting}>{submitting ? '执行中...' : '执行'}</Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}


