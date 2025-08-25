'use client'

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Badge } from '@/components/ui/badge'
import { Progress } from '@/components/ui/progress'
import { SampleStatsExtended, FileTypeDistribution, FileSizeDistribution, SourceDistribution, DailyUploadCount } from '@/lib/types'
import { formatBytes } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'
import { BarChart3, TrendingUp, FileType, HardDrive, MapPin } from 'lucide-react'

interface SampleStatsExtendedProps {
  data?: SampleStatsExtended
  loading?: boolean
}

export function SampleStatsExtendedComponent({ data, loading }: SampleStatsExtendedProps) {
  const { t } = useI18n()
  
  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center space-x-2 text-muted-foreground">
          <BarChart3 className="h-4 w-4 animate-pulse" />
          <span>{t('sampleStats.loading')}</span>
        </div>
      </div>
    )
  }

  if (!data) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <BarChart3 className="h-12 w-12 mx-auto mb-4 opacity-50" />
        <p>{t('sampleStats.noData')}</p>
      </div>
    )
  }

  return (
    <Tabs defaultValue="types" className="w-full">
      <TabsList className="grid w-full grid-cols-4">
        <TabsTrigger value="types">{t('sampleStats.fileTypes')}</TabsTrigger>
        <TabsTrigger value="sizes">{t('sampleStats.sizeDistribution')}</TabsTrigger>
        <TabsTrigger value="sources">{t('sampleStats.sourceDistribution')}</TabsTrigger>
        <TabsTrigger value="trends">{t('sampleStats.uploadTrends')}</TabsTrigger>
      </TabsList>

      <TabsContent value="types" className="space-y-4">
        <FileTypeDistributionCard data={data.file_type_distribution} />
      </TabsContent>

      <TabsContent value="sizes" className="space-y-4">
        <FileSizeDistributionCard data={data.file_size_distribution} />
      </TabsContent>

      <TabsContent value="sources" className="space-y-4">
        <SourceDistributionCard data={data.source_distribution} />
      </TabsContent>

      <TabsContent value="trends" className="space-y-4">
        <UploadTrendCard data={data.recent_upload_trend} />
      </TabsContent>
    </Tabs>
  )
}

function FileTypeDistributionCard({ data }: { data: FileTypeDistribution[] }) {
  const { t } = useI18n()
  
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <FileType className="h-4 w-4" />
          {t('sampleStats.fileTypeDistribution')}
        </CardTitle>
        <CardDescription>
          {t('sampleStats.fileTypeDesc')}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {data.map((item, index) => (
            <div key={index} className="flex items-center justify-between">
              <div className="flex items-center gap-3 flex-1">
                <Badge variant="outline" className="min-w-[80px] justify-center">
                  {item.file_type || t('sampleStats.unknown')}
                </Badge>
                <div className="flex-1">
                  <Progress value={item.percentage} className="h-2" />
                </div>
              </div>
              <div className="flex flex-col items-end gap-1 min-w-[120px]">
                <span className="text-sm font-medium">{t('sampleStats.filesCount', { count: item.count })}</span>
                <span className="text-xs text-muted-foreground">{formatBytes(item.size)}</span>
                <span className="text-xs text-muted-foreground">{item.percentage.toFixed(1)}%</span>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}

function FileSizeDistributionCard({ data }: { data: FileSizeDistribution[] }) {
  const { t } = useI18n()
  
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <HardDrive className="h-4 w-4" />
          {t('sampleStats.fileSizeDistribution')}
        </CardTitle>
        <CardDescription>
          {t('sampleStats.fileSizeDesc')}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {data.map((item, index) => (
            <div key={index} className="flex items-center justify-between">
              <div className="flex items-center gap-3 flex-1">
                <Badge variant="outline" className="min-w-[80px] justify-center">
                  {item.size_range}
                </Badge>
                <div className="flex-1">
                  <Progress value={item.percentage} className="h-2" />
                </div>
              </div>
              <div className="flex flex-col items-end gap-1 min-w-[120px]">
                <span className="text-sm font-medium">{t('sampleStats.filesCount', { count: item.count })}</span>
                <span className="text-xs text-muted-foreground">{formatBytes(item.total_size)}</span>
                <span className="text-xs text-muted-foreground">{item.percentage.toFixed(1)}%</span>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}

function SourceDistributionCard({ data }: { data: SourceDistribution[] }) {
  const { t } = useI18n()
  
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <MapPin className="h-4 w-4" />
          {t('sampleStats.sourceDistributionTitle')}
        </CardTitle>
        <CardDescription>
          {t('sampleStats.sourceDesc')}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {data.map((item, index) => (
            <div key={index} className="flex items-center justify-between">
              <div className="flex items-center gap-3 flex-1">
                <Badge variant="outline" className="min-w-[80px] justify-center">
                  {item.source || t('sampleStats.unknown')}
                </Badge>
                <div className="flex-1">
                  <Progress value={item.percentage} className="h-2" />
                </div>
              </div>
              <div className="flex flex-col items-end gap-1 min-w-[120px]">
                <span className="text-sm font-medium">{t('sampleStats.filesCount', { count: item.count })}</span>
                <span className="text-xs text-muted-foreground">{item.percentage.toFixed(1)}%</span>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}

function UploadTrendCard({ data }: { data: DailyUploadCount[] }) {
  const { t } = useI18n()
  const maxCount = Math.max(...data.map(item => item.count), 1)
  
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <TrendingUp className="h-4 w-4" />
          {t('sampleStats.uploadTrendTitle')}
        </CardTitle>
        <CardDescription>
          {t('sampleStats.uploadTrendDesc')}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {data.map((item, index) => (
            <div key={index} className="flex items-center justify-between">
              <div className="flex items-center gap-3 flex-1">
                <Badge variant="outline" className="min-w-[80px] justify-center">
                  {new Date(item.date).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })}
                </Badge>
                <div className="flex-1">
                  <Progress value={(item.count / maxCount) * 100} className="h-2" />
                </div>
              </div>
              <div className="flex flex-col items-end gap-1 min-w-[120px]">
                <span className="text-sm font-medium">{t('sampleStats.filesCount', { count: item.count })}</span>
                <span className="text-xs text-muted-foreground">{formatBytes(item.size)}</span>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}
