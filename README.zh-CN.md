# DSH Desktop

[English](README.md) | [中文](README.zh-CN.md)

[DeepSeek Harness](https://github.com/deepseek-ai/dsh) 的桌面外壳（Tauri 2）。把 `npx @deepseek-ai/dsh web` 装进一个原生 app。

## 功能

- 打开 app 自动启动 `npx @deepseek-ai/dsh web`（默认 http://127.0.0.1:3080）
- Web UI 铺满整个窗口；顶部小抓手悬停/点击可唤出悬浮工具栏（App/CLI 页签、状态灯），移开自动隐藏
- 「CLI」页签实时显示 npx stdout/stderr，支持 启动/停止/重启/升级/清空/复制（升级 = `npx --latest` 装最新版并自动重启服务）
- 关闭窗口即销毁进程（进程组 SIGTERM → SIGKILL）

## 前置

- Node.js 18+ 与 npm
- Rust 工具链：`brew install rust`

## 开发

```
npm install
npm run tauri dev
```

## 打包

```
npm run tauri build
# 产物在 src-tauri/target/release/bundle/
```

> 首次图标生成：`node scripts/gen-icon.mjs && npx tauri icon src-tauri/app-icon.png`

## 发布

打一个 `v*` 标签（如 `v0.2.1`）并 push 到 GitHub，[release 工作流](.github/workflows/release.yml) 会自动构建 macOS（Intel + Apple Silicon）和 Windows 安装包并创建 draft Release。

## License

[MIT](LICENSE) © mijuu
