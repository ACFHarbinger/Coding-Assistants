//! Read-only C13 owner-run preflight inspector (`ca preflight`).
//!
//! Never creates Hub/settings files and never writes `.agent/**`.

use crate::helpers::audit_file_hash;
use anyhow::{anyhow, Result};
use hub::{HarnessSessionRegistration, HubStore};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
pub(crate) struct PreflightReport {
    pub generated_at: String,
    pub hub_home: String,
    pub hub_present: bool,
    pub workspace: Option<String>,
    pub session: Option<PreflightSession>,
    pub team: Vec<String>,
    pub harness_sessions: Vec<PreflightHarness>,
    pub fallback_hashes: Vec<FallbackHash>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PreflightSession {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PreflightHarness {
    pub harness: String,
    pub workspace: String,
    pub disk_session_id: String,
    pub mode: String,
    pub state: String,
    pub leader_socket: Option<String>,
    pub socket_present: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct FallbackHash {
    pub path: String,
    pub sha256: Option<String>,
}

pub(crate) fn inspect(
    hub_home: &Path,
    workspace: Option<&Path>,
    session_id: Option<&str>,
) -> Result<PreflightReport> {
    if let Some(workspace) = workspace {
        if !workspace.is_absolute() {
            anyhow::bail!("--workspace must be an absolute path");
        }
    }

    let mut notes = Vec::new();
    let store = match HubStore::open_existing_read_only(hub_home) {
        Ok(store) => Some(store),
        Err(error) => {
            notes.push(format!("hub not opened: {error}"));
            None
        }
    };

    let mut session = None;
    let mut team = Vec::new();
    let mut harness_sessions = Vec::new();
    if let Some(store) = store.as_ref() {
        team = store
            .list_team_members()?
            .into_iter()
            .map(|agent| agent.id)
            .collect();
        if let Some(session_id) = session_id {
            let found = store
                .list_work_sessions()?
                .into_iter()
                .find(|row| row.id == session_id);
            let Some(found) = found else {
                anyhow::bail!("work session {session_id} was not found");
            };
            session = Some(PreflightSession {
                id: found.id,
                name: found.name,
                members: found.member_ids,
            });
        }
        for row in store.list_harness_sessions()? {
            if let Some(filter) = workspace {
                if Path::new(&row.workspace) != filter {
                    continue;
                }
            }
            harness_sessions.push(summarize_harness(row));
        }
    } else if session_id.is_some() {
        anyhow::bail!("--session requires an existing Hub database");
    }

    let hash_root = workspace.unwrap_or(Path::new("."));
    Ok(PreflightReport {
        generated_at: utc_now(),
        hub_home: hub_home.display().to_string(),
        hub_present: store.is_some(),
        workspace: workspace.map(|path| path.display().to_string()),
        session,
        team,
        harness_sessions,
        fallback_hashes: fallback_hashes(hash_root),
        notes,
    })
}

pub(crate) fn render_markdown(report: &PreflightReport) -> String {
    let mut out = String::from("## C13 preflight (ca preflight)\n\n");
    out.push_str(&format!("- **Date (UTC):** {}\n", report.generated_at));
    out.push_str(&format!("- **Hub home:** `{}`\n", report.hub_home));
    out.push_str(&format!("- **Hub present:** {}\n", report.hub_present));
    out.push_str(&format!(
        "- **Workspace root (absolute):** {}\n",
        report
            .workspace
            .as_deref()
            .unwrap_or("(not provided; hashed cwd)")
    ));
    match &report.session {
        Some(session) => {
            out.push_str(&format!(
                "- **Named session id / title:** `{}` / {}\n",
                session.id, session.name
            ));
            out.push_str(&format!(
                "- **Session members:** {}\n",
                session.members.join(", ")
            ));
        }
        None => out.push_str("- **Named session id / title:** (not requested)\n"),
    }
    out.push_str(&format!(
        "- **Enrolled team:** {}\n",
        if report.team.is_empty() {
            "(none / hub missing)".into()
        } else {
            report.team.join(", ")
        }
    ));
    out.push_str("\n### A. Preflight hashes\n\n**Before**\n\n");
    if report.fallback_hashes.is_empty() {
        out.push_str("    (no .agent fallback files found)\n");
    } else {
        for hash in &report.fallback_hashes {
            match &hash.sha256 {
                Some(digest) => out.push_str(&format!("    {digest}  {}\n", hash.path)),
                None => out.push_str(&format!("    MISSING  {}\n", hash.path)),
            }
        }
    }
    out.push_str("\n### Registered harness sessions\n\n");
    if report.harness_sessions.is_empty() {
        out.push_str("(none)\n");
    } else {
        for row in &report.harness_sessions {
            out.push_str(&format!(
                "- `{}` workspace=`{}` thread=`{}` mode={} state={}",
                row.harness, row.workspace, row.disk_session_id, row.mode, row.state
            ));
            if let Some(socket) = &row.leader_socket {
                out.push_str(&format!(
                    " socket=`{}` present={}",
                    socket,
                    row.socket_present.unwrap_or(false)
                ));
            }
            out.push('\n');
        }
    }
    if !report.notes.is_empty() {
        out.push_str("\n### Notes\n\n");
        for note in &report.notes {
            out.push_str(&format!("- {note}\n"));
        }
    }
    out.push_str(
        "\nThis inspector does **not** pass C13. Paste into the #113 evidence template after the live run.\n",
    );
    out
}

fn summarize_harness(row: HarnessSessionRegistration) -> PreflightHarness {
    let socket_present = row
        .leader_socket
        .as_ref()
        .map(|path| Path::new(path).exists());
    PreflightHarness {
        harness: row.harness,
        workspace: row.workspace,
        disk_session_id: row.disk_session_id,
        mode: row.mode.as_str().to_string(),
        state: row.state.as_str().to_string(),
        leader_socket: row.leader_socket,
        socket_present,
    }
}

fn fallback_hashes(root: &Path) -> Vec<FallbackHash> {
    let mut hashes = Vec::new();
    let bus = root.join(".agent/cache/AGENT_BUS.md");
    hashes.push(hash_entry(&bus));
    let messages = root.join(".agent/messages");
    if messages.is_dir() {
        if let Ok(mut files) = collect_files(&messages) {
            files.sort();
            for path in files {
                hashes.push(hash_entry(&path));
            }
        }
    }
    hashes
}

fn hash_entry(path: &Path) -> FallbackHash {
    FallbackHash {
        path: path.display().to_string(),
        sha256: if path.is_file() {
            audit_file_hash(path)
        } else {
            None
        },
    }
}

fn collect_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path)?);
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    chrono::DateTime::from_timestamp(secs as i64, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into())
}

