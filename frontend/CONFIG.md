# 前端配置指南

## 📋 配置架构

### 配置优先级
```
环境变量 > config.json > 默认配置
```

### 配置文件层次
```
frontend/
├── config.json              # 开发配置文件
├── public/config.json        # 运行时配置文件（由config.json复制）
├── env.example              # 环境变量示例
└── src/lib/config.ts        # 配置管理逻辑
```

## ⚙️ 配置项说明

### Backend 配置
```json
{
  "backend": {
    "url": "http://localhost:8080",    // 后端API地址
    "timeout": 30000,                  // 请求超时时间(ms)
    "retries": 3                       // 失败重试次数
  }
}
```

### App 配置
```json
{
  "app": {
    "title": "SampleFarm - 样本管理系统",     // 应用标题
    "description": "专业的恶意样本管理和分析平台", // 应用描述
    "version": "1.0.0"                      // 版本号
  }
}
```

### UI 配置
```json
{
  "ui": {
    "theme": "light",          // 主题模式
    "pageSize": 20,            // 默认分页大小
    "maxFileSize": "100MB"     // 文件上传大小限制
  }
}
```

## 🛠️ 使用方法

### 1. 开发环境配置

**方法一：修改 config.json**
```bash
# 编辑配置文件
vim config.json

# 复制到public目录
cp config.json public/config.json
```

**方法二：使用环境变量**
```bash
# 创建 .env.local 文件
NEXT_PUBLIC_BACKEND_URL=http://localhost:8080
NEXT_PUBLIC_API_TIMEOUT=30000
```

### 2. 生产环境配置

**Docker部署示例**
```dockerfile
# 构建时配置
ENV NEXT_PUBLIC_BACKEND_URL=https://api.samplefarm.com

# 运行时配置文件
COPY production-config.json /app/public/config.json
```

**K8s ConfigMap示例**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: frontend-config
data:
  config.json: |
    {
      "backend": {
        "url": "http://backend-service:8080",
        "timeout": 60000,
        "retries": 5
      }
    }
```

## 🔄 代理工作原理

### 请求流程
```
浏览器请求 → Next.js代理 → 后端API
     ↓             ↓            ↓
/api/samples → rewrites → http://localhost:8080/api/samples
```

### 代理配置逻辑
```javascript
// next.config.ts
async rewrites() {
  const backendURL = getBackendURL(); // 从配置获取
  return [
    {
      source: '/api/:path*',
      destination: `${backendURL}/api/:path*`,
    }
  ]
}
```

### API客户端配置
```javascript
// src/lib/api.ts
const apiClient = axios.create({
  baseURL: '',  // 相对路径，通过代理访问
  timeout: 30000  // 从配置文件动态更新
})
```

## 📊 配置加载日志

### 成功加载配置文件
```
📝 已加载前端配置文件: { backend: {...}, app: {...}, ui: {...} }
🔗 Next.js代理配置 - 后端URL: http://localhost:8080
📡 API客户端配置已更新: { timeout: 30000, retries: 3 }
```

### 使用环境变量
```
⚙️ 使用环境变量配置: { backend: {...}, app: {...}, ui: {...} }
```

### 配置加载失败
```
❌ 配置加载失败，使用默认配置: [error details]
```

## 🔧 故障排除

### 1. CORS错误
**问题**: `Access to XMLHttpRequest blocked by CORS policy`
**解决**: 确保API客户端使用相对路径(baseURL: '')

### 2. 代理不工作
**问题**: 请求直接访问后端端口
**检查**: 
- `next.config.ts`中rewrites配置
- 浏览器开发者工具Network面板
- Next.js控制台日志

### 3. 配置不生效
**问题**: 修改config.json后无变化
**解决**: 
- 重新复制到public目录: `cp config.json public/config.json`
- 重启开发服务器: `pnpm dev`
- 清除浏览器缓存

## 🚀 最佳实践

1. **开发环境**: 使用config.json便于快速修改
2. **测试环境**: 使用环境变量便于CI/CD
3. **生产环境**: 使用ConfigMap或外部配置中心
4. **监控**: 关注配置加载日志，及时发现问题
5. **备份**: 默认配置作为兜底方案