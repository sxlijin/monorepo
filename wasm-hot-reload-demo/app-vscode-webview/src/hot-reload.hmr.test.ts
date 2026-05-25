import { describe, test, expect, beforeAll, afterAll, afterEach } from 'vitest'
import { chromium, Browser, Page } from 'playwright'
import { spawn, ChildProcess, execSync } from 'child_process'
import { readFileSync, writeFileSync } from 'fs'
import { resolve, dirname } from 'path'
import { fileURLToPath } from 'node:url'

// Path constants
const projectRoot = dirname(fileURLToPath(import.meta.url)).replace('/src', '')
const playgroundDir = resolve(projectRoot, '../pkg-playground')
const wasmCrateDir = resolve(projectRoot, '../crates/playground_wasm')
const hotReloadSourcePath = resolve(wasmCrateDir, 'src/hot_reload_testdata.rs')

// Test strings
const KNOWN_GOOD_STRING = 'injected for hot reload test, see hot-reload.hmr.test.ts'
const MODIFIED_STRING = 'MODIFIED for hot reload test, see hot-reload.hmr.test.ts'

interface DevServer {
  proc: ChildProcess
  port: number
}

/**
 * Wait for a specific string to appear in process stdout/stderr
 */
function waitForOutput(
  proc: ChildProcess,
  match: string | RegExp,
  timeoutMs = 30_000
): Promise<void> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`Timeout waiting for output: ${match}`))
    }, timeoutMs)

    const handler = (data: Buffer) => {
      const text = data.toString()
      const matches = typeof match === 'string' ? text.includes(match) : match.test(text)

      if (matches) {
        clearTimeout(timeout)
        proc.stdout?.off('data', handler)
        proc.stderr?.off('data', handler)
        resolve()
      }
    }

    proc.stdout?.on('data', handler)
    proc.stderr?.on('data', handler)

    proc.on('error', (err) => {
      clearTimeout(timeout)
      reject(err)
    })

    proc.on('exit', (code) => {
      clearTimeout(timeout)
      if (code !== 0) {
        reject(new Error(`Process exited with code ${code}`))
      }
    })
  })
}

/**
 * Start the Vite dev server and wait for it to be ready.
 * Uses a random port between 4900 and 4999.
 */
async function startDevServer(): Promise<DevServer> {
  const randomPort = Math.floor(Math.random() * 100) + 4900
  console.log(`[vite] Starting dev server in ${projectRoot} on port ${randomPort}`)
  const proc = spawn('pnpm', ['dev', '--force', '--port', String(randomPort), '--strictPort', 'false'], {
    cwd: projectRoot,
    stdio: ['pipe', 'pipe', 'pipe'],
    shell: true,
    env: { ...process.env, NO_COLOR: '1' },
  })

  let output = ''
  let port: number | null = null

  const portPromise = new Promise<number>((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`Timeout waiting for Vite to start.\nOutput: ${output}`))
    }, 30_000)

    const handler = (data: Buffer) => {
      const text = data.toString()
      output += text
      if (process.env.DEBUG_HMR) {
        process.stdout.write(`[vite] ${text}`)
      }

      if (!port) {
        const match = output.match(/Local:\s*http:\/\/localhost:(\d+)/)
        if (match) {
          port = parseInt(match[1], 10)
          console.log(`[vite] Dev server running on port ${port}`)
          clearTimeout(timeout)
          resolve(port)
        }
      }
    }

    proc.stdout?.on('data', handler)
    proc.stderr?.on('data', handler)

    proc.on('error', (err) => {
      clearTimeout(timeout)
      reject(err)
    })

    proc.on('exit', (code) => {
      clearTimeout(timeout)
      if (code !== 0 && !port) {
        reject(new Error(`Vite exited with code ${code}\nOutput: ${output}`))
      }
    })
  })

  try {
    const resolvedPort = await portPromise
    return { proc, port: resolvedPort }
  } catch (err) {
    proc.kill()
    throw err
  }
}

