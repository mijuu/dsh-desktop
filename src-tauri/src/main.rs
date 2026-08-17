#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

const PORT: u16 = 3080;

/// Minimum supported Node.js version (dsh requires 22.19.0+).
const MIN_NODE_VERSION: (u64, u64, u64) = (22, 19, 0);

/// UI language state, set by the frontend via `set_ui_lang` (mirrors
/// `navigator.language`). Falls back to environment detection until then.
static UI_LANG_ZH: AtomicBool = AtomicBool::new(false);
static UI_LANG_SET: AtomicBool = AtomicBool::new(false);

/// Best-effort locale detection from the process environment (fallback only).
fn detect_zh_from_env() -> bool {
    #[cfg(not(windows))]
    {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if v.to_ascii_lowercase().starts_with("zh") {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(windows)]
    {
        std::env::var("LANG")
            .map(|v| v.to_ascii_lowercase().starts_with("zh"))
            .unwrap_or(false)
    }
}

/// Whether the UI language is Chinese. Prefers the value explicitly set by
/// the frontend so frontend and backend always agree.
fn ui_is_zh() -> bool {
    if UI_LANG_SET.load(Ordering::Relaxed) {
        UI_LANG_ZH.load(Ordering::Relaxed)
    } else {
        detect_zh_from_env()
    }
}

/// Pick the user-facing string for the detected UI language.
fn tr(en: &str, zh: &str) -> String {
    if ui_is_zh() { zh.to_string() } else { en.to_string() }
}

/// Set the UI language from the frontend (mirrors navigator.language).
#[tauri::command]
fn set_ui_lang(is_zh: bool) {
    UI_LANG_ZH.store(is_zh, Ordering::Relaxed);
    UI_LANG_SET.store(true, Ordering::Relaxed);
}

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

/// Result of the startup Node.js version check.
#[derive(Clone, serde::Serialize)]
struct NodeCheck {
    installed: bool,
    version: String,
    supported: bool,
    message: String,
}

/// Result of ensuring the global dsh CLI is installed.
#[derive(Clone, serde::Serialize)]
struct DshInstallResult {
    installed: bool,
    version: String,
    message: String,
}

/// A user-installed plugin in the web profile.
#[derive(Clone, serde::Serialize)]
struct PluginInfo {
    name: String,
    version: String,
    is_bundle: bool,
}

/// Resolve the dsh web profile directory (~/.dsh/profiles/web by default,
/// overridable via DSH_HOME).
fn profile_dir() -> PathBuf {
    if let Ok(dsh_home) = std::env::var("DSH_HOME") {
        return PathBuf::from(dsh_home).join("profiles").join("web");
    }
    let home = home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".dsh").join("profiles").join("web")
}

/// Best-effort home directory lookup across platforms.
fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Base command launcher for non-Windows (macOS/Linux). Prefers an explicit
/// fnm path so the app also works when launched from Finder/Dock, where the
/// GUI process has a minimal PATH without node/npm/dsh; falls back to the
/// bare binary name.
#[cfg(not(windows))]
fn base_cmd(bin: &str) -> Command {
    for fnm in [
        "/opt/homebrew/bin/fnm",
        "/opt/homebrew/opt/fnm/bin/fnm",
        "/usr/local/bin/fnm",
    ] {
        if Path::new(fnm).is_file() {
            let mut c = Command::new(fnm);
            c.args(["exec", "--using", "default", "--", bin]);
            return c;
        }
    }
    Command::new(bin)
}

/// Windows: GUI-launched processes may inherit a stale PATH (Node.js not
/// visible), and Rust's Command::new("npx") cannot resolve the .cmd batch shim
/// the way cmd.exe does. So run through "cmd /C <bin> ..." — exactly like a
/// user typing it in a terminal — and merge the standard Node.js install
/// directories into PATH as a safety net.
/// Windows: keep the child console hidden so no cmd window flashes up.
#[cfg(windows)]
fn hide_console(c: &mut Command) {
    use std::os::windows::process::CommandExt;
    c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}

