#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
#[cfg(not(windows))]
use portable_pty::Child as PtyChild;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

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
    /// Authenticated URL (with `?token=`) parsed from the launched
    /// `dsh web` output. New dsh versions reject plain `/` with 401.
    auth_url: Mutex<Option<String>>,
}

/// Which page the main window shows and where it should go next. dsh is
/// loaded top-level (its session cookie is SameSite=Strict, so an iframe in
/// the cross-site shell page could never authenticate); the shell page and
/// the floating bar window drive these transitions.
struct UiState {
    /// Frontend-reported URL of the shell page (dev vs custom-protocol).
    shell_url: Mutex<Option<String>>,
    /// User deliberately stays on the shell (CLI / plugins tab): do not
    /// yank the window into the app when the server becomes ready.
    pinned: AtomicBool,
    /// Main window currently shows the dsh app rather than the shell.
    app_shown: AtomicBool,
}

/// Navigate the main window. `as_app` records whether dsh or the shell is
/// on screen so server-exit events know to pull the user back.
fn goto_main(app: &AppHandle, target: &str, as_app: bool) {
    let Ok(url) = target.parse::<tauri::Url>() else {
        return;
    };
    if let Some(w) = app.get_webview_window("main") {
        if w.navigate(url).is_ok() {
            app.state::<UiState>().app_shown.store(as_app, Ordering::SeqCst);
        }
    }
}

fn shell_url(app: &AppHandle) -> Option<String> {
    app.state::<UiState>().shell_url.lock().unwrap().clone()
}

/// Shell URL with a query (e.g. `tab=cli`) so the reloaded page opens that
/// tab instead of bouncing back into the app.
fn shell_nav_url(app: &AppHandle, query: &str) -> Option<String> {
    let base = shell_url(app)?;
    let sep = if base.contains('?') { '&' } else { '?' };
    Some(format!("{base}{sep}{query}"))
}

/// After `server:ready`: show the app in the main window unless the user is
/// deliberately on the shell (CLI/plugins).
fn open_app_when_ready(app: &AppHandle, url: &str) {
    if !app.state::<UiState>().pinned.load(Ordering::SeqCst) {
        goto_main(app, url, true);
    }
}

/// Pull the main window back to the shell (after the server died while the
/// app page was on screen). `ret=1` stops the reloaded shell from
/// auto-starting a server: the user stopped/crashed it deliberately, and an
/// immediate restart loop would mask the exit logs.
fn return_to_shell(app: &AppHandle) {
    if app.state::<UiState>().app_shown.load(Ordering::SeqCst) {
        if let Some(u) = shell_nav_url(app, "ret=1") {
            goto_main(app, &u, false);
        }
    }
}

/// State for the interactive shell running in the CLI terminal.
struct ShellState {
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    pid: Mutex<Option<u32>>,
}

#[derive(Clone, serde::Serialize)]
struct Status {
    running: bool,
    port: u16,
    url: String,
    /// True when a foreign `dsh web` holds the port but its launch token is
    /// unknowable to us (token-auth version), so the app cannot embed it.
    needs_auth: bool,
}

fn status_of(app: &AppHandle, running: bool) -> Status {
    let state = app.state::<ServerState>();
    let auth = state.auth_url.lock().unwrap().clone();
    let owned = state.pid.lock().unwrap().is_some();
    let needs_auth = auth.is_none() && !owned && is_port_open(PORT) && probe_needs_auth();
    Status { running, port: PORT, url: auth.unwrap_or_else(url), needs_auth }
}

fn ready_url(app: &AppHandle) -> String {
    app.state::<ServerState>()
        .auth_url
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(url)
}

/// Ask the running server whether it requires token authentication: the new
/// `dsh web` answers 401 to a plain `GET /`, older versions serve the app.
fn probe_needs_auth() -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", PORT)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
    let req = format!(
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 32];
    match stream.read(&mut buf) {
        Ok(n) => {
            let head = String::from_utf8_lossy(&buf[..n]);
            head.starts_with("HTTP/1.1 401") || head.starts_with("HTTP/1.0 401")
        }
        Err(_) => false,
    }
}

