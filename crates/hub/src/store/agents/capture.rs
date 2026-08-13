use super::super::*;
impl HubStore {
    pub fn export_markdown_git(
        &self,
        out_dir: Option<&Path>,
        message: Option<&str>,
    ) -> Result<GitExportOutcome, HubError> {
        let path = self.export_markdown(out_dir)?;
        let dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.data_dir.join("markdown"));

        let in_work_tree = Command::new("git")
            .args([
                "-C",
                &dir.to_string_lossy(),
                "rev-parse",
                "--is-inside-work-tree",
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !in_work_tree {
            return Ok(GitExportOutcome {
                path,
                committed: false,
                detail: "not inside a Git work tree; commit skipped".into(),
            });
        }

        let add = Command::new("git")
            .args(["-C", &dir.to_string_lossy(), "add", "--"])
            .arg(&path)
            .output()?;
        if !add.status.success() {
            return Ok(GitExportOutcome {
                path,
                committed: false,
                detail: format!("git add failed: {}", String::from_utf8_lossy(&add.stderr)),
            });
        }

        let msg = message
            .map(str::to_string)
            .unwrap_or_else(|| "chore(hub): update shared memory export".to_string());
        let commit = Command::new("git")
            .args(["-C", &dir.to_string_lossy(), "commit", "-m", &msg, "--"])
            .arg(&path)
            .output()?;
        if commit.status.success() {
            Ok(GitExportOutcome {
                path,
                committed: true,
                detail: "committed".into(),
            })
        } else {
            // Commonly "nothing to commit" when the export is unchanged.
            Ok(GitExportOutcome {
                path,
                committed: false,
                detail: String::from_utf8_lossy(&commit.stderr).trim().to_string(),
            })
        }
    }

    /// Record a harness-authored message into the session transcript.
    /// Duplicate polls of the same (harness, agent, session, body) are no-ops.
    pub fn record_harness_capture(
        &self,
        harness: &str,
        agent_id: &str,
        session_id: Option<&str>,
        body: &str,
        workspace_path: Option<&str>,
    ) -> Result<Option<MessageRecord>, HubError> {
        if harness.trim().is_empty() || agent_id.trim().is_empty() {
            return Err(HubError::Invalid(
                "harness capture requires harness and agent_id".into(),
            ));
        }
        if body.trim().is_empty() {
            return Err(HubError::Invalid(
                "harness capture body must not be empty".into(),
            ));
        }
        let content_hash = sha256_hex(
            format!(
                "{harness}\0{agent_id}\0{}\0{body}",
                session_id.unwrap_or("")
            )
            .as_bytes(),
        );
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM harness_captures
                 WHERE harness = ?1 AND agent_id = ?2
                   AND IFNULL(session_id, '') = IFNULL(?3, '')
                   AND content_hash = ?4",
                params![harness, agent_id, session_id, content_hash],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Ok(None);
        }

        // Each capture needs its own unique subject — a fixed
        // "channel:session:<id>:capture" for every captured chunk, from
        // every harness/agent, collided with itself in the desktop chat's
        // per-post dedup (meant to collapse team fan-out *copies* of one
        // broadcast, not distinct sends): only the most recently captured
        // chunk ever displayed, so one agent's capture appeared to
        // overwrite another's. Mirrors the same fix already applied to
        // `record_channel_reply`'s subject.
        let subject = session_id.map(|id| format!("channel:session:{id}:capture:{}", Uuid::new_v4()));
        let to_agent = session_id
            .map(|id| format!("session:{id}"))
            .unwrap_or_else(|| "team".into());
        let message = self.send_message(
            agent_id,
            &to_agent,
            MessageKind::Message,
            body,
            subject.as_deref(),
            workspace_path,
            None,
        )?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO harness_captures(
                id, harness, agent_id, session_id, content_hash, message_id, body, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                id,
                harness,
                agent_id,
                session_id,
                content_hash,
                message.id,
                body,
                now
            ],
        )?;
        Ok(Some(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn distinct_captures_in_the_same_session_get_distinct_subjects() {
        // Regression: a fixed "channel:session:<id>:capture" subject for
        // every captured chunk, from every harness/agent, collided with
        // itself in the desktop chat's per-post dedup — one agent's
        // capture appeared to overwrite another's as soon as a second
        // capture (from any harness) landed in the same session.
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let session = store.create_work_session("capture regression").unwrap();

        let grok_capture = store
            .record_harness_capture("grok", "grok", Some(&session.id), "grok's update", None)
            .unwrap()
            .expect("distinct body is captured");
        let claude_capture = store
            .record_harness_capture(
                "claude",
                "claude",
                Some(&session.id),
                "claude's update",
                None,
            )
            .unwrap()
            .expect("distinct body from a distinct agent is captured");

        assert_ne!(grok_capture.subject, claude_capture.subject);

        let listed = store
            .list_channel_messages(&format!("session:{}", session.id), 20)
            .unwrap();
        assert_eq!(listed.len(), 2, "both captures must remain visible");
        assert!(listed.iter().any(|m| m.from_agent == "grok"));
        assert!(listed.iter().any(|m| m.from_agent == "claude"));
    }
}
