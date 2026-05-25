# wasm-hot-reload-demo

Self-contained replication of the BAML "Webview Tests" CI job from baml commit
`03e95d2d`. Demonstrates the Rust → WASM → Vite hot-reload pipeline plus
Vitest projects for unit (jsdom), browser (Playwright), and HMR tests.

## Layout

```
wasm-hot-reload-demo/
├── Cargo.toml                     # standalone cargo workspace (NOT a member
│                                  # of the outer monorepo workspace)
├── rust-toolchain.toml
├── crates/playground_wasm/        # Rust crate compiled to wasm32-unknown-unknown
├── package.json                   # pnpm workspace root
├── pnpm-workspace.yaml
├── pkg-playground/                # shared React + jotai package, imports the WASM
└── app-vscode-webview/            # Vite app with three vitest projects
```

## Setup

```bash
pnpm install
pnpm build:wasm
```

## Run tests

```bash
pnpm --filter app-vscode-webview test:unit:run     # jsdom
pnpm --filter app-vscode-webview test:browser:run  # Playwright/Chromium
pnpm --filter app-vscode-webview test:hmr          # spawns Vite + edits Rust source
```

The HMR test edits `crates/playground_wasm/src/hot_reload_testdata.rs`,
re-runs `wasm-pack build`, and verifies that the Vite dev server hot-reloads
the new string into the running browser without reloading the page.