/// Strip ANSI/VT escape sequences and CR so PTY output can be pattern-matched
/// as plain text.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\r' {
            continue;
        }
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for c in chars.by_ref() {
                    if matches!(c, '@'..='~') {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        chars.next();
                        break;
                    }
                }
            }
            Some(_) => {}
            None => break,
        }
    }
    out
}

const WEB_URL_MARK: &str = "dsh web: http";

/// Incrementally extracts the first `dsh web: <url>` line from streamed
/// process output, tolerating chunk boundaries and ANSI noise. Done once the
/// line is complete.
#[derive(Default)]
struct LaunchUrlScanner {
    tail: String,
    done: bool,
}

impl LaunchUrlScanner {
    fn feed(&mut self, text: &str) -> Option<String> {
        if self.done {
            return None;
        }
        let mut s = std::mem::take(&mut self.tail);
        s.push_str(text);
        let clean = strip_ansi(&s);
        if let Some(idx) = clean.find(WEB_URL_MARK) {
            let after: String = clean[idx + "dsh web: ".len()..].chars().collect();
            let url: String = after
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '(')
                .collect();
            if url.len() < after.len() {
                // A terminator (space before "(LAN: …)" or the line break)
                // followed the URL, so it is complete.
                self.done = true;
                if after.contains("token=") {
                    return Some(url);
                }
                return None;
            }
            // URL may still be growing across chunks; keep the whole line
            // (marker included) so the next feed resumes parsing it.
            self.tail = clean[idx..].to_string();
            if self.tail.len() > 8192 {
                self.done = true;
            }
            return None;
        }
        let keep: String = clean.chars().rev().take(32).collect::<Vec<_>>().into_iter().rev().collect();
        self.tail = keep;
        None
    }

    fn capture(&mut self, app: &AppHandle, bytes: &[u8]) {
        if self.done {
            return;
        }
        if let Some(u) = self.feed(&String::from_utf8_lossy(bytes)) {
            *app.state::<ServerState>().auth_url.lock().unwrap() = Some(u);
        }
    }
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

/// Compute the augmented PATH for child processes: prepend Node.js install
/// directories and the global npm bin directory when they exist, keeping the
/// inherited PATH as a fallback. Returns None when nothing to prepend.
#[cfg(windows)]
fn augmented_path() -> Option<String> {
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
        return None;
    }
    let mut path = dirs.join(";");
    if let Ok(p) = std::env::var("PATH") {
        if !p.is_empty() {
            path.push(';');
            path.push_str(&p);
        }
    }
    Some(path)
}

#[cfg(windows)]
fn augment_path_with_node(c: &mut Command) {
    if let Some(path) = augmented_path() {
        c.env("PATH", path);
    }
}

/// Build a PTY command for the given binary (macOS/Linux only; Windows uses
/// plain processes for non-interactive launches — see `spawn_dsh_web` and
/// `run_npm`). Routes through fnm so it also works when launched from the GUI.
#[cfg(not(windows))]
fn pty_base_cmd(bin: &str) -> CommandBuilder {
    #[cfg(not(windows))]
    {
        for fnm in [
            "/opt/homebrew/bin/fnm",
            "/opt/homebrew/opt/fnm/bin/fnm",
            "/usr/local/bin/fnm",
        ] {
            if Path::new(fnm).is_file() {
                let mut c = CommandBuilder::new(fnm);
                c.args(&["exec", "--using", "default", "--", bin]);
                c.env("TERM", "xterm-256color");
                c.env("LANG", "en_US.UTF-8");
                return c;
            }
        }
    }
    let mut c = CommandBuilder::new(bin);
    #[cfg(not(windows))]
    {
        // GUI-launched apps don't inherit TERM/LANG from launchd.
        c.env("TERM", "xterm-256color");
        c.env("LANG", "en_US.UTF-8");
    }
    c
}

#[cfg(not(windows))]
fn dsh_pty_command() -> CommandBuilder {
    let mut c = pty_base_cmd("dsh");
    c.args(&["web", "--no-open"]);
    c
}

