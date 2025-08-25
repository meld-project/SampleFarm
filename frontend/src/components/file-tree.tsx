"use client"

import { useState } from 'react'
import { Sample, FileTreeNode } from '@/lib/types'
import { formatBytes, formatRelativeTime, truncateHash } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { 
  File, 
  Archive, 
  ChevronRight, 
  ChevronDown,
  AlertTriangle, 
  Shield, 
  Download, 
  Trash2
} from 'lucide-react'
import { useI18n } from '@/lib/i18n'

interface FileTreeProps {
  samples: Sample[]
  selectedFiles: string[]
  onFileSelect: (fileId: string, checked: boolean) => void
  onDownload: (sample: Sample) => void
  onDelete: (sampleId: string) => void
  deletePending?: boolean
}

// 构建文件树结构
function buildFileTree(samples: Sample[]): FileTreeNode[] {
  const rootNodes: FileTreeNode[] = []
  const nodeMap = new Map<string, FileTreeNode>()

  // 创建所有节点
  samples.forEach(sample => {
    const node: FileTreeNode = {
      id: sample.id,
      sample,
      children: [],
      level: 0,
      isExpanded: false
    }
    nodeMap.set(sample.id, node)
  })

  // 构建树形结构
  samples.forEach(sample => {
    const node = nodeMap.get(sample.id)!
    
    if (sample.parent_id) {
      // 子文件：添加到父节点
      const parentNode = nodeMap.get(sample.parent_id)
      if (parentNode) {
        parentNode.children.push(node)
        node.level = parentNode.level + 1
        node.parent = parentNode
      } else {
        // 父节点不存在，作为根节点处理
        rootNodes.push(node)
      }
    } else {
      // 根文件
      rootNodes.push(node)
    }
  })

  return rootNodes
}

// 获取要显示的扁平化节点列表
function getFlattenedNodes(nodes: FileTreeNode[]): FileTreeNode[] {
  const result: FileTreeNode[] = []
  
  function traverse(node: FileTreeNode) {
    result.push(node)
    
    if (node.isExpanded && node.children.length > 0) {
      node.children.forEach(traverse)
    }
  }
  
  nodes.forEach(traverse)
  return result
}

function getFileIcon(sample: Sample, level: number) {
  if (sample.is_container) {
    return <Archive className={`h-4 w-4 text-blue-600 ${level > 0 ? 'ml-4' : ''}`} />
  }
  return <File className={`h-4 w-4 text-gray-600 ${level > 0 ? 'ml-4' : ''}`} />
}

function SampleTypeBadge({ type }: { type: Sample['sample_type'] }) {
  const { t } = useI18n()
  
  return (
    <Badge variant={type === 'Malicious' ? 'malicious' : 'benign'} className="text-xs">
      {type === 'Malicious' ? (
        <>
          <AlertTriangle className="h-3 w-3 mr-1" />
          {t('fileTable.malicious')}
        </>
      ) : (
        <>
          <Shield className="h-3 w-3 mr-1" />
          {t('fileTable.benign')}
        </>
      )}
    </Badge>
  )
}

