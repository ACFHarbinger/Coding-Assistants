//! Owned and observed harness panes lifecycle, terminal multiplexing, and safety boundary (T6).

use super::vt_parser::VtScreenBuffer;
use hub::HarnessId;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneKind {
    /// Explicitly owned interactive process spawned with an interactive PTY.
    Owned,
    /// Read-only captured session from the Hub bridge or transcript.
    Observed,
}

impl PaneKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Observed => "observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneStatus {
    Running,
    Exited(i32),
    Failed(String),
    Captured,
}

impl PaneStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Running => "●",
            Self::Exited(0) => "✓",
            Self::Exited(_) => "✗",
            Self::Failed(_) => "⚠",
            Self::Captured => "👁",
        }
    }

    pub fn ascii_badge(&self) -> &'static str {
        match self {
            Self::Running => "[*]",
            Self::Exited(0) => "[OK]",
            Self::Exited(_) => "[X]",
            Self::Failed(_) => "[!]",
            Self::Captured => "[OBS]",
        }
    }

    pub fn badge_for(&self, ascii_mode: bool) -> &'static str {
        if ascii_mode {
            self.ascii_badge()
        } else {
            self.badge()
        }
    }
}

pub struct HarnessPane {
    pub id: String,
    pub title: String,
    pub harness: HarnessId,
    pub kind: PaneKind,
    pub status: PaneStatus,
    pub workspace: PathBuf,
    pub buffer: VtScreenBuffer,
    pub writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    pub master: Option<Arc<Mutex<Box<dyn MasterPty + Send>>>>,
    pub child: Option<Arc<Mutex<Box<dyn Child + Send + Sync>>>>,
    pub output_rx: Option<Receiver<Vec<u8>>>,
}

impl HarnessPane {
    /// Create an observed (read-only) pane from existing transcript or captured output.
    pub fn new_observed(
        id: String,
        harness: HarnessId,
        workspace: PathBuf,
        initial_text: &str,
    ) -> Self {
        let mut buffer = VtScreenBuffer::default();
        buffer.feed_str(initial_text);
        Self {
            title: format!("{} (observed)", harness.as_str()),
            id,
            harness,
            kind: PaneKind::Observed,
            status: PaneStatus::Captured,
            workspace,
            buffer,
            writer: None,
            master: None,
            child: None,
            output_rx: None,
        }
    }

    /// Forward keystrokes / text to the owned process. Fails closed on observed panes.
    pub fn write_input(&mut self, text: &str) -> Result<(), String> {
        if self.kind == PaneKind::Observed {
            return Err("Observed session is strictly read-only; keystrokes not forwarded.".into());
        }
        if let Some(writer) = &self.writer {
            let mut guard = writer.lock().map_err(|e| format!("lock error: {e}"))?;
            guard
                .write_all(text.as_bytes())
                .map_err(|e| format!("write error: {e}"))?;
            guard.flush().map_err(|e| format!("flush error: {e}"))?;
            Ok(())
        } else {
            Err("No active writer for pane".into())
        }
    }

    /// Propagate terminal resize to the master PTY.
    pub fn resize(&self, rows: u16, cols: u16) {
        if let Some(master) = &self.master {
            if let Ok(guard) = master.lock() {
                let _ = guard.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
        }
    }

    /// Drain incoming output chunks non-blockingly into the VT screen buffer.
    pub fn drain_output(&mut self) {
        if let Some(rx) = &self.output_rx {
            while let Ok(chunk) = rx.try_recv() {
                self.buffer.feed_bytes(&chunk);
            }
        }
        // Check if child exited
        if let Some(child_arc) = &self.child {
            if let Ok(mut child) = child_arc.try_lock() {
                if let Ok(Some(status)) = child.try_wait() {
                    let code = if status.success() { 0 } else { 1 };
                    self.status = PaneStatus::Exited(code);
                }
            }
        }
    }

    /// Terminate owned process.
    pub fn kill(&mut self) {
        if let Some(child_arc) = &self.child {
            if let Ok(mut child) = child_arc.lock() {
                let _ = child.kill();
            }
        }
        self.status = PaneStatus::Exited(130);
    }
}

pub struct HarnessWorkspaceState {
    pub panes: Vec<HarnessPane>,
    pub active_pane_idx: usize,
    pub is_split_view: bool,
    pub is_launcher_open: bool,
    pub launcher_selected_idx: usize,
    pub launcher_mode_is_owned: bool,
    pub next_pane_id: usize,
}

impl Default for HarnessWorkspaceState {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessWorkspaceState {
    pub fn new() -> Self {
        Self {
            panes: Vec::new(),
            active_pane_idx: 0,
            is_split_view: false,
            is_launcher_open: false,
            launcher_selected_idx: 0,
            launcher_mode_is_owned: true,
            next_pane_id: 1,
        }
    }

