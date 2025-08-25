"use client"

import { Badge } from '@/components/ui/badge'
import { CfgInstanceStatus } from '@/lib/types'

export function CfgStatusBadge({ status }: { status: CfgInstanceStatus }) {
  const map: Record<CfgInstanceStatus, { label: string; variant: 'default' | 'secondary' | 'destructive' | 'outline' }> = {
    healthy: { label: '健康', variant: 'default' },
    unhealthy: { label: '异常', variant: 'destructive' },
    unknown: { label: '未知', variant: 'secondary' }
  }
  const cfg = map[status] || map.unknown
  return <Badge variant={cfg.variant}>{cfg.label}</Badge>
}


