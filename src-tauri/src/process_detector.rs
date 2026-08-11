use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct DetectedProcess {
    pub pid: u32,
    pub agent: String,
    pub provider: String,
    pub model: String,
    pub command: String,
}

/// Inspect the local process table without touching process stdin/stdout or
/// changing process ownership. This is intentionally discovery-only: a CLI
/// process cannot be safely attached after launch unless it exposes a typed
/// service endpoint.
pub fn detect_agent_processes() -> Result<Vec<DetectedProcess>, String> {
    let output = Command::new("ps")
        .args(["-eo", "pid=,args="])
        .output()
        .map_err(|e| format!("failed to inspect local processes: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let mut processes = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((pid, command)) = line.trim().split_once(char::is_whitespace) else {
            continue;
        };
        let Ok(pid) = pid.trim().parse::<u32>() else {
            continue;
        };
        let command = command.trim();
        let lower = command.to_ascii_lowercase();
        let (agent, provider, model) = if lower.contains("claude") {
            ("Claude", "anthropic", "external-process")
        } else if lower.contains("codex") || lower.contains("chatgpt") {
            ("Codex", "openai", "external-process")
        } else if lower.contains("antigravity") || lower.contains("agy") || lower.contains("gemini")
        {
            ("Gemini", "google", "external-process")
        } else if lower.contains("grok") || lower.contains("supergrok") {
            ("Grok", "xai", "external-process")
        } else {
            continue;
        };
        processes.push(DetectedProcess {
            pid,
            agent: agent.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            command: command.to_string(),
        });
    }
    processes.sort_by_key(|process| process.pid);
    processes.dedup_by_key(|process| process.pid);
    Ok(processes)
}
