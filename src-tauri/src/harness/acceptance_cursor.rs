//! C14.12 Cursor harness acceptance tests (#275).

#[cfg(test)]
mod tests {
    use hub::{cursor_spawn_args, inject_harness_with_store, HarnessInjectRequest, HubStore};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn c14_cursor_capture_and_managed_only_delivery_acceptance_row() {
        const HUB_SESSION_ID: &str = "c14-cursor-hub-session";
        const CHAT_ID: &str = "cursor-chat-id";

        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let root = tempdir().unwrap();
        let workspace = PathBuf::from("/tmp/c14-acceptance-cursor");
        let transcript_dir = root
            .path()
            .join(crate::harness::cursor::encode_workspace_dir_name(
                &workspace,
            ))
            .join("agent-transcripts")
            .join(CHAT_ID);
        std::fs::create_dir_all(&transcript_dir).unwrap();
        let mut transcript =
            std::fs::File::create(transcript_dir.join(format!("{CHAT_ID}.jsonl"))).unwrap();
        writeln!(
            transcript,
            r#"{{"type":"thinking","text":"private reasoning"}}"#
        )
        .unwrap();
        writeln!(
            transcript,
            r#"{{"role":"assistant","message":{{"content":[{{"type":"text","text":"[cursor] C14.12 acceptance fixture"}}]}}}}"#
        )
        .unwrap();
        store
            .register_harness_session("cursor", &workspace.to_string_lossy(), CHAT_ID, None)
            .unwrap();

        let outcome = crate::harness::cursor::capture_cursor_session_from(
            root.path(),
            &store,
            &workspace,
            Some(CHAT_ID),
            Some(HUB_SESSION_ID),
        )
        .unwrap();
        assert!(outcome.transcript_found);
        assert_eq!(outcome.scanned, 1);
        assert_eq!(outcome.captured.len(), 1);
        assert_eq!(outcome.captured[0].from_agent, "cursor");

        let dangerous = "; rm -rf / && echo pwned $(whoami) `id`";
        let args = cursor_spawn_args(&workspace, dangerous, None, None).unwrap();
        assert_eq!(args.iter().filter(|arg| *arg == dangerous).count(), 1);

        let delivery = inject_harness_with_store(
            &store,
            &HarnessInjectRequest {
                harness: "cursor".into(),
                workspace,
                session_id: Some(HUB_SESSION_ID.into()),
                message_id: Some("msg".into()),
                body: "do not spawn a replacement".into(),
                is_task: true,
                is_wake: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(delivery.pid, None);
        assert_eq!(delivery.status, "unavailable");
        assert!(!delivery.detail.to_ascii_lowercase().contains("spawned"));
    }

    #[test]
    fn task_only_inject_never_spawns_and_reports_truthful_outcomes() {
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = PathBuf::from("/tmp/c12-no-spawn");
        for harness in ["grok", "chat", "claude", "gemini", "cursor"] {
            let result = inject_harness_with_store(
                &store,
                &HarnessInjectRequest {
                    harness: harness.into(),
                    workspace: workspace.clone(),
                    session_id: Some("session".into()),
                    message_id: Some("msg".into()),
                    body: "do not spawn a replacement".into(),
                    is_task: true,
                    is_wake: false,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(result.pid, None, "{harness}: {result:?}");
            assert!(
                result.status == "unavailable" || result.status == "queued",
                "{harness}: {result:?}"
            );
            assert!(!result.detail.to_ascii_lowercase().contains("spawned"));
        }
    }
}
