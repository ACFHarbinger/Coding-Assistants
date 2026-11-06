//! Workspace-relative file IO for in-process agents.
//!
//! Every path is resolved against `work_dir`. Absolute paths, `..`, and
//! symlinks are refused so a model marker cannot leave the workspace.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Largest file a model-directed append or line edit may produce.
pub const MAX_WRITE_BYTES: usize = 1024 * 1024;

pub struct FileTools {
    pub work_dir: PathBuf,
}

impl FileTools {
    pub fn new(work_dir: String) -> Self {
        Self {
            work_dir: PathBuf::from(work_dir),
        }
    }

    pub async fn read_file(&self, relative_path: &str) -> Result<String, String> {
        let full_path = resolve_workspace_path(&self.work_dir, relative_path)?;
        tokio::fs::read_to_string(full_path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn write_file(&self, relative_path: &str, content: &str) -> Result<(), String> {
        let full_path = resolve_workspace_path(&self.work_dir, relative_path)?;
        write_atomic(&full_path, content).await
    }

    pub async fn append_file(&self, relative_path: &str, content: &str) -> Result<(), String> {
        bounded(content)?;
        let full_path = resolve_workspace_path(&self.work_dir, relative_path)?;
        let existing = match tokio::fs::read_to_string(&full_path).await {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.to_string()),
        };
        bounded(&existing)?;
        let mut combined = existing;
        combined.push_str(content);
        bounded(&combined)?;
        write_atomic(&full_path, &combined).await
    }

    pub async fn edit_lines(
        &self,
        relative_path: &str,
        start_line: usize,
        end_line: usize,
        replacement: &str,
    ) -> Result<(), String> {
        if start_line == 0 || end_line < start_line {
            return Err("line numbers start at 1".into());
        }
        bounded(replacement)?;
        let full_path = resolve_workspace_path(&self.work_dir, relative_path)?;
        let existing = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|error| error.to_string())?;
        bounded(&existing)?;
        let had_trailing_newline = existing.ends_with('\n');
        let lines = split_lines(&existing);
        if start_line > lines.len() || end_line > lines.len() {
            return Err(format!(
                "file has {} lines; cannot edit {start_line}-{end_line}",
                lines.len()
            ));
        }
        let mut next = Vec::new();
        next.extend_from_slice(&lines[..start_line - 1]);
        if !replacement.is_empty() {
            next.extend(replacement.split('\n').map(str::to_string));
        }
        next.extend_from_slice(&lines[end_line..]);
        let mut out = if next.is_empty() {
            String::new()
        } else {
            next.join("\n")
        };
        if !out.is_empty() && had_trailing_newline && !out.ends_with('\n') {
            out.push('\n');
        }
        bounded(&out)?;
        write_atomic(&full_path, &out).await
    }
}

pub fn split_lines(content: &str) -> Vec<String> {
    if content.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();
    if content.ends_with('\n') {
        lines.pop();
    }
    lines
}

fn bounded(content: &str) -> Result<(), String> {
    if content.len() > MAX_WRITE_BYTES {
        Err(format!(
            "content is {} bytes; the limit is {MAX_WRITE_BYTES}",
            content.len()
        ))
    } else {
        Ok(())
    }
}

/// Resolve `relative` inside `work_dir`.
///
/// The returned path is lexical for components that do not exist yet, and
/// every existing component must be a real directory (or a regular file at
/// the end). Symlinks are refused rather than followed.
pub fn resolve_workspace_path(work_dir: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = relative.trim();
    if relative.is_empty() {
        return Err("path is empty".into());
    }
    if relative.contains('\0') {
        return Err("path contains NUL".into());
    }
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err("absolute paths are not allowed".into());
    }
    let root = work_dir
        .canonicalize()
        .map_err(|error| format!("workspace is not accessible: {error}"))?;
    let mut acc = root.clone();
    let components: Vec<Component> = rel.components().collect();
    if components.is_empty() {
        return Err("path is empty".into());
    }
    for (index, component) in components.iter().enumerate() {
        let name = match component {
            Component::CurDir => continue,
            Component::Normal(name) => name,
            _ => return Err("path escapes the workspace".into()),
        };
        acc.push(name);
        match std::fs::symlink_metadata(&acc) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("refusing to follow a symlink".into());
            }
            Ok(meta) if meta.is_dir() => {
                if index + 1 == components.len() {
                    return Err("path is a directory".into());
                }
            }
            Ok(meta) if meta.is_file() => {
                if index + 1 != components.len() {
                    return Err("path passes through a non-directory".into());
                }
            }
            Ok(_) => return Err("path is not a regular file".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                for rest in &components[index + 1..] {
                    match rest {
                        Component::CurDir => {}
                        Component::Normal(name) => acc.push(name),
                        _ => return Err("path escapes the workspace".into()),
                    }
                }
                break;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    if acc == root || !acc.starts_with(&root) {
        return Err("path escapes the workspace".into());
    }
    if std::fs::symlink_metadata(&acc).is_ok_and(|meta| meta.is_dir()) {
        return Err("path is a directory".into());
    }
    Ok(acc)
}

async fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| "missing parent".to_string())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| error.to_string())?;
    let tmp = sibling_tmp(path);
    {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .await
            .map_err(|error| format!("failed to create temporary file: {error}"))?;
        use tokio::io::AsyncWriteExt;
        if let Err(error) = file.write_all(content.as_bytes()).await {
            let _ = std::fs::remove_file(&tmp);
            return Err(error.to_string());
        }
        if let Err(error) = file.flush().await {
            let _ = std::fs::remove_file(&tmp);
            return Err(error.to_string());
        }
    }
    if let Err(error) = tokio::fs::rename(&tmp, path).await {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("failed to replace {}: {error}", path.display()));
    }
    Ok(())
}

fn sibling_tmp(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let pid = std::process::id();
    path.with_file_name(format!(".{name}.{pid}.{n}.write-bridge.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools(dir: &tempfile::TempDir) -> FileTools {
        FileTools::new(dir.path().display().to_string())
    }

    #[tokio::test]
    async fn write_read_and_nested_create_stay_in_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let tools = tools(&dir);
        tools
            .write_file("./.agent/project_memory.md", "memory\n")
            .await
            .unwrap();
        assert_eq!(
            tools.read_file(".agent/project_memory.md").await.unwrap(),
            "memory\n"
        );
    }

    #[tokio::test]
    async fn append_adds_exact_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let tools = tools(&dir);
        tools.write_file("notes.txt", "hello").await.unwrap();
        tools.append_file("notes.txt", "\nthere").await.unwrap();
        assert_eq!(tools.read_file("notes.txt").await.unwrap(), "hello\nthere");
    }

    #[tokio::test]
    async fn edit_replaces_a_line_and_preserves_the_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let tools = tools(&dir);
        tools
            .write_file("src.txt", "one\ntwo\nthree\n")
            .await
            .unwrap();
        tools.edit_lines("src.txt", 2, 2, "TWO").await.unwrap();
        assert_eq!(
            tools.read_file("src.txt").await.unwrap(),
            "one\nTWO\nthree\n"
        );
    }

    #[tokio::test]
    async fn edit_range_can_delete_lines() {
        let dir = tempfile::tempdir().unwrap();
        let tools = tools(&dir);
        tools
            .write_file("src.txt", "one\ntwo\nthree\n")
            .await
            .unwrap();
        tools.edit_lines("src.txt", 2, 3, "").await.unwrap();
        assert_eq!(tools.read_file("src.txt").await.unwrap(), "one\n");
    }

    #[tokio::test]
    async fn edit_out_of_range_leaves_the_file_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let tools = tools(&dir);
        tools.write_file("src.txt", "one\n").await.unwrap();
        let error = tools.edit_lines("src.txt", 4, 4, "nope").await.unwrap_err();
        assert!(error.contains("1 lines"), "{error}");
        assert_eq!(tools.read_file("src.txt").await.unwrap(), "one\n");
    }

    #[tokio::test]
    async fn parent_dir_and_absolute_paths_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let tools = tools(&dir);
        let escaped = tools.write_file("../escape.txt", "x").await.unwrap_err();
        assert!(escaped.contains("escapes"), "{escaped}");
        let absolute = tools.write_file("/tmp/escape.txt", "x").await.unwrap_err();
        assert!(absolute.contains("absolute"), "{absolute}");
        assert!(!dir.path().join("escape.txt").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlink_escape_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(outside.path().join("secret.txt"), &link).unwrap();
        let tools = tools(&dir);
        let error = tools.write_file("link.txt", "nope").await.unwrap_err();
        assert!(error.contains("symlink"), "{error}");
        assert!(!outside.path().join("secret.txt").exists());

        std::os::unix::fs::symlink(outside.path(), dir.path().join("sub")).unwrap();
        let nested = tools.write_file("sub/file.txt", "nope").await.unwrap_err();
        assert!(nested.contains("symlink"), "{nested}");
        assert!(!outside.path().join("file.txt").exists());
    }
}
