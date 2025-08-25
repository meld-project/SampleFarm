// 前端配置管理
interface BackendConfig {
  url: string
  timeout: number
  retries: number
}

interface AppConfig {
  title: string
  description: string
  version: string
}

interface UIConfig {
  theme: string
  pageSize: number
  maxFileSize: string
}

export interface FrontendConfig {
  backend: BackendConfig
  app: AppConfig
  ui: UIConfig
}

// 默认配置（作为后备）
const defaultConfig: FrontendConfig = {
  backend: {
    url: "http://localhost:8080",
    timeout: 300000,
    retries: 3
  },
  app: {
    title: "SampleFarm - 样本管理系统",
    description: "专业的恶意样本管理和分析平台",
    version: "1.0.0"
  },
  ui: {
    theme: "light",
    pageSize: 20,
    maxFileSize: "100MB"
  }
}

let cachedConfig: FrontendConfig | null = null

/**
 * 加载前端配置
 * 优先级：环境变量 > config.json > 默认配置
 */
export async function loadConfig(): Promise<FrontendConfig> {
  if (cachedConfig) {
    return cachedConfig
  }

  try {
    // 在客户端环境中，从public目录加载配置文件
    if (typeof window !== 'undefined') {
      const response = await fetch('/config.json')
      if (response.ok) {
        const fileConfig = await response.json()
        cachedConfig = mergeConfig(defaultConfig, fileConfig)
        console.log('📝 已加载前端配置文件:', cachedConfig)
        return cachedConfig
      }
    }

    // 在服务器端或配置文件加载失败时，使用环境变量覆盖默认配置
    cachedConfig = mergeWithEnvVars(defaultConfig)
    console.log('⚙️ 使用环境变量配置:', cachedConfig)
    return cachedConfig
  } catch (error) {
    console.warn('❌ 配置加载失败，使用默认配置:', error)
    cachedConfig = mergeWithEnvVars(defaultConfig)
    return cachedConfig
  }
}

/**
 * 获取后端URL（用于代理配置）
 */
export function getBackendURL(): string {
  // 浏览器环境：使用同源相对路径，通过 Next.js rewrites 代理到后端
  if (typeof window !== 'undefined') {
    return ''
  }
  // 服务器端：使用环境变量（容器内同机访问后端）
  return process.env.NEXT_PUBLIC_BACKEND_URL || process.env.BACKEND_URL || 'http://localhost:8080'
}

/**
 * 合并配置对象
 */
function mergeConfig(defaultCfg: FrontendConfig, fileCfg: Partial<FrontendConfig>): FrontendConfig {
  return {
    backend: { ...defaultCfg.backend, ...fileCfg.backend },
    app: { ...defaultCfg.app, ...fileCfg.app },
    ui: { ...defaultCfg.ui, ...fileCfg.ui }
  }
}

/**
 * 使用环境变量覆盖配置
 */
function mergeWithEnvVars(config: FrontendConfig): FrontendConfig {
  return {
    ...config,
    backend: {
      ...config.backend,
      url: process.env.NEXT_PUBLIC_BACKEND_URL || config.backend.url,
      timeout: process.env.NEXT_PUBLIC_API_TIMEOUT ? 
        parseInt(process.env.NEXT_PUBLIC_API_TIMEOUT) : config.backend.timeout
    }
  }
}

/**
 * 获取缓存的配置（同步方法）
 */
export function getConfig(): FrontendConfig {
  return cachedConfig || defaultConfig
}