import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";

type Lang = "en" | "zh";

function detectLang(): Lang {
  const l = (navigator.language || "en").toLowerCase();
  return l.startsWith("zh") ? "zh" : "en";
}

const LANG: Lang = detectLang();

const I18N: Record<Lang, Record<string, string>> = {
  en: {
    "tab.app": "App",
    "tab.plugins": "Plugins",
    "status.starting": "Starting\u2026",
    "status.running": "Running \u00b7 ",
    "status.stopped": "Stopped",
    "status.exited": "Exited",
    "status.error": "Startup failed",
    "status.preparing": "Preparing dsh environment\u2026",
    "bar.close": "Hide toolbar",
    "bar.show": "Show toolbar",
    "bar.app": "App",
    "bar.running": "Running",
    "bar.stopped": "Stopped",
    "bar.starting": "Starting\u2026",
    "bar.error": "Error",
    "loading.starting": "Starting dsh web\u2026",
    "loading.checkingDsh": "Checking dsh environment\u2026",
    "loading.stopped": "Service stopped",
    "loading.hint": "Check the CLI tab for detailed logs, then retry after fixing dependencies.",
    "btn.retry": "Retry",
    "btn.nodejs": "Open Node.js website",
    "btn.start": "Start",
    "btn.stop": "Stop",
    "btn.restart": "Restart",
    "btn.upgrade": "Upgrade",
    "btn.clear": "Clear",
    "btn.copy": "Copy",
    "btn.install": "Install",
    "btn.refresh": "Refresh",
    "plugins.input": "Enter package, e.g. @scope/name or github:user/repo",
    "plugins.hint": "Restart the service after installing / uninstalling plugins. Installation logs are in the CLI tab.",
    "plugins.empty": "No plugins yet. Enter a package name and click Install.",
    "plugins.loadFailed": "Failed to load plugin list: ",
    "plugins.remove": "Uninstall",
    "plugins.confirmRemove": "Confirm",
    "plugins.enterName": "Please enter a package name.",
    "plugins.invalidName": "Invalid package name.",
    "log.stopService": "Stopping service\u2026",
    "log.notOwned": "This service was not started by this app, left running.",
    "log.waitingPort": "Waiting for port {port}\u2026",
    "log.nodeOk": "Node.js v{version} meets requirements.",
    "log.ready": "Ready: ",
    "log.reused": "Detected existing service at {url}, reusing it (not started by this app).",
    "log.checkingUpgrade": "Checking for the latest version and installing\u2026",
    "log.installedVersion": "Installed version {version} \u00b7 {message}",
    "log.upgradeFailed": "Upgrade failed: ",
    "log.copied": "Copied to clipboard.",
    "log.copyFailed": "Copy failed.",
    "log.openBrowserFailed": "Failed to open browser: ",
    "log.processExited": "Process exited (code={code})",
    "err.startFailed": "Startup failed: ",
    "err.checkNodeFailed": "Failed to check Node.js: ",
    "err.installDshFailed": "Failed to install dsh: ",
    "err.startupTimeout": "Startup timed out: no port listening within 90s. Check the CLI logs.",
    "err.exitedEarly": "Startup failed: process exited early (code {code}). Check the CLI logs.",
    "err.authRequired": "A newer dsh web is already running on port 3080, but its access token is printed only once in the terminal that started it, so this app cannot authenticate. Stop that dsh web process first, then start it from this app.",
    "version.unknown": "dsh unknown",
  },
  zh: {
    "tab.app": "\u5e94\u7528",
    "tab.plugins": "\u63d2\u4ef6",
    "status.starting": "\u542f\u52a8\u4e2d\u2026",
    "status.running": "\u8fd0\u884c\u4e2d \u00b7 ",
    "status.stopped": "\u5df2\u505c\u6b62",
    "status.exited": "\u5df2\u9000\u51fa",
    "status.error": "\u542f\u52a8\u5931\u8d25",
    "status.preparing": "\u51c6\u5907 dsh \u73af\u5883\u2026",
    "bar.close": "\u9690\u85cf\u5de5\u5177\u680f",
    "bar.show": "\u663e\u793a\u5de5\u5177\u680f",
    "bar.app": "\u5e94\u7528",
    "bar.running": "\u8fd0\u884c\u4e2d",
    "bar.stopped": "\u5df2\u505c\u6b62",
    "bar.starting": "\u542f\u52a8\u4e2d\u2026",
    "bar.error": "\u5f02\u5e38",
    "loading.starting": "\u6b63\u5728\u542f\u52a8 dsh web \u2026",
    "loading.checkingDsh": "\u6b63\u5728\u68c0\u67e5 dsh \u73af\u5883\u2026",
    "loading.stopped": "\u670d\u52a1\u5df2\u505c\u6b62",
    "loading.hint": "\u53ef\u5728\u4e0a\u65b9 CLI \u9875\u7b7e\u67e5\u770b\u8be6\u7ec6\u65e5\u5fd7\uff0c\u5b89\u88c5\u597d\u4f9d\u8d56\u540e\u53ef\u91cd\u8bd5\u3002",
    "btn.retry": "\u91cd\u8bd5",
    "btn.nodejs": "\u6253\u5f00 Node.js \u5b98\u7f51",
    "btn.start": "\u542f\u52a8",
    "btn.stop": "\u505c\u6b62",
    "btn.restart": "\u91cd\u542f",
    "btn.upgrade": "\u5347\u7ea7",
    "btn.clear": "\u6e05\u7a7a",
    "btn.copy": "\u590d\u5236",
    "btn.install": "\u5b89\u88c5",
    "btn.refresh": "\u5237\u65b0",
    "plugins.input": "\u8f93\u5165\u5305\uff0c\u5982 @scope/name \u6216 github:user/repo",
    "plugins.hint": "\u5b89\u88c5 / \u5378\u8f7d\u63d2\u4ef6\u540e\u9700\u91cd\u542f\u670d\u52a1\u624d\u80fd\u751f\u6548\uff1b\u5b89\u88c5\u8fc7\u7a0b\u65e5\u5fd7\u89c1\u300cCLI\u300d\u9875\u7b7e\u3002",
    "plugins.empty": "\u6682\u65e0\u63d2\u4ef6\uff0c\u8f93\u5165\u5305\u540d\u540e\u70b9\u51fb\u300c\u5b89\u88c5\u300d\u3002",
    "plugins.loadFailed": "\u52a0\u8f7d\u63d2\u4ef6\u5217\u8868\u5931\u8d25\uff1a",
    "plugins.remove": "\u5378\u8f7d",
    "plugins.confirmRemove": "\u786e\u8ba4\u5378\u8f7d",
    "plugins.enterName": "\u8bf7\u8f93\u5165\u63d2\u4ef6\u5305\u540d\u3002",
    "plugins.invalidName": "\u65e0\u6548\u7684\u5305\u540d\u3002",
    "log.stopService": "\u505c\u6b62\u670d\u52a1\u2026",
    "log.notOwned": "\u8be5\u670d\u52a1\u975e\u672c\u5e94\u7528\u542f\u52a8\uff0c\u672a\u505c\u6b62",
    "log.waitingPort": "\u7b49\u5f85\u7aef\u53e3 {port} \u5c31\u7eea\u2026",
    "log.nodeOk": "Node.js v{version} \u6ee1\u8db3\u8981\u6c42",
    "log.ready": "\u5c31\u7eea\uff1a",
    "log.reused": "\u68c0\u6d4b\u5230 {url} \u5df2\u6709\u670d\u52a1\uff0c\u76f4\u63a5\u590d\u7528\uff08\u8be5\u670d\u52a1\u975e\u672c\u5e94\u7528\u542f\u52a8\uff09",
    "log.checkingUpgrade": "\u6b63\u5728\u68c0\u67e5\u6700\u65b0\u7248\u672c\u5e76\u5b89\u88c5\u2026",
    "log.installedVersion": "\u5df2\u5b89\u88c5\u7248\u672c {version} \u00b7 {message}",
    "log.upgradeFailed": "\u5347\u7ea7\u5931\u8d25\uff1a",
    "log.copied": "\u5df2\u590d\u5236\u5230\u526a\u8d34\u677f",
    "log.copyFailed": "\u590d\u5236\u5931\u8d25",
    "log.openBrowserFailed": "\u6253\u5f00\u6d4f\u89c8\u5668\u5931\u8d25\uff1a",
    "log.processExited": "\u8fdb\u7a0b\u5df2\u9000\u51fa (code={code})",
    "err.startFailed": "\u542f\u52a8\u5931\u8d25\uff1a",
    "err.checkNodeFailed": "\u68c0\u67e5 Node.js \u5931\u8d25\uff1a",
    "err.installDshFailed": "\u5b89\u88c5 dsh \u5931\u8d25\uff1a",
    "err.startupTimeout": "\u542f\u52a8\u8d85\u65f6\uff1a90 \u79d2\u5185\u672a\u68c0\u6d4b\u5230\u7aef\u53e3\u76d1\u542c\uff0c\u8bf7\u68c0\u67e5 CLI \u65e5\u5fd7",
    "err.exitedEarly": "\u542f\u52a8\u5931\u8d25\uff1a\u8fdb\u7a0b\u63d0\u524d\u9000\u51fa\uff08\u9000\u51fa\u7801 {code}\uff09\uff0c\u8bf7\u68c0\u67e5 CLI \u65e5\u5fd7",
    "err.authRequired": "\u68c0\u6d4b\u5230\u65b0\u7248 dsh web \u5df2\u5728 3080 \u7aef\u53e3\u8fd0\u884c\uff0c\u4f46\u5b83\u7684\u8bbf\u95ee token \u53ea\u5728\u542f\u52a8\u5b83\u7684\u7ec8\u7aef\u6253\u5370\u4e00\u6b21\uff0c\u672c\u5e94\u7528\u65e0\u6cd5\u83b7\u53d6\u9274\u6743\u4fe1\u606f\u3002\u8bf7\u5148\u5173\u95ed\u90a3\u4e2a dsh web \u8fdb\u7a0b\uff0c\u518d\u91cd\u65b0\u4ece\u672c\u5e94\u7528\u542f\u52a8\u3002",
    "version.unknown": "dsh \u672a\u77e5",
  },
};