/**
 * Rebuild WASM directly using wasm-pack.
 * This is synchronous - it blocks until the build completes.
 */
function rebuildWasm(): void {
  console.log('[wasm] Rebuilding WASM...')
  execSync('pnpm build:wasm', {
    cwd: playgroundDir,
    stdio: 'inherit',
  })
  console.log('[wasm] WASM rebuild complete')
}

function killProcess(proc: ChildProcess): Promise<void> {
  return new Promise((resolve) => {
    if (proc.killed) {
      resolve()
      return
    }

    proc.on('exit', () => resolve())
    proc.kill('SIGTERM')

    setTimeout(() => {
      if (!proc.killed) {
        proc.kill('SIGKILL')
      }
      resolve()
    }, 5000)
  })
}

async function waitForHotReloadText(page: Page, text: string, timeoutMs = 30_000): Promise<void> {
  const startTime = Date.now()
  const logInterval = setInterval(async () => {
    const elapsed = Math.round((Date.now() - startTime) / 1000)
    const currentText = await getHotReloadText(page).catch(() => null)
    console.log(`[${elapsed}s] Waiting for "${text}", current: "${currentText}"`)
  }, 10_000)

  try {
    await page.waitForFunction(
      (expectedText) => {
        const el = document.querySelector('[data-testid="hot-reload-test"]')
        return el?.textContent?.includes(expectedText)
      },
      text,
      { timeout: timeoutMs }
    )
  } finally {
    clearInterval(logInterval)
  }
}

async function getHotReloadText(page: Page): Promise<string | null> {
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="hot-reload-test"]')
    return el?.textContent ?? null
  })
}

describe('WASM Build Pipeline', () => {
  let browser: Browser
  let page: Page
  let originalFileContent: string | null = null
  const processes: ChildProcess[] = []

  const cleanup = () => {
    processes.forEach((p) => {
      if (!p.killed) {
        p.kill('SIGKILL')
      }
    })
  }
  process.on('SIGINT', cleanup)
  process.on('SIGTERM', cleanup)
  process.on('exit', cleanup)

  beforeAll(async () => {
    browser = await chromium.launch({ headless: true })
  }, 30_000)

  afterAll(async () => {
    if (originalFileContent) {
      writeFileSync(hotReloadSourcePath, originalFileContent, 'utf8')
      rebuildWasm()
    }

    await browser?.close()
    await Promise.all(processes.map(killProcess))
  })

  afterEach(async () => {
    await page?.close()
  })

  test('initial page shows known good WASM content, then detects hot reload changes', async () => {
    const devServer = await startDevServer()
    processes.push(devServer.proc)

    page = await browser.newPage()

    page.on('console', (msg) => {
      console.log(`[browser ${msg.type()}] ${msg.text()}`)
    })
    page.on('pageerror', (err) => {
      console.log(`[browser error] ${err.message}`)
    })

    await page.goto(`http://localhost:${devServer.port}`)

    const pageContent = await page.content()
    console.log('[initial load] Page content:\n', pageContent)

    console.log('[waiting] Waiting for React to mount...')
    await page.waitForFunction(
      () => {
        const root = document.getElementById('root')
        return root && root.children.length > 0
      },
      { timeout: 30_000 }
    )
    console.log('[ready] React has mounted')

    await waitForHotReloadText(page, KNOWN_GOOD_STRING)
    const initialText = await getHotReloadText(page)
    expect(initialText).toBe(KNOWN_GOOD_STRING)

    originalFileContent = readFileSync(hotReloadSourcePath, 'utf8')
    const modified = originalFileContent.replace(KNOWN_GOOD_STRING, MODIFIED_STRING)
    writeFileSync(hotReloadSourcePath, modified, 'utf8')

    rebuildWasm()

    await waitForHotReloadText(page, MODIFIED_STRING)
    const modifiedText = await getHotReloadText(page)
    expect(modifiedText).toBe(MODIFIED_STRING)

    await killProcess(devServer.proc)
  }, 180_000)
})
