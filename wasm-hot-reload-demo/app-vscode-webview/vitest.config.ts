import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { playwright } from '@vitest/browser-playwright';

const projectRoot = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@demo/pkg-playground': resolve(projectRoot, '../pkg-playground/src'),
      '@demo/playground-wasm': resolve(projectRoot, '../pkg-playground/wasm/playground_wasm.js'),
    },
  },
  define: {
    __DEV__: true,
  },
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: 'unit',
          globals: true,
          environment: 'jsdom',
          setupFiles: ['./vitest.setup.ts'],
          include: ['src/**/*.test.{ts,tsx}'],
          exclude: ['src/**/*.browser.test.{ts,tsx}', 'src/**/*.hmr.test.{ts,tsx}'],
          css: true,
          browser: {
            enabled: false,
          },
        },
      },
      {
        extends: true,
        test: {
          name: 'browser',
          globals: true,
          setupFiles: ['./vitest.setup.browser.ts'],
          include: ['src/**/*.browser.test.{ts,tsx}'],
          browser: {
            enabled: true,
            provider: playwright(),
            instances: [{ browser: 'chromium' }],
            headless: true,
          },
        },
      },
      {
        extends: true,
        test: {
          name: 'hmr',
          globals: true,
          include: ['src/**/*.hmr.test.ts'],
          globalTimeout: 30_000,
          testTimeout: 120_000,
          hookTimeout: 60_000,
          pool: 'forks',
          singleFork: true,
          retry: 1,
        },
      },
    ],
  },
});
