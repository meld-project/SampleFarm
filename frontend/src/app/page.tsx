import { redirect } from 'next/navigation'

export default function HomePage() {
  // 重定向到文件管理页面作为默认页面
  redirect('/files')
}