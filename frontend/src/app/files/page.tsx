"use client"

import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { samplesApi } from '@/lib/api'
import { SampleFilters, Pagination } from '@/lib/types'
import { StatsBar } from '@/components/stats-bar'
import { AdvancedSearchFilters } from '@/components/advanced-search-filters'
import { FileTable } from '@/components/file-table'
import { FileTree } from '@/components/file-tree'
import { FileUploadDialog } from '@/components/file-upload-dialog'
import { SampleStatsExtendedComponent } from '@/components/sample-stats-extended'
import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Upload, Table, TreePine, RefreshCw } from 'lucide-react'
import { useI18n } from '@/lib/i18n'

export default function FilesPage() {
  const { t } = useI18n()
  const queryClient = useQueryClient()
  const [filters, setFilters] = useState<SampleFilters>({})
  const [pagination, setPagination] = useState<Pagination>({ page: 1, page_size: 20 })
  const [uploadDialogOpen, setUploadDialogOpen] = useState(false)
  const [viewMode, setViewMode] = useState<'table' | 'tree'>('table')

  // 查询样本列表
  const { data: samplesData, isLoading: samplesLoading, error: samplesError } = useQuery({
    queryKey: ['samples', filters, pagination],
    queryFn: () => samplesApi.list(filters, pagination),
  })

  // 查询统计信息
  const { data: statsData, isLoading: statsLoading } = useQuery({
    queryKey: ['samples-stats'],
    queryFn: () => samplesApi.getStats(),
    refetchInterval: 30000, // 30秒刷新一次
  })

  // 查询扩展统计信息
  const { data: statsExtendedData, isLoading: statsExtendedLoading } = useQuery({
    queryKey: ['samples-stats-extended'],
    queryFn: () => samplesApi.getStatsExtended(),
    refetchInterval: 60000, // 60秒刷新一次（扩展统计不需要太频繁）
  })

  const handleFiltersChange = (newFilters: SampleFilters) => {
    setFilters(newFilters)
    setPagination(prev => ({ ...prev, page: 1 })) // 重置到第一页
  }

  const handlePageChange = (page: number) => {
    setPagination(prev => ({ ...prev, page }))
  }

  const handlePageSizeChange = (pageSize: number) => {
    setPagination({ page: 1, page_size: pageSize })
  }

  const handleRefresh = () => {
    queryClient.invalidateQueries({ queryKey: ['samples'] })
    queryClient.invalidateQueries({ queryKey: ['samples-stats'] })
    queryClient.invalidateQueries({ queryKey: ['samples-stats-extended'] })
  }

  return (
    <div className="min-h-screen bg-background">
      <div className="container mx-auto px-4 py-6 space-y-6">
        {/* 页面标题 */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold">{t('pages.files.title')}</h1>
            <p className="text-muted-foreground mt-1">{t('pages.files.desc')}</p>
          </div>
          <div className="flex items-center space-x-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleRefresh}
              disabled={samplesLoading || statsLoading}
            >
              <RefreshCw className={`mr-2 h-4 w-4 ${samplesLoading || statsLoading ? 'animate-spin' : ''}`} />
              {t('common.refresh')}
            </Button>
            <Button onClick={() => setUploadDialogOpen(true)}>
              <Upload className="w-4 h-4 mr-2" />
              {t('pages.files.upload')}
            </Button>
          </div>
        </div>
        {/* 统计信息栏 */}
        <StatsBar data={statsData} loading={statsLoading} />

        {/* 主要内容区域 */}
        <Tabs defaultValue="files" className="w-full">
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="files">{t('tabs.files')}</TabsTrigger>
            <TabsTrigger value="analytics">{t('tabs.stats')}</TabsTrigger>
          </TabsList>

          <TabsContent value="files" className="space-y-6">
            {/* 搜索和筛选 */}
            <div className="space-y-4">
              <AdvancedSearchFilters
                filters={filters}
                onFiltersChange={handleFiltersChange}
              />
              
              {/* 视图切换 */}
              <div className="flex items-center justify-end gap-2">
                <Button
                  variant={viewMode === 'table' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setViewMode('table')}
                >
                  <Table className="h-4 w-4 mr-2" />
                  {t('tabs.files')}
                </Button>
                <Button
                  variant={viewMode === 'tree' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setViewMode('tree')}
                >
                  <TreePine className="h-4 w-4 mr-2" />
                  {t('tabs.tree')}
                </Button>
              </div>
            </div>

            {/* 文件列表 */}
            {viewMode === 'table' ? (
              <FileTable
                data={samplesData?.items || []}
                total={samplesData?.total || 0}
                page={pagination.page}
                pageSize={pagination.page_size}
                loading={samplesLoading}
                error={samplesError}
                onPageChange={handlePageChange}
                onPageSizeChange={handlePageSizeChange}
              />
            ) : (
              <FileTree
                samples={samplesData?.items || []}
                selectedFiles={[]}
                onFileSelect={() => {}}
                onDownload={async () => {}}
                onDelete={async () => {}}
              />
            )}
          </TabsContent>

          <TabsContent value="analytics" className="space-y-6">
            <SampleStatsExtendedComponent 
              data={statsExtendedData} 
              loading={statsExtendedLoading} 
            />
          </TabsContent>
        </Tabs>
      </div>

      {/* 文件上传对话框 */}
      <FileUploadDialog
        open={uploadDialogOpen}
        onOpenChange={setUploadDialogOpen}
      />
    </div>
  )
}