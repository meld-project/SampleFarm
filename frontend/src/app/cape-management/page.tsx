'use client'

import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { toast } from 'sonner'
import { Plus, RefreshCw, Activity, TrendingUp } from 'lucide-react'
import { useI18n } from '@/lib/i18n'
import { 
  CapeInstancesTable, 
  CapeInstanceDialog, 
  CapeStatusBadge 
} from '@/components/cape-management'
import { capeInstancesApi } from '@/lib/api'
import { 
  CapeInstance, 
  CreateCapeInstanceRequest, 
  UpdateCapeInstanceRequest,
  CapeInstanceQueryParams 
} from '@/lib/types'

export default function CapeManagementPage() {
  const { t } = useI18n()
  const queryClient = useQueryClient()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editingInstance, setEditingInstance] = useState<CapeInstance | undefined>()
  const [queryParams, setQueryParams] = useState<CapeInstanceQueryParams>({
    page: 1,
    page_size: 20
  })

  // 获取CAPE实例列表
  const { data: instancesData, isLoading, error } = useQuery({
    queryKey: ['cape-instances', queryParams],
    queryFn: () => capeInstancesApi.list(queryParams)
  })

  // 获取健康状态
  const { data: healthData } = useQuery({
    queryKey: ['cape-instances-health'],
    queryFn: () => capeInstancesApi.getAllHealthStatus(),
    refetchInterval: 30000, // 每30秒刷新一次
    enabled: !!instancesData?.items.length
  })

  // 创建实例
  const createMutation = useMutation({
    mutationFn: (data: CreateCapeInstanceRequest) => capeInstancesApi.create(data),
    onSuccess: () => {
      toast.success(t('cape.createSuccess'))
      queryClient.invalidateQueries({ queryKey: ['cape-instances'] })
      queryClient.invalidateQueries({ queryKey: ['cape-instances-health'] })
      setDialogOpen(false)
    },
    onError: (error: Error) => {
      toast.error(t('cape.createError', { message: error.message }))
    }
  })

  // 更新实例
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string, data: UpdateCapeInstanceRequest }) => 
      capeInstancesApi.update(id, data),
    onSuccess: () => {
      toast.success(t('cape.updateSuccess'))
      queryClient.invalidateQueries({ queryKey: ['cape-instances'] })
      queryClient.invalidateQueries({ queryKey: ['cape-instances-health'] })
      setDialogOpen(false)
      setEditingInstance(undefined)
    },
    onError: (error: Error) => {
      toast.error(t('cape.updateError', { message: error.message }))
    }
  })

  // 删除实例
  const deleteMutation = useMutation({
    mutationFn: (id: string) => capeInstancesApi.delete(id),
    onSuccess: () => {
      toast.success(t('cape.deleteSuccess'))
      queryClient.invalidateQueries({ queryKey: ['cape-instances'] })
      queryClient.invalidateQueries({ queryKey: ['cape-instances-health'] })
    },
    onError: (error: Error) => {
      toast.error(t('cape.deleteError', { message: error.message }))
    }
  })

  // 健康检查
  const healthCheckMutation = useMutation({
    mutationFn: (id: string) => capeInstancesApi.healthCheck(id),
    onSuccess: (data) => {
      toast.success(t('cape.healthCheckSuccess', { status: data.status === 'healthy' ? t('cape.healthy') : t('cape.unhealthy') }))
      queryClient.invalidateQueries({ queryKey: ['cape-instances-health'] })
    },
    onError: (error: Error) => {
      toast.error(t('cape.healthCheckError', { message: error.message }))
    }
  })

  const handleCreateInstance = () => {
    setEditingInstance(undefined)
    setDialogOpen(true)
  }

  const handleEditInstance = (instance: CapeInstance) => {
    setEditingInstance(instance)
    setDialogOpen(true)
  }

  const handleDeleteInstance = (instance: CapeInstance) => {
    if (confirm(t('cape.confirmDelete', { name: instance.name }))) {
      deleteMutation.mutate(instance.id)
    }
  }

  const handleHealthCheck = (instance: CapeInstance) => {
    healthCheckMutation.mutate(instance.id)
  }

  const handleToggleEnabled = (instance: CapeInstance, enabled: boolean) => {
    updateMutation.mutate({
      id: instance.id,
      data: { enabled }
    })
  }

  const handleSubmitInstance = async (data: CreateCapeInstanceRequest | UpdateCapeInstanceRequest) => {
    if (editingInstance) {
      await updateMutation.mutateAsync({
        id: editingInstance.id,
        data: data as UpdateCapeInstanceRequest
      })
    } else {
      await createMutation.mutateAsync(data as CreateCapeInstanceRequest)
    }
  }

  const handlePageChange = (page: number) => {
    setQueryParams(prev => ({ ...prev, page }))
  }

  const refreshData = () => {
    queryClient.invalidateQueries({ queryKey: ['cape-instances'] })
    queryClient.invalidateQueries({ queryKey: ['cape-instances-health'] })
  }

  // 计算统计信息
  const stats = instancesData?.items ? {
    total: instancesData.items.length,
    enabled: instancesData.items.filter(i => i.enabled).length,
    healthy: healthData?.filter(h => h.status === 'healthy').length || 0,
    unhealthy: healthData?.filter(h => h.status === 'unhealthy').length || 0
  } : null

  if (error) {
    return (
      <div className="container mx-auto p-6">
        <div className="text-center text-red-600">
          {t('cape.loadError', { message: error.message })}
        </div>
      </div>
    )
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* 页面标题和操作 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t('pages.cape.title')}</h1>
          <p className="text-muted-foreground">
            {t('pages.cape.desc')}
          </p>
        </div>
        <div className="flex items-center space-x-2">
          <Button
            variant="outline"
            size="sm"
            onClick={refreshData}
            disabled={isLoading}
          >
            <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
            {t('common.refresh')}
          </Button>
          <Button onClick={handleCreateInstance}>
            <Plus className="mr-2 h-4 w-4" />
            {t('pages.cape.create')}
          </Button>
        </div>
      </div>

      {/* 统计卡片 */}
      {stats && (
        <div className="grid gap-4 md:grid-cols-4">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">{t('cape.totalInstances')}</CardTitle>
              <Activity className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stats.total}</div>
            </CardContent>
          </Card>
          
          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">{t('cape.enabledInstances')}</CardTitle>
              <TrendingUp className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stats.enabled}</div>
              <p className="text-xs text-muted-foreground">
                {t('cape.percentage', { percentage: stats.total > 0 ? Math.round((stats.enabled / stats.total) * 100) : 0 })}
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">{t('cape.healthyInstances')}</CardTitle>
              <CapeStatusBadge status="healthy" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold text-green-600">{stats.healthy}</div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">{t('cape.unhealthyInstances')}</CardTitle>
              <CapeStatusBadge status="unhealthy" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold text-red-600">{stats.unhealthy}</div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* 实例表格 */}
      <Card>
        <CardHeader>
          <CardTitle>{t('cape.instanceList')}</CardTitle>
          <CardDescription>
            {t('cape.instanceListDesc')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <CapeInstancesTable
            data={instancesData}
            loading={isLoading}
            onEdit={handleEditInstance}
            onDelete={handleDeleteInstance}
            onHealthCheck={handleHealthCheck}
            onToggleEnabled={handleToggleEnabled}
            onPageChange={handlePageChange}
          />
        </CardContent>
      </Card>

      {/* 创建/编辑对话框 */}
      <CapeInstanceDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        instance={editingInstance}
        onSubmit={handleSubmitInstance}
        loading={createMutation.isPending || updateMutation.isPending}
      />
    </div>
  )
}
