//! Minimal glob matching + workspace file walk for
//! `GateCriterion::MaxLinesPerFile` (S2). No new dependency: `*`/`?`
//! within one path segment, `**` for zero-or-more segments — enough for
//! `**/*.rs`-style patterns without pulling in a crate for it.

use std::path::{Path, PathBuf};

const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".ca-worktrees",
    "dist",
    "build",
    ".venv",
    "__pycache__",
];

pub(crate) fn walk_workspace_files(workspace: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![workspace.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)?;
        for entry in entries {
            let entry = entry?;
            // Never follow a workspace symlink into another tree or a cycle.
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if kind.is_dir() {
                if !EXCLUDED_DIRS.contains(&name.as_ref()) {
                    stack.push(path);
                }
            } else if kind.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

pub(crate) fn glob_match(pattern: &str, relative_path: &str) -> bool {
    let pat: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let segs: Vec<&str> = relative_path.split('/').filter(|s| !s.is_empty()).collect();
    match_segments(&pat, &segs)
}

fn match_segments(pat: &[&str], segs: &[&str]) -> bool {
    match pat.first() {
        None => segs.is_empty(),
        Some(&"**") => {
            match_segments(&pat[1..], segs) || (!segs.is_empty() && match_segments(pat, &segs[1..]))
        }
        Some(p) => {
            !segs.is_empty() && match_segment(p, segs[0]) && match_segments(&pat[1..], &segs[1..])
        }
    }
}

fn match_segment(pattern: &str, text: &str) -> bool {
    fn helper(p: &[u8], t: &[u8]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some(b'*'), _) => helper(&p[1..], t) || (!t.is_empty() && helper(p, &t[1..])),
            (Some(b'?'), Some(_)) => helper(&p[1..], &t[1..]),
            (Some(pc), Some(tc)) if pc == tc => helper(&p[1..], &t[1..]),
            _ => false,
        }
    }
    helper(pattern.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn workspace_walk_does_not_follow_symlink_cycles_or_external_files() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("local.rs"), "local").unwrap();
        std::fs::write(outside.path().join("external.rs"), "outside").unwrap();
        std::os::unix::fs::symlink(root.path(), root.path().join("cycle")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("outside")).unwrap();
        let files = walk_workspace_files(root.path()).unwrap();
        assert_eq!(files, vec![root.path().join("local.rs")]);
    }

    #[test]
    fn missing_workspace_is_an_error_instead_of_an_empty_success() {
        let root = tempfile::tempdir().unwrap();
        assert!(walk_workspace_files(&root.path().join("missing")).is_err());
    }

    #[test]
    fn double_star_matches_any_depth() {
        assert!(glob_match("**/*.rs", "crates/hub/src/lib.rs"));
        assert!(glob_match("**/*.rs", "lib.rs"));
        assert!(!glob_match("**/*.rs", "crates/hub/src/lib.ts"));
    }

    #[test]
    fn single_star_does_not_cross_segment_boundary() {
        assert!(glob_match("src/*.rs", "src/lib.rs"));
        assert!(!glob_match("src/*.rs", "src/sub/lib.rs"));
    }

    #[test]
    fn question_mark_matches_one_char() {
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        assert!(!glob_match("a?c", "abbc"));
    }
}