/// Run `npm <action> -g @deepseek-ai/dsh`, streaming raw output to the given
/// frontend event, and report whether it exited successfully.
///
/// Platform split: on Windows, running npm through a ConPTY (`cmd /C npm ...`)
/// fails with 0xc0000142 on Windows 11 24H2+, so a plain process with piped
/// stdout/stderr is used instead (same as v0.2.6). macOS/Linux keep the PTY.
#[cfg(not(windows))]
fn run_npm(app: &AppHandle, action: &str, event: &'static str) -> Result<bool, String> {
    let mut cmd = pty_base_cmd("npm");
    cmd.args(&[action, "-g", "@deepseek-ai/dsh"]);
    cmd.env("npm_config_update_notifier", "false");
    run_pty(app, cmd, event)
}

#[cfg(windows)]
fn run_npm(app: &AppHandle, action: &str, event: &'static str) -> Result<bool, String> {
    let mut cmd = base_cmd("npm");
    cmd.args(&[action, "-g", "@deepseek-ai/dsh"]);
    cmd.env("npm_config_update_notifier", "false");
    run_plain(app, cmd, event)
}

/// Spawn a command in a PTY and return its output reader and child handle.
#[cfg(not(windows))]
fn spawn_pty(
    cmd: CommandBuilder,
) -> Result<(Box<dyn Read + Send>, Box<dyn PtyChild + Send + Sync>), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("{}: {e}", tr("Failed to open terminal", "打开终端失败")))?;
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("{}: {e}", tr("Failed to launch command", "启动命令失败")))?;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("{}: {e}", tr("Failed to read terminal", "读取终端失败")))?;
    Ok((reader, child))
}

/// Run a PTY command to completion, streaming its raw bytes to the given
/// frontend event, and report whether it exited successfully.
#[cfg(not(windows))]
fn run_pty(app: &AppHandle, cmd: CommandBuilder, event: &'static str) -> Result<bool, String> {
    let (mut reader, mut child) = spawn_pty(cmd)?;
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let _ = app.emit(event, &buf[..n].to_vec());
            }
            Err(_) => break,
        }
    }
    let status = child
        .wait()
        .map_err(|e| format!("{}: {e}", tr("Failed to wait for command", "等待命令结束失败")))?;
    Ok(status.success())
}

/// Run a plain (non-PTY) command to completion on Windows, streaming its raw
/// stdout/stderr bytes to the given frontend event, and report whether it
/// exited successfully. Used instead of ConPTY for `cmd /C ...` launches,
/// which fail with 0xc0000142 on Windows 11 24H2+.
#[cfg(windows)]
fn run_plain(app: &AppHandle, mut cmd: Command, event: &'static str) -> Result<bool, String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{}: {e}", tr("Failed to launch command", "启动命令失败")))?;

    // Stream stdout/stderr to the frontend (merged, like the PTY path).
    if let Some(out) = child.stdout.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut reader = out;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = app.emit(event, &buf[..n].to_vec());
                    }
                    Err(_) => break,
                }
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut reader = err;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = app.emit(event, &buf[..n].to_vec());
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

/// Stream a PTY reader's output to the frontend on a background thread.
/// When `watch_web_url` is set, also scan the output for the `dsh web:`
/// authenticated URL line.
fn stream_pty_output(
    app: AppHandle,
    mut reader: Box<dyn Read + Send>,
    event: &'static str,
    watch_web_url: bool,
) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut scanner = LaunchUrlScanner::default();
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if watch_web_url {
                        scanner.capture(&app, &buf[..n]);
                    }
                    let _ = app.emit(event, &buf[..n].to_vec());
                }
                Err(_) => break,
            }
        }
    });
}

