//! Policy for OS-level tool execution (`platform.md` P5).
//!
//! Never consults a shell. Decisions are `Allow` / `Deny` / `NeedsApproval`
//! from `SandboxStrictness` (default Standard = relaxed-default).

use crate::SandboxStrictness;
use std::path::Path;

const HARD_DENY: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "fish",
    "dash",
    "csh",
    "tcsh",
    "ksh",
    "cmd",
    "cmd.exe",
    "powershell",
    "pwsh",
    "sudo",
    "su",
    "doas",
    "shutdown",
    "reboot",
    "halt",
    "poweroff",
    "mkfs",
    "diskpart",
    "format",
];

const STRICT_AUTO: &[&str] = &["true", "false", "pwd"];

const STANDARD_AUTO: &[&str] = &[
    "true",
    "false",
    "pwd",
    "echo",
    "ls",
    "cat",
    "head",
    "tail",
    "wc",
    "date",
    "uname",
    "rg",
    "grep",
    "which",
    "basename",
    "dirname",
    "stat",
    "file",
    "sha256sum",
    "md5sum",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsToolDecision {
    Allow,
    Deny { reason: String },
    NeedsApproval { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsToolRequest {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub workspace: Option<String>,
}

impl OsToolRequest {
    pub fn basename(&self) -> &str {
        program_basename(&self.program)
    }
}

pub fn program_basename(program: &str) -> &str {
    program.rsplit(['/', '\\']).next().unwrap_or(program).trim()
}

fn is_hard_deny(basename: &str) -> bool {
    HARD_DENY
        .iter()
        .any(|denied| basename.eq_ignore_ascii_case(denied))
}

fn is_basename_only(program: &str) -> bool {
    let trimmed = program.trim();
    !trimmed.is_empty()
        && !trimmed.contains('/')
        && !trimmed.contains('\\')
        && !trimmed.contains('\0')
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

fn cwd_under_workspace(cwd: &str, workspace: &str) -> bool {
    let cwd = Path::new(cwd);
    let workspace = Path::new(workspace);
    if !cwd.is_absolute() || !workspace.is_absolute() {
        return false;
    }
    cwd.components()
        .take(workspace.components().count())
        .eq(workspace.components())
}

fn git_is_read_only(args: &[String]) -> bool {
    let mutating_flags = args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "-d" | "-D" | "-m" | "--delete" | "--force" | "-f"
        )
    });
    if mutating_flags {
        return false;
    }
    let sub = args
        .iter()
        .find(|arg| !arg.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("");
    matches!(
        sub,
        "" | "status"
            | "log"
            | "diff"
            | "show"
            | "rev-parse"
            | "describe"
            | "ls-files"
            | "cat-file"
            | "version"
            | "help"
            | "branch"
    )
}

fn find_needs_approval(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "-delete" || arg == "-exec")
}

/// Decide whether `request` may run under `strictness`.
pub fn decide(request: &OsToolRequest, strictness: SandboxStrictness) -> OsToolDecision {
    let program = request.program.trim();
    if program.is_empty() {
        return OsToolDecision::Deny {
            reason: "program must not be empty".into(),
        };
    }
    if program.contains('\0') || program.starts_with('-') {
        return OsToolDecision::Deny {
            reason: "program must be an executable name, not a flag".into(),
        };
    }
    let basename = program_basename(program);
    if is_hard_deny(basename) {
        return OsToolDecision::Deny {
            reason: format!("`{basename}` is blocked (shell or destructive system tool)"),
        };
    }

    let cwd = request.cwd.trim();
    if cwd.is_empty() || !Path::new(cwd).is_absolute() {
        return OsToolDecision::Deny {
            reason: "cwd must be an absolute path".into(),
        };
    }

    match strictness {
        SandboxStrictness::Permissive => {
            if !is_basename_only(program) && !Path::new(program).is_absolute() {
                return OsToolDecision::Deny {
                    reason: "program must be a PATH name or an absolute path".into(),
                };
            }
            OsToolDecision::Allow
        }
        SandboxStrictness::Standard | SandboxStrictness::Strict => {
            if !is_basename_only(program) {
                return OsToolDecision::Deny {
                    reason: "sandbox requires a PATH program name (no path separators)".into(),
                };
            }
            let Some(workspace) = request
                .workspace
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                return OsToolDecision::Deny {
                    reason: "sandbox requires an absolute workspace".into(),
                };
            };
            if !Path::new(workspace).is_absolute() || !cwd_under_workspace(cwd, workspace) {
                return OsToolDecision::Deny {
                    reason: "cwd must be inside the workspace".into(),
                };
            }
            if strictness == SandboxStrictness::Strict {
                if STRICT_AUTO.contains(&basename) {
                    return OsToolDecision::Allow;
                }
                return OsToolDecision::NeedsApproval {
                    reason: format!("strict sandbox requires approval to run `{basename}`"),
                };
            }
            if basename == "git" {
                return if git_is_read_only(&request.args) {
                    OsToolDecision::Allow
                } else {
                    OsToolDecision::NeedsApproval {
                        reason: "mutating git subcommand requires approval".into(),
                    }
                };
            }
            if basename == "find" && find_needs_approval(&request.args) {
                return OsToolDecision::NeedsApproval {
                    reason: "find -delete/-exec requires approval".into(),
                };
            }
            if STANDARD_AUTO.contains(&basename) {
                return OsToolDecision::Allow;
            }
            OsToolDecision::NeedsApproval {
                reason: format!("`{basename}` is not on the standard auto-allow list"),
            }
        }
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