function t(key: string, vars?: Record<string, string | number>): string {
  let s = I18N[LANG][key] ?? I18N.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replaceAll("{" + k + "}", String(v));
    }
  }
  return s;
}

/** Apply translations to static DOM nodes marked with data-i18n attributes. */
function applyTranslations(): void {
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (key) el.textContent = t(key);
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-placeholder]").forEach((el) => {
    const key = el.getAttribute("data-i18n-placeholder");
    if (key) (el as HTMLInputElement).placeholder = t(key);
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((el) => {
    const key = el.getAttribute("data-i18n-title");
    if (key) el.title = t(key);
  });
  document.documentElement.lang = LANG === "zh" ? "zh-CN" : "en";
}

const PORT = 3080;
const APP_URL = "http://127.0.0.1:" + PORT;

const PARAMS = new URLSearchParams(location.search);
/** The same document also serves the floating toolbar window. */
const BAR_VIEW = PARAMS.get("view") === "bar";

function q<T extends HTMLElement>(sel: string): T {
  return document.querySelector(sel) as T;
}

/** Base URL of this page without query — the shell window's URL. */
function shellUrlBase(): string {
  return location.href.split("?")[0];
}

const loading = q<HTMLDivElement>("#loading");
const dot = q<HTMLSpanElement>("#status-dot");
const statusText = q<HTMLSpanElement>("#status-text");
const loadingSpinner = q<HTMLDivElement>("#loading-spinner");
const loadingText = q<HTMLParagraphElement>("#loading-text");