/// Build the interactive shell command for the CLI terminal. On macOS/Linux
/// it routes through fnm so node/npm/dsh are on PATH even when launched from
/// the GUI.
fn shell_command() -> CommandBuilder {
    #[cfg(not(windows))]
    {
        // Prefer the user's login shell ($SHELL), then zsh (macOS default
        // since Catalina), then bash. This keeps the user's aliases/PATH/fnm
        // init from ~/.zshrc (or ~/.bashrc) working inside the built-in CLI.
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| Path::new(s).is_file())
            .or_else(|| {
                Some("/bin/zsh".to_string()).filter(|p| Path::new(p).is_file())
            })
            .or_else(|| {
                Some("/bin/bash".to_string()).filter(|p| Path::new(p).is_file())
            })
            .unwrap_or_else(|| "/bin/sh".to_string());

        for fnm in [
            "/opt/homebrew/bin/fnm",
            "/opt/homebrew/opt/fnm/bin/fnm",
            "/usr/local/bin/fnm",
        ] {
            if Path::new(fnm).is_file() {
                let mut c = CommandBuilder::new(fnm);
                c.args(&["exec", "--using", "default", "--", &shell]);
                c.env("TERM", "xterm-256color");
                c.env("LANG", "en_US.UTF-8");
                return c;
            }
        }
        let mut c = CommandBuilder::new(shell);
        // GUI-launched apps don't inherit TERM/LANG from launchd: without TERM
        // zsh's line editor can't map backspace/arrows/IME echo; without a
        // UTF-8 locale multibyte input degrades. Set both explicitly.
        c.env("TERM", "xterm-256color");
        c.env("LANG", "en_US.UTF-8");
        c
    }
    #[cfg(windows)]
    {
        CommandBuilder::new("cmd.exe")
    }
}

/// Spawn the interactive shell for the CLI terminal and start streaming its
/// output. Called once at startup.
#[tauri::command]
fn spawn_shell(app: AppHandle) -> Result<String, String> {
    // Idempotent: navigating the main window between the shell and the app
    // page reloads the shell, whose frontend calls this on every boot. A
    // second PTY would orphan the first shell and interleave term:data.
    if app.state::<ShellState>().pid.lock().unwrap().is_some() {
        return Ok("reused".to_string());
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("{}: {e}", tr("Failed to open terminal", "打开终端失败")))?;

    let mut child = pair
        .slave
        .spawn_command(shell_command())
        .map_err(|e| format!("{}: {e}", tr("Failed to launch shell", "启动 shell 失败")))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("{}: {e}", tr("Failed to read terminal", "读取终端失败")))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("{}: {e}", tr("Failed to write to terminal", "写入终端失败")))?;

    {
        let state = app.state::<ShellState>();
        *state.master.lock().unwrap() = Some(pair.master);
        *state.writer.lock().unwrap() = Some(writer);
        *state.pid.lock().unwrap() = child.process_id();
    }

    stream_pty_output(app.clone(), reader, "term:data", false);

    // Clean up state if the shell itself exits.
    std::thread::spawn(move || {
        let _ = child.wait();
        let state = app.state::<ShellState>();
        *state.master.lock().unwrap() = None;
        *state.writer.lock().unwrap() = None;
        *state.pid.lock().unwrap() = None;
    });

    Ok("spawned".to_string())
}

/// Forward keystrokes from the frontend terminal to the shell's PTY.
#[tauri::command]
fn term_input(app: AppHandle, data: Vec<u8>) -> Result<(), String> {
    let state = app.state::<ShellState>();
    let mut guard = state.writer.lock().unwrap();
    if let Some(writer) = guard.as_mut() {
        writer
            .write_all(&data)
            .map_err(|e| format!("{}: {e}", tr("Failed to write to terminal", "写入终端失败")))?;
        let _ = writer.flush();
    }
    Ok(())
}

