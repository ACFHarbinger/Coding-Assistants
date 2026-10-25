//! Allowlisted file exchange under the three configured roots.

use super::policy::{classify, Config, RootKind};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HASH_LIMIT: u64 = 64 * 1024 * 1024;
const LIST_LIMIT: usize = 200;

pub fn validate(config: &Config, args: &Value) -> Result<Value, String> {
    let path = required_str(args, "path")?;
    let intent = required_str(args, "intent")?;
    let write = match intent {
        "read" => false,
        "write" | "overwrite" => true,
        _ => return Err("intent must be read, write, or overwrite".into()),
    };
    let (kind, resolved) = classify(config, path, write)?;
    if intent == "read" && !resolved.is_file() {
        return Err("read intent requires an existing file".into());
    }
    if intent == "write" && resolved.exists() {
        return Err("write intent refuses an existing file; use overwrite".into());
    }
    if intent == "overwrite" && resolved.is_dir() {
        return Err("overwrite intent requires a file path".into());
    }
    Ok(json!({
        "ok": true,
        "root": kind.as_str(),
        "path": resolved.display().to_string(),
        "exists": resolved.exists(),
    }))
}

pub fn list_assets(config: &Config, args: &Value) -> Result<Value, String> {
    let root_name = required_str(args, "root")?;
    let root = match root_name {
        "input" => &config.input,
        "working" => &config.working,
        "exports" => &config.exports,
        _ => return Err("root must be input, working, or exports".into()),
    };
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    let truncated = files.len() > LIST_LIMIT;
    files.truncate(LIST_LIMIT);
    Ok(json!({ "root": root_name, "files": files, "truncated": truncated }))
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|err| format!("list {}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let meta = fs::symlink_metadata(entry.path()).map_err(|err| err.to_string())?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            collect(root, &entry.path(), out)?;
        } else if meta.is_file() {
            if let Ok(rel) = entry.path().strip_prefix(root) {
                out.push(rel.display().to_string());
            }
        }
    }
    Ok(())
}

pub fn file_info(config: &Config, args: &Value) -> Result<Value, String> {
    let path = required_str(args, "path")?;
    let (kind, resolved) = classify(config, path, false)?;
    if !resolved.is_file() {
        return Err("path is not a file".into());
    }
    info_json(kind, &resolved)
}

pub fn scan_exports(config: &Config, args: &Value) -> Result<Value, String> {
    let since = args.get("since_unix").and_then(|v| v.as_u64()).unwrap_or(0);
    let mut files = Vec::new();
    collect(&config.exports, &config.exports, &mut files)?;
    let mut recent = Vec::new();
    for rel in files {
        let path = config.exports.join(&rel);
        let (kind, resolved) = classify(config, &path.display().to_string(), false)?;
        let modified = modified_unix(&resolved)?;
        if modified >= since {
            recent.push(json!({
                "root": kind.as_str(),
                "path": resolved.display().to_string(),
                "modified_unix": modified,
            }));
        }
    }
    Ok(json!({ "files": recent }))
}

pub fn wait_for_file(config: &Config, args: &Value) -> Result<Value, String> {
    let path = required_str(args, "path")?;
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(200)
        .min(15_000);
    let (kind, resolved) = classify(config, path, true)?;
    if kind == RootKind::Input {
        return Err("wait_for_file does not watch the input root".into());
    }
    let deadline = SystemTime::now() + Duration::from_millis(timeout_ms);
    let mut previous: Option<(u64, SystemTime)> = None;
    loop {
        if let Some(sample) = sample_file(&resolved)? {
            if sample.0 > 0 && previous == Some(sample) {
                let mut body = info_json(kind, &resolved)?;
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("stable".into(), json!(true));
                }
                return Ok(body);
            }
            if timeout_ms == 0 && sample.0 > 0 {
                let mut body = info_json(kind, &resolved)?;
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("stable".into(), json!(false));
                }
                return Ok(body);
            }
            previous = Some(sample);
        } else {
            previous = None;
        }
        if SystemTime::now() >= deadline {
            return Err("file was not a stable non-empty output before timeout".into());
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

pub fn open_document(config: &Config, args: &Value) -> Result<Value, String> {
    let path = required_str(args, "path")?;
    let (kind, resolved) = classify(config, path, false)?;
    if kind != RootKind::Input || !resolved.is_file() {
        return Err("csp_open_document only accepts an existing file in the input root".into());
    }
    Ok(json!({
        "dispatched": false,
        "path": resolved.display().to_string(),
        "reason": "no attested CLIPStudioPaint.exe; dispatch was not attempted",
    }))
}

fn info_json(kind: RootKind, path: &Path) -> Result<Value, String> {
    let meta = fs::metadata(path).map_err(|err| err.to_string())?;
    if meta.len() > HASH_LIMIT {
        return Err("file exceeds the 64 MiB hash limit".into());
    }
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    let digest = Sha256::digest(&bytes);
    Ok(json!({
        "root": kind.as_str(),
        "path": path.display().to_string(),
        "bytes": meta.len(),
        "modified_unix": modified_unix(path)?,
        "sha256": hex_encode(&digest),
    }))
}

fn sample_file(path: &Path) -> Result<Option<(u64, SystemTime)>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let meta = fs::metadata(path).map_err(|err| err.to_string())?;
    let modified = meta.modified().map_err(|err| err.to_string())?;
    Ok(Some((meta.len(), modified)))
}

fn modified_unix(path: &Path) -> Result<u64, String> {
    let modified = fs::metadata(path)
        .and_then(|meta| meta.modified())
        .map_err(|err| err.to_string())?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0))
}

fn required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{key} is required"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn stability_sample_keeps_subsecond_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.png");
        fs::write(&path, b"same-length").unwrap();
        let base = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let file = fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(base).unwrap();
        let first = sample_file(&path).unwrap().unwrap();
        file.set_modified(base + Duration::from_millis(10)).unwrap();
        let second = sample_file(&path).unwrap().unwrap();
        assert_eq!(first.0, second.0);
        assert_eq!(
            first.1.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            second.1.duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        assert_ne!(first.1, second.1);
    }
}
