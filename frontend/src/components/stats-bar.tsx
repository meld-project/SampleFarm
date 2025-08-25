"use client"

import { Card, CardContent } from '@/components/ui/card'

import { SampleStats } from '@/lib/types'
import { formatBytes } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'
import { Database, Shield, AlertTriangle, Archive, HardDrive } from 'lucide-react'

interface StatsBarProps {
  data?: SampleStats
  loading?: boolean
}

interface StatCardProps {
  icon: React.ElementType
  label: string
  value: string | number
  color?: 'blue' | 'green' | 'red' | 'gray'
  loading?: boolean
}

function StatCard({ icon: Icon, label, value, color = 'blue', loading }: StatCardProps) {
  const colorClasses = {
    blue: 'text-blue-600 bg-blue-50',
    green: 'text-green-600 bg-green-50',
    red: 'text-red-600 bg-red-50',
    gray: 'text-gray-600 bg-gray-50',
  }

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-center">
          <div className={`rounded-lg p-2 ${colorClasses[color]}`}>
            <Icon className="h-6 w-6" />
          </div>
          <div className="ml-4">
            <p className="text-sm font-medium text-muted-foreground">{label}</p>
            {loading ? (
              <div className="h-7 w-16 bg-muted animate-pulse rounded mt-1" />
            ) : (
              <p className="text-2xl font-bold">{value}</p>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export function StatsBar({ data, loading }: StatsBarProps) {
  const { t } = useI18n()
  
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-5">
      <StatCard
        icon={Database}
        label={t('stats.totalFiles')}
        value={data?.total_samples ?? 0}
        color="blue"
        loading={loading}
      />
      <StatCard
        icon={Shield}
        label={t('stats.benignFiles')}
        value={data?.benign_samples ?? 0}
        color="green"
        loading={loading}
      />
      <StatCard
        icon={AlertTriangle}
        label={t('stats.maliciousFiles')}
        value={data?.malicious_samples ?? 0}
        color="red"
        loading={loading}
      />
      <StatCard
        icon={Archive}
        label={t('stats.containerFiles')}
        value={data?.container_files ?? 0}
        color="gray"
        loading={loading}
      />
      <StatCard
        icon={HardDrive}
        label={t('stats.storageUsed')}
        value={data ? formatBytes(data.total_size) : '0 B'}
        color="gray"
        loading={loading}
      />
    </div>
  )
}