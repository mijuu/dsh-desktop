import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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

type State = "starting" | "running" | "stopped" | "error";

let upgrading = false;
let cliMode = false;
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
}

async function start(): Promise<void> {
  setStatus("starting", "启动中…");
  appendLog("$ npx @deepseek-ai/dsh web", "sys");
  try {
    const s = await invoke<any>("start_server");
    appendLog("> 等待端口 " + s.port + " 就绪…", "sys");
  } catch (e) {
    setStatus("error", "启动失败");
    appendLog(String(e), "err");
  }
}

async function stop(): Promise<void> {
  appendLog("$ 停止服务…", "sys");
  const s = await invoke<any>("stop_server");
  if (s.running) {
    appendLog("> 该服务非本应用启动，未停止", "sys");
    setStatus("running", "运行中 · " + APP_URL);
  } else {
    setStatus("stopped", "已停止");
  }
}

async function restart(): Promise<void> {
  await stop();
  await start();
}

async function upgrade(): Promise<void> {
  if (upgrading) return;
  upgrading = true;
  const btn = q<HTMLButtonElement>("#btn-upgrade");
  btn.disabled = true;
  appendLog("> 正在检查最新版本并安装…", "sys");
  try {
    const r = await invoke<any>("upgrade_dsh");
    if (r.ok) {
      appendLog("> 已安装版本 " + r.version + " · " + r.message, "sys");
      if (r.restarted) {
        iframe.src = "about:blank";
        showApp();
      }
    } else {
      appendLog("> 升级失败：" + r.message, "err");
    }
  } catch (e2) {
    appendLog("> 升级失败：" + String(e2), "err");
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
    appendLog("> 已复制到剪贴板", "sys");
  } catch {
    appendLog("> 复制失败", "err");
  }
}

function setupTabs(): void {
  const tabs = document.querySelectorAll<HTMLButtonElement>(".tab");
  const panels: Record<string, HTMLElement> = {
    app: q("#panel-app"),
    cli: q("#panel-cli"),
  };
  tabs.forEach((t) => {
    t.addEventListener("click", () => {
      tabs.forEach((x) => x.classList.remove("active"));
      t.classList.add("active");
      const key = t.dataset.tab || "app";
      Object.keys(panels).forEach((k) => panels[k].classList.toggle("active", k === key));
      if (key === "cli") {
        cliMode = true;
        showBar();
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
    setStatus("running", "运行中 · " + APP_URL);
    showApp();
    appendLog("> 就绪：" + APP_URL, "sys");
  });
  await listen("server:timeout", () => {
    setStatus("error", "启动超时");
    appendLog("> 启动超时：90 秒内未检测到端口监听", "err");
  });
  await listen<number | null>("server:exited", (e) => {
    setStatus("stopped", "已退出");
    appendLog("> 进程已退出 (code=" + e.payload + ")", "sys");
  });
  await listen("server:stopped", () => setStatus("stopped", "已停止"));
  await listen<string>("upgrade:stdout", (e2) => appendLog(e2.payload, "out"));
  await listen<string>("upgrade:stderr", (e2) => appendLog(e2.payload, "err"));
}

function setupButtons(): void {
  q("#btn-start").addEventListener("click", start);
  q("#btn-stop").addEventListener("click", stop);
  q("#btn-restart").addEventListener("click", restart);
  q("#btn-upgrade").addEventListener("click", upgrade);
  q("#btn-clear").addEventListener("click", clearLog);
  q("#btn-copy").addEventListener("click", copyLog);
}

window.addEventListener("DOMContentLoaded", async () => {
  setupTabs();
  setupButtons();
  setupBar();
  showBar();
  window.setTimeout(() => {
    if (!cliMode) {
      topbar.classList.remove("visible");
      grabber.classList.remove("hidden");
    }
  }, 2500);
  await setupEvents();
  try {
    const s = await invoke<any>("server_status");
    if (s.running) {
      showApp();
      setStatus("running", "运行中 · " + s.url);
      appendLog("> 检测到 " + s.url + " 已有服务，直接复用（该服务非本应用启动）", "sys");
    } else {
      await start();
    }
  } catch {
    await start();
  }
});