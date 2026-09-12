//! Workspace validation and bootstrapping guardrails (#215).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceValidation {
    pub path: String,
    pub valid: bool,
    pub exists: bool,
    pub is_dir: bool,
    pub is_bootstrapped: bool,
    pub parent_exists: bool,
    pub is_system_dir: bool,
    pub error: Option<String>,
}

/// Expands a leading `~` or `~/` to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    } else if path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(path)
}

/// Checks if a path is a root filesystem or critical system directory.
pub fn is_system_directory(path: &Path) -> bool {
    #[cfg(unix)]
    {
        const FORBIDDEN_DIRS: &[&str] = &[
            "/",
            "/bin",
            "/boot",
            "/dev",
            "/etc",
            "/lib",
            "/lib64",
            "/lost+found",
            "/media",
            "/mnt",
            "/opt",
            "/proc",
            "/root",
            "/run",
            "/sbin",
            "/sys",
            "/usr",
            "/var",
        ];
        let is_forbidden = |candidate: &Path| {
            FORBIDDEN_DIRS.iter().any(|dir| {
                if *dir == "/" {
                    candidate == Path::new("/")
                } else {
                    candidate.starts_with(Path::new(dir))
                }
            })
        };
        // Check every existing ancestor as well as the target. This catches
        // both a non-existent child such as `/etc/new-workspace` and a path
        // whose existing ancestor is a symlink into a protected directory.
        if path
            .ancestors()
            .filter_map(|ancestor| ancestor.canonicalize().ok())
            .filter(|canonical| canonical != Path::new("/"))
            .any(|canonical| is_forbidden(&canonical))
        {
            return true;
        }
        if is_forbidden(path) {
            return true;
        }
        let s = path.to_string_lossy();
        let s_trimmed = s.trim_end_matches('/');
        let s_check = if s_trimmed.is_empty() { "/" } else { s_trimmed };
        if FORBIDDEN_DIRS.contains(&s_check) {
            return true;
        }
        if path.parent().is_none() || path == Path::new("/") {
            return true;
        }
    }
    #[cfg(windows)]
    {
        if path.parent().is_none() {
            return true;
        }
        let s = path.to_string_lossy().to_uppercase();
        if s == "C:\\" || s.starts_with("C:\\WINDOWS") || s.starts_with("C:\\PROGRAM FILES") {
            return true;
        }
    }
    false
}

/// Performs non-destructive validation of a prospective workspace directory.
pub fn validate_workspace(path_str: &str) -> WorkspaceValidation {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return WorkspaceValidation {
            path: String::new(),
            valid: false,
            exists: false,
            is_dir: false,
            is_bootstrapped: false,
            parent_exists: false,
            is_system_dir: false,
            error: Some("Workspace path cannot be empty".to_string()),
        };
    }

    let work_path = Path::new(trimmed);
    if !work_path.is_absolute() {
        return WorkspaceValidation {
            path: trimmed.to_string(),
            valid: false,
            exists: false,
            is_dir: false,
            is_bootstrapped: false,
            parent_exists: false,
            is_system_dir: false,
            error: Some("Workspace root must be an absolute path".to_string()),
        };
    }

    if is_system_directory(work_path) {
        return WorkspaceValidation {
            path: trimmed.to_string(),
            valid: false,
            exists: work_path.exists(),
            is_dir: work_path.is_dir(),
            is_bootstrapped: false,
            parent_exists: true,
            is_system_dir: true,
            error: Some(format!(
                "Cannot use system directory '{}' as workspace",
                trimmed
            )),
        };
    }

    let exists = work_path.exists();
    let is_dir = work_path.is_dir();
    let is_bootstrapped = exists && work_path.join(".agent").is_dir();
    let parent_exists = work_path
        .parent()
        .map(|p| p.exists() && p.is_dir())
        .unwrap_or(false);

    let error = if !exists {
        Some(format!("Workspace directory '{}' does not exist", trimmed))
    } else if !is_dir {
        Some(format!(
            "Workspace path '{}' is a file, not a directory",
            trimmed
        ))
    } else {
        None
    };

    WorkspaceValidation {
        path: trimmed.to_string(),
        valid: exists && is_dir,
        exists,
        is_dir,
        is_bootstrapped,
        parent_exists,
        is_system_dir: false,
        error,
    }
}

