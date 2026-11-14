import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import monacoEditorPlugin from 'vite-plugin-monaco-editor'

// https://vite.dev/config/
export default defineConfig({
  base: '/pacer/',
  plugins: [
    vue(),
    (monacoEditorPlugin as any).default({
      languageWorkers: ['editorWorkerService', 'css', 'html', 'json', 'typescript']
    })
  ],
  server: {
    host: '0.0.0.0',
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:8088',
        changeOrigin: true
      }
    }
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules')) {
            // monaco-editor 不再手动分块：手动分块会把 Vite 的预加载辅助函数一并吸进去，
            // 入口依赖它后，连登录页都要先下载 4MB 的编辑器。交给打包器按懒加载路由自然拆分。
            if (id.includes('monaco-editor')) return undefined
            if (id.includes('element-plus')) return 'element-plus'
            if (id.includes('bpmn-js')) return 'bpmn-js'
            if (id.includes('echarts')) return 'echarts'
            if (id.includes('vue') || id.includes('pinia')) return 'vue-vendor'
            return 'vendor'
          }
        }
      }
    }
  }
})
