import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vue: ['vue'],
          codemirror: [
            '@codemirror/state',
            '@codemirror/view',
            '@codemirror/commands',
            '@codemirror/language',
            '@codemirror/search',
            '@codemirror/autocomplete',
            '@codemirror/lang-sql',
            '@codemirror/lang-markdown',
            '@codemirror/theme-one-dark',
          ],
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