#[cfg(windows)]
fn base_cmd(bin: &str) -> Command {
    let mut c = Command::new("cmd");
    c.args(["/C", bin]);
    augment_path_with_node(&mut c);
    hide_console(&mut c);
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
    // Global npm bin directory (%APPDATA%\npm) holds globally-installed CLIs
    // such as dsh.cmd.
    if let Ok(appdata) = std::env::var("APPDATA") {
        let npm_global = format!("{appdata}/npm");
        if Path::new(&npm_global).is_dir() {
            dirs.push(npm_global);
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
    let mut c = base_cmd("dsh");
    c.args(["web"]);
    c
}

fn upgrade_command() -> Command {
    let mut c = base_cmd("npm");
    c.args(["update", "-g", "@deepseek-ai/dsh"]);
    c.env("npm_config_update_notifier", "false");
    c
}

fn install_dsh_command() -> Command {
    let mut c = base_cmd("npm");
    c.args(["install", "-g", "@deepseek-ai/dsh"]);
    c.env("npm_config_update_notifier", "false");
    c
}

fn extract_version(lines: &[String]) -> Option<String> {
    // Scan from the last line backwards for the first semver pattern, so it
    // works with bare "0.1.0", "v0.1.0", "dsh 0.1.0-rc.6", npm download lines
    // ("...dsh-0.1.0.tgz"), etc.
    for line in lines.iter().rev() {
        if let Some(v) = find_semver(line) {
            return Some(v);
        }
    }
    None
}

/// Find a semver-like pattern (digits.digits.digits with optional
/// -prerelease / +build suffix) anywhere inside a line.
fn find_semver(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i;
        let mut dots = 0u32;
        while j < n {
            if b[j].is_ascii_digit() {
                j += 1;
            } else if b[j] == b'.' && dots < 2 && j + 1 < n && b[j + 1].is_ascii_digit() {
                dots += 1;
                j += 1;
            } else {
                break;
            }
        }
        if dots == 2 {
            let mut end = j;
            if end < n && b[end] == b'-' {
                let mut k = end + 1;
                while k < n
                    && (b[k].is_ascii_alphanumeric() || b[k] == b'.' || b[k] == b'-')
                {
                    k += 1;
                }
                end = k;
            }
            if end > start {
                return Some(s[start..end].to_string());
            }
        }
        i = if j > i { j } else { i + 1 };
    }
    None
}

/// Parse a semver core (major.minor.patch) from a version string, ignoring
/// any `-prerelease` / `+build` suffix. Returns None when not parseable.
fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    let patch: u64 = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

/// Whether version `v` is at least `min`, comparing major.minor.patch.
fn version_at_least(v: (u64, u64, u64), min: (u64, u64, u64)) -> bool {
    v >= min
}

/// Run a command synchronously, capturing its trimmed stdout when it exits 0.
fn run_capture(mut cmd: Command) -> Option<String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Report the global dsh CLI version, or None when it is not installed.
fn try_dsh_version() -> Option<String> {
    let mut cmd = base_cmd("dsh");
    cmd.arg("--version");
    cmd.env("npm_config_update_notifier", "false");
    let out = run_capture(cmd)?;
    extract_version(&[out])
}

/// Run a command, streaming its stdout/stderr lines as `<prefix>:stdout` /
/// `<prefix>:stderr` events, and report whether it exited successfully.
fn stream_and_wait(app: &AppHandle, mut cmd: Command, prefix: &'static str) -> Result<bool, String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{}: {e}", tr("Failed to launch command", "无法启动命令")))?;

    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        let event = format!("{prefix}:stdout");
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        let _ = app.emit(&event, &l);
                    }
                    Err(_) => break,
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        let event = format!("{prefix}:stderr");
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                match line {
                    Ok(l) => {
                        let _ = app.emit(&event, &l);
                    }
                    Err(_) => break,
                }
            }
        });
    }

    let status = child
        .wait()
        .map_err(|e| format!("{}: {e}", tr("Failed to wait for command", "等待命令结束失败")))?;
    Ok(status.success())
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
    let mut c = Command::new("taskkill");
    c.args(["/PID", &pid.to_string(), "/T", "/F"]);
    hide_console(&mut c);
    let _ = c.status();
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
        .map_err(|e| format!("{}: {e}", tr("Failed to launch dsh web", "无法启动 dsh web")))?;
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
    // port opens (e.g. node/dsh missing) so the UI reports failure at once
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
    // Update the global dsh installation, then read the resulting version.
    let ok = stream_and_wait(&app, upgrade_command(), "upgrade")?;
    std::thread::sleep(Duration::from_millis(150));
    let version = try_dsh_version().unwrap_or_else(|| "unknown".to_string());

    if !ok {
        return Ok(UpgradeResult {
            ok: false,
            version,
            restarted: false,
            message: tr(
                "Upgrade failed: npm update command did not succeed. Check the CLI logs.",
                "升级失败: npm update 命令未成功，请检查 CLI 日志",
            ),
        });
    }

    let owns = app.state::<ServerState>().pid.lock().unwrap().is_some();
    let (restarted, message) = if owns {
        stop_server(app.clone());
        std::thread::sleep(Duration::from_millis(600));
        match start_internal(&app) {
            Ok(_) => (
                true,
                format!(
                    "{} (v{version})",
                    tr(
                        "Upgrade complete, service restarted with the new version",
                        "升级完成，服务已用新版本重启"
                    )
                ),
            ),
            Err(e) => (
                false,
                format!(
                    "{} (v{version}) - {}: {e}",
                    tr("Upgrade complete but restart failed", "升级成功但重启失败"),
                    tr("please restart manually", "请手动点击重启")
                ),
            ),
        }
    } else {
        (
            false,
            format!(
                "{} (v{version})",
                tr(
                    "Upgrade complete, no service running (effective on next launch)",
                    "升级完成，当前无本应用运行的服务，下次启动即生效"
                )
            ),
        )
    };

    Ok(UpgradeResult { ok: true, version, restarted, message })
}

