import '@testing-library/jest-dom/vitest';
import { vi, beforeAll } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(fileURLToPath(import.meta.url));
const wasmDir = resolve(projectRoot, '../pkg-playground/wasm');

// Patch global fetch so the wasm-bindgen JS shim can load the .wasm file
// from disk in jsdom (Node) environments.
const originalFetch = globalThis.fetch;
globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
  const url = input instanceof Request ? input.url : input.toString();

  if (url.startsWith('file://')) {
    const filePath = fileURLToPath(url);
    const buffer = readFileSync(filePath);
    return new Response(buffer, {
      status: 200,
      headers: { 'Content-Type': 'application/wasm' },
    });
  }

  if (url.endsWith('.wasm')) {
    const wasmPath = resolve(wasmDir, 'playground_wasm_bg.wasm');
    const buffer = readFileSync(wasmPath);
    return new Response(buffer, {
      status: 200,
      headers: { 'Content-Type': 'application/wasm' },
    });
  }

  return originalFetch(input, init);
};

beforeAll(async () => {
  const initWasm = (await import('@demo/playground-wasm')).default;
  await initWasm();
});

vi.mock('jotai-devtools', () => ({
  DevTools: () => null,
}));