    pub fn active_pane(&self) -> Option<&HarnessPane> {
        self.panes.get(self.active_pane_idx)
    }

    pub fn active_pane_mut(&mut self) -> Option<&mut HarnessPane> {
        self.panes.get_mut(self.active_pane_idx)
    }

    pub fn next_pane(&mut self) {
        if !self.panes.is_empty() {
            self.active_pane_idx = (self.active_pane_idx + 1) % self.panes.len();
        }
    }

    pub fn prev_pane(&mut self) {
        if !self.panes.is_empty() {
            if self.active_pane_idx == 0 {
                self.active_pane_idx = self.panes.len() - 1;
            } else {
                self.active_pane_idx -= 1;
            }
        }
    }

    pub fn close_active_pane(&mut self) {
        if !self.panes.is_empty() {
            let mut pane = self.panes.remove(self.active_pane_idx);
            pane.kill();
            if self.active_pane_idx >= self.panes.len() && !self.panes.is_empty() {
                self.active_pane_idx = self.panes.len() - 1;
            }
        }
    }

    pub fn toggle_split(&mut self) {
        self.is_split_view = !self.is_split_view;
    }

    /// Spawn and attach an explicitly owned harness process with PTY.
    pub fn launch_owned(
        &mut self,
        harness: HarnessId,
        workspace: PathBuf,
        rows: u16,
        cols: u16,
    ) -> Result<String, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {e}"))?;

        let exe_name = harness.executable();
        let program = find_in_path(exe_name)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| {
                // The fallback remains an explicitly owned interactive shell,
                // but is spawned directly. Do not build a `sh -c` string from
                // the workspace path: valid paths may contain shell syntax.
                if cfg!(windows) {
                    "cmd.exe".into()
                } else {
                    "sh".into()
                }
            });

        let mut cmd = CommandBuilder::new(&program);
        cmd.cwd(&workspace);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn {}: {e}", program))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone master reader: {e}"))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take master writer: {e}"))?;

        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });

        let id = format!("pane-{}", self.next_pane_id);
        self.next_pane_id += 1;

        let pane = HarnessPane {
            id: id.clone(),
            title: format!("{} (owned)", harness.as_str()),
            harness,
            kind: PaneKind::Owned,
            status: PaneStatus::Running,
            workspace,
            buffer: VtScreenBuffer::default(),
            writer: Some(Arc::new(Mutex::new(writer))),
            master: Some(Arc::new(Mutex::new(pair.master))),
            child: Some(Arc::new(Mutex::new(child))),
            output_rx: Some(rx),
        };

        self.panes.push(pane);
        self.active_pane_idx = self.panes.len() - 1;
        Ok(id)
    }

    /// Add an observed read-only captured session.
    pub fn add_observed(
        &mut self,
        harness: HarnessId,
        workspace: PathBuf,
        initial_text: &str,
    ) -> String {
        let id = format!("pane-{}", self.next_pane_id);
        self.next_pane_id += 1;
        let pane = HarnessPane::new_observed(id.clone(), harness, workspace, initial_text);
        self.panes.push(pane);
        self.active_pane_idx = self.panes.len() - 1;
        id
    }

    /// Non-blocking drain of all child output streams into their respective buffers.
    pub fn drain_all(&mut self) {
        for pane in &mut self.panes {
            pane.drain_output();
        }
    }
}

fn find_in_path(exe: &str) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