/// Report the DeepSeek Harness (dsh) CLI version that will actually run.
#[tauri::command]
async fn dsh_version() -> Result<String, String> {
    Ok(try_dsh_version().unwrap_or_else(|| "unknown".to_string()))
}

/// Check that Node.js is installed and meets the minimum supported version.
#[tauri::command]
async fn check_node() -> NodeCheck {
    let mut cmd = base_cmd("node");
    cmd.arg("--version");
    let version = run_capture(cmd).and_then(|out| extract_version(&[out]));

    let Some(version) = version else {
        return NodeCheck {
            installed: false,
            version: String::new(),
            supported: false,
            message: tr(
                "Node.js not detected. DSH Desktop requires Node.js 22.19.0 or newer. Please install it and retry.",
                "未检测到 Node.js。DSH Desktop 需要 Node.js 22.19.0 或更高版本，请安装后重试。",
            ),
        };
    };

    let supported = parse_semver(&version)
        .map(|v| version_at_least(v, MIN_NODE_VERSION))
        .unwrap_or(false);

    let message = if supported {
        format!("Node.js v{version} {}", tr("meets requirements", "满足要求"))
    } else {
        format!(
            "{} v{version}: {}",
            tr("Detected Node.js", "检测到 Node.js"),
            tr(
                "requires 22.19.0 or newer, please upgrade and retry",
                "需要 22.19.0 或更高版本，请升级后重试"
            )
        )
    };

    NodeCheck { installed: true, version, supported, message }
}

/// Ensure the global dsh CLI is installed; install it via npm when missing.
#[tauri::command]
async fn ensure_dsh(app: AppHandle) -> Result<DshInstallResult, String> {
    if let Some(version) = try_dsh_version() {
        return Ok(DshInstallResult {
            installed: true,
            version,
            message: tr("dsh is already installed", "dsh 已安装"),
        });
    }

    let _ = app.emit(
        "install:status",
        tr(
            "Installing @deepseek-ai/dsh (first install may take a few minutes)…",
            "正在安装 @deepseek-ai/dsh（首次安装可能需要几分钟）…",
        ),
    );
    let ok = stream_and_wait(&app, install_dsh_command(), "install")?;
    if !ok {
        return Err(tr(
            "Failed to install dsh: npm install command did not succeed. Check the CLI logs.",
            "安装 dsh 失败: npm install 命令未成功，请检查 CLI 日志",
        ));
    }

    let version = try_dsh_version().unwrap_or_else(|| "unknown".to_string());
    Ok(DshInstallResult {
        installed: true,
        version,
        message: tr("dsh installation complete", "dsh 安装完成"),
    })
}

/// Open the Node.js download page in the system browser.
#[tauri::command]
fn open_nodejs_website() -> Result<(), String> {
    let url = "https://nodejs.org/";
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("{}: {e}", tr("Failed to open browser", "无法打开浏览器")))?;
    }
    #[cfg(target_os = "windows")]
    {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", url]);
        hide_console(&mut c);
        c.spawn().map_err(|e| format!("{}: {e}", tr("Failed to open browser", "无法打开浏览器")))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("{}: {e}", tr("Failed to open browser", "无法打开浏览器")))?;
    }
    Ok(())
}

// ===== Plugin management =====

/// Validate an npm package name to reject shell metacharacters.
fn valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '@' | '/' | '.' | '_' | '~' | '-')
        })
}

/// Read the web profile's package.json as JSON.
fn read_profile_manifest() -> Result<serde_json::Value, String> {
    let path = profile_dir().join("package.json");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: {e} ({})",
            tr(
                "Failed to read profile config, please launch dsh once to initialize the profile",
                "读取 profile 配置失败，请先启动 dsh 一次以初始化 profile"
            ),
            path.display()
        )
    })?;
    serde_json::from_str(&content)
        .map_err(|e| format!("{}: {e}", tr("Failed to parse package.json", "解析 package.json 失败")))
}

/// Write the web profile's package.json (pretty-printed).
fn write_profile_manifest(json: &serde_json::Value) -> Result<(), String> {
    let path = profile_dir().join("package.json");
    let content = serde_json::to_string_pretty(json)
        .map_err(|e| format!("{}: {e}", tr("Failed to serialize package.json", "序列化 package.json 失败")))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("{}: {e}", tr("Failed to write package.json", "写入 package.json 失败")))
}

