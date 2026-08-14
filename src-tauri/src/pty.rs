//! Embedded terminal sessions: a real PTY per session, spawned and owned by
//! this Rust process, streamed to the frontend over Tauri events instead of
//! opening a separate OS terminal-emulator window. Backs the "Resume in
//! terminal" flow and any future in-app terminal surface.
//!
//! Output is forwarded as base64 (`pty-output:<session_id>` events) so
//! arbitrary bytes — ANSI escapes, non-UTF8 tool output — survive the JSON
//! event boundary intact; the frontend's xterm.js instance decodes and
//! feeds them to the terminal buffer as raw bytes, the same way it already
//! expects a real PTY's output to look. Input (`pty_write`) stays plain
//! UTF-8 text, since it only ever carries keystrokes typed in the browser.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Default)]
pub struct PtySessions(Mutex<HashMap<String, PtyHandle>>);

fn emit_exit(app: &AppHandle, session_id: &str, detail: &str) {
    let _ = app.emit(&format!("pty-exit:{session_id}"), detail.to_string());
}

/// Spawns `program` (with `args`, in `cwd`) attached to a new PTY, and
/// starts a background thread forwarding its output to the frontend as
/// `pty-output:<session_id>` events (base64-encoded chunks) until it exits
/// or `pty_kill` is called, at which point a `pty-exit:<session_id>` event
/// carries the exit status.
///
/// Replaces any existing session already registered under `session_id`
/// (killing it first) rather than leaking it — the frontend is expected to
/// pass a fresh id per terminal instance, but this stays safe either way.
#[tauri::command]
// `app`/`state` are Tauri-injected, not part of the invoke payload; the
// remaining six are the actual spawn parameters.
#[allow(clippy::too_many_arguments)]
pub fn pty_spawn(
    app: AppHandle,
    state: State<PtySessions>,
    session_id: String,
    program: String,
    args: Vec<String>,
    cwd: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    kill_session(&state, &session_id);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("failed to open a pty: {error}"))?;

    let mut cmd = CommandBuilder::new(&program);
    cmd.args(&args);
    cmd.cwd(&cwd);

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|error| format!("failed to spawn {program}: {error}"))?;
    // The slave side belongs to the child now; holding it open in this
    // process too would keep the pty alive even after the child exits.
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("failed to clone pty reader: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("failed to take pty writer: {error}"))?;

    {
        let mut sessions = state
            .0
            .lock()
            .map_err(|_| "pty session lock poisoned".to_string())?;
        sessions.insert(
            session_id.clone(),
            PtyHandle {
                master: pair.master,
                writer,
                child,
            },
        );
    }

    // Output-forwarding thread: reads until EOF (the child exiting closes
    // its end of the pty), then leaves exit reporting to the wait thread
    // below rather than guessing a reason from a read error.
    {
        let app = app.clone();
        let session_id = session_id.clone();
        std::thread::spawn(move || {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let encoded = STANDARD.encode(&buf[..n]);
                        if app
                            .emit(&format!("pty-output:{session_id}"), encoded)
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    // Wait thread: reports the real exit status and cleans up the
    // registry entry once the child actually exits, whether that's from
    // the harness quitting on its own or a later pty_kill.
    {
        let app = app.clone();
        let session_id = session_id.clone();
        std::thread::spawn(move || {
            let sessions_handle = app.state::<PtySessions>();
            let status = {
                let mut sessions = match sessions_handle.0.lock() {
                    Ok(guard) => guard,
                    Err(_) => return,
                };
                sessions
                    .get_mut(&session_id)
                    .and_then(|handle| handle.child.wait().ok())
            };
            if let Ok(mut sessions) = sessions_handle.0.lock() {
                sessions.remove(&session_id);
            }
            let detail = match status {
                Some(status) => format!("exited ({})", exit_status_detail(&status)),
                None => "exited".to_string(),
            };
            emit_exit(&app, &session_id, &detail);
        });
    }

    Ok(())
}

fn exit_status_detail(status: &portable_pty::ExitStatus) -> String {
    if status.success() {
        "success".to_string()
    } else {
        format!("code {}", status.exit_code())
    }
}

#[tauri::command]
pub fn pty_write(
    state: State<PtySessions>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let mut sessions = state
        .0
        .lock()
        .map_err(|_| "pty session lock poisoned".to_string())?;
    let handle = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("no live pty session {session_id}"))?;
    handle
        .writer
        .write_all(data.as_bytes())
        .and_then(|_| handle.writer.flush())
        .map_err(|error| format!("failed to write to pty: {error}"))
}

#[tauri::command]
pub fn pty_resize(
    state: State<PtySessions>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let sessions = state
        .0
        .lock()
        .map_err(|_| "pty session lock poisoned".to_string())?;
    let handle = sessions
        .get(&session_id)
        .ok_or_else(|| format!("no live pty session {session_id}"))?;
    handle
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("failed to resize pty: {error}"))
}

fn kill_session(state: &State<PtySessions>, session_id: &str) {
    if let Ok(mut sessions) = state.0.lock() {
        if let Some(mut handle) = sessions.remove(session_id) {
            let _ = handle.child.kill();
        }
    }
}

/// Kills the session's child process. The wait thread started in
/// `pty_spawn` (not this command) is what actually removes the registry
/// entry and emits `pty-exit`, once the kill has really taken effect —
/// this just requests it.
#[tauri::command]
pub fn pty_kill(state: State<PtySessions>, session_id: String) -> Result<(), String> {
    let mut sessions = state
        .0
        .lock()
        .map_err(|_| "pty session lock poisoned".to_string())?;
    let handle = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("no live pty session {session_id}"))?;
    handle
        .child
        .kill()
        .map_err(|error| format!("failed to kill pty session: {error}"))
}
