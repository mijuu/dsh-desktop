#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

const PORT: u16 = 3080;

fn url() -> String {
    format!("http://127.0.0.1:{PORT}")
}

struct ServerState {
    pid: Mutex<Option<u32>>,
}

#[derive(Clone, serde::Serialize)]
struct Status {
    running: bool,
    port: u16,
    url: String,
}

fn status_of(running: bool) -> Status {
    Status { running, port: PORT, url: url() }
}

#[derive(Clone, serde::Serialize)]
struct UpgradeResult {
    ok: bool,
    version: String,
    restarted: bool,
    message: String,
}

/// Base npx launcher for non-Windows (macOS/Linux). Prefers an explicit
/// fnm path so the app also works when launched from Finder/Dock, where the
/// GUI process has a minimal PATH without node/npx; falls back to plain npx.
#[cfg(not(windows))]
fn base_npx_cmd() -> Command {
    for fnm in [
        "/opt/homebrew/bin/fnm",
        "/opt/homebrew/opt/fnm/bin/fnm",
        "/usr/local/bin/fnm",
    ] {
        if Path::new(fnm).is_file() {
            let mut c = Command::new(fnm);
            c.args(["exec", "--using", "default", "--", "npx"]);
            return c;
        }
    }
    Command::new("npx")
}

/// Windows: GUI-launched processes may inherit a stale PATH (Node.js not
/// visible), and Rust's Command::new("npx") cannot resolve the npx.cmd batch
/// shim the way cmd.exe does. So run through "cmd /C npx ..." — exactly like
/// a user typing it in a terminal — and merge the standard Node.js install
/// directories into PATH as a safety net.
#[cfg(windows)]
fn base_npx_cmd() -> Command {
    let mut c = Command::new("cmd");
    c.args(["/C", "npx"]);
    augment_path_with_node(&mut c);
    c
}

#[cfg(windows)]
fn augment_path_with_node(c: &mut Command) {
    let mut dirs: Vec<String> = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(base) = std::env::var(var) {
            let candidate = if var == "LOCALAPPDATA" {
                format!("{base}/Programs/nodejs")
            } else {
                format!("{base}/nodejs")
            };
            if Path::new(&candidate).is_dir() {
                dirs.push(candidate);
            }
        }
    }
    if dirs.is_empty() {
        return;
    }
    let mut path = dirs.join(";");
    if let Ok(p) = std::env::var("PATH") {
        if !p.is_empty() {
            path.push(';');
            path.push_str(&p);
        }
    }
    c.env("PATH", path);
}

fn dsh_command() -> Command {
    let mut c = base_npx_cmd();
    c.args(["--yes", "@deepseek-ai/dsh", "web"]);
    c
}

fn upgrade_command() -> Command {
    let mut c = base_npx_cmd();
    c.args(["--yes", "--latest", "@deepseek-ai/dsh", "--", "--version"]);
    c.env("npm_config_update_notifier", "false");
    c
}

fn version_command() -> Command {
    let mut c = base_npx_cmd();
    c.args(["--yes", "@deepseek-ai/dsh", "--", "--version"]);
    c.env("npm_config_update_notifier", "false");
    c
}

fn extract_version(lines: &[String]) -> Option<String> {
    for line in lines.iter().rev() {
        let t = line.trim();
        let candidate = t
            .strip_prefix("v")
            .or_else(|| t.strip_prefix("version "))
            .unwrap_or(t)
            .trim();
        if looks_like_version(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn looks_like_version(s: &str) -> bool {
    let mut saw_digit = false;
    let mut dots = 0u32;
    for c in s.chars() {
        if c.is_ascii_digit() {
            saw_digit = true;
        } else if c == '.' {
            dots += 1;
        } else if c == '-' || c == '+' || c.is_ascii_alphabetic() {
            // semver prerelease / build / letters allowed
        } else {
            return false;
        }
    }
    saw_digit && dots >= 1
}

fn is_port_open(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

#[cfg(unix)]
fn kill_process_group(pid: u32) {
    unsafe { libc::kill(-(pid as i32), libc::SIGTERM); }
    std::thread::sleep(Duration::from_millis(400));
    unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
}

#[cfg(not(unix))]
fn kill_process_group(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status();
}

fn start_internal(app: &AppHandle) -> Result<Status, String> {
    {
        let state = app.state::<ServerState>();
        if state.pid.lock().unwrap().is_some() {
            return Ok(status_of(true));
        }
    }

    // Port already serving (e.g. the user's browser dsh session)? Reuse it
    // instead of spawning a duplicate that would fail with EADDRINUSE.
    if is_port_open(PORT) {
        let _ = app.emit("server:ready", ());
        return Ok(status_of(true));
    }

    let mut cmd = dsh_command();
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 npx @deepseek-ai/dsh web：{e}"))?;
    let pid = child.id();

    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        let _ = app.emit("server:stdout", l);
                    }
                    Err(_) => break,
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                match line {
                    Ok(l) => {
                        let _ = app.emit("server:stderr", l);
                    }
                    Err(_) => break,
                }
            }
        });
    }

    {
        let state = app.state::<ServerState>();
        let mut guard = state.pid.lock().unwrap();
        *guard = Some(pid);
    }

    // watcher: clear state and notify on exit
    {
        let app = app.clone();
        std::thread::spawn(move || {
            let code = child.wait().ok().and_then(|s| s.code());
            let state = app.state::<ServerState>();
            let mut guard = state.pid.lock().unwrap();
            *guard = None;
            drop(guard);
            let _ = app.emit("server:exited", code);
        });
    }

    // readiness polling; aborts early when the process exits before the
    // port opens (e.g. node/npx missing) so the UI reports failure at once
    {
        let app = app.clone();
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(90);
            loop {
                if is_port_open(PORT) {
                    let _ = app.emit("server:ready", ());
                    return;
                }
                if app.state::<ServerState>().pid.lock().unwrap().is_none() {
                    return;
                }
                if Instant::now() >= deadline {
                    let _ = app.emit("server:timeout", ());
                    return;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        });
    }

    Ok(status_of(true))
}

