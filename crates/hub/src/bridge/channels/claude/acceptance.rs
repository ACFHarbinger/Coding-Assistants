//! C14.3 automated acceptance test — Claude Channel bridge end-to-end round trip.
//!
//! Exercises the complete C14.3 contract in one isolated flow against a real
//! `HubStore` backed by a temporary directory. No MCP server or Claude Code
//! process is spawned; every function is called through the same public API
//! the bridge binary uses. The test is a compile-time (and eventually
//! runtime-verified) acceptance gate matching the manually verified
//! `--channels` session ping round-trip documented in issue #150.

#[cfg(test)]
mod tests {
    use crate::{
        bridge::channels::claude::{
            get_permission_request, poll_channel_events, poll_quiet_channel_events,
            record_channel_reply, record_permission_request, resolve_permission_request,
            PermissionVerdict,
        },
        HubStore, MessageKind,
    };
    use tempfile::tempdir;

    /// Full C14.3 round-trip:
    ///
    /// 1. Enroll an agent as a team member (authenticated-sender gate prerequisite).
    /// 2. Send a task-tagged message to `claude` from the enrolled sender.
    /// 3. Confirm it surfaces via `poll_channel_events` (disturbing path).
    /// 4. Send a plain (non-task, non-wake) message and prove the disturb/quiet split:
    ///    it must be invisible to `poll_channel_events` but visible to
    ///    `poll_quiet_channel_events`.
    /// 5. Call `record_channel_reply` simulating Claude's reply to the task message
    ///    and assert the reply is addressed back to the original enrolled sender.
    /// 6. Exercise the permission-relay lifecycle: record a request, assert it starts
    ///    `Pending`, resolve it explicitly, and verify nothing auto-approved it
    ///    before that call.
    #[test]
    fn c14_3_channel_bridge_full_round_trip() {
        // ── Setup ────────────────────────────────────────────────────────────

        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // Step 1: enroll the sending agent so it passes the authenticated-sender
        // gate checked inside `poll_channel_events` / `poll_quiet_channel_events`.
        // `set_team_member(..., true)` is the same call the unit tests in
        // events.rs use to establish enrollment.
        store.upsert_agent("orchestrator", "Orchestrator").unwrap();
        store.set_team_member("orchestrator", true).unwrap();
        assert!(
            store.is_team_member("orchestrator").unwrap(),
            "agent must appear as enrolled before the authenticated-sender gate will admit its messages"
        );

        // ── Step 2: task-tagged message (disturbing) ──────────────────────────

        // A task-tagged `MessageKind::Message` is "disturbing" (see
        // `is_disturbing` in events.rs: task_id.is_some() || kind == Wake),
        // so `poll_channel_events` must surface it.
        let task_msg = store
            .send_message(
                "orchestrator",
                "claude",
                MessageKind::Message,
                "please implement the feature described in task-42",
                Some("channel:session:sess-001:inbox"),
                None,
                Some("task-42"),
            )
            .unwrap();

        // ── Step 3: poll_channel_events returns the task-tagged message ────────

        let events = poll_channel_events(&store).unwrap();
        assert_eq!(
            events.len(),
            1,
            "exactly one task-tagged message should be returned by the disturbing poll"
        );
        let event = &events[0];
        assert_eq!(event.from_agent, "orchestrator");
        assert_eq!(event.task_id.as_deref(), Some("task-42"));
        assert_eq!(
            event.session_id.as_deref(),
            Some("sess-001"),
            "session id must be extracted from the channel:session:<id>:... subject"
        );
        assert_eq!(event.message_id, task_msg.id);

        // Draining via poll_channel_events acks the message; a second call
        // must find nothing (no replay after restart).
        assert!(
            poll_channel_events(&store).unwrap().is_empty(),
            "acked task message must not be replayed on a second poll"
        );

        // ── Step 4: plain (non-task, non-wake) message — disturb/quiet split ──

        // A plain `MessageKind::Message` with no task_id is *not* disturbing.
        // It must stay pending for `poll_quiet_channel_events` (the `check_inbox`
        // tool) and must never surface via `poll_channel_events`.
        store
            .send_message(
                "orchestrator",
                "claude",
                MessageKind::Message,
                "just a status update, no rush",
                None,
                None,
                None,
            )
            .unwrap();

        assert!(
            poll_channel_events(&store).unwrap().is_empty(),
            "a plain non-task message must NOT appear via the disturbing poll path"
        );

        let quiet_events = poll_quiet_channel_events(&store).unwrap();
        assert_eq!(
            quiet_events.len(),
            1,
            "the plain message must appear via the quiet poll path"
        );
        assert_eq!(quiet_events[0].body, "just a status update, no rush");
        assert_eq!(quiet_events[0].from_agent, "orchestrator");

        // Once drained by the quiet poll, a repeat call returns nothing.
        assert!(
            poll_quiet_channel_events(&store).unwrap().is_empty(),
            "acked quiet message must not be replayed on a second quiet poll"
        );

        // ── Step 5: Claude replies to the task-tagged message ─────────────────

        // `record_channel_reply` routes the reply to the *original sender* of
        // `in_reply_to` (see reply.rs: it reads `message.from_agent` from the
        // store). We use `task_msg.id` as the reply target.
        let reply = record_channel_reply(
            &store,
            Some(&task_msg.id),
            None, // no session scope — simple direct reply
            "Done — feature implemented and tests green.",
        )
        .unwrap();

        // The reply must come *from* claude and must be addressed *to* the
        // original enrolled sender, not to the literal string "human"
        // (which is only the fallback when in_reply_to is None or unknown).
        assert_eq!(
            reply.from_agent, "claude",
            "reply must be authored by the claude agent"
        );
        assert_eq!(
            reply.to_agent, "orchestrator",
            "reply must be routed back to the original sender, not the human fallback"
        );

        // ── Step 6: permission-relay lifecycle — never auto-approved ──────────

        // Record a permission request arriving from Claude Code (simulates the
        // `notifications/claude/channel/permission_request` MCP notification).
        record_permission_request(
            &store,
            "perm-req-001",
            "Bash",
            "run the integration test suite",
            "cargo test --workspace",
        )
        .unwrap();

        // The request must start as Pending — no code path auto-approves it.
        // This directly validates C14.3's "permission relay only after explicit
        // human approval" acceptance criterion.
        let verdict_before = get_permission_request(&store, "perm-req-001").unwrap();
        assert_eq!(
            verdict_before,
            Some(PermissionVerdict::Pending),
            "a freshly recorded permission request must start Pending, never auto-approved"
        );

        // Simulate unrelated store activity to confirm it cannot change the
        // verdict (matching the invariant unit-tested in permissions.rs).
        store
            .send_message(
                "orchestrator",
                "claude",
                MessageKind::System,
                "heartbeat",
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            get_permission_request(&store, "perm-req-001").unwrap(),
            Some(PermissionVerdict::Pending),
            "unrelated store activity must not auto-resolve a pending permission request"
        );

        // Only an explicit human resolve call must move the verdict.
        let resolved = resolve_permission_request(&store, "perm-req-001", true).unwrap();
        assert_eq!(resolved, PermissionVerdict::Allowed);
        assert_eq!(
            get_permission_request(&store, "perm-req-001").unwrap(),
            Some(PermissionVerdict::Allowed),
            "after explicit approval the verdict must be Allowed"
        );

        // Resolving a non-existent request must fail (no silent no-ops).
        assert!(
            resolve_permission_request(&store, "does-not-exist", true).is_err(),
            "resolving an unknown permission request id must return an error"
        );
    }
}
