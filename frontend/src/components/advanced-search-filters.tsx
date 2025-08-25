"use client"

import { useState, useEffect } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { SampleFilters } from '@/lib/types'
import { debounce, isValidHash } from '@/lib/utils'
import { useI18n } from '@/lib/i18n'
import { Search, X, ChevronDown, ChevronUp, Filter, Calendar } from 'lucide-react'

interface AdvancedSearchFiltersProps {
  filters: SampleFilters
  onFiltersChange: (filters: SampleFilters) => void
}

export function AdvancedSearchFilters({ filters, onFiltersChange }: AdvancedSearchFiltersProps) {
  const { t } = useI18n()
  const [localFilters, setLocalFilters] = useState<SampleFilters>(filters)
  const [expanded, setExpanded] = useState(false)
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({})

  // 防抖应用筛选
  const debouncedApply = debounce((newFilters: SampleFilters) => {
    onFiltersChange(newFilters)
  }, 500)

  useEffect(() => {
    debouncedApply(localFilters)
  }, [localFilters, debouncedApply])

  const handleFieldChange = (key: keyof SampleFilters, value: unknown) => {
    const newFilters = {
      ...localFilters,
      [key]: value || undefined // 空字符串转为 undefined
    }
    
    // 验证哈希值格式
    const newErrors = { ...validationErrors }
    if (key === 'md5' && value && typeof value === 'string' && !isValidHash(value, 'md5')) {
      newErrors.md5 = t('advancedSearch.md5Invalid')
    } else if (key === 'md5') {
      delete newErrors.md5
    }
    
    if (key === 'sha1' && value && typeof value === 'string' && !isValidHash(value, 'sha1')) {
      newErrors.sha1 = t('advancedSearch.sha1Invalid')
    } else if (key === 'sha1') {
      delete newErrors.sha1
    }
    
    if (key === 'sha256' && value && typeof value === 'string' && !isValidHash(value, 'sha256')) {
      newErrors.sha256 = t('advancedSearch.sha256Invalid')
    } else if (key === 'sha256') {
      delete newErrors.sha256
    }
    
    setValidationErrors(newErrors)
    setLocalFilters(newFilters)
  }

  const clearAllFilters = () => {
    setLocalFilters({})
    setValidationErrors({})
    onFiltersChange({})
  }

  const hasActiveFilters = Object.keys(localFilters).some(key => 
    localFilters[key as keyof SampleFilters] !== undefined && 
    localFilters[key as keyof SampleFilters] !== ''
  )

  const getActiveFiltersCount = () => {
    return Object.keys(localFilters).filter(key => 
      localFilters[key as keyof SampleFilters] !== undefined && 
      localFilters[key as keyof SampleFilters] !== ''
    ).length
  }

  return (
    <Card className="w-full">
      <Collapsible open={expanded} onOpenChange={setExpanded}>
        <CollapsibleTrigger asChild>
          <CardHeader className="cursor-pointer hover:bg-muted/50 transition-colors">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Filter className="h-5 w-5" />
                <CardTitle className="text-lg">{t('advancedSearch.title')}</CardTitle>
                {hasActiveFilters && (
                  <Badge variant="secondary">
                    {t('advancedSearch.activeFilters', { count: getActiveFiltersCount() })}
                  </Badge>
                )}
              </div>
              {expanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
            </div>
          </CardHeader>
        </CollapsibleTrigger>

        <CollapsibleContent>
          <CardContent className="space-y-6">
            {/* 基本搜索 */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="filename">{t('advancedSearch.filename')}</Label>
                <div className="relative">
                  <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground h-4 w-4" />
                  <Input
                    id="filename"
                    placeholder={t('advancedSearch.filenamePlaceholder')}
                    value={localFilters.filename || ''}
                    onChange={(e) => handleFieldChange('filename', e.target.value)}
                    className="pl-10"
                  />
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="source">{t('advancedSearch.source')}</Label>
                <Input
                  id="source"
                  placeholder={t('advancedSearch.sourcePlaceholder')}
                  value={localFilters.source || ''}
                  onChange={(e) => handleFieldChange('source', e.target.value)}
                />
              </div>
            </div>

            {/* 哈希值搜索 */}
            <div className="space-y-4">
              <h4 className="text-sm font-medium text-muted-foreground">{t('advancedSearch.hashSearch')}</h4>
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="md5">MD5</Label>
                  <Input
                    id="md5"
                    placeholder={t('advancedSearch.md5Placeholder')}
                    value={localFilters.md5 || ''}
                    onChange={(e) => handleFieldChange('md5', e.target.value)}
                    className={validationErrors.md5 ? 'border-destructive' : ''}
                  />
                  {validationErrors.md5 && (
                    <p className="text-sm text-destructive">{validationErrors.md5}</p>
                  )}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="sha1">SHA1</Label>
                  <Input
                    id="sha1"
                    placeholder={t('advancedSearch.sha1Placeholder')}
                    value={localFilters.sha1 || ''}
                    onChange={(e) => handleFieldChange('sha1', e.target.value)}
                    className={validationErrors.sha1 ? 'border-destructive' : ''}
                  />
                  {validationErrors.sha1 && (
                    <p className="text-sm text-destructive">{validationErrors.sha1}</p>
                  )}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="sha256">SHA256</Label>
                  <Input
                    id="sha256"
                    placeholder={t('advancedSearch.sha256Placeholder')}
                    value={localFilters.sha256 || ''}
                    onChange={(e) => handleFieldChange('sha256', e.target.value)}
                    className={validationErrors.sha256 ? 'border-destructive' : ''}
                  />
                  {validationErrors.sha256 && (
                    <p className="text-sm text-destructive">{validationErrors.sha256}</p>
                  )}
                </div>
              </div>
            </div>

            {/* 分类和属性筛选 */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="sample_type">{t('advancedSearch.sampleType')}</Label>
                <Select 
                  value={localFilters.sample_type || 'all'} 
                  onValueChange={(value) => handleFieldChange('sample_type', value === 'all' ? undefined : value)}
                >
                  <SelectTrigger>
                    <SelectValue placeholder={t('advancedSearch.sampleType')} />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t('advancedSearch.sampleTypeAll')}</SelectItem>
                    <SelectItem value="Malicious">{t('advancedSearch.sampleTypeMalicious')}</SelectItem>
                    <SelectItem value="Benign">{t('advancedSearch.sampleTypeBenign')}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="is_container">{t('advancedSearch.fileType')}</Label>
                <Select 
                  value={localFilters.is_container === undefined ? 'all' : String(localFilters.is_container)} 
                  onValueChange={(value) => handleFieldChange('is_container', value === 'all' ? undefined : value === 'true')}
                >
                  <SelectTrigger>
                    <SelectValue placeholder={t('advancedSearch.fileType')} />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t('advancedSearch.fileTypeAll')}</SelectItem>
                    <SelectItem value="true">{t('advancedSearch.fileTypeArchive')}</SelectItem>
                    <SelectItem value="false">{t('advancedSearch.fileTypeOther')}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="parent_id">Parent Sample ID</Label>
                <Input
                  id="parent_id"
                  placeholder="Enter parent sample UUID..."
                  value={localFilters.parent_id || ''}
                  onChange={(e) => handleFieldChange('parent_id', e.target.value)}
                />
              </div>
            </div>

            {/* 标签和时间范围 */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="labels">{t('advancedSearch.labels')}</Label>
                <Input
                  id="labels"
                  placeholder={t('advancedSearch.labelsPlaceholder')}
                  value={localFilters.labels || ''}
                  onChange={(e) => handleFieldChange('labels', e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="start_time">{t('advancedSearch.startDate')}</Label>
                <Input
                  id="start_time"
                  type="datetime-local"
                  value={localFilters.start_time || ''}
                  onChange={(e) => handleFieldChange('start_time', e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="end_time">{t('advancedSearch.endDate')}</Label>
                <Input
                  id="end_time"
                  type="datetime-local"
                  value={localFilters.end_time || ''}
                  onChange={(e) => handleFieldChange('end_time', e.target.value)}
                />
              </div>
            </div>

            {/* 操作按钮 */}
            <div className="flex items-center justify-between pt-4 border-t">
              <div className="text-sm text-muted-foreground">
                {hasActiveFilters ? t('advancedSearch.activeFilters', { count: getActiveFiltersCount() }) : t('advancedSearch.clearAll')}
              </div>
              
              <div className="flex items-center gap-2">
                {hasActiveFilters && (
                  <Button variant="outline" size="sm" onClick={clearAllFilters}>
                    <X className="h-4 w-4 mr-1" />
                    {t('advancedSearch.clearAll')}
                  </Button>
                )}
                <Button 
                  size="sm" 
                  onClick={() => setExpanded(false)}
                  disabled={Object.keys(validationErrors).length > 0}
                >
                  <Search className="h-4 w-4 mr-1" />
                  {t('advancedSearch.applyFilters')}
                </Button>
              </div>
            </div>
          </CardContent>
        </CollapsibleContent>
      </Collapsible>

      {/* 简化的激活筛选条件显示 */}
      {hasActiveFilters && !expanded && (
        <CardContent className="pt-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-sm text-muted-foreground">筛选条件:</span>
            
            {localFilters.filename && (
              <Badge variant="secondary">
                文件名: {localFilters.filename}
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => handleFieldChange('filename', undefined)}
                />
              </Badge>
            )}
            
            {localFilters.sample_type && (
              <Badge variant={localFilters.sample_type === 'Malicious' ? 'malicious' : 'benign'}>
                {localFilters.sample_type === 'Malicious' ? '恶意文件' : '安全文件'}
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => handleFieldChange('sample_type', undefined)}
                />
              </Badge>
            )}
            
            {localFilters.is_container && (
              <Badge variant="container">
                容器文件
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => handleFieldChange('is_container', undefined)}
                />
              </Badge>
            )}
            
            {localFilters.source && (
              <Badge variant="outline">
                来源: {localFilters.source}
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => handleFieldChange('source', undefined)}
                />
              </Badge>
            )}

            {(localFilters.md5 || localFilters.sha1 || localFilters.sha256) && (
              <Badge variant="outline">
                哈希查询
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => {
                    handleFieldChange('md5', undefined)
                    handleFieldChange('sha1', undefined)
                    handleFieldChange('sha256', undefined)
                  }}
                />
              </Badge>
            )}

            {localFilters.labels && (
              <Badge variant="outline">
                标签: {localFilters.labels}
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => handleFieldChange('labels', undefined)}
                />
              </Badge>
            )}

            {(localFilters.start_time || localFilters.end_time) && (
              <Badge variant="outline">
                <Calendar className="h-3 w-3 mr-1" />
                时间范围
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => {
                    handleFieldChange('start_time', undefined)
                    handleFieldChange('end_time', undefined)
                  }}
                />
              </Badge>
            )}

            {localFilters.parent_id && (
              <Badge variant="outline">
                父级: {localFilters.parent_id.substring(0, 8)}...
                <X 
                  className="h-3 w-3 ml-1 cursor-pointer" 
                  onClick={() => handleFieldChange('parent_id', undefined)}
                />
              </Badge>
            )}
          </div>
        </CardContent>
      )}
    </Card>
  )
}
