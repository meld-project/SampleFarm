import type { NextConfig } from "next";

// 在构建时静态决定后端地址，运行时通过代理仅暴露前端端口
const backendURL = process.env.NEXT_PUBLIC_BACKEND_URL || process.env.BACKEND_URL || 'http://localhost:8080'

const nextConfig: NextConfig = {
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: `${backendURL}/api/:path*`,
      },
      {
        source: '/health',
        destination: `${backendURL}/health`,
      },
      {
        source: '/swagger-ui',
        destination: `${backendURL}/swagger-ui`,
      },
      {
        source: '/api-docs/:path*',
        destination: `${backendURL}/api-docs/:path*`,
      },
    ]
  },
  images: {
    domains: ['localhost'],
  },
  experimental: {
    optimizePackageImports: ['lucide-react'],
  },
};

export default nextConfig;