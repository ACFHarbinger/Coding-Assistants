//! OS-level tool execution (`platform.md` P5 / #333).
//!
//! Policy + audited spawn. Callers never get a shell. Existing
//! `SandboxStrictness` is the approval/sandbox knob (default Standard).

mod exec;
mod policy;

pub use exec::{run, OsToolOutput, DEFAULT_TIMEOUT};
pub use policy::{decide, program_basename, OsToolDecision, OsToolRequest};

use crate::store::AuditEvent;
use crate::HubStore;
use crate::SandboxStrictness;
use serde_json::json;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct OsToolResult {
    pub status: String,
    pub audit_id: String,
    pub decision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub timed_out: bool,
}

fn process_json(
    request: &OsToolRequest,
    decision: &str,
    output: Option<&OsToolOutput>,
    proposed_id: Option<&str>,
) -> String {
    json!({
        "kind": "os_tool",
        "cmdline": std::iter::once(request.program.clone())
            .chain(request.args.iter().cloned())
            .collect::<Vec<_>>(),
        "exe": program_basename(&request.program),
        "cwd": request.cwd,
        "decision": decision,
        "exit_code": output.and_then(|out| out.exit_code),
        "timed_out": output.map(|out| out.timed_out).unwrap_or(false),
        "duration_ms": output.map(|out| out.duration_ms),
        "stdout_preview": output.map(|out| out.stdout.as_str()).unwrap_or(""),
        "stderr_preview": output.map(|out| out.stderr.as_str()).unwrap_or(""),
        "proposed_id": proposed_id,
    })
    .to_string()
}

fn record(
    store: &HubStore,
    request: &OsToolRequest,
    operation: &str,
    decision: &str,
    output: Option<&OsToolOutput>,
    proposed_id: Option<&str>,
) -> Result<AuditEvent, String> {
    let root = request
        .workspace
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(request.cwd.as_str());
    store
        .record_audit_event(
            Path::new(root),
            Path::new(&request.cwd),
            operation,
            &process_json(request, decision, output, proposed_id),
            None,
        )
        .map_err(|error| error.to_string())
}

fn outcome(
    status: &str,
    audit: &AuditEvent,
    decision: &str,
    reason: Option<String>,
    output: Option<&OsToolOutput>,
) -> OsToolResult {
    OsToolResult {
        status: status.into(),
        audit_id: audit.id.clone(),
        decision: decision.into(),
        reason,
        exit_code: output.and_then(|out| out.exit_code),
        stdout: output.map(|out| out.stdout.clone()).unwrap_or_default(),
        stderr: output.map(|out| out.stderr.clone()).unwrap_or_default(),
        timed_out: output.map(|out| out.timed_out).unwrap_or(false),
    }
}

/// Evaluate policy, maybe run, always audit.
pub fn submit(
    store: &HubStore,
    request: OsToolRequest,
    strictness: SandboxStrictness,
    timeout: Duration,
) -> Result<OsToolResult, String> {
    match decide(&request, strictness) {
        OsToolDecision::Deny { reason } => {
            let audit = record(store, &request, "os_tool.denied", "deny", None, None)?;
            let _ = store.set_audit_status(&audit.id, "quarantined");
            Ok(outcome("denied", &audit, "deny", Some(reason), None))
        }
        OsToolDecision::NeedsApproval { reason } => {
            let audit = record(
                store,
                &request,
                "os_tool.proposed",
                "needs_approval",
                None,
                None,
            )?;
            Ok(outcome(
                "needs_approval",
                &audit,
                "needs_approval",
                Some(reason),
                None,
            ))
        }
        OsToolDecision::Allow => {
            let output = run(&request, timeout)?;
            let audit = record(store, &request, "os_tool.ran", "allow", Some(&output), None)?;
            let _ = store.set_audit_status(&audit.id, "approved");
            Ok(outcome("ran", &audit, "allow", None, Some(&output)))
        }
    }
}

fn request_from_audit(event: &AuditEvent) -> Result<OsToolRequest, String> {
    let value: serde_json::Value = serde_json::from_str(&event.process_json)
        .map_err(|error| format!("proposed os_tool audit is not JSON: {error}"))?;
    if value.get("kind").and_then(|kind| kind.as_str()) != Some("os_tool") {
        return Err("audit event is not an os_tool proposal".into());
    }
    let cmdline = value
        .get("cmdline")
        .and_then(|value| value.as_array())
        .ok_or("os_tool proposal missing cmdline")?;
    let mut parts = cmdline
        .iter()
        .filter_map(|item| item.as_str().map(str::to_string));
    let program = parts
        .next()
        .ok_or("os_tool proposal has an empty cmdline")?;
    Ok(OsToolRequest {
        program,
        args: parts.collect(),
        cwd: value
            .get("cwd")
            .and_then(|value| value.as_str())
            .unwrap_or(&event.path)
            .to_string(),
        workspace: Some(event.root_path.clone()).filter(|value| !value.is_empty()),
    })
}

/// Run a previously proposed tool after human approval.
pub fn approve(
    store: &HubStore,
    audit_id: &str,
    timeout: Duration,
) -> Result<OsToolResult, String> {
    let event = store
        .get_audit_event(audit_id)
        .map_err(|error| error.to_string())?;
    if event.operation != "os_tool.proposed" {
        return Err("audit event is not a pending os_tool proposal".into());
    }
    if event.status != "pending" {
        return Err(format!("os_tool proposal is already {}", event.status));
    }
    let request = request_from_audit(&event)?;
    if matches!(
        decide(&request, SandboxStrictness::Permissive),
        OsToolDecision::Deny { .. }
    ) {
        let _ = store.set_audit_status(audit_id, "quarantined");
        return Err("os_tool proposal is hard-denied and cannot be approved".into());
    }
    let output = run(&request, timeout)?;
    let ran = record(
        store,
        &request,
        "os_tool.ran",
        "approved",
        Some(&output),
        Some(audit_id),
    )?;
    let _ = store.set_audit_status(audit_id, "approved");
    let _ = store.set_audit_status(&ran.id, "approved");
    Ok(outcome("ran", &ran, "approved", None, Some(&output)))
}
