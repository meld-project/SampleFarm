"use client"

import { useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ChevronDown, ChevronRight, Copy, Search, X } from 'lucide-react'
import { toast } from 'sonner'

interface JsonViewerProps {
  value: unknown
  collapsed?: boolean
  className?: string
  searchable?: boolean
}

type JsonNode = string | number | boolean | null | JsonNode[] | { [k: string]: JsonNode }

function isObject(v: unknown): v is Record<string, unknown> { return typeof v === 'object' && v !== null && !Array.isArray(v) }
function isArray(v: unknown): v is unknown[] { return Array.isArray(v) }

export function JsonViewer({ value, collapsed = true, className, searchable = false }: JsonViewerProps) {
  const [open, setOpen] = useState(!collapsed)
  const [searchTerm, setSearchTerm] = useState('')
  const [showSearch, setShowSearch] = useState(false)
  const text = useMemo(() => JSON.stringify(value, null, 2), [value])

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      toast.success('JSON 已复制到剪贴板')
    } catch {
      toast.error('复制失败')
    }
  }

  const highlightText = (text: string, highlight: string) => {
    if (!highlight.trim()) return text
    const regex = new RegExp(`(${highlight})`, 'gi')
    const parts = text.split(regex)
    return parts.map((part, i) => 
      regex.test(part) ? 
        <mark key={i} className="bg-yellow-200 px-0.5 rounded">{part}</mark> : 
        part
    )
  }

  const renderNode = (node: unknown, depth = 0): React.ReactNode => {
    if (!isObject(node) && !isArray(node)) {
      const nodeText = JSON.stringify(node)
      return (
        <span className="text-blue-700 break-words">
          {searchTerm ? highlightText(nodeText, searchTerm) : nodeText}
        </span>
      )
    }
    const entries = isArray(node) ? node.map((v, i) => [i, v]) : Object.entries(node)
    
    // 如果有搜索词，过滤包含搜索词的条目
    const filteredEntries = searchTerm ? 
      entries.filter(([k, v]) => {
        const keyMatch = String(k).toLowerCase().includes(searchTerm.toLowerCase())
        const valueMatch = JSON.stringify(v).toLowerCase().includes(searchTerm.toLowerCase())
        return keyMatch || valueMatch
      }) : entries

    return (
      <div className="ml-4 border-l pl-3">
        {filteredEntries.map(([k, v]) => (
          <div key={String(k)} className="py-0.5 text-xs">
            <span className="text-muted-foreground">
              {searchTerm ? highlightText(String(k), searchTerm) : String(k)}:
            </span>{' '}
            {renderNode(v, depth + 1)}
          </div>
        ))}
        {searchTerm && filteredEntries.length === 0 && (
          <div className="text-muted-foreground italic py-1">无匹配项</div>
        )}
      </div>
    )
  }

  return (
    <div className={className}>
      <div className="flex items-center justify-between mb-2">
        <Button variant="ghost" size="sm" className="h-7 px-2" onClick={() => setOpen(!open)}>
          {open ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
          <span className="ml-1 text-sm">{open ? '折叠' : '展开'}</span>
        </Button>
        <div className="flex items-center gap-2">
          {searchable && (
            <Button 
              variant="outline" 
              size="sm" 
              className="h-7" 
              onClick={() => setShowSearch(!showSearch)}
            >
              <Search className="h-3 w-3 mr-1" />搜索
            </Button>
          )}
          <Button variant="outline" size="sm" className="h-7" onClick={handleCopy}>
            <Copy className="h-3 w-3 mr-1" />复制
          </Button>
        </div>
      </div>
      
      {searchable && showSearch && (
        <div className="mb-2 flex items-center gap-2">
          <Input
            placeholder="搜索JSON内容..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="h-8 text-sm"
          />
          {searchTerm && (
            <Button 
              variant="ghost" 
              size="sm" 
              className="h-8 w-8 p-0" 
              onClick={() => setSearchTerm('')}
            >
              <X className="h-3 w-3" />
            </Button>
          )}
        </div>
      )}
      
      {open && (
        <div className="bg-muted rounded p-3 text-xs overflow-auto max-h-[60vh]">
          {renderNode(value as JsonNode)}
        </div>
      )}
    </div>
  )
}


