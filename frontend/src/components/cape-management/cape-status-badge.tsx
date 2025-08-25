import { Badge } from '@/components/ui/badge'
import { CapeInstanceStatus } from '@/lib/types'

interface CapeStatusBadgeProps {
  status: CapeInstanceStatus
  className?: string
}

export function CapeStatusBadge({ status, className }: CapeStatusBadgeProps) {
  const statusConfig = {
    healthy: {
      variant: 'default' as const,
      label: '健康',
      className: 'bg-green-100 text-green-800 hover:bg-green-200'
    },
    unhealthy: {
      variant: 'destructive' as const,
      label: '异常',
      className: 'bg-red-100 text-red-800 hover:bg-red-200'
    },
    unknown: {
      variant: 'secondary' as const,
      label: '未知',
      className: 'bg-gray-100 text-gray-800 hover:bg-gray-200'
    }
  }

  const config = statusConfig[status] || statusConfig.unknown

  return (
    <Badge 
      variant={config.variant}
      className={`${config.className} ${className}`}
    >
      {config.label}
    </Badge>
  )
}
