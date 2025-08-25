"use client"

import { Sample } from '@/lib/types'
import { formatBytes, formatDate } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { 
  X,
  File, 
  Archive, 
  AlertTriangle, 
  Shield,
  Copy,
  Calendar,
  Hash,
  Tag,
  Database,
  FolderOpen
} from 'lucide-react'
import { toast } from 'sonner'

interface SampleDetailDialogProps {
  sample: Sample | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

function copyToClipboard(text: string, label: string) {
  navigator.clipboard.writeText(text).then(() => {
    toast.success(`${label} 已复制到剪贴板`)
  }).catch(() => {
    toast.error('复制失败')
  })
}

export function SampleDetailDialog({ sample, open, onOpenChange }: SampleDetailDialogProps) {
  if (!sample) return null

  return (
    <>
      {open && (
        <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
          <div className="bg-background rounded-lg shadow-lg max-w-6xl w-full max-h-[95vh] overflow-hidden">
            {/* 头部 */}
            <div className="flex items-center justify-between p-6 border-b">
              <div className="flex items-center gap-3">
                {sample.is_container ? (
                  <Archive className="h-6 w-6 text-blue-600" />
                ) : (
                  <File className="h-6 w-6 text-gray-600" />
                )}
                <div>
                  <h2 className="text-xl font-semibold">{sample.file_name}</h2>
                  <div className="flex items-center gap-2 mt-1">
                    <Badge variant={sample.sample_type === 'Malicious' ? 'malicious' : 'benign'}>
                      {sample.sample_type === 'Malicious' ? (
                        <>
                          <AlertTriangle className="h-3 w-3 mr-1" />
                          恶意样本
                        </>
                      ) : (
                        <>
                          <Shield className="h-3 w-3 mr-1" />
                          安全样本
                        </>
                      )}
                    </Badge>
                    {sample.is_container && (
                      <Badge variant="container">
                        <Archive className="h-3 w-3 mr-1" />
                        容器文件
                      </Badge>
                    )}
                  </div>
                </div>
              </div>
              <Button variant="ghost" size="sm" onClick={() => onOpenChange(false)}>
                <X className="h-4 w-4" />
              </Button>
            </div>

            {/* 内容区域 */}
            <div className="overflow-y-auto max-h-[80vh]">
              {/* 重点信息区域 */}
              <div className="bg-gradient-to-r from-blue-50 to-indigo-50 border-b px-6 py-4">
                <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                  {/* 文件基本信息 */}
                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                      <File className="h-4 w-4" />
                      文件信息
                    </div>
                    <div className="space-y-1">
                      <div className="text-sm font-medium">{sample.file_name}</div>
                      <div className="text-xs text-muted-foreground">
                        {formatBytes(sample.file_size)} • {sample.file_type}
                        {sample.file_extension && ` • ${sample.file_extension}`}
                      </div>
                    </div>
                  </div>

                  {/* 哈希值（重点） */}
                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                      <Hash className="h-4 w-4" />
                      文件哈希
                    </div>
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-mono bg-background px-2 py-1 rounded">
                          {sample.file_hash_md5.slice(0, 16)}...
                        </span>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="h-6 w-6 p-0"
                          onClick={() => copyToClipboard(sample.file_hash_md5, 'MD5')}
                        >
                          <Copy className="h-3 w-3" />
                        </Button>
                      </div>
                      <div className="text-xs text-muted-foreground">MD5</div>
                    </div>
                  </div>

                  {/* 时间信息 */}
                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
                      <Calendar className="h-4 w-4" />
                      创建时间
                    </div>
                    <div className="text-sm font-medium">{formatDate(sample.created_at)}</div>
                  </div>
                </div>
              </div>

              {/* 详细信息区域 */}
              <div className="p-6 space-y-6">
                {/* 标签区域 */}
                {sample.labels && sample.labels.length > 0 && (
                  <div className="border rounded-lg p-4">
                    <div className="flex items-center gap-2 mb-3">
                      <Tag className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm font-medium">标签</span>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {sample.labels.map((label, index) => (
                        <Badge key={index} variant="secondary" className="text-xs">
                          {label}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {/* 哈希值详情 */}
                <div className="border rounded-lg p-4">
                  <div className="flex items-center gap-2 mb-3">
                    <Hash className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm font-medium">完整哈希值</span>
                  </div>
                  <div className="space-y-3">
                    <div className="flex items-center justify-between p-2 bg-muted/30 rounded">
                      <div>
                        <div className="text-xs font-medium text-muted-foreground">MD5</div>
                        <div className="font-mono text-xs">{sample.file_hash_md5}</div>
                      </div>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => copyToClipboard(sample.file_hash_md5, 'MD5')}
                      >
                        <Copy className="h-3 w-3" />
                      </Button>
                    </div>
                    <div className="flex items-center justify-between p-2 bg-muted/30 rounded">
                      <div>
                        <div className="text-xs font-medium text-muted-foreground">SHA1</div>
                        <div className="font-mono text-xs">{sample.file_hash_sha1}</div>
                      </div>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => copyToClipboard(sample.file_hash_sha1, 'SHA1')}
                      >
                        <Copy className="h-3 w-3" />
                      </Button>
                    </div>
                    <div className="flex items-center justify-between p-2 bg-muted/30 rounded">
                      <div>
                        <div className="text-xs font-medium text-muted-foreground">SHA256</div>
                        <div className="font-mono text-xs">{sample.file_hash_sha256}</div>
                      </div>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => copyToClipboard(sample.file_hash_sha256, 'SHA256')}
                      >
                        <Copy className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                </div>

                {/* 两列布局：其他信息 */}
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                  {/* 左列 */}
                  <div className="space-y-4">
                    {/* 来源信息 */}
                    {sample.source && (
                      <div className="border rounded-lg p-4">
                        <div className="flex items-center gap-2 mb-2">
                          <FolderOpen className="h-4 w-4 text-muted-foreground" />
                          <span className="text-sm font-medium">来源</span>
                        </div>
                        <div className="text-sm">{sample.source}</div>
                      </div>
                    )}

                    {/* 容器信息 */}
                    {sample.is_container && (
                      <div className="border rounded-lg p-4">
                        <div className="flex items-center gap-2 mb-2">
                          <Archive className="h-4 w-4 text-muted-foreground" />
                          <span className="text-sm font-medium">容器信息</span>
                        </div>
                        <div className="space-y-2 text-sm">
                          {sample.zip_password && (
                            <div className="flex justify-between">
                              <span className="text-muted-foreground">密码保护:</span>
                              <span>是</span>
                            </div>
                          )}
                          {sample.run_filename && (
                            <div className="flex justify-between">
                              <span className="text-muted-foreground">运行文件:</span>
                              <span className="font-mono">{sample.run_filename}</span>
                            </div>
                          )}
                        </div>
                      </div>
                    )}

                    {/* 层级关系 */}
                    {(sample.parent_id || sample.file_path_in_zip) && (
                      <div className="border rounded-lg p-4">
                        <div className="flex items-center gap-2 mb-2">
                          <FolderOpen className="h-4 w-4 text-muted-foreground" />
                          <span className="text-sm font-medium">层级关系</span>
                        </div>
                        <div className="space-y-2 text-sm">
                          {sample.parent_id && (
                            <div>
                              <span className="text-muted-foreground">父文件ID:</span>
                              <div className="flex items-center gap-2 mt-1">
                                <span className="font-mono text-xs bg-muted/30 px-2 py-1 rounded">
                                  {sample.parent_id}
                                </span>
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  className="h-6 w-6 p-0"
                                  onClick={() => copyToClipboard(sample.parent_id!, '父文件ID')}
                                >
                                  <Copy className="h-3 w-3" />
                                </Button>
                              </div>
                            </div>
                          )}
                          {sample.file_path_in_zip && (
                            <div>
                              <span className="text-muted-foreground">ZIP内路径:</span>
                              <div className="font-mono text-xs mt-1">{sample.file_path_in_zip}</div>
                            </div>
                          )}
                        </div>
                      </div>
                    )}
                  </div>

                  {/* 右列 */}
                  <div className="space-y-4">
                    {/* 时间信息 */}
                    <div className="border rounded-lg p-4">
                      <div className="flex items-center gap-2 mb-2">
                        <Calendar className="h-4 w-4 text-muted-foreground" />
                        <span className="text-sm font-medium">时间信息</span>
                      </div>
                      <div className="space-y-2 text-sm">
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">创建时间:</span>
                          <span>{formatDate(sample.created_at)}</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">更新时间:</span>
                          <span>{formatDate(sample.updated_at)}</span>
                        </div>
                      </div>
                    </div>

                    {/* 系统信息 */}
                    <div className="border rounded-lg p-4">
                      <div className="flex items-center gap-2 mb-2">
                        <Database className="h-4 w-4 text-muted-foreground" />
                        <span className="text-sm font-medium">系统信息</span>
                      </div>
                      <div className="space-y-2 text-sm">
                        <div>
                          <span className="text-muted-foreground">样本ID:</span>
                          <div className="flex items-center gap-2 mt-1">
                            <span className="font-mono text-xs bg-muted/30 px-2 py-1 rounded">
                              {sample.id}
                            </span>
                            <Button
                              size="sm"
                              variant="ghost"
                              className="h-6 w-6 p-0"
                              onClick={() => copyToClipboard(sample.id, '样本ID')}
                            >
                              <Copy className="h-3 w-3" />
                            </Button>
                          </div>
                        </div>
                        <div>
                          <span className="text-muted-foreground">存储路径:</span>
                          <div className="font-mono text-xs mt-1 break-all">{sample.storage_path}</div>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">自定义元数据:</span>
                          <span>{sample.has_custom_metadata ? "是" : "否"}</span>
                        </div>
                      </div>
                    </div>

                    {/* 自定义元数据 */}
                    {sample.custom_metadata && (
                      <div className="border rounded-lg p-4">
                        <div className="flex items-center gap-2 mb-2">
                          <Database className="h-4 w-4 text-muted-foreground" />
                          <span className="text-sm font-medium">自定义元数据</span>
                        </div>
                        <pre className="text-xs bg-muted/30 p-3 rounded font-mono overflow-x-auto">
                          {JSON.stringify(sample.custom_metadata, null, 2)}
                        </pre>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  )
}