pub(crate) fn run(
    hub_home: PathBuf,
    workspace: Option<PathBuf>,
    session: Option<String>,
    json: bool,
) -> Result<()> {
    let report = inspect(&hub_home, workspace.as_deref(), session.as_deref())?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render_markdown(&report));
    }
    if !report.hub_present && session.is_some() {
        return Err(anyhow!("--session requires an existing Hub database"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_hub_does_not_create_files() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("hub-home");
        let report = inspect(&home, None, None).unwrap();
        assert!(!report.hub_present);
        assert!(!home.join("hub.db").exists());
        assert!(
            !home.exists()
                || home
                    .read_dir()
                    .map(|mut i| i.next().is_none())
                    .unwrap_or(true)
        );
    }

    #[test]
    fn relative_workspace_is_rejected() {
        let dir = tempdir().unwrap();
        let err = inspect(dir.path(), Some(Path::new("relative")), None).unwrap_err();
        assert!(err.to_string().contains("absolute"), "{err}");
    }

    #[test]
    fn existing_hub_is_read_without_new_sidecars() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        drop(store);
        let db = dir.path().join("hub.db");
        let before = audit_file_hash(&db);
        let report = inspect(dir.path(), Some(dir.path()), None).unwrap();
        assert!(report.hub_present);
        assert!(report.team.iter().any(|id| id == "grok" || id == "human"));
        assert_eq!(audit_file_hash(&db), before);
    }

    #[test]
    fn unknown_session_is_an_error() {
        let dir = tempdir().unwrap();
        let _store = HubStore::open(dir.path()).unwrap();
        let err = inspect(dir.path(), None, Some("no-such-session")).unwrap_err();
        assert!(err.to_string().contains("no-such-session"), "{err}");
    }

    #[test]
    fn hashes_agent_fallback_files_when_present() {
        let dir = tempdir().unwrap();
        let bus = dir.path().join(".agent/cache/AGENT_BUS.md");
        fs::create_dir_all(bus.parent().unwrap()).unwrap();
        fs::write(&bus, "coordination snapshot\n").unwrap();
        let report = inspect(dir.path(), Some(dir.path()), None).unwrap();
        assert!(report
            .fallback_hashes
            .iter()
            .any(|row| row.path.ends_with("AGENT_BUS.md") && row.sha256.is_some()));
        assert_eq!(fs::read_to_string(&bus).unwrap(), "coordination snapshot\n");
    }
}
