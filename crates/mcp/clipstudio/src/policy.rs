//! Path roots and the single-use preflight token.
//!
//! The production profile this bridge mirrors is Windows, CLIP STUDIO PAINT
//! 5.1.2.0, `zh-CN`, and DPI 120. This crate does not attest that executable
//! and does not send pointer or key input.

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const TOKEN_TTL: Duration = Duration::from_secs(45);
pub const PROFILE: &str = "Windows + CLIP STUDIO PAINT 5.1.2.0 production + zh-CN + DPI 120";
pub const NO_SESSION: &str = "no attested CLIP STUDIO PAINT 5.1.2.0 session; this bridge does not send pointer, key, or window input";

pub const ATOMIC_INPUT: &[&str] = &[
    "csp_move_pointer",
    "csp_click",
    "csp_drag",
    "csp_scroll",
    "csp_type_text",
    "csp_navigation_key",
    "csp_run_command",
    "csp_run_auto_action",
    "csp_run_command_sequence",
    "csp_native_file_dialog",
];

pub const SESSION_TOOLS: &[&str] = &[
    "csp_get_activity",
    "csp_list_windows",
    "csp_get_state",
    "csp_capture",
    "csp_compare_captures",
    "csp_sample_pixel",
    "csp_inspect_ui",
    "csp_launch",
    "csp_focus",
    "csp_window_action",
    "csp_wait_visual_change",
    "csp_cancel",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootKind {
    Input,
    Working,
    Exports,
}

impl RootKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Working => "working",
            Self::Exports => "exports",
        }
    }

    pub fn allows_write(self) -> bool {
        !matches!(self, Self::Input)
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub input: PathBuf,
    pub working: PathBuf,
    pub exports: PathBuf,
    pub commands: Vec<String>,
    pub auto_actions: Vec<String>,
}

#[derive(Deserialize)]
struct RawConfig {
    input: String,
    working: String,
    exports: String,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    auto_actions: Vec<String>,
}

pub fn load_config(path: &Path) -> Result<Config, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let raw: RawConfig = serde_json::from_str(&raw).map_err(|err| format!("config json: {err}"))?;
    let input = require_dir("input", &raw.input)?;
    let working = require_dir("working", &raw.working)?;
    let exports = require_dir("exports", &raw.exports)?;
    for (left, right) in [(&input, &working), (&input, &exports), (&working, &exports)] {
        if left.starts_with(right) || right.starts_with(left) {
            return Err("asset roots must be separate directories".into());
        }
    }
    Ok(Config {
        input,
        working,
        exports,
        commands: raw.commands,
        auto_actions: raw.auto_actions,
    })
}

fn require_dir(label: &str, raw: &str) -> Result<PathBuf, String> {
    reject_syntax(raw)?;
    let path = PathBuf::from(raw.trim());
    if !path.is_absolute() {
        return Err(format!("{label} root must be absolute"));
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(format!("{label} root rejects parent segments"));
    }
    let meta = fs::symlink_metadata(&path).map_err(|err| format!("{label} root: {err}"))?;
    if meta.file_type().is_symlink() {
        return Err(format!("{label} root rejects reparse points"));
    }
    if !meta.is_dir() {
        return Err(format!("{label} root must be a directory"));
    }
    let canon = fs::canonicalize(&path).map_err(|err| format!("{label} root: {err}"))?;
    Ok(canon)
}

pub fn classify(config: &Config, raw: &str, write: bool) -> Result<(RootKind, PathBuf), String> {
    reject_syntax(raw)?;
    let path = PathBuf::from(raw.trim());
    if !path.is_absolute() {
        return Err("path must be absolute".into());
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("parent segments are rejected".into());
    }
    let resolved = canonical_candidate(&path)?;
    let roots = [
        (RootKind::Input, &config.input),
        (RootKind::Working, &config.working),
        (RootKind::Exports, &config.exports),
    ];
    let (kind, root) = roots
        .iter()
        .find(|(_, root)| resolved.starts_with(root))
        .copied()
        .ok_or_else(|| "path is outside the configured asset roots".to_string())?;
    if write && !kind.allows_write() {
        return Err("the input root is read-only".into());
    }
    reject_symlinks(root, &resolved)?;
    if let Ok(meta) = fs::symlink_metadata(&resolved) {
        if meta.is_file() && link_count(&meta) > 1 {
            return Err("hard links are rejected".into());
        }
    }
    Ok((kind, resolved))
}

fn canonical_candidate(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        let meta = fs::symlink_metadata(path).map_err(|err| format!("path: {err}"))?;
        if meta.file_type().is_symlink() {
            return Err("reparse points are rejected".into());
        }
        return fs::canonicalize(path).map_err(|err| format!("path: {err}"));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or("path has no parent")?;
    let name = path.file_name().ok_or("path has no file name")?;
    let parent = fs::canonicalize(parent).map_err(|err| format!("path: {err}"))?;
    Ok(parent.join(name))
}

fn reject_symlinks(root: &Path, start: &Path) -> Result<(), String> {
    let mut cur = start.to_path_buf();
    loop {
        if let Ok(meta) = fs::symlink_metadata(&cur) {
            if meta.file_type().is_symlink() {
                return Err("reparse points are rejected".into());
            }
        }
        if cur == root || !cur.pop() {
            break;
        }
    }
    Ok(())
}

fn reject_syntax(raw: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("path is empty".into());
    }
    if trimmed.starts_with(r"\\") || trimmed.starts_with("//") {
        return Err("UNC paths are rejected".into());
    }
    if trimmed.contains('*') || trimmed.contains('?') {
        return Err("wildcards are rejected".into());
    }
    if has_extra_colon(trimmed) {
        return Err("alternate data streams are rejected".into());
    }
    Ok(())
}

fn has_extra_colon(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let rest = if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        &bytes[3..]
    } else {
        bytes
    };
    rest.contains(&b':')
}

fn link_count(meta: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        std::os::unix::fs::MetadataExt::nlink(meta)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::MetadataExt::number_of_links(meta)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = meta;
        1
    }
}

#[derive(Debug)]
struct Ticket {
    tool: String,
    params: Value,
    expires: Instant,
}

#[derive(Debug, Default)]
pub struct Tokens {
    next: u64,
    live: HashMap<String, Ticket>,
}

impl Tokens {
    pub fn issue(&mut self, tool: &str, params: &Value) -> String {
        self.next = self.next.saturating_add(1);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut hasher = Sha256::new();
        hasher.update(format!("{nanos}:{}:{tool}", self.next));
        let token = hex_encode(&hasher.finalize());
        self.live.insert(
            token.clone(),
            Ticket {
                tool: tool.to_string(),
                params: params.clone(),
                expires: Instant::now() + TOKEN_TTL,
            },
        );
        token
    }

    /// Match the tool and canonical parameters, then consume the token.
    /// A mismatch still consumes it.
    pub fn take(&mut self, token: &str, tool: &str, params: &Value) -> Result<(), String> {
        let Some(ticket) = self.live.remove(token) else {
            return Err("confirmToken was not issued".into());
        };
        if Instant::now() > ticket.expires {
            return Err("confirmToken expired".into());
        }
        if ticket.tool != tool || ticket.params != *params {
            return Err("confirmToken does not match this call".into());
        }
        Ok(())
    }
}

pub fn business_params(args: &Value) -> Result<Value, String> {
    let mut obj = args
        .as_object()
        .cloned()
        .ok_or("arguments must be an object")?;
    obj.remove("captureId");
    obj.remove("confirmToken");
    Ok(Value::Object(obj))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}
