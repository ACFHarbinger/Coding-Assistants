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
        let content_hash = harness_capture_content_hash(harness, agent_id, session_id, body);
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
        let subject =
            session_id.map(|id| format!("channel:session:{id}:capture:{}", Uuid::new_v4()));
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

    /// Marks `(harness, agent_id, session_id, body)` as already captured
    /// without posting a new message — for content that was already
    /// delivered to the Hub through a different path (e.g. C14.2's explicit
    /// turn-completion reply routing), so the C12 passive on-disk-transcript
    /// poller's next pass recognizes the same text via `record_harness_capture`'s
    /// existing dedup check and does not post a visible duplicate.
    ///
    /// `message_id` should be the id of the message that already carries this
    /// content, for provenance; pass `None` if there isn't one. Uses `INSERT
    /// OR IGNORE` against `harness_captures`' existing UNIQUE(harness,
    /// agent_id, session_id, content_hash) constraint, so calling this after
    /// `record_harness_capture` has already independently captured the same
    /// content is a safe no-op either direction.
    ///
    /// This only prevents the duplicate when both paths agree on
    /// `session_id` — the passive poller keys on whatever Hub session the
    /// desktop UI currently has active, which may differ from the session a
    /// task was actually delivered under if the two aren't the same.
    pub fn mark_harness_capture_seen(
        &self,
        harness: &str,
        agent_id: &str,
        session_id: Option<&str>,
        body: &str,
        message_id: Option<&str>,
    ) -> Result<(), HubError> {
        let content_hash = harness_capture_content_hash(harness, agent_id, session_id, body);
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT OR IGNORE INTO harness_captures(
                id, harness, agent_id, session_id, content_hash, message_id, body, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                id,
                harness,
                agent_id,
                session_id,
                content_hash,
                message_id,
                body,
                now
            ],
        )?;
        Ok(())
    }
}

fn harness_capture_content_hash(
    harness: &str,
    agent_id: &str,
    session_id: Option<&str>,
    body: &str,
) -> String {
    sha256_hex(
        format!(
            "{harness}\0{agent_id}\0{}\0{body}",
            session_id.unwrap_or("")
        )
        .as_bytes(),
    )
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

    #[test]
    fn mark_harness_capture_seen_suppresses_a_later_passive_capture_of_the_same_text() {
        // Regression: C14.2's explicit turn-completion reply routing posts
        // Codex's reply through record_codex_reply — a different insert
        // path than record_harness_capture. Since send_codex_turn's
        // thread/resume runs against the same on-disk thread the passive
        // C12 poller (capture_codex_session) also scans, that poller would
        // otherwise rediscover the identical text and post a visible
        // duplicate. mark_harness_capture_seen must make the poller's own
        // dedup check treat it as already-seen.
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let session = store.create_work_session("dedup regression").unwrap();

        // Simulates record_codex_reply already having posted the reply
        // through send_message/send_session_message, then C14.2 marking it
        // seen for the passive poller's benefit.
        store
            .mark_harness_capture_seen(
                "codex",
                "chat",
                Some(&session.id),
                "already delivered via C14.2",
                Some("reply-message-id"),
            )
            .unwrap();

        let captured = store
            .record_harness_capture(
                "codex",
                "chat",
                Some(&session.id),
                "already delivered via C14.2",
                None,
            )
            .unwrap();
        assert!(
            captured.is_none(),
            "passive capture of already-marked-seen content must be a no-op"
        );
    }

    #[test]
    fn mark_harness_capture_seen_is_idempotent_with_a_real_prior_capture() {
        // Calling mark_harness_capture_seen after record_harness_capture
        // already captured the same content (the reverse ordering) must
        // not error or create a second harness_captures row — the UNIQUE
        // constraint plus INSERT OR IGNORE covers either direction.
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let first = store
            .record_harness_capture("codex", "chat", None, "hello from codex", None)
            .unwrap();
        assert!(first.is_some());

        store
            .mark_harness_capture_seen("codex", "chat", None, "hello from codex", None)
            .unwrap();

        let second = store
            .record_harness_capture("codex", "chat", None, "hello from codex", None)
            .unwrap();
        assert!(second.is_none(), "content already captured stays deduped");
    }
}