#[tauri::command]
fn start_server(app: AppHandle) -> Result<Status, String> {
    start_internal(&app)
}

#[tauri::command]
fn stop_server(app: AppHandle) -> Status {
    let state = app.state::<ServerState>();
    let pid = {
        let mut guard = state.pid.lock().unwrap();
        guard.take()
    };
    match pid {
        Some(pid) => {
            kill_process_group(pid);
            let _ = app.emit("server:stopped", ());
            status_of(false)
        }
        // Nothing we own: reflect whether 3080 is still up (reused server).
        None => status_of(is_port_open(PORT)),
    }
}

#[tauri::command]
fn restart_server(app: AppHandle) -> Result<Status, String> {
    stop_server(app.clone());
    std::thread::sleep(Duration::from_millis(600));
    start_internal(&app)
}

#[tauri::command]
async fn upgrade_dsh(app: AppHandle) -> Result<UpgradeResult, String> {
    let mut cmd = upgrade_command();
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    let mut child = cmd.spawn().map_err(|e| {
        format!("无法启动升级命令（npx --yes --latest @deepseek-ai/dsh --version）：{e}")
    })?;

    let collected: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        let collected = collected.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        let _ = app.emit("upgrade:stdout", &l);
                        collected.lock().unwrap().push(l);
                    }
                    Err(_) => break,
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        let collected = collected.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                match line {
                    Ok(l) => {
                        let _ = app.emit("upgrade:stderr", &l);
                        collected.lock().unwrap().push(l);
                    }
                    Err(_) => break,
                }
            }
        });
    }

    let exit = child.wait();
    let ok = matches!(&exit, Ok(s) if s.success());
    std::thread::sleep(Duration::from_millis(150));

    let lines = collected.lock().unwrap().clone();
    let version = extract_version(&lines).unwrap_or_else(|| "unknown".to_string());

    let mut restarted = false;
    let message;
    if ok {
        let owns = app.state::<ServerState>().pid.lock().unwrap().is_some();
        if owns {
            stop_server(app.clone());
            std::thread::sleep(Duration::from_millis(600));
            match start_internal(&app) {
                Ok(_) => {
                    restarted = true;
                    message = "升级完成，服务已用新版本重启".to_string();
                }
                Err(e) => {
                    message = format!("升级成功，但重启失败：{e}，请手动点击重启");
                }
            }
        } else {
            message =
                "升级完成，已安装最新版（当前无本应用运行的服务，下次启动即生效）".to_string();
        }
    } else {
        let code = exit.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        message = format!("升级失败（退出码 {code}），请检查网络后重试");
    }

    Ok(UpgradeResult { ok, version, restarted, message })
}

/// Report the DeepSeek Harness (dsh) CLI version that will actually run.
#[tauri::command]
async fn dsh_version() -> Result<String, String> {
    let mut cmd = version_command();
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("无法获取 dsh 版本：{e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("读取 dsh 版本失败：{e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    let version = extract_version(&lines).unwrap_or_else(|| "unknown".to_string());
    Ok(version)
}

#[tauri::command]
fn server_status(app: AppHandle) -> Status {
    let state = app.state::<ServerState>();
    let running = state.pid.lock().unwrap().is_some() || is_port_open(PORT);
    status_of(running)
}

fn main() {
    tauri::Builder::default()
        .manage(ServerState { pid: Mutex::new(None) })
        .invoke_handler(tauri::generate_handler![
            start_server,
            stop_server,
            restart_server,
            server_status,
            upgrade_dsh,
            dsh_version
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.app_handle().exit(0);
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                let state = app_handle.state::<ServerState>();
                let pid = {
                    let mut guard = state.pid.lock().unwrap();
                    guard.take()
                };
                if let Some(pid) = pid {
                    kill_process_group(pid);
                }
            }
        });
}