use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DetectedProcess {
    pub pid: u32,
    pub agent: String,
    pub provider: String,
    pub model: String,
    pub command: String,
}

fn classify_command(command: &str) -> Option<(&'static str, &'static str)> {
    // Only classify the executable itself. Matching arbitrary arguments or
    // parent paths incorrectly surfaced Claude Desktop helpers, Codex utility
    // processes, and Gemini's Chromium sandbox services as agent sessions.
    let executable = command.split_whitespace().next()?;
    let executable = Path::new(executable)
        .file_name()?
        .to_string_lossy()
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    match executable.as_str() {
        "claude" | "claude-code" => Some(("Claude", "anthropic")),
        "codex" | "chatgpt" => Some(("Codex", "openai")),
        "agy" | "gemini" => Some(("Gemini", "google")),
        "grok" | "supergrok" => Some(("Grok", "xai")),
        "opencode" => Some(("OpenCode", "opencode")),
        "vibe" => Some(("Mistral", "mistral")),
        _ => None,
    }
}

/// Inspect the local process table without touching process stdin/stdout or
/// changing process ownership. This is intentionally discovery-only: a CLI
/// process cannot be safely attached after launch unless it exposes a typed
/// service endpoint.
///
/// The platform process listing comes from [`hub::proc::list_process_lines`]
/// (`ps` on Unix, PowerShell on Windows — `ps` does not exist there, which
/// used to surface as "failed to inspect local processes: program not
/// found").
pub fn detect_agent_processes() -> Result<Vec<DetectedProcess>, String> {
    let mut processes = Vec::new();
    for (pid, command) in hub::proc::list_process_lines()? {
        let Some((agent, provider)) = classify_command(&command) else {
            continue;
        };
        processes.push(DetectedProcess {
            pid,
            agent: agent.to_string(),
            provider: provider.to_string(),
            model: "external-process".to_string(),
            command,
        });
    }
    processes.sort_by_key(|process| process.pid);
    processes.dedup_by_key(|process| process.pid);
    Ok(processes)
}

#[cfg(test)]
mod tests {
    use super::classify_command;

    #[test]
    fn classifies_agent_executables_only() {
        assert_eq!(
            classify_command("claude --continue"),
            Some(("Claude", "anthropic"))
        );
        assert_eq!(
            classify_command("/usr/local/bin/codex --dangerously-bypass-approvals-and-sandbox"),
            Some(("Codex", "openai"))
        );
        assert_eq!(
            classify_command("agy --dangerously-skip-permissions"),
            Some(("Gemini", "google"))
        );
        assert_eq!(classify_command("grok"), Some(("Grok", "xai")));
        assert_eq!(
            classify_command("gemini --resume"),
            Some(("Gemini", "google"))
        );
        assert_eq!(
            classify_command("/home/user/.opencode/bin/opencode run -m deepseek/deepseek-chat"),
            Some(("OpenCode", "opencode"))
        );
        assert_eq!(
            classify_command("vibe -p review --trust --output text"),
            Some(("Mistral", "mistral"))
        );
    }

    #[test]
    fn ignores_helpers_and_runtime_processes() {
        assert_eq!(classify_command("/usr/lib/claude-desktop/resources/co-work-linux-helper --socket /run/user/1000/claude.sock"), None);
        assert_eq!(
            classify_command(
                "/proc/self/exe --type=utility --utility-sub-type=node.mojom.NodeService"
            ),
            None
        );
        assert_eq!(
            classify_command(
                "/home/user/.codex/packages/standalone/releases/0.147.0/bin/codex-code-mode"
            ),
            None
        );
        assert_eq!(classify_command("node /opt/gemini/worker.js"), None);
        assert_eq!(classify_command("/usr/share/antigravity/antigravity"), None);
    }
}