/// Core logic for bootstrapping `.agent` skeletons in a workspace.
///
/// Guardrails:
/// 1. Path must be non-empty and absolute.
/// 2. Path cannot be a filesystem root or critical system directory.
/// 3. If directory does not exist, `create_dir` must be `Some(true)`.
/// 4. If directory does not exist, immediate parent directory must exist to prevent arbitrary deep creation.
/// 5. Directory cannot be already bootstrapped (`.agent` already exists).
pub async fn bootstrap_workspace_core(
    work_dir: &str,
    create_dir: Option<bool>,
) -> Result<(), String> {
    let trimmed = work_dir.trim();
    if trimmed.is_empty() {
        return Err("Workspace path cannot be empty".to_string());
    }

    let work_path = Path::new(trimmed);
    if !work_path.is_absolute() {
        return Err("Workspace root must be an absolute path".to_string());
    }

    if is_system_directory(work_path) {
        return Err(format!(
            "Cannot bootstrap workspace in system directory '{}'",
            trimmed
        ));
    }

    if !work_path.exists() {
        if create_dir != Some(true) {
            return Err(format!("Workspace directory '{}' does not exist", trimmed));
        }
        if let Some(parent) = work_path.parent() {
            if !parent.exists() || !parent.is_dir() {
                return Err(format!(
                    "Cannot create workspace: parent directory '{}' does not exist",
                    parent.display()
                ));
            }
        }
        tokio::fs::create_dir_all(work_path)
            .await
            .map_err(|e| format!("Failed to create workspace directory: {}", e))?;
    } else if !work_path.is_dir() {
        return Err(format!("Workspace path '{}' is not a directory", trimmed));
    }

    let base = work_path.join(".agent");
    if base.exists() {
        return Err("Workspace is already bootstrapped (.agent directory exists)".to_string());
    }

    tokio::fs::create_dir_all(base.join("rules"))
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(base.join("prompts"))
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(base.join("workflows"))
        .await
        .map_err(|e| e.to_string())?;

    let mcp_config = r#"{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    }
  }
}"#;

    tokio::fs::write(base.join("mcp_config.json"), mcp_config)
        .await
        .map_err(|e| e.to_string())?;

    let agents_md = r#"# AGENTS.md

See [.agent/AGENTS.md](.agent/AGENTS.md) — the authoritative instructions file for AI coding assistants working on this repository.
"#;
    tokio::fs::write(base.join("AGENTS.md"), agents_md)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_workspace_empty_and_relative() {
        let v1 = validate_workspace("");
        assert!(!v1.valid);
        assert_eq!(v1.error.as_deref(), Some("Workspace path cannot be empty"));

        let v2 = validate_workspace("relative/path/to/repo");
        assert!(!v2.valid);
        assert_eq!(
            v2.error.as_deref(),
            Some("Workspace root must be an absolute path")
        );
    }

    #[test]
    fn test_validate_workspace_system_directory() {
        let v = validate_workspace("/etc");
        assert!(!v.valid);
        assert!(v.is_system_dir);
        assert!(v.error.unwrap().contains("system directory"));

        assert!(is_system_directory(Path::new("/etc/new-workspace")));
    }

    #[test]
    fn test_validate_workspace_nonexistent_and_parent() {
        let temp = std::env::temp_dir().join(format!("test-ws-val-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);

        let v = validate_workspace(&temp.to_string_lossy());
        assert!(!v.valid);
        assert!(!v.exists);
        assert!(v.parent_exists);
        assert!(!v.is_bootstrapped);
        assert!(v.error.unwrap().contains("does not exist"));

        let deep = temp.join("a").join("b").join("c");
        let v_deep = validate_workspace(&deep.to_string_lossy());
        assert!(!v_deep.valid);
        assert!(!v_deep.parent_exists);
    }

    #[tokio::test]
    async fn test_bootstrap_workspace_guardrails() {
        let temp_base = std::env::temp_dir().join(format!("test-bs-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_base);
        std::fs::create_dir_all(&temp_base).unwrap();

        let target = temp_base.join("new_workspace");

        // 1. Missing target without create_dir errors
        let err1 = bootstrap_workspace_core(&target.to_string_lossy(), None).await;
        assert!(err1.is_err());
        assert!(err1.unwrap_err().contains("does not exist"));

        // 2. Missing deep target with missing parent errors even if create_dir is true
        let deep = temp_base.join("missing_parent").join("deep_target");
        let err2 = bootstrap_workspace_core(&deep.to_string_lossy(), Some(true)).await;
        assert!(err2.is_err());
        assert!(err2.unwrap_err().contains("parent directory"));

        // 3. System dir errors
        let err3 = bootstrap_workspace_core("/bin", Some(true)).await;
        assert!(err3.is_err());
        assert!(err3.unwrap_err().contains("system directory"));

        // 4. Valid creation with create_dir: true succeeds
        let res = bootstrap_workspace_core(&target.to_string_lossy(), Some(true)).await;
        assert!(res.is_ok());
        assert!(target.join(".agent").join("rules").is_dir());
        assert!(target.join(".agent").join("mcp_config.json").is_file());
        assert!(target.join(".agent").join("AGENTS.md").is_file());

        // 5. Already bootstrapped errors
        let err4 = bootstrap_workspace_core(&target.to_string_lossy(), None).await;
        assert!(err4.is_err());
        assert!(err4.unwrap_err().contains("already bootstrapped"));

        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_expand_tilde() {
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            assert_eq!(expand_tilde("~/test/path"), home_path.join("test/path"));
            assert_eq!(expand_tilde("~"), home_path);
        }
        assert_eq!(expand_tilde("/var/log"), PathBuf::from("/var/log"));
    }
}
