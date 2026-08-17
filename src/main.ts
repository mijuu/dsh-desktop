import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Lang = "en" | "zh";

function detectLang(): Lang {
  const l = (navigator.language || "en").toLowerCase();
  return l.startsWith("zh") ? "zh" : "en";
}

const LANG: Lang = detectLang();

const I18N: Record<Lang, Record<string, string>> = {
  en: {
    "tab.plugins": "Plugins",
    "status.starting": "Starting\u2026",
    "status.running": "Running \u00b7 ",
    "status.stopped": "Stopped",
    "status.exited": "Exited",
    "status.error": "Startup failed",
    "status.preparing": "Preparing dsh environment\u2026",
    "bar.close": "Hide toolbar",
    "bar.show": "Show toolbar",
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
    "plugins.input": "Enter npm package name, e.g. @scope/name",
    "plugins.hint": "Restart the service after installing / uninstalling plugins. Installation logs are in the CLI tab.",
    "plugins.empty": "No plugins yet. Enter a package name and click Install.",
    "plugins.loadFailed": "Failed to load plugin list: ",
    "plugins.remove": "Uninstall",
    "plugins.confirmRemove": "Confirm",
    "plugins.enterName": "Please enter a package name.",
    "plugins.installed": "Plugin {name} installed. Restart to take effect.",
    "plugins.installFailed": "Failed to install plugin: ",
    "plugins.uninstalled": "Plugin {name} uninstalled. Restart to take effect.",
    "plugins.uninstallFailed": "Failed to uninstall plugin: ",
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
    "version.unknown": "dsh unknown",
  },
  zh: {
    "tab.plugins": "\u63d2\u4ef6",
    "status.starting": "\u542f\u52a8\u4e2d\u2026",
    "status.running": "\u8fd0\u884c\u4e2d \u00b7 ",
    "status.stopped": "\u5df2\u505c\u6b62",
    "status.exited": "\u5df2\u9000\u51fa",
    "status.error": "\u542f\u52a8\u5931\u8d25",
    "status.preparing": "\u51c6\u5907 dsh \u73af\u5883\u2026",
    "bar.close": "\u9690\u85cf\u5de5\u5177\u680f",
    "bar.show": "\u663e\u793a\u5de5\u5177\u680f",
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
    "plugins.input": "\u8f93\u5165 npm \u5305\u540d\uff0c\u5982 @scope/name",
    "plugins.hint": "\u5b89\u88c5 / \u5378\u8f7d\u63d2\u4ef6\u540e\u9700\u91cd\u542f\u670d\u52a1\u624d\u80fd\u751f\u6548\uff1b\u5b89\u88c5\u8fc7\u7a0b\u65e5\u5fd7\u89c1\u300cCLI\u300d\u9875\u7b7e\u3002",
    "plugins.empty": "\u6682\u65e0\u63d2\u4ef6\uff0c\u8f93\u5165\u5305\u540d\u540e\u70b9\u51fb\u300c\u5b89\u88c5\u300d\u3002",
    "plugins.loadFailed": "\u52a0\u8f7d\u63d2\u4ef6\u5217\u8868\u5931\u8d25\uff1a",
    "plugins.remove": "\u5378\u8f7d",
    "plugins.confirmRemove": "\u786e\u8ba4\u5378\u8f7d",
    "plugins.enterName": "\u8bf7\u8f93\u5165\u63d2\u4ef6\u5305\u540d\u3002",
    "plugins.installed": "\u63d2\u4ef6 {name} \u5b89\u88c5\u5b8c\u6210\uff0c\u91cd\u542f\u670d\u52a1\u540e\u751f\u6548",
    "plugins.installFailed": "\u5b89\u88c5\u63d2\u4ef6\u5931\u8d25\uff1a",
    "plugins.uninstalled": "\u63d2\u4ef6 {name} \u5df2\u5378\u8f7d\uff0c\u91cd\u542f\u670d\u52a1\u540e\u751f\u6548",
    "plugins.uninstallFailed": "\u5378\u8f7d\u63d2\u4ef6\u5931\u8d25\uff1a",
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

function q<T extends HTMLElement>(sel: string): T {
  return document.querySelector(sel) as T;
}

const iframe = q<HTMLIFrameElement>("#app-iframe");
const loading = q<HTMLDivElement>("#loading");
const log = q<HTMLPreElement>("#log");
const dot = q<HTMLSpanElement>("#status-dot");
const statusText = q<HTMLSpanElement>("#status-text");
const topbar = q<HTMLElement>("#topbar");
const grabber = q<HTMLDivElement>("#grabber");
const loadingSpinner = q<HTMLDivElement>("#loading-spinner");
const loadingText = q<HTMLParagraphElement>("#loading-text");

type State = "starting" | "running" | "stopped" | "error";

let upgrading = false;
let cliMode = false;
let ready = false;
let hideTimer: number | undefined;

function setStatus(state: State, text: string): void {
  dot.className = "dot " + state;
  statusText.textContent = text;
}

function appendLog(line: string, kind: "out" | "err" | "sys"): void {
  const span = document.createElement("span");
  span.className = kind;
  span.textContent = line + "\n";
  log.appendChild(span);
  while (log.childElementCount > 2000) {
    if (log.firstElementChild) log.removeChild(log.firstElementChild);
  }
  log.scrollTop = log.scrollHeight;
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
  setStatus("error", t("status.error"));
  setLoading(message, { error: true, retry: true, nodejs: opts.nodejs });
  showBar();
  appendLog("> " + message, "err");
}

function showApp(): void {
  loading.style.display = "none";
  if (iframe.src !== APP_URL) iframe.src = APP_URL;
}

function showBar(): void {
  if (hideTimer !== undefined) {
    window.clearTimeout(hideTimer);
    hideTimer = undefined;
  }
  topbar.classList.add("visible");
  grabber.classList.add("hidden");
}

function scheduleHide(): void {
  if (hideTimer !== undefined) window.clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => {
    topbar.classList.remove("visible");
    grabber.classList.remove("hidden");
    hideTimer = undefined;
  }, 1500);
}

function setupBar(): void {
  grabber.addEventListener("mouseenter", showBar);
  grabber.addEventListener("click", showBar);
  topbar.addEventListener("mouseenter", () => {
    if (hideTimer !== undefined) {
      window.clearTimeout(hideTimer);
      hideTimer = undefined;
    }
  });
  topbar.addEventListener("mouseleave", () => {
    if (!cliMode) scheduleHide();
  });
  q("#bar-close").addEventListener("click", () => {
    topbar.classList.remove("visible");
    grabber.classList.remove("hidden");
  });
}

async function start(): Promise<void> {
  ready = false;
  setStatus("starting", t("status.starting"));
  setLoading(t("loading.starting"));
  appendLog("$ dsh web", "sys");
  try {
    const s = await invoke<any>("start_server");
    appendLog("> " + t("log.waitingPort", { port: s.port }), "sys");
  } catch (e) {
    showStartupError(t("err.startFailed") + String(e));
  }
}

async function stop(): Promise<void> {
  appendLog("$ " + t("log.stopService"), "sys");
  const s = await invoke<any>("stop_server");
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
 */
async function boot(): Promise<void> {
  // 服务已在运行则直接复用，无需前置检查。
  try {
    const s = await invoke<any>("server_status");
    if (s.running) {
      showApp();
      setStatus("running", t("status.running") + s.url);
      appendLog("> " + t("log.reused", { url: s.url }), "sys");
      return;
    }
  } catch {
    // 忽略，走正常启动流程
  }

  if (!(await prepare())) return;
  await start();
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
        iframe.src = "about:blank";
        showApp();
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
  log.textContent = "";
}

async function copyLog(): Promise<void> {
  try {
    await navigator.clipboard.writeText(log.textContent || "");
    appendLog("> " + t("log.copied"), "sys");
  } catch {
    appendLog("> " + t("log.copyFailed"), "err");
  }
}

function setupTabs(): void {
  const tabs = document.querySelectorAll<HTMLButtonElement>(".tab");
  const panels: Record<string, HTMLElement> = {
    app: q("#panel-app"),
    cli: q("#panel-cli"),
    plugins: q("#panel-plugins"),
  };
  tabs.forEach((t) => {
    t.addEventListener("click", () => {
      tabs.forEach((x) => x.classList.remove("active"));
      t.classList.add("active");
      const key = t.dataset.tab || "app";
      Object.keys(panels).forEach((k) => panels[k].classList.toggle("active", k === key));
      if (key === "cli" || key === "plugins") {
        cliMode = true;
        showBar();
        if (key === "plugins") refreshPlugins();
      } else {
        cliMode = false;
        scheduleHide();
      }
    });
  });
}

async function setupEvents(): Promise<void> {
  await listen<string>("server:stdout", (e) => appendLog(e.payload, "out"));
  await listen<string>("server:stderr", (e) => appendLog(e.payload, "err"));
  await listen("server:ready", () => {
    ready = true;
    setStatus("running", t("status.running") + APP_URL);
    showApp();
    appendLog("> " + t("log.ready") + APP_URL, "sys");
    refreshDshVersion();
    if (!cliMode) scheduleHide();
  });
  await listen("server:timeout", () => {
    showStartupError(t("err.startupTimeout"));
  });
  await listen<number | null>("server:exited", (e) => {
    if (ready) {
      setStatus("stopped", t("status.exited"));
      appendLog("> " + t("log.processExited", { code: e.payload ?? 0 }), "sys");
    } else {
      showStartupError(t("err.exitedEarly", { code: e.payload ?? 0 }));
    }
  });
  await listen("server:stopped", () => {
    if (!ready) setLoading(t("loading.stopped"), { spinner: false });
    setStatus("stopped", t("status.stopped"));
  });
  await listen<string>("upgrade:stdout", (e2) => appendLog(e2.payload, "out"));
  await listen<string>("upgrade:stderr", (e2) => appendLog(e2.payload, "err"));
  await listen<string>("install:stdout", (e2) => appendLog(e2.payload, "out"));
  await listen<string>("install:stderr", (e2) => appendLog(e2.payload, "err"));
  await listen<string>("install:status", (e2) => {
    if (!ready) setLoading(e2.payload);
  });
  await listen<string>("plugin:stdout", (e2) => appendLog(e2.payload, "out"));
  await listen<string>("plugin:stderr", (e2) => appendLog(e2.payload, "err"));
  await listen<string>("plugin:status", (e2) => appendLog("> " + e2.payload, "sys"));
}

function setupButtons(): void {
  q("#btn-start").addEventListener("click", start);
  q("#btn-stop").addEventListener("click", stop);
  q("#btn-restart").addEventListener("click", restart);
  q("#btn-upgrade").addEventListener("click", upgrade);
  q("#btn-clear").addEventListener("click", clearLog);
  q("#btn-copy").addEventListener("click", copyLog);
  q("#btn-retry").addEventListener("click", boot);
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

async function addPlugin(): Promise<void> {
  const input = q<HTMLInputElement>("#plugin-name");
  const name = input.value.trim();
  if (!name) {
    appendLog("> " + t("plugins.enterName"), "sys");
    return;
  }
  const btn = q<HTMLButtonElement>("#btn-add-plugin");
  btn.disabled = true;
  try {
    await invoke("add_plugin", { name });
    appendLog("> " + t("plugins.installed", { name }), "sys");
    input.value = "";
    await refreshPlugins();
  } catch (e) {
    appendLog("> " + t("plugins.installFailed") + String(e), "err");
  } finally {
    btn.disabled = false;
  }
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
  try {
    await invoke("remove_plugin", { name });
    appendLog("> " + t("plugins.uninstalled", { name }), "sys");
    await refreshPlugins();
  } catch (e) {
    appendLog("> " + t("plugins.uninstallFailed") + String(e), "err");
    btn.disabled = false;
    delete btn.dataset.confirm;
    btn.textContent = t("plugins.remove");
    btn.classList.remove("confirm");
  }
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

window.addEventListener("DOMContentLoaded", async () => {
  applyTranslations();
  // 同步语言给 Rust 端，保证前后端消息语言一致
  await invoke("set_ui_lang", { isZh: LANG === "zh" });
  setupTabs();
  setupButtons();
  setupBar();
  // The bar is visible at startup so users discover the controls,
  // then auto-hides after 5s (or on mouseleave / × button).
  showBar();
  window.setTimeout(() => {
    if (!cliMode) scheduleHide();
  }, 5000);
  q("#version").textContent = "dsh …";
  await setupEvents();
  await boot();
});