type State = "starting" | "running" | "stopped" | "error";

let upgrading = false;
let ready = false;
let runningNow = false;
let startupErrorTimer: number | undefined;

/** Last known (tokenized when applicable) dsh web URL. */
let currentUrl = APP_URL;

function setStatus(state: State, text: string): void {
  dot.className = "dot " + state;
  statusText.textContent = text;
}

function appendLog(line: string, kind: "out" | "err" | "sys"): void {
  const open = kind === "sys" ? "\x1b[90m" : kind === "err" ? "\x1b[91m" : "";
  const close = kind === "out" ? "" : "\x1b[0m";
  cliTerm?.write(open + line + close + "\r\n");
}

/** Switch the loading overlay between "starting", "error" and "stopped" states. */
function setLoading(
  message: string,
  opts: { error?: boolean; retry?: boolean; spinner?: boolean; nodejs?: boolean } = {},
): void {
  loadingSpinner.style.display = opts.error || opts.spinner === false ? "none" : "";
  loadingText.textContent = message;
  loadingText.classList.toggle("error-text", !!opts.error);
  q<HTMLParagraphElement>("#loading-hint").hidden = !opts.error;
  q<HTMLButtonElement>("#btn-retry").hidden = !opts.retry;
  q<HTMLButtonElement>("#btn-nodejs").hidden = !opts.nodejs;
}

