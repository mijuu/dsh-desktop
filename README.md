# DSH Desktop

[English](README.md) | [中文](README.zh-CN.md)

A native desktop application (Tauri 2) that unifies the [DeepSeek Harness](https://github.com/deepseek-ai/dsh) CLI and Web UI into a single, easy-to-manage experience.

## Screenshots

<p align="center">
  <img src="screenshots/main-window.png" width="32%" />
  <img src="screenshots/toolbar.png" width="32%" />
  <img src="screenshots/cli-windows.png" width="32%" />
</p>

## Purpose

DeepSeek Harness provides both a command-line interface and a web-based UI, but managing them separately can be inconvenient. **DSH Desktop** solves this by:

- **Unified Management**: Run the CLI and Web UI together in one native app window
- **Zero Custom Logic**: Fully delegates to the original \`@deepseek-ai/dsh\` package — no reimplementation, no feature drift
- **Instant Updates**: Uses \`npx\` to dynamically fetch the latest version, so you always get the newest features without waiting for app updates

Think of it as a native shell that wraps \`npx @deepseek-ai/dsh web\` with process management, real-time logs, and a clean UI.

## Features

- **Automatic Server Launch**: Starts \`npx @deepseek-ai/dsh web\` on startup (default: http://127.0.0.1:3080)
- **Immersive Web UI**: The web interface fills the entire window; a small handle at the top reveals a floating toolbar (App / CLI tabs, status indicator, version badge) on hover or click, and auto-hides when the pointer moves away
- **Real-time CLI Logs**: The "CLI" tab streams npx stdout/stderr in real time
- **Process Management**: Start / Stop / Restart / Upgrade / Clear / Copy actions
  - **Upgrade**: Runs \`npx --yes @deepseek-ai/dsh@latest --version\` to fetch the newest version, then automatically restarts the service
- **Clean Exit**: Closing the window destroys the entire process tree (SIGTERM → SIGKILL on macOS/Linux, \`taskkill /T\` on Windows)
- **Version Display**: Shows the installed dsh CLI version in the topbar (e.g., "dsh v0.1.0-rc.6")

## Prerequisites

- **Node.js 18+** and npm (required — the app uses \`npx\` to run dsh)
  - Download: https://nodejs.org/
  - Verify: \`node --version\` and \`npx --version\` should both work in your terminal

## Installation

### macOS

1. Download \`DSH.Desktop_*_aarch64.dmg\` (Apple Silicon) or \`DSH.Desktop_*_x64.dmg\` (Intel) from the [Releases page](https://github.com/mijuu/dsh-desktop/releases)
2. Open the .dmg and drag "DSH Desktop" to Applications
3. **First launch may show "App is damaged" or "cannot be opened"** — this is because the app is not signed with an Apple Developer ID. You have two options:
   - **Option A**: Remove the quarantine attribute:

     ```bash
     sudo xattr -cr /Applications/DSH\ Desktop.app
     ```

   - **Option B**: Build from source (see [Development](#development) below)

### Windows

1. Download \`DSH.Desktop_*_x64-setup.exe\` from the [Releases page](https://github.com/mijuu/dsh-desktop/releases)
2. Run the installer and follow the prompts
3. Launch "DSH Desktop" from the Start Menu

## Usage

1. **Launch the app** — it will automatically start the dsh web server
2. **Wait for "Ready"** — the status indicator turns green and shows the server URL
3. **Interact with the Web UI** — use the app normally (the web interface fills the window)
4. **View CLI logs** — hover over or click the top handle to reveal the toolbar, then switch to the "CLI" tab
5. **Upgrade dsh** — click the "Upgrade" button in the toolbar to fetch the latest version and restart
6. **Stop/Restart** — use the toolbar buttons to control the server process

## Development

```bash
# Install dependencies
npm install

# Run in development mode (hot reload)
npm run tauri dev

# Build for production
npm run tauri build
# Output in src-tauri/target/release/bundle/
```

> First-time icon generation: \`node scripts/gen-icon.mjs && npx tauri icon src-tauri/app-icon.png\`

## How It Works

The app is a thin native wrapper:

1. **Startup**: Spawns \`npx --yes @deepseek-ai/dsh web\` as a child process
2. **Readiness Check**: Polls http://127.0.0.1:3080 every 300ms until the server responds
3. **Web UI**: Embeds the web interface in a Tauri webview (full window)
4. **CLI Logs**: Pipes npx stdout/stderr to the "CLI" tab in real time
5. **Process Tree**: The app tracks the entire process tree (cmd → npx → node → dsh) and kills it cleanly on exit

On Windows, the app uses \`cmd /C\` to launch npx (resolving the \`.cmd\` shim) and sets \`CREATE_NO_WINDOW\` to hide the console window. It also merges standard Node.js install directories into the child PATH to handle GUI-launched processes with stale environments.

## License

[MIT](LICENSE) © mijuu