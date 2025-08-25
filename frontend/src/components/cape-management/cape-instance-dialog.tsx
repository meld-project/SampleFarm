'use client'

import { useState, useEffect } from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import * as z from 'zod'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { CapeInstance, CreateCapeInstanceRequest, UpdateCapeInstanceRequest } from '@/lib/types'
import { useI18n } from '@/lib/i18n'
import { Loader2 } from 'lucide-react'

const formSchema = z.object({
  name: z.string().min(1, '实例名称不能为空').max(100, '实例名称过长'),
  base_url: z.string()
    .min(1, 'API地址不能为空')
    .url('请输入有效的URL')
    .refine(url => url.startsWith('http://') || url.startsWith('https://'), {
      message: 'URL必须以http://或https://开头'
    }),
  description: z.string().max(500, '描述过长').optional(),
  // timeout_seconds 和 max_concurrent_tasks 已移至任务级别配置，此处保留但隐藏
  health_check_interval: z.number()
    .min(10, '健康检查间隔不能少于10秒')
    .max(3600, '健康检查间隔不能超过1小时')
    .optional(),
  enabled: z.boolean().optional()
})

type FormData = z.infer<typeof formSchema>

interface CapeInstanceDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  instance?: CapeInstance // 如果提供则为编辑模式
  onSubmit: (data: CreateCapeInstanceRequest | UpdateCapeInstanceRequest) => Promise<void>
  loading?: boolean
}

export function CapeInstanceDialog({
  open,
  onOpenChange,
  instance,
  onSubmit,
  loading = false
}: CapeInstanceDialogProps) {
  const { t } = useI18n()
  const isEditing = !!instance
  const [isSubmitting, setIsSubmitting] = useState(false)

  const form = useForm<FormData>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: '',
      base_url: '',
      description: '',
      health_check_interval: 60,
      enabled: true
    }
  })

  // 当实例数据变化时重置表单
  useEffect(() => {
    if (instance) {
      form.reset({
        name: instance.name,
        base_url: instance.base_url,
        description: instance.description || '',
        health_check_interval: instance.health_check_interval,
        enabled: instance.enabled
      })
    } else {
      form.reset({
        name: '',
        base_url: '',
        description: '',
        health_check_interval: 60,
        enabled: true
      })
    }
  }, [instance, form])

  const handleSubmit = async (data: FormData) => {
    try {
      setIsSubmitting(true)
      
      if (isEditing) {
        // 编辑模式：只传递修改的字段
        const updateData: UpdateCapeInstanceRequest = {}
        if (data.name !== instance.name) updateData.name = data.name
        if (data.base_url !== instance.base_url) updateData.base_url = data.base_url
        if (data.description !== instance.description) updateData.description = data.description
        // timeout_seconds 和 max_concurrent_tasks 已移至任务级别，不再在实例级别更新
        if (data.health_check_interval !== instance.health_check_interval) updateData.health_check_interval = data.health_check_interval
        if (data.enabled !== instance.enabled) updateData.enabled = data.enabled
        
        await onSubmit(updateData)
      } else {
        // 创建模式
        const createData: CreateCapeInstanceRequest = {
          name: data.name,
          base_url: data.base_url,
          description: data.description || undefined,
          health_check_interval: data.health_check_interval
          // timeout_seconds 和 max_concurrent_tasks 已移至任务级别，使用后端默认值
        }
        
        await onSubmit(createData)
      }
      
      onOpenChange(false)
    } catch (error) {
      console.error(t('capeDialog.submitError'), error)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>
            {isEditing ? t('capeDialog.editTitle') : t('capeDialog.createTitle')}
          </DialogTitle>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-6">
            <div className="grid gap-4">
              <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t('capeDialog.instanceName')}</FormLabel>
                    <FormControl>
                      <Input 
                        placeholder={t('capeDialog.instanceNamePlaceholder')}
                        {...field} 
                      />
                    </FormControl>
                    <FormDescription>
                      {t('capeDialog.instanceNameDesc')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="base_url"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t('capeDialog.apiAddress')}</FormLabel>
                    <FormControl>
                      <Input 
                        placeholder={t('capeDialog.apiAddressPlaceholder')}
                        {...field} 
                      />
                    </FormControl>
                    <FormDescription>
                      {t('capeDialog.apiAddressDesc')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="description"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t('capeDialog.description')}</FormLabel>
                    <FormControl>
                      <Textarea 
                        placeholder={t('capeDialog.descriptionPlaceholder')}
                        className="min-h-[80px]"
                        {...field} 
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              {/* 超时时间和最大并发数已移至任务级别配置，此处不再显示 */}

              <FormField
                control={form.control}
                name="health_check_interval"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t('capeDialog.healthCheckInterval')}</FormLabel>
                    <FormControl>
                      <Input 
                        type="number"
                        min={10}
                        max={3600}
                        {...field}
                        onChange={(e) => field.onChange(parseInt(e.target.value) || 60)}
                      />
                    </FormControl>
                    <FormDescription>
                      {t('capeDialog.healthCheckIntervalDesc')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              {isEditing && (
                <FormField
                  control={form.control}
                  name="enabled"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-start space-x-3 space-y-0">
                      <FormControl>
                        <Checkbox
                          checked={field.value}
                          onCheckedChange={field.onChange}
                        />
                      </FormControl>
                      <div className="space-y-1 leading-none">
                        <FormLabel>
                          {t('capeDialog.enabled')}
                        </FormLabel>
                        <FormDescription>
                          {t('capeDialog.enabledDesc')}
                        </FormDescription>
                      </div>
                    </FormItem>
                  )}
                />
              )}
            </div>

            <DialogFooter>
              <Button 
                type="button" 
                variant="outline" 
                onClick={() => onOpenChange(false)}
              >
                {t('common.cancel')}
              </Button>
              <Button 
                type="submit" 
                disabled={isSubmitting || loading}
              >
                {isSubmitting ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    {isEditing ? '更新中...' : '创建中...'}
                  </>
                ) : (
                  isEditing ? '更新' : '创建'
                )}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  )
}
