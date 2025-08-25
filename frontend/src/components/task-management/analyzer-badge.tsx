"use client"

import { Badge } from '@/components/ui/badge'
import { AnalyzerType } from '@/lib/types'
import { Shield, Settings } from 'lucide-react'
import { cn } from '@/lib/utils'

interface AnalyzerBadgeProps {
  type: AnalyzerType
  showDescription?: boolean
  className?: string
}

// 分析器配置映射
type AnalyzerConfig = {
  variant: 'default' | 'secondary' | 'destructive' | 'outline' | 'container'
  icon: React.ComponentType<{ className?: string }>
  label: string
  description: string
  color?: string
  bgColor?: string
  fullName: string
}

const analyzerConfig: Record<AnalyzerType, AnalyzerConfig> = {
  'CAPE': {
    variant: 'container' as const,
    icon: Shield,
    label: 'CAPE',
    description: '沙箱分析',
    color: '', // 使用 container variant 的默认颜色
    bgColor: '', // 使用 container variant 的默认背景
    fullName: 'CAPE Sandbox'
  },
  'CFG': {
    variant: 'secondary' as const,
    icon: Shield,
    label: 'CFG',
    description: '控制流分析',
    color: 'text-violet-600',
    bgColor: '',
    fullName: 'CFG Analysis'
  },
  // 未来扩展的分析器
  // 'YARA': {
  //   variant: 'secondary' as const,
  //   icon: Search,
  //   label: 'YARA',
  //   description: '规则匹配',
  //   color: 'text-green-600',
  //   bgColor: 'bg-green-100',
  //   fullName: 'YARA Rules'
  // },
  // 'VT': {
  //   variant: 'outline' as const,
  //   icon: Eye,
  //   label: 'VT',
  //   description: '在线检测',
  //   color: 'text-purple-600',
  //   bgColor: 'bg-purple-100',
  //   fullName: 'VirusTotal'
  // },
  // 'CUSTOM': {
  //   variant: 'secondary' as const,
  //   icon: Settings,
  //   label: 'CUSTOM',
  //   description: '自定义分析',
  //   color: 'text-gray-600',
  //   bgColor: 'bg-gray-100',
  //   fullName: 'Custom Analyzer'
  // }
}

export function AnalyzerBadge({ type, showDescription = false, className }: AnalyzerBadgeProps) {
  const config = analyzerConfig[type]
  
  if (!config) {
    console.warn(`Unknown analyzer type: ${type}`)
    return (
      <Badge variant="outline" className={className}>
        <Settings className="h-3 w-3 mr-1" />
        未知分析器
      </Badge>
    )
  }

  const Icon = config.icon

  return (
    <Badge 
      variant={config.variant} 
      className={cn(
        "inline-flex items-center gap-1",
        config.color && config.color, // 只在有自定义颜色时应用
        className
      )}
      title={showDescription ? `${config.fullName} - ${config.description}` : config.fullName}
    >
      <Icon className="h-3 w-3" />
      {config.label}
      {showDescription && (
        <span className="text-xs opacity-75 ml-1">
          {config.description}
        </span>
      )}
    </Badge>
  )
}

// 获取分析器显示名称
export function getAnalyzerDisplayName(type: AnalyzerType): string {
  const config = analyzerConfig[type]
  return config?.fullName || type
}

// 获取分析器描述
export function getAnalyzerDescription(type: AnalyzerType): string {
  const config = analyzerConfig[type]
  return config?.description || '未知分析器'
}

// 获取分析器颜色
export function getAnalyzerColor(type: AnalyzerType): string {
  const config = analyzerConfig[type]
  return config?.color || 'text-gray-600'
}

// 检查分析器是否启用
export function isAnalyzerEnabled(type: AnalyzerType): boolean {
  return type === 'CAPE' || type === 'CFG'
}

// 获取所有可用的分析器
export function getAvailableAnalyzers(): AnalyzerType[] {
  return Object.keys(analyzerConfig) as AnalyzerType[]
}