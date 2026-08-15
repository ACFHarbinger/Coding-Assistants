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
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

/// Cap for `PtyHandle::output_tail` — keep only the most recent bytes,
/// enough to read a fast-failing command's error output without unbounded
/// retention across every session that ever exited.
const OUTPUT_TAIL_CAP: usize = 64 * 1024;

/// How long an exited session entry is retained after exit so a frontend
/// that attached late can still query its output and exit detail.
const EXITED_RETENTION: Duration = Duration::from_secs(60);

struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    /// Bounded tail of everything the child wrote, kept after exit so a
    /// frontend that attaches after a fast exit can still show the output
    /// (and the failure reason) instead of a blank terminal.
    output_tail: Vec<u8>,
    exited: bool,
    exit_detail: Option<String>,
    exited_at: Option<Instant>,
}

/// Appends `chunk` to `tail`, keeping only the most recent `cap` bytes.
fn push_tail(tail: &mut Vec<u8>, chunk: &[u8], cap: usize) {
    if cap == 0 || chunk.is_empty() {
        return;
    }
    if chunk.len() >= cap {
        tail.clear();
        tail.extend_from_slice(&chunk[chunk.len() - cap..]);
        return;
    }
    if tail.len() + chunk.len() > cap {
        tail.drain(..tail.len() + chunk.len() - cap);
    }
    tail.extend_from_slice(chunk);
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
        // Bound the registry: drop exited sessions whose retained output
        // window has expired before adding the new one.
        let cutoff = Instant::now() - EXITED_RETENTION;
        sessions
            .retain(|_, handle| !handle.exited || handle.exited_at.is_none_or(|at| at >= cutoff));
        sessions.insert(
            session_id.clone(),
            PtyHandle {
                master: pair.master,
                writer,
                child,
                output_tail: Vec::new(),
                exited: false,
                exit_detail: None,
                exited_at: None,
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
                        // Retain a bounded tail alongside the live event so a
                        // frontend that attaches after a fast exit can still
                        // see what the child printed. Lock is brief and only
                        // held while appending; the wait thread never holds
                        // the map lock across its poll.
                        if let Ok(mut sessions) = app.state::<PtySessions>().0.lock() {
                            if let Some(handle) = sessions.get_mut(&session_id) {
                                push_tail(&mut handle.output_tail, &buf[..n], OUTPUT_TAIL_CAP);
                            }
                        }
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

    // Wait thread: polls the child with non-blocking try_wait so the map
    // lock is never held across the wait (the reader thread needs it to
    // append to the retained tail), then records the real exit status,
    // keeps the entry so a late-attaching frontend can still query output
    // and exit detail, and emits pty-exit. The next pty_spawn for the same
    // id replaces the entry; stale exited entries are swept by pty_spawn.
    {
        let app = app.clone();
        let session_id = session_id.clone();
        std::thread::spawn(move || {
            let sessions_handle = app.state::<PtySessions>();
            let mut error_count = 0u8;
            let status = loop {
                let polled = {
                    let mut sessions = match sessions_handle.0.lock() {
                        Ok(guard) => guard,
                        Err(_) => return,
                    };
                    let Some(handle) = sessions.get_mut(&session_id) else {
                        return;
                    };
                    handle.child.try_wait()
                };
                match polled {
                    Ok(Some(status)) => break Some(status),
                    Ok(None) => {
                        error_count = 0;
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => {
                        error_count += 1;
                        if error_count >= 20 {
                            break None;
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            };
            let detail = match status {
                Some(status) => format!("exited ({})", exit_status_detail(&status)),
                None => "exited (status unavailable)".to_string(),
            };
            if let Ok(mut sessions) = sessions_handle.0.lock() {
                if let Some(handle) = sessions.get_mut(&session_id) {
                    handle.exited = true;
                    handle.exit_detail = Some(detail.clone());
                    handle.exited_at = Some(Instant::now());
                }
            }
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
    // Keystrokes into a session that already exited are moot, not an error
    // the user needs to see; the frontend learns the truth via
    // pty_session_status / pty-exit instead.
    if handle.exited {
        return Ok(());
    }
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
    // Resizing a session that already exited is a no-op, not an error —
    // resize events keep firing from ResizeObservers after the child dies.
    if handle.exited {
        return Ok(());
    }
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
/// `pty_spawn` (not this command) records the real exit state and emits
/// `pty-exit` once the kill takes effect — this just requests it.
/// Killing an already-exited session is a successful no-op (idempotent
/// cleanup), not an error.
#[tauri::command]
pub fn pty_kill(state: State<PtySessions>, session_id: String) -> Result<(), String> {
    let mut sessions = state
        .0
        .lock()
        .map_err(|_| "pty session lock poisoned".to_string())?;
    let handle = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("no live pty session {session_id}"))?;
    if handle.exited {
        return Ok(());
    }
    handle
        .child
        .kill()
        .map_err(|error| format!("failed to kill pty session: {error}"))
}

/// Truthful snapshot of one PTY session for the frontend. Called on
/// EmbeddedTerminal mount so a session that already exited (fast-failing
/// harness CLI, killed by a later replace) can still show its retained
/// output and exit reason instead of a silently blank terminal — the
/// #161 "no terminal opens, no visible error" class.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySessionStatus {
    pub found: bool,
    pub running: bool,
    pub exited: bool,
    pub exit_detail: Option<String>,
    pub output_tail_b64: String,
}

#[tauri::command]
pub fn pty_session_status(state: State<PtySessions>, session_id: String) -> PtySessionStatus {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let sessions = match state.0.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return PtySessionStatus {
                found: false,
                running: false,
                exited: false,
                exit_detail: None,
                output_tail_b64: String::new(),
            };
        }
    };
    let Some(handle) = sessions.get(&session_id) else {
        return PtySessionStatus {
            found: false,
            running: false,
            exited: false,
            exit_detail: None,
            output_tail_b64: String::new(),
        };
    };
    PtySessionStatus {
        found: true,
        running: !handle.exited,
        exited: handle.exited,
        exit_detail: handle.exit_detail.clone(),
        output_tail_b64: STANDARD.encode(&handle.output_tail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_tail_keeps_only_the_most_recent_bytes() {
        let mut tail = Vec::new();
        push_tail(&mut tail, b"hello ", 1024);
        push_tail(&mut tail, b"world", 1024);
        assert_eq!(tail, b"hello world");

        // A chunk larger than the cap replaces the whole tail.
        let mut tail = Vec::new();
        push_tail(&mut tail, b"prefix", 8);
        push_tail(&mut tail, b"0123456789", 8);
        assert_eq!(tail, b"23456789");

        // Many small chunks keep only the last cap bytes.
        let mut tail = Vec::new();
        for i in 0..20u8 {
            push_tail(&mut tail, &[b'a' + i], 8);
        }
        assert_eq!(tail, b"mnopqrst");

        // Zero-cap and empty chunks are safe no-ops.
        let mut tail = vec![b'x'];
        push_tail(&mut tail, b"", 0);
        push_tail(&mut tail, b"y", 0);
        assert_eq!(tail, vec![b'x']);
        push_tail(&mut tail, b"", 8);
        assert_eq!(tail, vec![b'x']);
    }
}
