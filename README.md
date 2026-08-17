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
- **One-Click Setup & Upgrade**: Installs \`dsh\` globally on first launch, and the Upgrade button updates it via \`npm update -g\`

Think of it as a native shell that wraps \`dsh web\` with process management, real-time logs, and a clean UI.

## Features

- **Automatic Server Launch**: Starts \`dsh web\` on startup (default: http://127.0.0.1:3080)
- **Environment Check**: Verifies Node.js 22.19.0+ on launch and prompts with a clear message (and a "Open Node.js website" button) when Node.js is missing or too old
- **One-Click dsh Setup**: Installs \`@deepseek-ai/dsh\` globally via npm on first launch if it is not already installed
- **Immersive Web UI**: The web interface fills the entire window; a small handle at the top reveals a floating toolbar (App / CLI tabs, status indicator, version badge) on hover or click, and auto-hides when the pointer moves away
- **Built-in Interactive CLI Terminal**: The "CLI" tab is a real terminal (xterm + PTY). Type commands, run \`dsh\`/\`npm\` directly, and press Ctrl+C to interrupt. The shell follows your login shell (\`$SHELL\`; zsh by default on macOS), so your aliases and PATH just work
- **Plugin Management**: The "Plugins" tab installs/uninstalls profile plugins via \`dsh plugin --profile web add/remove <package>\`, executed in the CLI terminal with live progress and Ctrl+C support
- **Process Management**: Start / Stop / Restart / Upgrade / Clear / Copy actions
  - **Upgrade**: Runs \`npm update -g @deepseek-ai/dsh\` to update the global dsh, then automatically restarts the service
- **Clean Exit**: Closing the window destroys the entire process tree (SIGTERM → SIGKILL on macOS/Linux, \`taskkill /T\` on Windows)
- **Version Display**: Shows the installed dsh CLI version in the topbar (e.g., "dsh v0.1.0-rc.6")

## Prerequisites

- **Node.js 22.19.0+** and npm (required — dsh itself requires Node.js 22.19.0+; the app checks the version on launch and guides you if it is missing or too old)
  - Download: https://nodejs.org/
  - Verify: \`node --version\` should output v22.19.0 or newer
  - The \`dsh\` CLI is installed globally by the app on first launch (\`npm install -g @deepseek-ai/dsh\`), so you do not need to install it yourself

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

### Linux

1. Download the package for your distribution from the [Releases page](https://github.com/mijuu/dsh-desktop/releases):
   - `.deb` (Debian / Ubuntu / Linux Mint)
   - `.rpm` (Fedora / RHEL / openSUSE)
   - `.AppImage` (any distribution, no installation required)
2. Install:
   - **Debian/Ubuntu**: `sudo dpkg -i DSH.Desktop_*_amd64.deb` (or `sudo apt install ./DSH.Desktop_*_amd64.deb`)
   - **Fedora/RHEL**: `sudo rpm -i DSH.Desktop_*_x86_64.rpm`
   - **AppImage**: `chmod +x DSH.Desktop_*_x86_64.AppImage && ./DSH.Desktop_*_x86_64.AppImage`

## Usage

1. **Launch the app** — it will automatically start the dsh web server
2. **Wait for "Ready"** — the status indicator turns green and shows the server URL
3. **Interact with the Web UI** — use the app normally (the web interface fills the window)
4. **Use the CLI terminal** — hover over or click the top handle to reveal the toolbar, then switch to the "CLI" tab. It is a full interactive shell: run \`dsh web\`, \`dsh plugin --profile web add <package>\`, or any command, and press Ctrl+C to stop the foreground process
5. **Manage plugins** — open the "Plugins" tab, enter a package name (e.g. \`github:owner/repo\`), and click Install. The command runs in the CLI terminal with live progress
6. **Upgrade dsh** — click the "Upgrade" button in the toolbar to fetch the latest version and restart
7. **Stop/Restart** — use the toolbar buttons to control the server process

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

> First-time icon generation: \`node scripts/gen-icon.mjs && npx tauri icon src-tauri/app-icon.png && node scripts/gen-win-ico.mjs\`

## How It Works

The app is a thin native wrapper:

1. **Environment Check**: Runs \`node --version\` and verifies it meets 22.19.0+ (shows a clear error with a Node.js download link otherwise)
2. **dsh Setup**: Runs \`dsh --version\`; if missing, installs \`@deepseek-ai/dsh\` globally via \`npm install -g\`
3. **Startup**: Spawns \`dsh web\` as a child process
4. **Readiness Check**: Polls http://127.0.0.1:3080 every 300ms until the server responds
5. **Web UI**: Embeds the web interface in a Tauri webview (full window)
6. **CLI Terminal**: Runs an interactive shell in a PTY (via portable-pty, routed through \`fnm exec\` so node/npm/dsh are on PATH); keystrokes from the xterm frontend are forwarded to the PTY, and output streams back in real time. Install/upgrade/plugin tasks run through this same terminal
7. **Process Tree**: The app tracks the entire process tree and kills it cleanly on exit

On Windows, the app uses \`cmd /C\` to launch dsh/npm/node (resolving the \`.cmd\` shim) and sets \`CREATE_NO_WINDOW\` to hide the console window. It also merges standard Node.js install directories and the global npm bin directory (\`%APPDATA%\\npm\`) into the child PATH to handle GUI-launched processes with stale environments.

## License

[MIT](LICENSE) © mijuu