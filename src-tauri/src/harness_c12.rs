//! C12 four-harness acceptance test.
//!
//! Exercises all four capture adapters (Grok, Codex, Claude, Gemini) against
//! fixture transcripts in the same shapes their real on-disk logs use, and
//! confirms every capture lands in one shared hub session channel. Also
//! confirms the `inject_harness` contract returns a structured `Result`
//! (never panics) and that every harness's argv builder keeps a prompt
//! containing shell metacharacters as a single, unsplit argument — proof the
//! delivery path never concatenates one into a shell string. No live
//! process is spawned anywhere in this module; every check runs against
//! pure/deterministic paths (fixture files, or `inject_harness`'s
//! synchronous validation, which runs before any spawn attempt).

#[cfg(test)]
mod tests {
    use ca_hub::{
        claude_spawn_args, codex_spawn_args, gemini_spawn_args, grok_spawn_args, inject_harness,
        HarnessInjectRequest, HubStore,
    };
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    const HUB_SESSION_ID: &str = "c12-acceptance-hub-session";

    fn write_grok_fixture(root: &Path, workspace: &Path, disk_session_id: &str) {
        let dir = root
            .join(crate::harness_grok::encode_workspace_dir_name(workspace))
            .join(disk_session_id);
        fs::create_dir_all(&dir).unwrap();
        let mut file = fs::File::create(dir.join("chat_history.jsonl")).unwrap();
        writeln!(file, r#"{{"type":"user","content":"status?"}}"#).unwrap();
        writeln!(file, r#"{{"type":"reasoning","content":"thinking"}}"#).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","content":"[grok] C12 acceptance fixture"}}"#
        )
        .unwrap();
    }

    fn write_codex_fixture(root: &Path, workspace: &Path, disk_session_id: &str) {
        let dir = root.join("2026").join("08").join("13");
        fs::create_dir_all(&dir).unwrap();
        let mut file = fs::File::create(dir.join("rollout.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"type":"session_meta","payload":{{"cwd":"{}","session_id":"{}"}}}}"#,
            workspace.display(),
            disk_session_id
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"assistant","content":[{{"type":"output_text","text":"[codex] C12 acceptance fixture"}}]}}}}"#
        )
        .unwrap();
    }

    fn write_claude_fixture(root: &Path, workspace: &Path, disk_session_id: &str) {
        let dir = root.join(crate::harness_claude::encode_workspace_dir_name(workspace));
        fs::create_dir_all(&dir).unwrap();
        let mut file = fs::File::create(dir.join(format!("{disk_session_id}.jsonl"))).unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"thinking","thinking":"private"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"[claude] C12 acceptance fixture"}}]}}}}"#
        )
        .unwrap();
    }

    fn write_gemini_fixture(root: &Path, disk_session_id: &str) {
        let dir = root
            .join(disk_session_id)
            .join(".system_generated")
            .join("logs");
        fs::create_dir_all(&dir).unwrap();
        let mut file = fs::File::create(dir.join("transcript.jsonl")).unwrap();
        writeln!(
            file,
            r#"{{"source":"MODEL","type":"PLANNER_RESPONSE","content":"[gemini] C12 acceptance fixture"}}"#
        )
        .unwrap();
    }

    #[test]
    fn c12_all_four_harness_captures_land_on_the_same_hub_session() {
        let store_dir = tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();

        let grok_root = tempdir().unwrap();
        let grok_workspace = PathBuf::from("/tmp/c12-acceptance-grok");
        write_grok_fixture(grok_root.path(), &grok_workspace, "grok-disk-session");
        let grok_outcome = crate::harness_grok::capture_grok_session_from(
            grok_root.path(),
            &store,
            &grok_workspace,
            Some("grok-disk-session"),
            Some(HUB_SESSION_ID),
        )
        .unwrap();

        let codex_root = tempdir().unwrap();
        let codex_workspace = PathBuf::from("/tmp/c12-acceptance-codex");
        write_codex_fixture(codex_root.path(), &codex_workspace, "codex-disk-session");
        let codex_outcome = crate::harness_codex::capture_codex_session_from(
            codex_root.path(),
            &store,
            &codex_workspace,
            Some("codex-disk-session"),
            Some(HUB_SESSION_ID),
        )
        .unwrap();

        let claude_root = tempdir().unwrap();
        let claude_workspace = PathBuf::from("/tmp/c12-acceptance-claude");
        write_claude_fixture(claude_root.path(), &claude_workspace, "claude-disk-session");
        let claude_outcome = crate::harness_claude::capture_claude_session_from(
            claude_root.path(),
            &store,
            &claude_workspace,
            Some("claude-disk-session"),
            Some(HUB_SESSION_ID),
        )
        .unwrap();

        let gemini_root = tempdir().unwrap();
        let gemini_workspace = PathBuf::from("/tmp/c12-acceptance-gemini");
        write_gemini_fixture(gemini_root.path(), "gemini-disk-session");
        let gemini_outcome = crate::harness_gemini::capture_gemini_session_from(
            gemini_root.path(),
            &store,
            &gemini_workspace,
            Some("gemini-disk-session"),
            Some(HUB_SESSION_ID),
        )
        .unwrap();

        assert!(grok_outcome.transcript_found);
        assert_eq!(grok_outcome.captured.len(), 1);
        assert!(codex_outcome.transcript_found);
        assert_eq!(codex_outcome.captured.len(), 1);
        assert!(claude_outcome.transcript_found);
        assert_eq!(claude_outcome.captured.len(), 1);
        assert!(gemini_outcome.transcript_found);
        assert_eq!(gemini_outcome.captured.len(), 1);

        // All four adapters wrote into the same hub session channel.
        let recorded = store
            .list_channel_messages(&format!("session:{HUB_SESSION_ID}"), 50)
            .unwrap();
        assert_eq!(
            recorded.len(),
            4,
            "all four captures must share one session"
        );
        let bodies: Vec<&str> = recorded.iter().map(|m| m.body.as_str()).collect();
        assert!(bodies.iter().any(|b| b.contains("[grok]")));
        assert!(bodies.iter().any(|b| b.contains("[codex]")));
        assert!(bodies.iter().any(|b| b.contains("[claude]")));
        assert!(bodies.iter().any(|b| b.contains("[gemini]")));

        // Each capture is attributed to the correct authoring agent, not a
        // shared/generic identity.
        let from_agents: Vec<&str> = recorded.iter().map(|m| m.from_agent.as_str()).collect();
        assert!(from_agents.contains(&"grok"));
        assert!(from_agents.contains(&"chat")); // Codex captures post as "chat".
        assert!(from_agents.contains(&"claude"));
        assert!(from_agents.contains(&"gemini"));
    }

    /// `inject_harness` runs its validation (empty body, relative workspace)
    /// synchronously, before any process spawn is attempted — so these paths
    /// exercise its `Result` contract deterministically and safely, with no
    /// real harness process ever launched.
    #[test]
    fn inject_harness_returns_a_structured_result_and_never_panics_on_bad_input() {
        let workspace = PathBuf::from("/tmp/c12-acceptance-inject");
        let empty_body = inject_harness(&HarnessInjectRequest {
            harness: "claude".into(),
            workspace: workspace.clone(),
            session_id: Some(HUB_SESSION_ID.into()),
            message_id: None,
            body: "   ".into(),
            is_task: true,
            is_wake: false,
        });
        assert!(empty_body.is_err(), "an empty body must be rejected");

        let relative_workspace = inject_harness(&HarnessInjectRequest {
            harness: "claude".into(),
            workspace: PathBuf::from("relative/workspace"),
            session_id: Some(HUB_SESSION_ID.into()),
            message_id: None,
            body: "do the thing".into(),
            is_task: true,
            is_wake: false,
        });
        assert!(
            relative_workspace.is_err(),
            "a relative workspace must be rejected before any spawn is attempted"
        );

        let unknown_harness = inject_harness(&HarnessInjectRequest {
            harness: "not-a-real-harness".into(),
            workspace,
            session_id: None,
            message_id: None,
            body: "do the thing".into(),
            is_task: false,
            is_wake: true,
        });
        assert!(
            unknown_harness.is_err(),
            "an unknown harness id must be rejected"
        );
    }

    /// Every harness's argv builder must keep a prompt with shell
    /// metacharacters as one literal argument, never split or interpreted —
    /// proof the delivery path passes explicit argv to `Command`, not a
    /// concatenated shell string.
    #[test]
    fn all_four_spawn_arg_builders_never_split_or_interpret_shell_metacharacters() {
        let workspace = PathBuf::from("/tmp/c12-acceptance-shell-safety");
        let dangerous = "; rm -rf / && echo pwned $(whoami) `id` | cat > /tmp/evil";

        let grok_args = grok_spawn_args(&workspace, dangerous).unwrap();
        assert!(grok_args.iter().any(|arg| arg == dangerous));

        let codex_args = codex_spawn_args(&workspace, dangerous).unwrap();
        assert!(codex_args.iter().any(|arg| arg == dangerous));

        let claude_args = claude_spawn_args(&workspace, dangerous).unwrap();
        assert!(claude_args.iter().any(|arg| arg == dangerous));

        let gemini_args = gemini_spawn_args(&workspace, dangerous).unwrap();
        assert!(gemini_args.iter().any(|arg| arg == dangerous));

        // None of the builders may have split the dangerous string into
        // multiple argv entries at its embedded whitespace/operators — the
        // prompt must appear as exactly one element, verbatim.
        for args in [&grok_args, &codex_args, &claude_args, &gemini_args] {
            let matches = args.iter().filter(|arg| *arg == dangerous).count();
            assert_eq!(matches, 1);
        }
    }
}
