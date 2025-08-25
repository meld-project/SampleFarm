"use client"

import { useEffect, useState } from 'react'
import { cfgInstancesApi } from '@/lib/api'
import { CfgInstance, CfgInstanceQueryParams, PagedResult, CreateCfgInstanceRequest } from '@/lib/types'
import { CfgInstanceEditDialog } from '@/components/cfg-management/cfg-instance-edit-dialog'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/checkbox'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { toast } from 'sonner'
import { CfgStatusBadge } from '@/components/cfg-management/cfg-status-badge'
import { CfgInstanceStatsDialog } from '@/components/cfg-management/cfg-instance-stats-dialog'
import { useI18n } from '@/lib/i18n'

export default function CfgManagementPage() {
  const { t } = useI18n()
  const [data, setData] = useState<PagedResult<CfgInstance> | null>(null)
  const [loading, setLoading] = useState(false)
  const [creating, setCreating] = useState(false)
  const [form, setForm] = useState<CreateCfgInstanceRequest>({ name: '', base_url: '', description: '' })
  const [editOpen, setEditOpen] = useState(false)
  const [editing, setEditing] = useState<CfgInstance | null>(null)
  const [query, setQuery] = useState<CfgInstanceQueryParams>({ page: 1, page_size: 20 })
  const [statsOpen, setStatsOpen] = useState(false)
  const [statsId, setStatsId] = useState<string | null>(null)

  useEffect(() => {
    setLoading(true)
    cfgInstancesApi.list(query)
      .then(setData)
      .finally(() => setLoading(false))
  }, [query])

  const refresh = async () => {
    setLoading(true)
    try {
      const list = await cfgInstancesApi.list(query)
      setData(list)
    } finally {
      setLoading(false)
    }
  }

  const onCreate = async () => {
    if (!form.name.trim() || !form.base_url.trim()) {
      toast.error(t('cfg.fillRequired'))
      return
    }
    setCreating(true)
    try {
      await cfgInstancesApi.create(form)
      toast.success(t('cfg.createSuccess'))
      setForm({ name: '', base_url: '', description: '' })
      await refresh()
    } catch (e) {
      const err = e as Error
      toast.error(t('cfg.createFailed', { error: err.message }))
    } finally {
      setCreating(false)
    }
  }

  return (
    <div className="container mx-auto p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{t('pages.cfg.title')}</h1>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={refresh}>{t('common.refresh')}</Button>
        </div>
      </div>

      <Card className="p-4">
        {/* 过滤器 */}
        <div className="flex flex-wrap items-end gap-4 mb-4">
          <div>
            <Label>{t('cfg.status')}</Label>
            <Select value={query.status ?? 'all'} onValueChange={(v) => setQuery(prev => ({ ...prev, status: v === 'all' ? undefined : v, page: 1 }))}>
              <SelectTrigger className="mt-2 w-40">
                <SelectValue placeholder={t('cfg.all')} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('cfg.all')}</SelectItem>
                <SelectItem value="healthy">{t('status.healthy')}</SelectItem>
                <SelectItem value="unhealthy">{t('status.unhealthy')}</SelectItem>
                <SelectItem value="unknown">{t('files.unknown')}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="flex items-center gap-2 mt-6">
            <Checkbox id="enabled-only" checked={!!query.enabled_only} onCheckedChange={(v) => setQuery(prev => ({ ...prev, enabled_only: !!v, page: 1 }))} />
            <Label htmlFor="enabled-only">{t('cfg.enabledOnly')}</Label>
          </div>
        </div>

        {loading && <div>{t('cfg.loading')}</div>}
        {!loading && data && (
          <div className="space-y-2">
            <div className="text-sm text-muted-foreground">{t('cfg.totalRecords', { count: data.total })}</div>
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left border-b">
                  <th className="py-2">{t('cfg.name')}</th>
                  <th className="py-2">{t('cfg.url')}</th>
                  <th className="py-2">{t('cfg.enabled')}</th>
                  <th className="py-2">{t('cfg.status')}</th>
                  <th className="py-2">{t('cfg.lastCheck')}</th>
                  <th className="py-2 text-right">{t('cfg.actions')}</th>
                </tr>
              </thead>
              <tbody>
                {data.items.map(inst => (
                  <tr key={inst.id} className="border-b">
                    <td className="py-2">{inst.name}</td>
                    <td className="py-2">{inst.base_url}</td>
                    <td className="py-2">{inst.enabled ? t('cfg.yes') : t('cfg.no')}</td>
                    <td className="py-2"><CfgStatusBadge status={inst.status} /></td>
                    <td className="py-2">{inst.last_health_check || '-'}</td>
                    <td className="py-2">
                      <div className="flex items-center gap-2 justify-end">
                        <Button 
                          variant="outline" size="sm"
                          onClick={() => { setEditing(inst); setEditOpen(true) }}
                        >{t('cfg.edit')}</Button>
                        <Button 
                          variant="outline" size="sm"
                          onClick={async () => { 
                            try { 
                              await cfgInstancesApi.healthCheck(inst.id)
                              toast.success(t('cfg.healthCheckSuccess'))
                              await refresh()
                            } catch (e) { const err = e as Error; toast.error(t('cfg.healthCheckFailed', { error: err.message })) }
                          }}
                        >{t('cfg.healthCheck')}</Button>
                        <Button 
                          variant="outline" size="sm"
                          onClick={() => { setStatsId(inst.id); setStatsOpen(true) }}
                        >{t('cfg.stats')}</Button>
                        <Button 
                          variant="destructive" size="sm"
                          onClick={async () => { 
                            if (!confirm(t('cfg.deleteConfirm', { name: inst.name }))) return
                            try { 
                              await cfgInstancesApi.delete(inst.id)
                              toast.success(t('cfg.deleteSuccess'))
                              await refresh()
                            } catch (e) { const err = e as Error; toast.error(t('cfg.deleteFailed', { error: err.message })) }
                          }}
                        >{t('cfg.delete')}</Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {/* 分页 */}
            {data.total_pages > 1 && (
              <div className="flex items-center justify-between mt-3">
                <div className="text-sm text-muted-foreground">{t('cfg.page', { current: data.page, total: data.total_pages })}</div>
                <div className="flex items-center gap-2">
                  <Button variant="outline" size="sm" disabled={data.page <= 1} onClick={() => setQuery(prev => ({ ...prev, page: Math.max(1, (prev.page || 1) - 1) }))}>{t('cfg.prevPage')}</Button>
                  <Button variant="outline" size="sm" disabled={data.page >= data.total_pages} onClick={() => setQuery(prev => ({ ...prev, page: Math.min(data.total_pages, (prev.page || 1) + 1) }))}>{t('cfg.nextPage')}</Button>
                </div>
              </div>
            )}
          </div>
        )}
      </Card>
      <CfgInstanceEditDialog open={editOpen} onOpenChange={setEditOpen} instance={editing} onSaved={refresh} />
      <CfgInstanceStatsDialog open={statsOpen} onOpenChange={setStatsOpen} instanceId={statsId} days={7} />

      <Card className="p-4">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-base font-semibold">{t('cfg.newInstance')}</h2>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <Label htmlFor="name">{t('cfg.name')}</Label>
            <Input id="name" value={form.name || ''} onChange={e => setForm(prev => ({ ...prev, name: e.target.value }))} />
          </div>
          <div className="md:col-span-2">
            <Label htmlFor="url">API URL</Label>
            <Input id="url" placeholder="http://host:port" value={form.base_url || ''} onChange={e => setForm(prev => ({ ...prev, base_url: e.target.value }))} />
          </div>
          <div className="md:col-span-3">
            <Label htmlFor="desc">{t('cfg.description')}</Label>
            <Input id="desc" value={form.description || ''} onChange={e => setForm(prev => ({ ...prev, description: e.target.value }))} />
          </div>
        </div>
        <div className="mt-4 flex justify-end">
          <Button onClick={onCreate} disabled={creating}>{creating ? t('cfg.creating') : t('cfg.create')}</Button>
        </div>
      </Card>
    </div>
  )
}