function TreeRow({ 
  node, 
  selectedFiles, 
  onFileSelect, 
  onDownload, 
  onDelete, 
  deletePending,
  onToggleExpand 
}: {
  node: FileTreeNode
  selectedFiles: string[]
  onFileSelect: (fileId: string, checked: boolean) => void
  onDownload: (sample: Sample) => void
  onDelete: (sampleId: string) => void
  deletePending?: boolean
  onToggleExpand: (nodeId: string) => void
}) {
  const { t } = useI18n()
  const { sample, level, children, isExpanded } = node
  const hasChildren = children.length > 0
  const indentStyle = { paddingLeft: `${level * 20}px` }

  return (
    <div className="border-b hover:bg-muted/50 transition-colors">
      <div className="flex items-center py-3 px-4" style={indentStyle}>
        {/* 展开/收起按钮 */}
        <div className="w-6 h-6 flex items-center justify-center mr-2">
          {hasChildren && (
            <Button
              variant="ghost"
              size="sm"
              className="w-4 h-4 p-0"
              onClick={() => onToggleExpand(node.id)}
            >
              {isExpanded ? (
                <ChevronDown className="h-3 w-3" />
              ) : (
                <ChevronRight className="h-3 w-3" />
              )}
            </Button>
          )}
        </div>

        {/* 选择框 */}
        <Checkbox
          checked={selectedFiles.includes(sample.id)}
          onCheckedChange={(checked) => onFileSelect(sample.id, checked as boolean)}
          className="mr-3"
        />

        {/* 文件图标和信息 */}
        <div className="flex items-center gap-3 flex-1 min-w-0">
          {getFileIcon(sample, level)}
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <p className="font-medium truncate">{sample.file_name}</p>
              {sample.is_container && (
                <Badge variant="container" className="text-xs">
                  <Archive className="h-3 w-3 mr-1" />
                  {t('fileTree.filesCount', { count: children.length })}
                </Badge>
              )}
            </div>
            {sample.file_path_in_zip && (
              <p className="text-xs text-muted-foreground truncate">
                📍 {sample.file_path_in_zip}
              </p>
            )}
            {sample.source && (
              <p className="text-xs text-muted-foreground truncate">
                {t('fileTree.source')}: {sample.source}
              </p>
            )}
          </div>
        </div>

        {/* 类型标签 */}
        <div className="ml-4">
          <SampleTypeBadge type={sample.sample_type} />
        </div>

        {/* 文件大小 */}
        <div className="ml-4 text-sm min-w-0">
          <div>{formatBytes(sample.file_size)}</div>
          <div className="text-xs text-muted-foreground">
            {sample.file_type}
          </div>
        </div>

        {/* 哈希值 */}
        <div className="ml-4 text-xs font-mono min-w-0">
          <div title={`MD5: ${sample.file_hash_md5}`}>
            MD5: {truncateHash(sample.file_hash_md5)}
          </div>
          <div title={`SHA256: ${sample.file_hash_sha256}`}>
            SHA256: {truncateHash(sample.file_hash_sha256)}
          </div>
        </div>

        {/* 创建时间 */}
        <div className="ml-4 text-sm">
          {formatRelativeTime(sample.created_at)}
        </div>

        {/* 操作按钮 */}
        <div className="ml-4 flex items-center gap-1">
          <Button 
            size="sm" 
            variant="ghost"
            onClick={() => onDownload(sample)}
            title={t('fileTable.downloadFile')}
          >
            <Download className="h-4 w-4" />
          </Button>
          <Button 
            size="sm" 
            variant="ghost"
            onClick={() => onDelete(sample.id)}
            disabled={deletePending}
            title={t('fileTable.deleteFile')}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  )
}

export function FileTree({ 
  samples, 
  selectedFiles, 
  onFileSelect, 
  onDownload, 
  onDelete, 
  deletePending 
}: FileTreeProps) {
  const { t } = useI18n()
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set())

  // 构建文件树
  const treeNodes = buildFileTree(samples)
  
  // 更新节点展开状态
  const updateNodeExpansion = (nodes: FileTreeNode[]): void => {
    nodes.forEach(node => {
      node.isExpanded = expandedNodes.has(node.id)
      updateNodeExpansion(node.children)
    })
  }
  
  updateNodeExpansion(treeNodes)
  
  // 获取要显示的节点
  const visibleNodes = getFlattenedNodes(treeNodes)

  const handleToggleExpand = (nodeId: string) => {
    setExpandedNodes(prev => {
      const newSet = new Set(prev)
      if (newSet.has(nodeId)) {
        newSet.delete(nodeId)
      } else {
        newSet.add(nodeId)
      }
      return newSet
    })
  }

  if (samples.length === 0) {
    return (
      <div className="text-center py-8">
        <File className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <p className="text-muted-foreground">{t('fileTable.noData')}</p>
      </div>
    )
  }

  return (
    <div className="border rounded-lg">
      {/* 表头 */}
      <div className="border-b bg-muted/50 px-4 py-3">
        <div className="flex items-center text-sm font-medium text-muted-foreground">
          <div className="w-6 mr-2"></div> {/* 展开按钮占位 */}
          <div className="w-6 mr-3"></div> {/* 选择框占位 */}
          <div className="flex-1">{t('fileTable.fileInfo')}</div>
          <div className="w-20 ml-4">{t('fileTable.type')}</div>
          <div className="w-20 ml-4">{t('fileTable.size')}</div>
          <div className="w-32 ml-4">{t('fileTable.hash')}</div>
          <div className="w-24 ml-4">{t('fileTable.createdAt')}</div>
          <div className="w-20 ml-4">{t('fileTable.actions')}</div>
        </div>
      </div>

      {/* 树形列表 */}
      <div>
        {visibleNodes.map(node => (
          <TreeRow
            key={node.id}
            node={node}
            selectedFiles={selectedFiles}
            onFileSelect={onFileSelect}
            onDownload={onDownload}
            onDelete={onDelete}
            deletePending={deletePending}
            onToggleExpand={handleToggleExpand}
          />
        ))}
      </div>
    </div>
  )
}