function showStartupError(message: string, opts: { nodejs?: boolean } = {}): void {
  ready = false;
  runningNow = false;
  setStatus("error", t("status.error"));
  setLoading(message, { error: true, retry: true, nodejs: opts.nodejs });
  appendLog("> " + message, "err");
}

/** Send the main window to the dsh web app top-level. dsh's session cookie
 *  is SameSite=Strict, so it can never authenticate inside a cross-site
 *  iframe; the window itself must navigate there. */
function navigateToApp(url: string): void {
  invoke("navigate_main", { url, asApp: true }).catch(() => {});
}

async function start(): Promise<void> {
  ready = false;
  if (startupErrorTimer !== undefined) {
    window.clearTimeout(startupErrorTimer);
    startupErrorTimer = undefined;
  }
  setStatus("starting", t("status.starting"));
  setLoading(t("loading.starting"));
  appendLog("$ dsh web --no-open", "sys");
  try {
    const s = await invoke<any>("start_server");
    appendLog("> " + t("log.waitingPort", { port: s.port }), "sys");
  } catch (e) {
    showStartupError(t("err.startFailed") + String(e));
  }
}

async function stop(): Promise<void> {
  appendLog("$ " + t("log.stopService"), "sys");
  ready = false;
  const s = await invoke<any>("stop_server");
  currentUrl = APP_URL;
  runningNow = !!s.running;
  if (s.running) {
    appendLog("> " + t("log.notOwned"), "sys");
    setStatus("running", t("status.running") + APP_URL);
  } else {
    setStatus("stopped", t("status.stopped"));
  }
}

async function restart(): Promise<void> {
  await stop();
  await start();
}

/**
 * 启动前的环境准备：检查 Node.js 版本，然后确保全局 dsh 已安装。
 * 返回 true 表示可以继续启动，false 表示已显示错误提示需用户处理。
 */
async function prepare(): Promise<boolean> {
  // 1. 检查 Node.js
  let node: { installed: boolean; version: string; supported: boolean; message: string };
  try {
    node = await invoke("check_node");
  } catch (e) {
    showStartupError(t("err.checkNodeFailed") + String(e));
    return false;
  }
  if (!node.installed) {
    showStartupError(node.message, { nodejs: true });
    return false;
  }
  if (!node.supported) {
    showStartupError(node.message, { nodejs: true });
    return false;
  }
  appendLog("> " + t("log.nodeOk", { version: node.version }), "sys");

  // 2. 确保全局 dsh 已安装（未安装时后台会发 install:status 更新文案）
  setStatus("starting", t("status.preparing"));
  setLoading(t("loading.checkingDsh"));
  try {
    const d = await invoke<{ installed: boolean; version: string; message: string }>("ensure_dsh");
    appendLog("> " + d.message + " (dsh v" + d.version + ")", "sys");
  } catch (e) {
    showStartupError(t("err.installDshFailed") + String(e));
    return false;
  }
  return true;
}

/**
 * 完整启动流程：前置检查 + 复用/启动服务。
 * `returned` 表示主窗口是从 dsh 应用退回壳页面的（停止/异常退出）：
 * 不自动重启，改为展示状态并等待用户操作。
 */
async function boot(returned = false): Promise<void> {
  const tab = PARAMS.get("tab");
  if (tab === "cli" || tab === "plugins") {
    switchTab(tab);
    await invoke("set_pinned", { pinned: true }).catch(() => {});
  }

  // 服务已在运行则直接复用，无需前置检查。
  try {
    const s = await invoke<any>("server_status");
    if (s.running) {
      if (s.needs_auth) {
        showStartupError(t("err.authRequired"));
        spawnShellOnce();
        return;
      }
      currentUrl = s.url || APP_URL;
      ready = true;
      runningNow = true;
      setStatus("running", t("status.running") + APP_URL);
      appendLog("> " + t("log.reused", { url: s.url }), "sys");
      void spawnShellOnce();
      void refreshDshVersion();
      // 顶层加载 dsh（除非用户明确停留在 CLI/插件页）。
      if (tab !== "cli" && tab !== "plugins") navigateToApp(currentUrl);
      else loading.style.display = "none";
      return;
    }
  } catch {
    // 忽略，走正常启动流程
  }

  if (returned) {
    // 返回壳页面且服务已不在运行：展示停止状态并提供重试，不自动重启，
    // 否则会掩盖退出日志（或陷入崩溃重启循环）。
    runningNow = false;
    setStatus("stopped", t("status.exited"));
    setLoading(t("loading.stopped"), { retry: true, spinner: false });
    showCliHint();
    await spawnShellOnce();
    return;
  }

  if (!(await prepare())) return;
  await start();
}

