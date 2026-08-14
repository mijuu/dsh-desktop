# DSH Desktop

[English](README.md) | [中文](README.zh-CN.md)

A native desktop shell (Tauri 2) for [DeepSeek Harness](https://github.com/deepseek-ai/dsh). It wraps `npx @deepseek-ai/dsh web` into a native app.

## Features

- Launches `npx @deepseek-ai/dsh web` automatically on startup (default http://127.0.0.1:3080)
- The web UI fills the entire window; a small handle at the top reveals a floating toolbar (App / CLI tabs, status light) on hover or click, and auto-hides when the pointer moves away
- The "CLI" tab streams npx stdout/stderr in real time, with Start / Stop / Restart / Upgrade / Clear / Copy actions (Upgrade = `npx --latest` to install the newest version and restart the service automatically)
- Closing the window destroys the process (process group SIGTERM → SIGKILL)

## Prerequisites

- Node.js 18+ and npm
- Rust toolchain: `brew install rust`

## Development

```
npm install
npm run tauri dev
```

## Build

```
npm run tauri build
# Output in src-tauri/target/release/bundle/
```

> First-time icon generation: `node scripts/gen-icon.mjs && npx tauri icon src-tauri/app-icon.png`

## Release

Push a `v*` tag (e.g. `v0.2.1`); the [release workflow](.github/workflows/release.yml) builds macOS (Intel + Apple Silicon) and Windows installers and creates a draft Release on GitHub.

## License

[MIT](LICENSE) © mijuu
