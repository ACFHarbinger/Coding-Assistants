//! Three-way tree reconcile (S6 / #96).
//!
//! Auto-applies independent paths and proven journal/markdown appends.
//! Conflicts are staged under `sync/conflicts/` and never overwrite live
//! `hub.db` or a disagreed live path.

use super::journal::append_merge;
use super::policy::{self, classify};
use super::types::{sha256_hex, Category, SyncResult};
use super::SyncError;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn remember_base(home: &Path) -> Result<(), SyncError> {
    let dest = home.join("sync").join("staging").join("base");
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(io_err)?;
    }
    for relative in collect_mergeable(home) {
        copy_file(home, &dest, &relative)?;
    }
    Ok(())
}

pub fn reconcile(local: &Path, base: &Path, remote: &Path) -> Result<SyncResult, SyncError> {
    let mut paths = BTreeSet::new();
    collect_into(local, &mut paths);
    collect_into(base, &mut paths);
    collect_into(remote, &mut paths);
    let mut result = SyncResult::default();
    for relative in paths {
        match decide(&relative, local, base, remote)? {
            Action::Keep => {}
            Action::ApplyRemote => {
                copy_file(remote, local, &relative)?;
                result.downloaded += 1;
            }
            Action::Write(bytes) => {
                write_relative(local, &relative, &bytes)?;
                result.downloaded += 1;
            }
            Action::Conflict(reason) => {
                stage_conflict(local, base, remote, &relative, reason)?;
                result.conflicts += 1;
            }
        }
    }
    result.warnings.push("live hub.db was not replaced".into());
    Ok(result)
}

enum Action {
    Keep,
    ApplyRemote,
    Write(Vec<u8>),
    Conflict(&'static str),
}

fn decide(relative: &Path, local: &Path, base: &Path, remote: &Path) -> Result<Action, SyncError> {
    if is_live_db(relative) {
        let local_h = hash_at(local, relative);
        let remote_h = hash_at(remote, relative);
        if local_h != remote_h && remote_h.is_some() {
            return Ok(Action::Conflict("hub-db-divergence"));
        }
        return Ok(Action::Keep);
    }
    let base_b = read_at(base, relative);
    let local_b = read_at(local, relative);
    let remote_b = read_at(remote, relative);
    Ok(
        match (base_b.as_deref(), local_b.as_deref(), remote_b.as_deref()) {
            (_, l, r) if l == r => Action::Keep,
            (_, Some(_), None) | (_, None, Some(_)) if base_b.is_some() => {
                Action::Conflict("delete-confirm")
            }
            (_, None, Some(_)) => Action::ApplyRemote,
            (Some(base), Some(local), Some(remote)) if local == base && remote != base => {
                Action::ApplyRemote
            }
            (Some(base), Some(local), Some(remote)) if remote == base && local != base => {
                Action::Keep
            }
            (Some(base), Some(local), Some(remote)) if local != remote => {
                if append_ok(relative) {
                    if let Some(merged) = append_merge(base, local, remote) {
                        Action::Write(merged)
                    } else {
                        Action::Conflict("journal-rewrite")
                    }
                } else {
                    Action::Conflict("same-path-edit")
                }
            }
            _ => Action::Keep,
        },
    )
}

fn append_ok(relative: &Path) -> bool {
    matches!(
        classify(relative),
        Category::PrivateJournal | Category::MarkdownExport
    )
}

fn collect_mergeable(home: &Path) -> Vec<PathBuf> {
    let mut files = BTreeSet::new();
    collect_into(home, &mut files);
    files.into_iter().collect()
}

fn collect_into(root: &Path, files: &mut BTreeSet<PathBuf>) {
    visit(root, root, files);
}

fn visit(home: &Path, dir: &Path, files: &mut BTreeSet<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            visit(home, &path, files);
            continue;
        }
        let relative = path.strip_prefix(home).unwrap_or(&path).to_path_buf();
        if skip_merge(&relative) {
            continue;
        }
        files.insert(relative);
    }
}

fn skip_merge(relative: &Path) -> bool {
    if matches!(relative.to_str(), Some("hub.db-wal" | "hub.db-shm")) {
        return true;
    }
    !policy::may_upload(policy::default_policy(classify(relative)))
}

fn is_live_db(relative: &Path) -> bool {
    matches!(
        relative.to_str(),
        Some("hub.db" | "hub.db-wal" | "hub.db-shm")
    )
}

fn read_at(root: &Path, relative: &Path) -> Option<Vec<u8>> {
    fs::read(root.join(relative)).ok()
}

fn hash_at(root: &Path, relative: &Path) -> Option<String> {
    read_at(root, relative).map(|bytes| sha256_hex(&bytes))
}

fn copy_file(from: &Path, to: &Path, relative: &Path) -> Result<(), SyncError> {
    let src = from.join(relative);
    if !src.is_file() {
        return Ok(());
    }
    write_relative(to, relative, &fs::read(&src).map_err(io_err)?)
}