/** Show the "check the CLI tab" hint under an error/stopped overlay. */
function showCliHint(): void {
  q<HTMLParagraphElement>("#loading-hint").hidden = false;
}

async function upgrade(): Promise<void> {
  if (upgrading) return;
  upgrading = true;
  const btn = q<HTMLButtonElement>("#btn-upgrade");
  btn.disabled = true;
  appendLog("> " + t("log.checkingUpgrade"), "sys");
  try {
    const r = await invoke<any>("upgrade_dsh");
    if (r.ok) {
      appendLog("> " + t("log.installedVersion", { version: r.version, message: r.message }), "sys");
      if (r.restarted) {
        // The old process is gone; `server:ready` will arrive with the new
        // (possibly token-authenticated) URL and Rust will navigate the
        // main window back into the app.
        currentUrl = APP_URL;
        runningNow = true;
        setStatus("starting", t("status.starting"));
        setLoading(t("loading.starting"));
      }
    } else {
      appendLog("> " + t("log.upgradeFailed") + r.message, "err");
    }
  } catch (e2) {
    appendLog("> " + t("log.upgradeFailed") + String(e2), "err");
  } finally {
    upgrading = false;
    btn.disabled = false;
  }
}

function clearLog(): void {
  cliTerm?.reset();
}

async function copyLog(): Promise<void> {
  try {
    cliTerm?.selectAll();
    const text = cliTerm?.getSelection() ?? "";
    cliTerm?.clearSelection();
    await navigator.clipboard.writeText(text);
    appendLog("> " + t("log.copied"), "sys");
  } catch {
    appendLog("> " + t("log.copyFailed"), "err");
  }
}

function switchTab(key: string): void {
  const panels: Record<string, HTMLElement> = {
    app: q("#panel-app"),
    cli: q("#panel-cli"),
    plugins: q("#panel-plugins"),
  };
  document
    .querySelectorAll<HTMLButtonElement>(".tab")
    .forEach((t) => t.classList.toggle("active", t.dataset.tab === key));
  Object.keys(panels).forEach((k) => panels[k].classList.toggle("active", k === key));
  window.requestAnimationFrame(fitTerminals);
  if (key === "cli" || key === "plugins") {
    // Staying on the shell deliberately: stop Rust from yanking the window
    // into the app when the server becomes ready.
    invoke("set_pinned", { pinned: true }).catch(() => {});
    if (key === "plugins") refreshPlugins();
  } else {
    invoke("set_pinned", { pinned: false }).catch(() => {});
    // The dsh app is loaded top-level, so "showing" it means navigating
    // the whole window there.
    if (runningNow) navigateToApp(currentUrl);
  }
}

function setupTabs(): void {
  document.querySelectorAll<HTMLButtonElement>(".tab").forEach((t) => {
    t.addEventListener("click", () => switchTab(t.dataset.tab || "app"));
  });
}

