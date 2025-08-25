"use client"

import { useMemo, useState, useCallback } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ChevronDown, ChevronRight, Copy, Search, X } from 'lucide-react'
import { toast } from 'sonner'

interface VirtualJsonViewerProps {
  value: unknown
  collapsed?: boolean
  className?: string
  searchable?: boolean
}

interface JsonTreeNode {
  key: string | number
  value: unknown
  path: string[]
  depth: number
  type: 'object' | 'array' | 'primitive'
  isExpanded: boolean
  hasChildren: boolean
  childCount?: number
}

function isObject(v: unknown): v is Record<string, unknown> { 
  return typeof v === 'object' && v !== null && !Array.isArray(v) 
}

function isArray(v: unknown): v is unknown[] { 
  return Array.isArray(v) 
}

// 将JSON扁平化为虚拟化可渲染的节点列表
function flattenJson(
  data: unknown, 
  expandedNodes: Set<string>, 
  searchTerm: string = '',
  maxDepth: number = 100
): JsonTreeNode[] {
  const nodes: JsonTreeNode[] = []
  
  function traverse(
    obj: unknown, 
    path: string[] = [], 
    depth: number = 0,
    parentKey: string | number = 'root'
  ) {
    if (depth > maxDepth) return // 防止过深递归
    
    const currentPath = path.join('.')
    const isExpanded = expandedNodes.has(currentPath)
    
    if (isObject(obj) || isArray(obj)) {
      const entries = isArray(obj) ? 
        obj.map((v, i) => [i, v] as const) : 
        Object.entries(obj)
      
      // 添加容器节点
      nodes.push({
        key: parentKey,
        value: obj,
        path,
        depth,
        type: isArray(obj) ? 'array' : 'object',
        isExpanded,
        hasChildren: entries.length > 0,
        childCount: entries.length
      })
      
      // 如果展开，添加子节点
      if (isExpanded) {
        for (const [key, value] of entries) {
          const newPath = [...path, String(key)]

          
          // 搜索过滤
          if (searchTerm) {
            const keyMatch = String(key).toLowerCase().includes(searchTerm.toLowerCase())
            const valueMatch = JSON.stringify(value).toLowerCase().includes(searchTerm.toLowerCase())
            if (!keyMatch && !valueMatch) continue
          }
          
          traverse(value, newPath, depth + 1, key)
        }
      }
    } else {
      // 原始值节点
      if (!searchTerm || 
          String(parentKey).toLowerCase().includes(searchTerm.toLowerCase()) ||
          JSON.stringify(obj).toLowerCase().includes(searchTerm.toLowerCase())) {
        nodes.push({
          key: parentKey,
          value: obj,
          path,
          depth,
          type: 'primitive',
          isExpanded: false,
          hasChildren: false
        })
      }
    }
  }
  
  traverse(data)
  return nodes
}

