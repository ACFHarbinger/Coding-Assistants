//! Allowlisted file exchange under the three configured roots.

use super::checked_file::CheckedFile;
use super::policy::{classify, Config, RootKind};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
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
    let (kind, resolved, mut opened) =
        CheckedFile::open(config, path, false)?.ok_or("path is not a file")?;
    let sample = sample_file(&opened.file)?;
    info_json(config, path, false, kind, &resolved, &mut opened, sample)
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
    wait_with_poll(config, args, || {})
}

fn wait_with_poll(
    config: &Config,
    args: &Value,
    mut before_poll: impl FnMut(),
) -> Result<Value, String> {
    let path = required_str(args, "path")?;
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(200)
        .min(15_000);
    let (kind, _) = classify(config, path, true)?;
    if kind == RootKind::Input {
        return Err("wait_for_file does not watch the input root".into());
    }
    let deadline = SystemTime::now() + Duration::from_millis(timeout_ms);
    let mut previous: Option<(u64, SystemTime)> = None;
    loop {
        before_poll();
        // Reclassify the caller's original path on every poll, including when
        // a previously missing output first appears. Never reopen it to hash.
        if let Some((kind, resolved, mut opened)) = CheckedFile::open(config, path, true)? {
            let sample = sample_file(&opened.file)?;
            if sample.0 > 0 && previous == Some(sample) {
                let mut body = info_json(config, path, true, kind, &resolved, &mut opened, sample)?;
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("stable".into(), json!(true));
                }
                return Ok(body);
            }
            if timeout_ms == 0 && sample.0 > 0 {
                let mut body = info_json(config, path, true, kind, &resolved, &mut opened, sample)?;
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

fn info_json(
    config: &Config,
    raw: &str,
    write: bool,
    kind: RootKind,
    path: &Path,
    opened: &mut CheckedFile,
    expected: (u64, SystemTime),
) -> Result<Value, String> {
    opened.revalidate(config, raw, write)?;
    let before = sample_file(&opened.file)?;
    if before != expected {
        return Err("file changed before hashing".into());
    }
    if before.0 > HASH_LIMIT {
        return Err("file exceeds the 64 MiB hash limit".into());
    }
    // Bound reads even if an exporter grows the file after the size check.
    let mut hasher = Sha256::new();
    let mut reader = (&mut opened.file).take(HASH_LIMIT + 1);
    let mut bytes = 0u64;
    let mut buffer = [0; 8192];
    loop {
        let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > HASH_LIMIT {
            return Err("file exceeds the 64 MiB hash limit".into());
        }
        hasher.update(&buffer[..count]);
    }
    opened.revalidate(config, raw, write)?;
    if sample_file(&opened.file)? != before || bytes != before.0 {
        return Err("file changed while hashing".into());
    }
    Ok(json!({
        "root": kind.as_str(),
        "path": path.display().to_string(),
        "bytes": bytes,
        "modified_unix": before.1.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        "sha256": hex_encode(&hasher.finalize()),
    }))
}

fn sample_file(file: &fs::File) -> Result<(u64, SystemTime), String> {
    let meta = file.metadata().map_err(|err| err.to_string())?;
    let modified = meta.modified().map_err(|err| err.to_string())?;
    Ok((meta.len(), modified))
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
        let first = sample_file(&file).unwrap();
        file.set_modified(base + Duration::from_millis(10)).unwrap();
        let second = sample_file(&file).unwrap();
        assert_eq!(first.0, second.0);
        assert_eq!(
            first.1.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            second.1.duration_since(UNIX_EPOCH).unwrap().as_secs()
        );
        assert_ne!(first.1, second.1);
    }
}

#[cfg(test)]
mod wait_policy_tests {
    use super::*;
    use crate::tests::sample_config;

    #[test]
    fn missing_output_can_appear_as_a_regular_file() {
        let (_dir, config) = sample_config();
        let path = config.exports.join("new.png");
        let mut first = true;
        let result = wait_with_poll(&config, &json!({"path": path, "timeout_ms": 500}), || {
            if first {
                fs::write(&path, b"safe output").unwrap();
                first = false;
            }
        })
        .unwrap();
        assert_eq!(result["stable"], true);
        assert_eq!(
            result["sha256"],
            hex_encode(&Sha256::digest(b"safe output"))
        );
    }

    #[test]
    fn hard_link_created_after_initial_validation_is_rejected() {
        let (dir, config) = sample_config();
        let outside = dir.path().join("outside.png");
        fs::write(&outside, b"private").unwrap();
        let path = config.exports.join("new.png");
        let mut polls = 0;
        let error = wait_with_poll(&config, &json!({"path": path}), || {
            polls += 1;
            if polls != 2 {
                return;
            }
            fs::hard_link(&outside, &path).unwrap();
        })
        .unwrap_err();
        assert!(error.contains("hard link"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_created_after_initial_validation_is_rejected() {
        let (dir, config) = sample_config();
        let outside = dir.path().join("outside.png");
        fs::write(&outside, b"private").unwrap();
        let path = config.exports.join("new.png");
        let mut polls = 0;
        let error = wait_with_poll(&config, &json!({"path": path}), || {
            polls += 1;
            if polls != 2 {
                return;
            }
            std::os::unix::fs::symlink(&outside, &path).unwrap();
        })
        .unwrap_err();
        assert!(error.contains("reparse"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn parent_replaced_by_a_symlink_while_waiting_is_rejected() {
        let (dir, config) = sample_config();
        let parent = config.exports.join("pending");
        fs::create_dir(&parent).unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("page.png"), b"private").unwrap();
        let path = parent.join("page.png");
        let mut polls = 0;
        let error = wait_with_poll(&config, &json!({"path": path}), || {
            polls += 1;
            if polls != 2 {
                return;
            }
            fs::remove_dir(&parent).unwrap();
            std::os::unix::fs::symlink(&outside, &parent).unwrap();
        })
        .unwrap_err();
        assert!(error.contains("reparse"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn replacing_an_open_file_cannot_redirect_the_hash_read() {
        let (dir, config) = sample_config();
        let path = config.exports.join("page.png");
        fs::write(&path, b"safe").unwrap();
        let raw = path.to_str().unwrap();
        let (kind, resolved, mut opened) = CheckedFile::open(&config, raw, true).unwrap().unwrap();
        let sample = sample_file(&opened.file).unwrap();
        let outside = dir.path().join("outside.png");
        fs::write(&outside, b"private").unwrap();
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert!(info_json(&config, raw, true, kind, &resolved, &mut opened, sample).is_err());
        let mut actual = String::new();
        opened.file.read_to_string(&mut actual).unwrap();
        assert_eq!(actual, "safe");
    }

    #[test]
    fn file_modified_after_the_stability_sample_is_rejected() {
        let (_dir, config) = sample_config();
        let path = config.exports.join("page.png");
        fs::write(&path, b"safe").unwrap();
        let raw = path.to_str().unwrap();
        let (kind, resolved, mut opened) = CheckedFile::open(&config, raw, true).unwrap().unwrap();
        let sample = sample_file(&opened.file).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(sample.1 + Duration::from_millis(10))
            .unwrap();
        assert!(info_json(&config, raw, true, kind, &resolved, &mut opened, sample).is_err());
    }

    #[test]
    fn hard_link_added_after_opening_is_rejected() {
        let (dir, config) = sample_config();
        let path = config.exports.join("page.png");
        fs::write(&path, b"safe").unwrap();
        let raw = path.to_str().unwrap();
        let (kind, resolved, mut opened) = CheckedFile::open(&config, raw, true).unwrap().unwrap();
        let sample = sample_file(&opened.file).unwrap();
        fs::hard_link(&path, dir.path().join("another-link.png")).unwrap();
        assert!(info_json(&config, raw, true, kind, &resolved, &mut opened, sample).is_err());
    }
}
