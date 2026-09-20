use super::*;

fn req(program: &str, args: &[&str], cwd: &str, workspace: Option<&str>) -> OsToolRequest {
    OsToolRequest {
        program: program.into(),
        args: args.iter().map(|s| (*s).to_string()).collect(),
        cwd: cwd.into(),
        workspace: workspace.map(str::to_string),
    }
}

#[test]
fn default_standard_allows_read_git_and_echo() {
    let git = req("git", &["status"], "/abs/ws", Some("/abs/ws"));
    assert_eq!(
        decide(&git, SandboxStrictness::Standard),
        OsToolDecision::Allow
    );
    let echo = req("echo", &["p5"], "/abs/ws/src", Some("/abs/ws"));
    assert_eq!(
        decide(&echo, SandboxStrictness::Standard),
        OsToolDecision::Allow
    );
}

#[test]
fn standard_requires_approval_for_mutating_git() {
    let commit = req("git", &["commit", "-m", "x"], "/abs/ws", Some("/abs/ws"));
    assert!(matches!(
        decide(&commit, SandboxStrictness::Standard),
        OsToolDecision::NeedsApproval { .. }
    ));
    let branch_delete = req("git", &["branch", "-d", "old"], "/abs/ws", Some("/abs/ws"));
    assert!(matches!(
        decide(&branch_delete, SandboxStrictness::Standard),
        OsToolDecision::NeedsApproval { .. }
    ));
}

#[test]
fn shells_and_sudo_are_always_denied() {
    for program in ["bash", "sh", "sudo", "powershell"] {
        let request = req(program, &["-c", "echo hi"], "/abs/ws", Some("/abs/ws"));
        assert!(
            matches!(
                decide(&request, SandboxStrictness::Permissive),
                OsToolDecision::Deny { .. }
            ),
            "{program} must be denied even when permissive"
        );
    }
}

#[test]
fn never_treats_args_as_a_shell_string() {
    let request = req(
        "echo",
        &["; rm -rf /", "$(whoami)"],
        "/abs/ws",
        Some("/abs/ws"),
    );
    assert_eq!(
        decide(&request, SandboxStrictness::Standard),
        OsToolDecision::Allow
    );
}

#[test]
fn standard_rejects_relative_cwd_and_missing_workspace() {
    let relative = req("echo", &["x"], "rel", Some("/abs/ws"));
    assert!(matches!(
        decide(&relative, SandboxStrictness::Standard),
        OsToolDecision::Deny { .. }
    ));
    let no_ws = req("echo", &["x"], "/abs/ws", None);
    assert!(matches!(
        decide(&no_ws, SandboxStrictness::Standard),
        OsToolDecision::Deny { .. }
    ));
    let outside = req("echo", &["x"], "/other", Some("/abs/ws"));
    assert!(matches!(
        decide(&outside, SandboxStrictness::Standard),
        OsToolDecision::Deny { .. }
    ));
}

#[test]
fn prefix_workspace_does_not_match_sibling() {
    let sibling = req("echo", &["x"], "/abs/workspace2", Some("/abs/workspace"));
    assert!(matches!(
        decide(&sibling, SandboxStrictness::Standard),
        OsToolDecision::Deny { .. }
    ));
}

#[test]
fn strict_auto_allows_only_the_tiny_list() {
    let pwd = req("pwd", &[], "/abs/ws", Some("/abs/ws"));
    assert_eq!(
        decide(&pwd, SandboxStrictness::Strict),
        OsToolDecision::Allow
    );
    let echo = req("echo", &["x"], "/abs/ws", Some("/abs/ws"));
    assert!(matches!(
        decide(&echo, SandboxStrictness::Strict),
        OsToolDecision::NeedsApproval { .. }
    ));
}

#[test]
fn permissive_allows_absolute_program_but_not_relative_path() {
    let abs = req("/usr/bin/echo", &["x"], "/tmp", None);
    assert_eq!(
        decide(&abs, SandboxStrictness::Permissive),
        OsToolDecision::Allow
    );
    let rel = req("./echo", &["x"], "/tmp", None);
    assert!(matches!(
        decide(&rel, SandboxStrictness::Permissive),
        OsToolDecision::Deny { .. }
    ));
}

#[test]
fn empty_or_flag_program_is_denied() {
    assert!(matches!(
        decide(
            &req("", &[], "/abs/ws", Some("/abs/ws")),
            SandboxStrictness::Standard
        ),
        OsToolDecision::Deny { .. }
    ));
    assert!(matches!(
        decide(
            &req("-c", &["echo"], "/abs/ws", Some("/abs/ws")),
            SandboxStrictness::Standard
        ),
        OsToolDecision::Deny { .. }
    ));
}