async function setupEvents(): Promise<void> {
  await listen<number[]>("term:data", (e) => {
    cliTerm?.write(new Uint8Array(e.payload));
  });
  await listen<string | null>("server:ready", (e) => {
    if (startupErrorTimer !== undefined) {
      window.clearTimeout(startupErrorTimer);
      startupErrorTimer = undefined;
    }
    ready = true;
    runningNow = true;
    currentUrl = e.payload || APP_URL;
    setStatus("running", t("status.running") + APP_URL);
    appendLog("> " + t("log.ready") + currentUrl, "sys");
    void spawnShellOnce();
    void refreshDshVersion();
    // No navigation here: Rust's open_app_when_ready already moves the
    // main window into the app unless the user is pinned to the shell.
  });
  await listen("server:auth-error", () => {
    if (startupErrorTimer !== undefined) {
      window.clearTimeout(startupErrorTimer);
      startupErrorTimer = undefined;
    }
    showStartupError(t("err.authRequired"));
    spawnShellOnce();
  });
  await listen("server:timeout", () => {
    if (startupErrorTimer !== undefined) {
      window.clearTimeout(startupErrorTimer);
      startupErrorTimer = undefined;
    }
    spawnShellOnce();
    showStartupError(t("err.startupTimeout"));
  });
  await listen<number | null>("server:exited", (e) => {
    runningNow = false;
    if (ready) {
      setStatus("stopped", t("status.exited"));
      appendLog("> " + t("log.processExited", { code: e.payload ?? 0 }), "sys");
    } else {
      // A restart kills the old process (and the `dsh web` launcher exits
      // right after the server is up), so an exit event can arrive while
      // `ready` is still false even though the new server is coming up
      // fine. Wait a grace period; `server:ready` cancels this so a
      // successful restart is not reported as a failed startup.
      if (startupErrorTimer !== undefined) window.clearTimeout(startupErrorTimer);
      startupErrorTimer = window.setTimeout(() => {
        startupErrorTimer = undefined;
        spawnShellOnce();
        showStartupError(t("err.exitedEarly", { code: e.payload ?? 0 }));
      }, 3000);
    }
  });
  await listen("server:stopped", () => {
    runningNow = false;
    if (!ready) setLoading(t("loading.stopped"), { spinner: false });
    setStatus("stopped", t("status.stopped"));
  });
  await listen<string>("install:status", (e2) => {
    if (!ready) setLoading(e2.payload);
  });
}

function setupButtons(): void {
  q("#btn-start").addEventListener("click", start);
  q("#btn-stop").addEventListener("click", stop);
  q("#btn-restart").addEventListener("click", restart);
  q("#btn-upgrade").addEventListener("click", upgrade);
  q("#btn-clear").addEventListener("click", clearLog);
  q("#btn-copy").addEventListener("click", copyLog);
  q("#btn-retry").addEventListener("click", () => void boot(false));
  q("#btn-nodejs").addEventListener("click", async () => {
    try {
      await invoke("open_nodejs_website");
    } catch (e) {
      appendLog("> " + t("log.openBrowserFailed") + String(e), "err");
    }
  });
  q("#btn-add-plugin").addEventListener("click", addPlugin);
  q("#btn-refresh-plugins").addEventListener("click", refreshPlugins);
  q<HTMLInputElement>("#plugin-name").addEventListener("keydown", (e) => {
    if (e.key === "Enter") addPlugin();
  });
}

interface PluginEntry {
  name: string;
  version: string;
  is_bundle: boolean;
}

/** Render the user-installed plugin list from the web profile. */
async function refreshPlugins(): Promise<void> {
  const list = q<HTMLDivElement>("#plugins-list");
  try {
    const plugins = await invoke<PluginEntry[]>("list_plugins");
    list.innerHTML = "";
    if (plugins.length === 0) {
      const p = document.createElement("p");
      p.className = "plugins-empty";
      p.textContent = t("plugins.empty");
      list.appendChild(p);
      return;
    }
    for (const pl of plugins) {
      const row = document.createElement("div");
      row.className = "plugin-row";

      const info = document.createElement("div");
      info.className = "plugin-info";
      const name = document.createElement("span");
      name.className = "plugin-name";
      name.textContent = pl.name;
      info.appendChild(name);
      if (pl.version) {
        const ver = document.createElement("span");
        ver.className = "plugin-version";
        ver.textContent = "v" + pl.version;
        info.appendChild(ver);
      }
      if (pl.is_bundle) {
        const badge = document.createElement("span");
        badge.className = "plugin-badge";
        badge.textContent = "bundle";
        info.appendChild(badge);
      }

      const btn = document.createElement("button");
      btn.className = "plugin-remove";
      btn.textContent = t("plugins.remove");
      btn.addEventListener("click", () => handleRemove(pl.name, btn));

      row.appendChild(info);
      row.appendChild(btn);
      list.appendChild(row);
    }
  } catch (e) {
    list.innerHTML = "";
    const p = document.createElement("p");
    p.className = "plugins-empty error-text";
    p.textContent = t("plugins.loadFailed") + String(e);
    list.appendChild(p);
  }
}

