import '@testing-library/jest-dom/vitest';
import { beforeAll } from 'vitest';

// In browser mode fetch works natively, so no patching is needed.
beforeAll(async () => {
  const initWasm = (await import('@demo/playground-wasm')).default;
  await initWasm();
});
