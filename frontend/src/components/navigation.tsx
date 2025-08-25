"use client"

import { useState } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useI18n } from '@/lib/i18n'
import {
  Files,
  ClipboardList,
  Menu,
  X,
  Home,
  Settings
} from 'lucide-react'

const navigationItems = [
  {
    href: '/files',
    labelKey: 'nav.files',
    icon: Files,
    descriptionKey: 'nav.files.desc'
  },
  {
    href: '/tasks',
    labelKey: 'nav.tasks',
    icon: ClipboardList,
    descriptionKey: 'nav.tasks.desc'
  },
  {
    href: '/cape-management',
    labelKey: 'nav.cape',
    icon: Settings,
    descriptionKey: 'nav.cape.desc'
  }
  ,
  {
    href: '/cfg-management',
    labelKey: 'nav.cfg',
    icon: Settings,
    descriptionKey: 'nav.cfg.desc'
  }
]

export function Navigation() {
  const [isOpen, setIsOpen] = useState(false)
  const pathname = usePathname()
  const { t, lang, setLang } = useI18n()

  return (
    <>
      {/* 桌面端导航 */}
      <nav className="hidden md:block border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container mx-auto px-4">
          <div className="flex items-center justify-between h-16">
            {/* Logo */}
            <Link href="/files" className="flex items-center space-x-2">
              <Home className="h-6 w-6" />
              <span className="font-bold text-lg">{t('nav.brand')}</span>
            </Link>

            {/* 导航菜单 */}
            <div className="flex items-center space-x-1">
              {navigationItems.map((item) => {
                const Icon = item.icon
                const isActive = pathname.startsWith(item.href)
                
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={cn(
                      "flex items-center space-x-2 px-3 py-2 rounded-md text-sm font-medium transition-colors",
                      isActive
                        ? "bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:text-foreground hover:bg-muted"
                    )}
                  >
                    <Icon className="h-4 w-4" />
                    <span>{t(item.labelKey)}</span>
                  </Link>
                )
              })}
              {/* API 文档入口（桌面端） */}
              <Link
                href="/swagger-ui"
                className={cn(
                  "flex items-center space-x-2 px-3 py-2 rounded-md text-sm font-medium transition-colors text-muted-foreground hover:text-foreground hover:bg-muted"
                )}
              >
                <span>{t('nav.api')}</span>
              </Link>
              <Link
                href="/api-docs/openapi.json"
                target="_blank"
                rel="noopener noreferrer"
                className={cn(
                  "flex items-center space-x-2 px-3 py-2 rounded-md text-sm font-medium transition-colors text-muted-foreground hover:text-foreground hover:bg-muted"
                )}
              >
                <span>{t('nav.api.download')}</span>
              </Link>
              {/* 语言切换器（桌面端） */}
              <div className="w-32">
                <Select value={lang} onValueChange={(v) => setLang(v as 'en' | 'zh-CN')}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="en">{t('lang.en')}</SelectItem>
                    <SelectItem value="zh-CN">{t('lang.zh')}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>
        </div>
      </nav>

      {/* 移动端导航 */}
      <nav className="md:hidden border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container mx-auto px-4">
          <div className="flex items-center justify-between h-16">
            {/* Logo */}
            <Link href="/files" className="flex items-center space-x-2">
              <Home className="h-6 w-6" />
              <span className="font-bold text-lg">{t('nav.brand')}</span>
            </Link>

            {/* 移动端菜单按钮 */}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setIsOpen(!isOpen)}
              className="md:hidden"
            >
              {isOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
            </Button>
          </div>
        </div>

        {/* 移动端菜单 */}
        {isOpen && (
          <div className="border-t bg-background">
            <div className="container mx-auto px-4 py-2">
              <div className="space-y-1">
                {navigationItems.map((item) => {
                  const Icon = item.icon
                  const isActive = pathname.startsWith(item.href)
                  
                  return (
                    <Link
                      key={item.href}
                      href={item.href}
                      onClick={() => setIsOpen(false)}
                      className={cn(
                        "flex items-center space-x-3 px-3 py-3 rounded-md text-sm font-medium transition-colors",
                        isActive
                          ? "bg-primary text-primary-foreground"
                          : "text-muted-foreground hover:text-foreground hover:bg-muted"
                      )}
                    >
                      <Icon className="h-5 w-5" />
                      <div>
                        <div>{t(item.labelKey)}</div>
                        <div className="text-xs opacity-60">{t(item.descriptionKey)}</div>
                      </div>
                    </Link>
                  )
                })}
                {/* API 文档入口（移动端） */}
                <Link
                  href="/swagger-ui"
                  onClick={() => setIsOpen(false)}
                  className={cn(
                    "flex items-center space-x-3 px-3 py-3 rounded-md text-sm font-medium transition-colors text-muted-foreground hover:text-foreground hover:bg-muted"
                  )}
                >
                  <div>{t('nav.api')}</div>
                </Link>
                <Link
                  href="/api-docs/openapi.json"
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={() => setIsOpen(false)}
                  className={cn(
                    "flex items-center space-x-3 px-3 py-3 rounded-md text-sm font-medium transition-colors text-muted-foreground hover:text-foreground hover:bg-muted"
                  )}
                >
                  <div>{t('nav.api.download')}</div>
                </Link>
                {/* 语言切换器（移动端） */}
                <div className="px-3 py-3">
                  <Select value={lang} onValueChange={(v) => setLang(v as 'en' | 'zh-CN')}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="en">{t('lang.en')}</SelectItem>
                      <SelectItem value="zh-CN">{t('lang.zh')}</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </div>
          </div>
        )}
      </nav>
    </>
  )
}
