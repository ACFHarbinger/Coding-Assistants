//! Long-lived web-app harnesses.
//!
//! A web harness is a local HTTP server embedded in the desktop shell, not a
//! PTY. DeepSeek's official `dsh --profile web` is the first one: startup
//! prints `dsh web: <tokenized url>` and `--no-open` keeps the OS browser
//! closed so the grid can iframe that URL.

use crate::commands::settings::creative_tools::resolve_binary;
use hub::{
    deepseek_web_spawn_args, is_pid_running, parse_dsh_web_port, parse_dsh_web_url, HarnessId,
    DSH_WEB_HOST,
};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebHarnessSession {
    pub harness: String,
    pub workspace: String,
    pub url: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub detail: String,
}

struct RunningWeb {
    child: Child,
    session: WebHarnessSession,
}

fn registry() -> &'static Mutex<HashMap<String, RunningWeb>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, RunningWeb>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn session_key(harness: &str, workspace: &str) -> String {
    format!("{harness}\n{workspace}")
}

fn dsh_command() -> (String, Vec<String>) {
    if resolve_binary("dsh").is_some() {
        return ("dsh".to_string(), Vec::new());
    }
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../submodules/deepseek/apps/cli/lib/bin.js");
    if checkout.is_file() {
        return (
            "node".to_string(),
            vec![checkout.to_string_lossy().into_owned()],
        );
    }
    ("dsh".to_string(), Vec::new())
}

fn reap(running: &mut RunningWeb) {
    let _ = running.child.kill();
    let _ = running.child.wait();
}

/// Read stdout until `dsh web: <url>`, then keep draining so a full pipe
/// cannot stall the server. Stderr is drained the same way.
fn read_launch_url(child: &mut Child, timeout: Duration) -> Result<String, String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "dsh web produced no stdout".to_string())?;
    let stderr = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let _ = tx.send(line.trim_end_matches(['\r', '\n']).to_string());
                }
            }
        }
    });
    let stderr_tail = Arc::new(Mutex::new(String::new()));
    if let Some(stderr) = stderr {
        let tail = Arc::clone(&stderr_tail);
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                let mut buf = tail.lock().unwrap_or_else(|poison| poison.into_inner());
                if buf.len() < 4_000 {
                    buf.push_str(&line);
                    buf.push('\n');
                }
            }
        });
    }
    let started = Instant::now();
    let mut combined = String::new();
    while started.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                combined.push_str(&line);
                combined.push('\n');
                if let Some(url) = parse_dsh_web_url(&combined) {
                    return Ok(url);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Some(status) = child
                    .try_wait()
                    .map_err(|error| format!("failed while waiting for dsh web: {error}"))?
                {
                    let tail = stderr_tail
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner())
                        .clone();
                    return Err(format!(
                        "dsh web exited ({status}) before printing a URL. {tail}"
                    ));
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(
        "dsh web did not print a `dsh web:` URL. Install @deepseek-ai/dsh, or build submodules/deepseek (`pnpm install && pnpm run build`) so the web frontend exists."
            .into(),
    )
}

fn require_web_harness(harness: &str) -> Result<HarnessId, String> {
    let id = HarnessId::parse(harness).map_err(|error| error.to_string())?;
    if !id.is_web_app() {
        return Err(format!(
            "{harness} is a {} harness, not a web app",
            match id.surface() {
                hub::HarnessSurface::Cli => "CLI",
                hub::HarnessSurface::Tui => "TUI",
                hub::HarnessSurface::WebApp => "web",
            }
        ));
    }
    Ok(id)
}

/// Start (or reuse) the official web harness for `workspace`.
#[tauri::command]
pub fn web_harness_start(harness: String, workspace: String) -> Result<WebHarnessSession, String> {
    let id = require_web_harness(&harness)?;
    if !Path::new(&workspace).is_absolute() {
        return Err("workspace must be an absolute path".into());
    }
    let key = session_key(&harness, &workspace);
    {
        let mut guard = registry()
            .lock()
            .map_err(|_| "web harness registry lock poisoned".to_string())?;
        if let Some(running) = guard.get_mut(&key) {
            let alive = running.session.pid.is_some_and(|pid| is_pid_running(pid));
            if alive {
                return Ok(running.session.clone());
            }
            reap(running);
            guard.remove(&key);
        }
    }

    // `--port 0` lets dsh ask the OS for a free port. The printed URL is the
    // one the pane embeds, including the launch token. The registry lock is
    // not held across this wait.
    let spawn_args = match id {
        HarnessId::DeepSeek => deepseek_web_spawn_args(0),
        other => {
            return Err(format!(
                "{} is marked as a web app but has no launch argv yet",
                other.as_str()
            ))
        }
    };
    let (program, prefix) = dsh_command();
    let mut args = prefix;
    args.extend(
        spawn_args
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned()),
    );
    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(&workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(secret) = hub::secret::resolve("DEEPSEEK_API_KEY") {
        command.env("DEEPSEEK_API_KEY", secret.expose());
    }
    let mut child = command.spawn().map_err(|error| {
        format!(
            "DeepSeek web harness unavailable: could not start {program} ({error}). Install @deepseek-ai/dsh or build submodules/deepseek."
        )
    })?;
    let url = match read_launch_url(&mut child, Duration::from_secs(25)) {
        Ok(url) => url,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let port = parse_dsh_web_port(&url).unwrap_or(0);
    let session = WebHarnessSession {
        harness: harness.clone(),
        workspace: workspace.clone(),
        url,
        port,
        pid: Some(child.id()),
        detail: if port == 0 {
            "DeepSeek web app is serving on loopback".into()
        } else {
            format!("DeepSeek web app on http://{DSH_WEB_HOST}:{port}")
        },
    };
    let mut guard = registry()
        .lock()
        .map_err(|_| "web harness registry lock poisoned".to_string())?;
    if let Some(existing) = guard.get_mut(&key) {
        if existing.session.pid.is_some_and(|pid| is_pid_running(pid)) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(existing.session.clone());
        }
        reap(existing);
    }
    guard.insert(
        key,
        RunningWeb {
            child,
            session: session.clone(),
        },
    );
    Ok(session)
}

#[tauri::command]
pub fn web_harness_status(
    harness: String,
    workspace: String,
) -> Result<Option<WebHarnessSession>, String> {
    let _ = require_web_harness(&harness)?;
    let key = session_key(&harness, &workspace);
    let mut guard = registry()
        .lock()
        .map_err(|_| "web harness registry lock poisoned".to_string())?;
    let Some(running) = guard.get(&key) else {
        return Ok(None);
    };
    let alive = running.session.pid.is_some_and(|pid| is_pid_running(pid));
    if !alive {
        if let Some(mut running) = guard.remove(&key) {
            reap(&mut running);
        }
        return Ok(None);
    }
    Ok(Some(running.session.clone()))
}

#[tauri::command]
pub fn web_harness_stop(harness: String, workspace: String) -> Result<(), String> {
    let _ = require_web_harness(&harness)?;
    let key = session_key(&harness, &workspace);
    let mut guard = registry()
        .lock()
        .map_err(|_| "web harness registry lock poisoned".to_string())?;
    if let Some(mut running) = guard.remove(&key) {
        let _ = running.child.kill();
        let _ = running.child.wait();
    }
    Ok(())
}