/// Resize the shell's PTY to match the frontend terminal.
#[tauri::command]
fn term_resize(app: AppHandle, rows: u16, cols: u16) -> Result<(), String> {
    let state = app.state::<ShellState>();
    let guard = state.master.lock().unwrap();
    if let Some(master) = guard.as_ref() {
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("{}: {e}", tr("Failed to resize terminal", "调整终端大小失败")))?;
    }
    Ok(())
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
/// On Windows, retries a few times to handle newly-installed .cmd shims that
/// may not be immediately visible due to filesystem/PATH caching.
fn try_dsh_version() -> Option<String> {
    let max_attempts = if cfg!(windows) { 5 } else { 1 };
    for attempt in 1..=max_attempts {
        let mut cmd = base_cmd("dsh");
        cmd.arg("--version");
        cmd.env("npm_config_update_notifier", "false");
        if let Some(out) = run_capture(cmd) {
            if let Some(v) = extract_version(&[out]) {
                return Some(v);
            }
        }
        if attempt < max_attempts {
            std::thread::sleep(Duration::from_millis(300));
        }
    }
    None
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

/// Spawn the dsh web server, returning its pid and a closure that waits for
/// exit. Output is streamed to the `term:data` event.
///
/// Platform split: on Windows, spawning `cmd /C dsh web` through a ConPTY
/// fails with "application failed to start (0xc0000142)" on Windows 11 24H2+
/// (the newly-created cmd.exe fails DLL initialization). The interactive CLI
/// shell (no `/C`) works fine, so only these non-interactive launches fall
/// back to a plain process with piped stdout/stderr, exactly like v0.2.6.
#[cfg(not(windows))]
fn spawn_dsh_web(
    app: &AppHandle,
) -> Result<(u32, Box<dyn FnOnce() -> i32 + Send>), String> {
    let (reader, mut child) = spawn_pty(dsh_pty_command())
        .map_err(|e| format!("{}: {e}", tr("Failed to launch dsh web", "无法启动 dsh web")))?;
    let pid = child.process_id().unwrap_or(0);
    stream_pty_output(app.clone(), reader, "term:data", true);
    let waiter = Box::new(move || child.wait().ok().map(|s| s.exit_code() as i32).unwrap_or(-1));
    Ok((pid, waiter))
}

#[cfg(windows)]
fn spawn_dsh_web(
    app: &AppHandle,
) -> Result<(u32, Box<dyn FnOnce() -> i32 + Send>), String> {
    let mut cmd = base_cmd("dsh");
    cmd.args(["web", "--no-open"]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{}: {e}", tr("Failed to launch dsh web", "无法启动 dsh web")))?;
    let pid = child.id();

    // Stream stdout/stderr to the CLI terminal (merged, like the PTY path),
    // scanning for the `dsh web:` authenticated-URL line.
    if let Some(out) = child.stdout.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut reader = out;
            let mut scanner = LaunchUrlScanner::default();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        scanner.capture(&app, &buf[..n]);
                        let _ = app.emit("term:data", &buf[..n].to_vec());
                    }
                    Err(_) => break,
                }
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let app = app.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut reader = err;
            let mut scanner = LaunchUrlScanner::default();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        scanner.capture(&app, &buf[..n]);
                        let _ = app.emit("term:data", &buf[..n].to_vec());
                    }
                    Err(_) => break,
                }
            }
        });
    }

    let waiter = Box::new(move || child.wait().ok().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1));
    Ok((pid, waiter))
}

