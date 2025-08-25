import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * 格式化文件大小
 */
export function formatBytes(bytes: number, decimals = 2): string {
  if (bytes === 0) return '0 Bytes'

  const k = 1024
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']

  const i = Math.floor(Math.log(bytes) / Math.log(k))

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i]
}

/**
 * 格式化日期时间
 */
export function formatDate(date: string | Date): string {
  const d = new Date(date)
  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(d)
}

/**
 * 格式化相对时间
 * 注意：这个函数需要在组件中使用 useI18n() 来获取国际化支持
 * 建议在组件中直接使用 formatRelativeTimeI18n
 */
export function formatRelativeTime(date: string | Date): string {
  const now = new Date()
  const d = new Date(date)
  const diff = now.getTime() - d.getTime()
  
  const minutes = Math.floor(diff / (1000 * 60))
  const hours = Math.floor(diff / (1000 * 60 * 60))
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))
  
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  if (hours < 24) return `${hours}小时前`
  if (days < 7) return `${days}天前`
  
  return formatDate(date)
}

/**
 * 格式化相对时间（国际化版本）
 * 在组件中使用：const relativeTime = formatRelativeTimeI18n(date, t)
 */
export function formatRelativeTimeI18n(
  date: string | Date, 
  t: (key: string, params?: Record<string, string | number>) => string
): string {
  const now = new Date()
  const d = new Date(date)
  const diff = now.getTime() - d.getTime()
  
  const minutes = Math.floor(diff / (1000 * 60))
  const hours = Math.floor(diff / (1000 * 60 * 60))
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))
  
  if (minutes < 1) return t('time.justNow')
  if (minutes < 60) return `${minutes} ${t('time.minutesAgo')}`
  if (hours < 24) return `${hours} ${t('time.hoursAgo')}`
  if (days < 7) return `${days} ${t('time.daysAgo')}`
  
  return formatDate(date)
}

/**
 * 截断哈希值显示
 */
export function truncateHash(hash: string, length = 8): string {
  if (!hash) return ''
  return `${hash.slice(0, length)}...${hash.slice(-4)}`
}

/**
 * 获取文件扩展名
 */
export function getFileExtension(filename: string): string {
  const parts = filename.split('.')
  return parts.length > 1 ? parts.pop()?.toLowerCase() || '' : ''
}

/**
 * 验证是否为有效的哈希值
 */
export function isValidHash(hash: string, type: 'md5' | 'sha1' | 'sha256'): boolean {
  const patterns = {
    md5: /^[a-fA-F0-9]{32}$/,
    sha1: /^[a-fA-F0-9]{40}$/,
    sha256: /^[a-fA-F0-9]{64}$/
  }
  
  return patterns[type].test(hash)
}

/**
 * 防抖函数
 */
export function debounce<T extends (...args: never[]) => unknown>(
  func: T,
  wait: number
): T {
  let timeout: NodeJS.Timeout
  return ((...args: Parameters<T>) => {
    clearTimeout(timeout)
    timeout = setTimeout(() => func(...args), wait)
  }) as T
}

/**
 * 节流函数
 */
export function throttle<T extends (...args: never[]) => unknown>(
  func: T,
  limit: number
): T {
  let inThrottle: boolean
  return ((...args: Parameters<T>) => {
    if (!inThrottle) {
      func(...args)
      inThrottle = true
      setTimeout(() => inThrottle = false, limit)
    }
  }) as T
}