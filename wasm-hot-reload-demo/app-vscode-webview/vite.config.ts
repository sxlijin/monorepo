import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(fileURLToPath(import.meta.url));

// Plugin to disable caching for WASM files so hot reload picks up rebuilds.
const wasmNoCachePlugin = () => ({
  name: 'wasm-no-cache',
  configureServer(server: any) {
    server.middlewares.use((req: any, res: any, next: any) => {
      if (req.url?.includes('.wasm') || req.url?.includes('playground_wasm')) {
        res.setHeader('Cache-Control', 'no-store, no-cache, must-revalidate');
        res.setHeader('Pragma', 'no-cache');
        res.setHeader('Expires', '0');
      }
      next();
    });
  },
});

export default defineConfig({
  plugins: [react(), wasmNoCachePlugin()],
  resolve: {
    alias: {
      '@demo/pkg-playground': resolve(projectRoot, '../pkg-playground/src'),
      '@demo/playground-wasm': resolve(projectRoot, '../pkg-playground/wasm/playground_wasm.js'),
    }
  },
  server: {
    port: 4000,
    strictPort: true,
    cors: true,
    headers: {
      'Access-Control-Allow-Origin': '*',
    },
    watch: {
      ignored: ['!**/pkg-playground/wasm/**'],
    },
  },
  optimizeDeps: {
    exclude: ['@demo/playground-wasm'],
  },
  build: {
    rollupOptions: {
      output: {
        entryFileNames: 'assets/index.js',
        chunkFileNames: 'assets/[name].js',
        assetFileNames: 'assets/[name].[ext]'
      }
    }
  },
  define: {
    __DEV__: process.env.NODE_ENV !== 'production'
  }
});