fn start_internal(app: &AppHandle) -> Result<Status, String> {
    {
        let state = app.state::<ServerState>();
        if state.pid.lock().unwrap().is_some() {
            return Ok(status_of(app, true));
        }
    }

    // Port already serving (e.g. the user's browser dsh session)? Reuse it
    // instead of spawning a duplicate that would fail with EADDRINUSE.
    if is_port_open(PORT) {
        let s = status_of(app, true);
        if s.needs_auth {
            let _ = app.emit("server:auth-error", ());
        } else {
            let _ = app.emit("server:ready", s.url.clone());
            open_app_when_ready(app, &s.url);
        }
        return Ok(s);
    }

    let (pid, waiter) = spawn_dsh_web(app)?;

    {
        let state = app.state::<ServerState>();
        let mut guard = state.pid.lock().unwrap();
        *guard = Some(pid);
    }

    // watcher: clear state and notify on exit
    {
        let app = app.clone();
        std::thread::spawn(move || {
            let code = waiter();
            // The `dsh web` launcher can exit once its server is up: an
            // exit while the port still serves is not a shutdown. Only
            // tear state down once the port is actually closed.
            if is_port_open(PORT) {
                return;
            }
            let state = app.state::<ServerState>();
            let mut guard = state.pid.lock().unwrap();
            *guard = None;
            *state.auth_url.lock().unwrap() = None;
            drop(guard);
            let _ = app.emit("server:exited", code);
            return_to_shell(&app);
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
                    // Token-auth dsh versions only reveal the launch token
                    // in the `dsh web:` line they print at startup; wait a
                    // short grace period for the scanner to capture it. A
                    // plain 200 probe means the older, auth-free version is
                    // serving and no wait is needed.
                    let token_deadline = Instant::now() + Duration::from_secs(8);
                    loop {
                        let has_token = app
                            .state::<ServerState>()
                            .auth_url
                            .lock()
                            .unwrap()
                            .is_some();
                        if has_token || !probe_needs_auth() || Instant::now() >= token_deadline {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                    if ready_url(&app) == url() && probe_needs_auth() {
                        let _ = app.emit("server:auth-error", ());
                    } else {
                        let u = ready_url(&app);
                        let _ = app.emit("server:ready", u.clone());
                        open_app_when_ready(&app, &u);
                    }
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

    Ok(status_of(app, true))
}

#[tauri::command]
fn start_server(app: AppHandle) -> Result<Status, String> {
    start_internal(&app)
}

fn stop_internal(app: &AppHandle) -> Status {
    let state = app.state::<ServerState>();
    let pid = {
        let mut guard = state.pid.lock().unwrap();
        guard.take()
    };
    let s = match pid {
        Some(pid) => {
            kill_process_group(pid);
            *state.auth_url.lock().unwrap() = None;
            let _ = app.emit("server:stopped", ());
            status_of(app, false)
        }
        // Nothing we own: reflect whether 3080 is still up (reused server).
        None => status_of(app, is_port_open(PORT)),
    };
    return_to_shell(app);
    s
}

#[tauri::command]
fn stop_server(app: AppHandle) -> Status {
    stop_internal(&app)
}

/// The shell page reports its own URL once at startup (dev URL vs the
/// custom-protocol origin in production) so the Rust side can navigate back
/// to it.
#[tauri::command]
fn register_shell_url(app: AppHandle, url: String) {
    let state = app.state::<UiState>();
    let mut guard = state.shell_url.lock().unwrap();
    if guard.is_none() {
        *guard = Some(url);
    }
}

#[tauri::command]
fn navigate_main(app: AppHandle, url: String, as_app: bool) {
    goto_main(&app, &url, as_app);
}

/// true while the user is deliberately on the shell (CLI/plugins tab).
#[tauri::command]
fn set_pinned(app: AppHandle, pinned: bool) {
    app.state::<UiState>().pinned.store(pinned, Ordering::SeqCst);
}

#[tauri::command]
fn hide_bar_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("bar") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn show_bar_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("bar") {
        let _ = w.show();
        let _ = w.set_focus();
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
    let ok = run_npm(&app, "update", "term:data")?;
    // Wait for the npm process and filesystem to settle before checking version.
    std::thread::sleep(Duration::from_millis(500));
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
    let ok = run_npm(&app, "install", "term:data")?;
    if !ok {
        return Err(tr(
            "Failed to install dsh: npm install command did not succeed. Check the CLI logs.",
            "安装 dsh 失败: npm install 命令未成功，请检查 CLI 日志",
        ));
    }

    // Wait for the npm process and filesystem to settle before checking version.
    std::thread::sleep(Duration::from_millis(500));
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

#[tauri::command]
fn server_status(app: AppHandle) -> Status {
    let state = app.state::<ServerState>();
    let running = state.pid.lock().unwrap().is_some() || is_port_open(PORT);
    status_of(&app, running)
}

/// Floating always-on-top control bar. Lives as its own top-level webview
/// (same origin as the shell) so it stays reachable while the main window
/// shows the dsh app.
fn create_bar_window(app: &AppHandle) -> Result<(), String> {
    let url = WebviewUrl::App("index.html?view=bar".into());
    let mut builder = WebviewWindowBuilder::new(app, "bar", url)
        .inner_size(660.0, 44.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(true);
    if let Some(main) = app.get_webview_window("main") {
        if let (Ok(pos), Ok(size)) = (main.outer_position(), main.inner_size()) {
            let scale = main.scale_factor().unwrap_or(1.0);
            let x = pos.x as f64 / scale + (size.width as f64 / scale - 660.0) / 2.0;
            let y = pos.y as f64 / scale + 64.0;
            builder = builder.position(x, y);
        } else {
            builder = builder.center();
        }
    } else {
        builder = builder.center();
    }
    builder.build().map_err(|e| e.to_string())?;
    Ok(())
}

fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_app = MenuItem::with_id(app, "show-app", tr("Open DSH App", "打开 DSH 应用"), true, None::<&str>)?;
    let show_cli = MenuItem::with_id(app, "show-cli", tr("CLI Terminal", "CLI 终端"), true, None::<&str>)?;
    let show_plugins = MenuItem::with_id(app, "show-plugins", tr("Plugins", "插件管理"), true, None::<&str>)?;
    let show_bar = MenuItem::with_id(app, "show-bar", tr("Show Toolbar", "显示工具栏"), true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let restart = MenuItem::with_id(app, "restart", tr("Restart Service", "重启服务"), true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", tr("Stop Service", "停止服务"), true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", tr("Quit", "退出"), true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show_app, &show_cli, &show_plugins, &show_bar, &sep1, &restart, &stop, &sep2, &quit],
    )?;
    let mut tray = TrayIconBuilder::with_id("main-tray").menu(&menu).tooltip("DSH Desktop");
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(|app, event| match event.id.as_ref() {
        "show-app" => {
            let s = status_of(app, true);
            if s.running {
                goto_main(app, &s.url, true);
            }
        }
        "show-cli" | "show-plugins" => {
            let tab = if event.id.as_ref() == "show-cli" { "cli" } else { "plugins" };
            if let Some(u) = shell_nav_url(app, &format!("tab={tab}")) {
                app.state::<UiState>().pinned.store(true, Ordering::SeqCst);
                goto_main(app, &u, false);
            }
        }
        "show-bar" => {
            if let Some(w) = app.get_webview_window("bar") {
                let _ = w.show();
            }
        }
        "restart" => {
            let app = app.clone();
            std::thread::spawn(move || {
                stop_internal(&app);
                std::thread::sleep(Duration::from_millis(600));
                let _ = start_internal(&app);
            });
        }
        "stop" => {
            stop_internal(app);
        }
        "quit" => app.exit(0),
        _ => {}
    })
    .build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(ServerState {
            pid: Mutex::new(None),
            auth_url: Mutex::new(None),
        })
        .manage(UiState {
            shell_url: Mutex::new(None),
            pinned: AtomicBool::new(false),
            app_shown: AtomicBool::new(false),
        })
        .manage(ShellState {
            master: Mutex::new(None),
            writer: Mutex::new(None),
            pid: Mutex::new(None),
        })
        .setup(|app| {
            create_bar_window(&app.handle())?;
            build_tray(&app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            spawn_shell,
            term_input,
            term_resize,
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
            register_shell_url,
            navigate_main,
            set_pinned,
            hide_bar_window,
            show_bar_window
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(target_os = "macos")]
                {
                    // Hide into the Dock instead of quitting: the server keeps
                    // running and the app stays one click away (Reopen below).
                    api.prevent_close();
                    let _ = window.hide();
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = api;
                    window.app_handle().exit(0);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            // macOS: clicking the Dock icon after the window was hidden
            // (close-to-Dock) brings the window back. The Reopen variant only
            // exists on macOS (cfg-gated in tauri), so the whole arm must be
            // cfg-gated or Windows/Linux builds fail to compile.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } => {
                if !has_visible_windows {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                // Clean up the server process
                let state = app_handle.state::<ServerState>();
                let pid = {
                    let mut guard = state.pid.lock().unwrap();
                    guard.take()
                };
                if let Some(pid) = pid {
                    kill_process_group(pid);
                }

                // Clean up the shell PTY and kill the shell process to avoid
                // ConPTY resource leaks / leftover cmd.exe on Windows.
                let shell_state = app_handle.state::<ShellState>();
                let shell_pid = {
                    let mut guard = shell_state.pid.lock().unwrap();
                    guard.take()
                };
                if let Some(pid) = shell_pid {
                    kill_process_group(pid);
                }
                let mut master_guard = shell_state.master.lock().unwrap();
                let mut writer_guard = shell_state.writer.lock().unwrap();
                *master_guard = None;
                *writer_guard = None;
            }
            _ => {}
        });
}