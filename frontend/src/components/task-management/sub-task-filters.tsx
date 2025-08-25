"use client"

import { useMemo, useState } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useI18n } from '@/lib/i18n'
import { Search, X } from 'lucide-react'

interface SubTaskFiltersProps {
  filters: { status?: string; keyword?: string }
  onFiltersChange: (filters: { status?: string; keyword?: string }) => void
}

export function SubTaskFilters({ filters, onFiltersChange }: SubTaskFiltersProps) {
  const { t } = useI18n()
  const [localKeyword, setLocalKeyword] = useState(filters.keyword || '')
  const [searchField, setSearchField] = useState<'auto' | 'name' | 'md5' | 'sha1' | 'sha256'>('auto')

  const statusOptions = [
    { value: 'all', label: t('subTaskFilters.allStatus') },
    { value: 'pending', label: t('taskStatus.pending') },
    { value: 'submitting', label: t('taskStatus.submitting') },
    { value: 'submitted', label: t('taskStatus.submitted') },
    { value: 'analyzing', label: t('taskStatus.analyzing') },
    { value: 'paused', label: t('taskStatus.paused') },
    { value: 'completed', label: t('taskStatus.completed') },
    { value: 'failed', label: t('taskStatus.failed') },
    { value: 'cancelled', label: t('taskStatus.cancelled') },
  ]

  const handleStatusChange = (status: string) => {
    const newStatus = status === 'all' ? undefined : status
    onFiltersChange({ ...filters, status: newStatus })
  }

  const handleKeywordSubmit = () => {
    const raw = localKeyword.trim()
    const keyword = raw === '' ? undefined : raw
    // 与后端保持一致：后端仅接受 keyword，一个参数内兼容 文件名(模糊) 与 MD5/SHA1/SHA256(精确)
    // 这里的搜索字段选择仅用于前端提示与输入引导，不改变传参格式
    onFiltersChange({ ...filters, keyword })
  }

  const handleKeywordClear = () => {
    setLocalKeyword('')
    onFiltersChange({ ...filters, keyword: undefined })
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleKeywordSubmit()
    }
  }

  const hasActiveFilters = filters.status || filters.keyword

  const placeholder = useMemo(() => {
    switch (searchField) {
      case 'name':
        return t('subTaskFilters.searchByName')
      case 'md5':
        return t('subTaskFilters.searchByMd5')
      case 'sha1':
        return t('subTaskFilters.searchBySha1')
      case 'sha256':
        return t('subTaskFilters.searchBySha256')
      default:
        return t('subTaskFilters.smartPlaceholder')
    }
  }, [searchField, t])

  return (
    <div className="flex flex-col sm:flex-row gap-4 p-4 bg-muted/50 rounded-lg">
      <div className="flex-1">
        <div className="flex items-center gap-2">
          {/* 搜索字段选择（仅前端引导，后端仍用 keyword 统一处理） */}
          <Select value={searchField} onValueChange={(v: string) => setSearchField(v as 'auto' | 'name' | 'md5' | 'sha1' | 'sha256')}>
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder={t('subTaskFilters.searchRange')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="auto">{t('subTaskFilters.smartSearch')}</SelectItem>
              <SelectItem value="name">{t('subTaskFilters.filename')}</SelectItem>
              <SelectItem value="md5">MD5</SelectItem>
              <SelectItem value="sha1">SHA1</SelectItem>
              <SelectItem value="sha256">SHA256</SelectItem>
            </SelectContent>
          </Select>
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground h-4 w-4" />
            <Input
              placeholder={placeholder}
              value={localKeyword}
              onChange={(e) => setLocalKeyword(e.target.value)}
              onKeyDown={handleKeyDown}
              className="pl-10 pr-10"
            />
            {localKeyword && (
              <Button
                variant="ghost"
                size="sm"
                className="absolute right-1 top-1/2 transform -translate-y-1/2 h-6 w-6 p-0"
                onClick={handleKeywordClear}
              >
                <X className="h-3 w-3" />
              </Button>
            )}
          </div>
          <Button onClick={handleKeywordSubmit} size="sm">
            {t('subTaskFilters.search')}
          </Button>
        </div>
      </div>
      
      <div className="flex items-center gap-2">
        <span className="text-sm text-muted-foreground whitespace-nowrap">状态:</span>
        <Select
          value={filters.status || 'all'}
          onValueChange={handleStatusChange}
        >
          <SelectTrigger className="w-[140px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {statusOptions.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {hasActiveFilters && (
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setLocalKeyword('')
            onFiltersChange({})
          }}
          className="whitespace-nowrap"
        >
          <X className="h-4 w-4 mr-1" />
          清除筛选
        </Button>
      )}
    </div>
  )
}