function validPackageName(name: string): boolean {
  return (
    name.length > 0 &&
    !name.startsWith(".") &&
    /^[a-zA-Z0-9@/:._\-+#^~%=?]+$/.test(name)
  );
}

async function addPlugin(): Promise<void> {
  const input = q<HTMLInputElement>("#plugin-name");
  const name = input.value.trim();
  if (!name) {
    appendLog("> " + t("plugins.enterName"), "sys");
    return;
  }
  if (!validPackageName(name)) {
    appendLog("> " + t("plugins.invalidName"), "err");
    return;
  }
  const btn = q<HTMLButtonElement>("#btn-add-plugin");
  btn.disabled = true;
  input.value = "";
  switchTab("cli");
  const cmd = `dsh plugin --profile web add '${name}'\r\n`;
  await spawnShellOnce();
  try {
    await invoke("term_input", {
      data: Array.from(new TextEncoder().encode(cmd)),
    });
  } catch (e) {
    appendLog("> " + String(e), "err");
  }
  // The command runs in the shell; refresh once it has likely finished.
  window.setTimeout(() => void refreshPlugins(), 10000);
  btn.disabled = false;
}

/** Two-step confirm removal: first click arms, second click confirms. */
async function handleRemove(name: string, btn: HTMLButtonElement): Promise<void> {
  if (btn.dataset.confirm !== "1") {
    btn.dataset.confirm = "1";
    btn.textContent = t("plugins.confirmRemove");
    btn.classList.add("confirm");
    window.setTimeout(() => {
      if (btn.isConnected) {
        delete btn.dataset.confirm;
        btn.textContent = t("plugins.remove");
        btn.classList.remove("confirm");
      }
    }, 2000);
    return;
  }
  btn.disabled = true;
  switchTab("cli");
  const cmd = `dsh plugin --profile web remove '${name}'\r\n`;
  await spawnShellOnce();
  try {
    await invoke("term_input", {
      data: Array.from(new TextEncoder().encode(cmd)),
    });
  } catch (e) {
    appendLog("> " + String(e), "err");
  }
  window.setTimeout(() => void refreshPlugins(), 10000);
  delete btn.dataset.confirm;
  btn.textContent = t("plugins.remove");
  btn.classList.remove("confirm");
  btn.disabled = false;
}

/** Show the DeepSeek Harness (dsh) version in the topbar, not the app's own. */
async function refreshDshVersion(): Promise<void> {
  const el = q<HTMLSpanElement>("#version");
  try {
    const v = await invoke<string>("dsh_version");
    el.textContent = v && v !== "unknown" ? "dsh v" + v : t("version.unknown");
  } catch {
    el.textContent = t("version.unknown");
  }
}

// ===== CLI 终端（交互式 shell） =====
let cliTerm: Terminal | null = null;
let cliTermFit: FitAddon | null = null;
let shellSpawned = false;

/** Start the interactive CLI shell exactly once. Deferred until the service
 *  is ready (or the app reuses an already-running service) so the shell's
 *  first prompt is drawn on a clean line — otherwise a partial line from
 *  `dsh web` output (no trailing newline) makes zsh print `%` + padding
 *  via PROMPT_SP. Kept app-side so users never need to touch their shell. */
async function spawnShellOnce(): Promise<void> {
  if (shellSpawned) return;
  shellSpawned = true;
  try {
    const r = await invoke<string>("spawn_shell");
    // Returning from the app page reloads the shell but reuses the PTY:
    // the fresh terminal is blank, so nudge the shell to redraw its prompt.
    if (r === "reused") window.setTimeout(() => cliTerm?.write("\r"), 300);
  } catch (e) {
    appendLog("> " + String(e), "err");
  }
}

function initTerminals(): void {
  const el = q<HTMLDivElement>("#cli-term");
  cliTerm = new Terminal({
    fontSize: 12,
    // SF Mono/Menlo/monospace have no CJK glyphs; WKWebView's canvas renders
    // missing glyphs as "?" — append CJK fallbacks so Chinese works.
    fontFamily:
      '"SF Mono", ui-monospace, Menlo, Consolas, "PingFang SC", "Hiragino Sans GB", "Noto Sans Mono CJK SC", "Noto Sans CJK SC", "Microsoft YaHei", monospace',
    cursorBlink: true,
    theme: { background: "#0d1117", foreground: "#e6edf3" },
  });
  cliTermFit = new FitAddon();
  cliTerm.loadAddon(cliTermFit);
  cliTerm.open(el);
  cliTermFit.fit();
  cliTerm.onData((data) => {
    const bytes = Array.from(new TextEncoder().encode(data));
    invoke("term_input", { data: bytes }).catch(() => {});
  });
  cliTerm.onResize(({ cols, rows }) => {
    if (cols > 0 && rows > 0) {
      invoke("term_resize", { cols, rows }).catch(() => {});
    }
  });
}

function fitTerminals(): void {
  if (q<HTMLElement>("#panel-cli").classList.contains("active")) cliTermFit?.fit();
}

// ===== 浮动工具栏窗口（index.html?view=bar） =====
async function initBar(): Promise<void> {
  const bdot = q<HTMLSpanElement>("#bar-dot");
  const bstatus = q<HTMLSpanElement>("#bar-status");
  let barRunning = false;
  let barUrl = APP_URL;
  const paint = (state: State, text: string): void => {
    bdot.className = "dot " + state;
    bstatus.textContent = text;
  };

  try {
    const s = await invoke<any>("server_status");
    if (s.running) {
      barRunning = true;
      barUrl = s.url || APP_URL;
      paint("running", t("bar.running"));
    } else {
      paint("stopped", t("bar.stopped"));
    }
  } catch {
    paint("error", t("bar.error"));
  }

  await listen<string | null>("server:ready", (e) => {
    barRunning = true;
    barUrl = e.payload || APP_URL;
    paint("running", t("bar.running"));
  });
  await listen("server:auth-error", () => paint("error", t("bar.error")));
  await listen("server:timeout", () => paint("error", t("bar.error")));
  await listen("server:stopped", () => {
    barRunning = false;
    paint("stopped", t("bar.stopped"));
  });
  await listen<number | null>("server:exited", () => {
    barRunning = false;
    paint("stopped", t("bar.stopped"));
  });
  await listen<string>("install:status", (e) => paint("starting", e.payload));

  q("#bar-app").addEventListener("click", async () => {
    try {
      await invoke("set_pinned", { pinned: false });
      if (barRunning) {
        await invoke("navigate_main", { url: barUrl, asApp: true });
      } else {
        paint("starting", t("bar.starting"));
        await invoke("start_server");
      }
    } catch {
      paint("error", t("bar.error"));
    }
  });
  const gotoTab = (tab: string) => (): void => {
    const url = shellUrlBase() + "?tab=" + tab;
    invoke("navigate_main", { url, asApp: false }).catch(() => {});
  };
  q("#bar-cli").addEventListener("click", gotoTab("cli"));
  q("#bar-plugins").addEventListener("click", gotoTab("plugins"));
  q("#bar-restart").addEventListener("click", async () => {
    paint("starting", t("bar.starting"));
    try {
      await invoke("restart_server");
    } catch {
      paint("error", t("bar.error"));
    }
  });
  q("#bar-stop").addEventListener("click", async () => {
    try {
      const s = await invoke<any>("stop_server");
      barRunning = !!s.running;
      paint(barRunning ? "running" : "stopped", t(barRunning ? "bar.running" : "bar.stopped"));
    } catch {
      paint("error", t("bar.error"));
    }
  });
  q("#bar-hide").addEventListener("click", () => {
    invoke("hide_bar_window").catch(() => {});
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  applyTranslations();
  // 同步语言给 Rust 端，保证前后端消息语言一致
  await invoke("set_ui_lang", { isZh: LANG === "zh" });
  if (BAR_VIEW) {
    await initBar();
    return;
  }
  // 报告壳页面地址，服务停止/退出时 Rust 用它把主窗口导航回来
  invoke("register_shell_url", { url: shellUrlBase() }).catch(() => {});
  setupTabs();
  setupButtons();
  initTerminals();
  window.addEventListener("resize", fitTerminals);
  q("#version").textContent = "dsh …";
  await setupEvents();
  // ret=1：主窗口是从 dsh 应用退回来的，不自动重启服务。
  await boot(PARAMS.get("ret") === "1");
});
