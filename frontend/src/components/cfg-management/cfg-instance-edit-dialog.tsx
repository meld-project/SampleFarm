"use client"

import { useEffect, useState } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { cfgInstancesApi } from '@/lib/api'
import { CfgInstance, UpdateCfgInstanceRequest } from '@/lib/types'
import { toast } from 'sonner'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  instance: CfgInstance | null
  onSaved?: () => void
}

export function CfgInstanceEditDialog({ open, onOpenChange, instance, onSaved }: Props) {
  const [form, setForm] = useState<UpdateCfgInstanceRequest>({})
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (instance) {
      setForm({
        name: instance.name,
        base_url: instance.base_url,
        description: instance.description,
        enabled: instance.enabled,
        health_check_interval: instance.health_check_interval,
        // timeout_seconds 和 max_concurrent_tasks 已移至任务级别配置
      })
    }
  }, [instance])

  const handleSave = async () => {
    if (!instance) return
    setSaving(true)
    try {
      await cfgInstancesApi.update(instance.id, form)
      toast.success('更新成功')
      onOpenChange(false)
      if (onSaved) onSaved()
    } catch (e) {
      const err = e as Error
      toast.error(`更新失败: ${err.message}`)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>编辑CFG实例</DialogTitle>
          <DialogDescription>修改实例的基本信息与开关</DialogDescription>
        </DialogHeader>
        {instance && (
          <div className="space-y-4">
            <div>
              <Label>名称</Label>
              <Input value={form.name || ''} onChange={e => setForm(prev => ({ ...prev, name: e.target.value }))} />
            </div>
            <div>
              <Label>API URL</Label>
              <Input value={form.base_url || ''} onChange={e => setForm(prev => ({ ...prev, base_url: e.target.value }))} />
            </div>
            <div>
              <Label>描述</Label>
              <Input value={form.description || ''} onChange={e => setForm(prev => ({ ...prev, description: e.target.value }))} />
            </div>
            <div className="flex items-center gap-2">
              <Checkbox id="enabled" checked={!!form.enabled} onCheckedChange={v => setForm(prev => ({ ...prev, enabled: !!v }))} />
              <Label htmlFor="enabled">启用实例</Label>
            </div>
            <div className="grid grid-cols-3 gap-3">
              <div>
                <Label>健康间隔(秒)</Label>
                <Input type="number" value={form.health_check_interval ?? 60} onChange={e => setForm(prev => ({ ...prev, health_check_interval: parseInt(e.target.value) || 0 }))} />
              </div>
              <div className="col-span-2 text-xs text-muted-foreground flex items-center">
                说明：超时与并发由任务管理参数控制，此处仅保留健康检查设置
              </div>
            </div>
            <div className="flex justify-end gap-2 pt-2">
              <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
              <Button onClick={handleSave} disabled={saving}>{saving ? '保存中...' : '保存'}</Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}


