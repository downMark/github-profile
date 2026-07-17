import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    // 本地开发同源反代到后端（backend 本地默认监听 3000，见 AGENTS.md T-001 约定）
    proxy: {
      '/api/auth': { target: 'http://localhost:3002', changeOrigin: true },
      '/.well-known': { target: 'http://localhost:3002', changeOrigin: true },
      // 集合、带查询参数的列表以及单条 Todo 都必须先于通用 /api 规则匹配。
      '^/api/users/[^/]+/todos': {
        target: 'http://localhost:3001',
        changeOrigin: true,
      },
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },
})