export function VirtualJsonViewer({ 
  value, 
  className, 
  searchable = false 
}: VirtualJsonViewerProps) {
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set())
  const [searchTerm, setSearchTerm] = useState('')
  const [showSearch, setShowSearch] = useState(false)
  const [viewMode, setViewMode] = useState<'tree' | 'raw'>('tree')
  
  const text = useMemo(() => JSON.stringify(value, null, 2), [value])
  
  // 分块处理：如果JSON太大，限制初始展开深度
  const jsonSize = useMemo(() => {
    const blob = new Blob([text])
    return blob.size
  }, [text])
  
  const maxInitialDepth = useMemo(() => {
    if (jsonSize > 5 * 1024 * 1024) return 1  // 5MB+: 只展开1层
    if (jsonSize > 1 * 1024 * 1024) return 2  // 1MB+: 只展开2层
    if (jsonSize > 100 * 1024) return 3       // 100KB+: 只展开3层
    return 5                                  // 小文件: 展开5层
  }, [jsonSize])

  const virtualNodes = useMemo(() => {
    if (viewMode === 'raw') return []
    
    // 对于大JSON，使用防抖搜索和限制深度
    return flattenJson(value, expandedNodes, searchTerm, maxInitialDepth + 3)
  }, [value, expandedNodes, searchTerm, viewMode, maxInitialDepth])

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text)
      toast.success('JSON 已复制到剪贴板')
    } catch {
      toast.error('复制失败')
    }
  }, [text])

  const toggleNode = useCallback((path: string[]) => {
    const pathStr = path.join('.')
    setExpandedNodes(prev => {
      const newSet = new Set(prev)
      if (newSet.has(pathStr)) {
        newSet.delete(pathStr)
      } else {
        newSet.add(pathStr)
      }
      return newSet
    })
  }, [])

  const highlightText = useCallback((text: string, highlight: string) => {
    if (!highlight.trim()) return text
    
    const regex = new RegExp(`(${highlight.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi')
    const parts = text.split(regex)
    
    return parts.map((part, i) => 
      regex.test(part) ? 
        <mark key={i} className="bg-yellow-200 px-0.5 rounded">{part}</mark> : 
        part
    )
  }, [])

  const renderTreeNode = useCallback((node: JsonTreeNode, index: number) => {
    const indentStyle = { paddingLeft: `${node.depth * 16}px` }
    
    return (
      <div 
        key={`${node.path.join('.')}-${index}`} 
        className="py-0.5 text-xs border-l border-muted hover:bg-muted/30 transition-colors"
        style={indentStyle}
      >
        <div className="flex items-start gap-1">
          {node.hasChildren && (
            <Button
              variant="ghost"
              size="sm"
              className="h-4 w-4 p-0 flex-shrink-0"
              onClick={() => toggleNode(node.path)}
            >
              {node.isExpanded ? 
                <ChevronDown className="h-3 w-3" /> : 
                <ChevronRight className="h-3 w-3" />
              }
            </Button>
          )}
          
          <span className="text-muted-foreground flex-shrink-0">
            {searchTerm ? 
              highlightText(String(node.key), searchTerm) : 
              String(node.key)
            }:
          </span>
          
          <span className="text-blue-700 break-words min-w-0">
            {node.type === 'primitive' ? (
              searchTerm ? 
                highlightText(JSON.stringify(node.value), searchTerm) :
                JSON.stringify(node.value)
            ) : (
              <span className="text-muted-foreground">
                {node.type === 'array' ? `Array(${node.childCount})` : `Object(${node.childCount})`}
              </span>
            )}
          </span>
        </div>
      </div>
    )
  }, [searchTerm, highlightText, toggleNode])

  // 如果JSON过大，默认显示原始模式
  const shouldDefaultToRaw = jsonSize > 10 * 1024 * 1024 // 10MB

  return (
    <div className={className}>
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <div className="flex rounded-md border">
            <Button 
              variant={viewMode === 'tree' ? 'default' : 'ghost'} 
              size="sm" 
              className="h-7 rounded-r-none"
              onClick={() => setViewMode('tree')}
              disabled={shouldDefaultToRaw}
            >
              树形
            </Button>
            <Button 
              variant={viewMode === 'raw' ? 'default' : 'ghost'} 
              size="sm" 
              className="h-7 rounded-l-none"
              onClick={() => setViewMode('raw')}
            >
              原始
            </Button>
          </div>
          
          {jsonSize > 1024 * 1024 && (
            <span className="text-xs text-muted-foreground">
              ({(jsonSize / 1024 / 1024).toFixed(1)}MB)
            </span>
          )}
        </div>
        
        <div className="flex items-center gap-2">
          {searchable && viewMode === 'tree' && (
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
      
      {searchable && showSearch && viewMode === 'tree' && (
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
      
      {shouldDefaultToRaw && viewMode === 'tree' && (
        <div className="mb-2 p-2 bg-yellow-100 rounded text-sm text-yellow-800">
          JSON文件过大({(jsonSize / 1024 / 1024).toFixed(1)}MB)，建议使用&quot;原始&quot;模式查看
        </div>
      )}
      
      <div className="bg-muted rounded p-3 text-xs overflow-auto max-h-[60vh]">
        {viewMode === 'raw' ? (
          <pre className="whitespace-pre-wrap break-words">{text}</pre>
        ) : (
          <div>
            {virtualNodes.length === 0 && searchTerm ? (
              <div className="text-muted-foreground italic py-4">无匹配项</div>
            ) : (
              virtualNodes.map((node, index) => renderTreeNode(node, index))
            )}
          </div>
        )}
      </div>
    </div>
  )
}