fn write_relative(root: &Path, relative: &Path, bytes: &[u8]) -> Result<(), SyncError> {
    if relative.is_absolute() || relative.components().any(|c| c.as_os_str() == "..") {
        return Err(SyncError::Invalid("merge path is not relative".into()));
    }
    let dest = root.join(relative);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    fs::write(dest, bytes).map_err(io_err)
}

fn stage_conflict(
    local: &Path,
    base: &Path,
    remote: &Path,
    relative: &Path,
    reason: &str,
) -> Result<(), SyncError> {
    let slug = relative.to_string_lossy().replace(['/', '\\'], "__");
    let dest = local.join("sync").join("conflicts").join(slug);
    fs::create_dir_all(&dest).map_err(io_err)?;
    fs::write(dest.join("reason.txt"), reason).map_err(io_err)?;
    if let Some(bytes) = read_at(base, relative) {
        fs::write(dest.join("base"), bytes).map_err(io_err)?;
    }
    if let Some(bytes) = read_at(local, relative) {
        fs::write(dest.join("local"), bytes).map_err(io_err)?;
    }
    if let Some(bytes) = read_at(remote, relative) {
        fs::write(dest.join("remote"), bytes).map_err(io_err)?;
    }
    Ok(())
}

fn io_err(error: std::io::Error) -> SyncError {
    SyncError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn two_devices_ahead_merge_independent_paths_and_journal_appends() {
        let base = tempdir().unwrap();
        let a = tempdir().unwrap();
        let b = tempdir().unwrap();
        write(
            base.path(),
            "journals/claude.md",
            b"# base\n<!--ENC-->gAAAAABx\n",
        );
        write(
            a.path(),
            "journals/claude.md",
            b"# base\n<!--ENC-->gAAAAABx\nA\n",
        );
        write(
            b.path(),
            "journals/claude.md",
            b"# base\n<!--ENC-->gAAAAABx\nB\n",
        );
        write(base.path(), "markdown/shared.md", b"same");
        write(a.path(), "markdown/shared.md", b"same");
        write(b.path(), "markdown/shared.md", b"same");
        write(a.path(), "markdown/only-a.md", b"from-a");
        write(b.path(), "markdown/only-b.md", b"from-b");
        write(base.path(), "hub.db", b"db0");
        write(a.path(), "hub.db", b"db-a");
        write(b.path(), "hub.db", b"db-b");
        write(a.path(), "keys/cloud-sync.key", b"secret-key");

        let result = reconcile(a.path(), base.path(), b.path()).unwrap();
        assert!(result.downloaded >= 2);
        assert!(result.conflicts >= 1);
        assert_eq!(
            fs::read(a.path().join("journals/claude.md")).unwrap(),
            b"# base\n<!--ENC-->gAAAAABx\nA\nB\n"
        );
        assert_eq!(
            fs::read(a.path().join("markdown/only-a.md")).unwrap(),
            b"from-a"
        );
        assert_eq!(
            fs::read(a.path().join("markdown/only-b.md")).unwrap(),
            b"from-b"
        );
        assert_eq!(fs::read(a.path().join("hub.db")).unwrap(), b"db-a");
        assert!(a.path().join("sync/conflicts/hub.db").is_dir());
        assert!(!a.path().join("keys/cloud-sync.key.from-b").exists());
    }

    #[test]
    fn unclean_same_path_and_journal_rewrite_leave_live_untouched() {
        let base = tempdir().unwrap();
        let a = tempdir().unwrap();
        let b = tempdir().unwrap();
        write(base.path(), "markdown/note.md", b"base");
        write(a.path(), "markdown/note.md", b"edit-a");
        write(b.path(), "markdown/note.md", b"edit-b");
        write(
            base.path(),
            "journals/claude.md",
            b"# base\n<!--ENC-->gAAAAABold\n",
        );
        write(
            a.path(),
            "journals/claude.md",
            b"# base\n<!--ENC-->gAAAAABnew\nA\n",
        );
        write(
            b.path(),
            "journals/claude.md",
            b"# base\n<!--ENC-->gAAAAABold\nB\n",
        );
        let before_note = fs::read(a.path().join("markdown/note.md")).unwrap();
        let before_journal = fs::read(a.path().join("journals/claude.md")).unwrap();
        let result = reconcile(a.path(), base.path(), b.path()).unwrap();
        assert!(result.conflicts >= 2);
        assert_eq!(
            fs::read(a.path().join("markdown/note.md")).unwrap(),
            before_note
        );
        assert_eq!(
            fs::read(a.path().join("journals/claude.md")).unwrap(),
            before_journal
        );
        assert!(a
            .path()
            .join("sync/conflicts/markdown__note.md/reason.txt")
            .exists());
    }
}