/// Whether an installed package declares a dsh bundle (dsh.bundle.patch).
fn plugin_is_bundle(name: &str) -> bool {
    let pkg_path = profile_dir()
        .join("node_modules")
        .join(name)
        .join("package.json");
    let Ok(content) = std::fs::read_to_string(pkg_path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    json.get("dsh")
        .and_then(|d| d.get("bundle"))
        .and_then(|b| b.get("patch"))
        .is_some()
}

/// Add `name` to dsh.profile.bundles (idempotent).
fn add_to_bundles(name: &str) -> Result<(), String> {
    let mut json = read_profile_manifest()?;
    let bundles = json
        .pointer_mut("/dsh/profile/bundles")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            tr(
                "package.json is missing the dsh.profile.bundles field",
                "package.json 缺少 dsh.profile.bundles 字段",
            )
        })?;
    if !bundles.iter().any(|b| b.as_str() == Some(name)) {
        bundles.push(serde_json::Value::String(name.to_string()));
    }
    write_profile_manifest(&json)
}

/// Remove `name` from dsh.profile.bundles (no-op when absent).
fn remove_from_bundles(name: &str) -> Result<(), String> {
    let mut json = read_profile_manifest()?;
    if let Some(bundles) = json
        .pointer_mut("/dsh/profile/bundles")
        .and_then(|v| v.as_array_mut())
    {
        bundles.retain(|b| b.as_str() != Some(name));
    }
    write_profile_manifest(&json)
}

fn add_plugin_command(name: &str) -> Command {
    let mut c = base_cmd("npm");
    c.args(["install", name, "--save"]);
    c.current_dir(profile_dir());
    c.env("npm_config_update_notifier", "false");
    c
}

fn remove_plugin_command(name: &str) -> Command {
    let mut c = base_cmd("npm");
    c.args(["uninstall", name]);
    c.current_dir(profile_dir());
    c.env("npm_config_update_notifier", "false");
    c
}

/// List user-installed plugins (the profile's `dependencies`).
#[tauri::command]
async fn list_plugins() -> Result<Vec<PluginInfo>, String> {
    let json = read_profile_manifest()?;
    let bundles: Vec<&str> = json
        .pointer("/dsh/profile/bundles")
        .and_then(|b| b.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();

    let mut plugins: Vec<PluginInfo> = Vec::new();
    if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
        for (name, version) in deps {
            let ver = version
                .as_str()
                .map(|s| {
                    s.trim_start_matches('^')
                        .trim_start_matches('~')
                        .to_string()
                })
                .unwrap_or_default();
            let is_bundle = bundles.iter().any(|b| *b == name.as_str());
            plugins.push(PluginInfo { name: name.clone(), version: ver, is_bundle });
        }
    }
    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(plugins)
}

/// Install a plugin into the web profile and reconcile dsh.profile.bundles.
#[tauri::command]
async fn add_plugin(app: AppHandle, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if !valid_package_name(&name) {
        return Err(format!("{}: {name}", tr("Invalid package name", "无效的包名")));
    }

    let _ = app.emit(
        "plugin:status",
        format!("{} {name}", tr("Installing plugin", "正在安装插件")),
    );
    let ok = stream_and_wait(&app, add_plugin_command(&name), "plugin")?;
    if !ok {
        return Err(format!(
            "{name}: {}",
            tr(
                "install failed: npm install command did not succeed. Check the CLI logs.",
                "安装失败: npm install 命令未成功，请检查 CLI 日志"
            )
        ));
    }

    if plugin_is_bundle(&name) {
        add_to_bundles(&name)?;
    }
    Ok(())
}

/// Remove a plugin from the web profile and reconcile dsh.profile.bundles.
#[tauri::command]
async fn remove_plugin(app: AppHandle, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if !valid_package_name(&name) {
        return Err(format!("{}: {name}", tr("Invalid package name", "无效的包名")));
    }

    let _ = app.emit(
        "plugin:status",
        format!("{} {name}", tr("Uninstalling plugin", "正在卸载插件")),
    );
    let ok = stream_and_wait(&app, remove_plugin_command(&name), "plugin")?;
    if !ok {
        return Err(format!(
            "{name}: {}",
            tr(
                "uninstall failed: npm uninstall command did not succeed. Check the CLI logs.",
                "卸载失败: npm uninstall 命令未成功，请检查 CLI 日志"
            )
        ));
    }

    remove_from_bundles(&name)?;
    Ok(())
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
            dsh_version,
            check_node,
            ensure_dsh,
            open_nodejs_website,
            set_ui_lang,
            list_plugins,
            add_plugin,
            remove_plugin
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