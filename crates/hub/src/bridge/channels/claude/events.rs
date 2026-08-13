//! The disturb/quiet poll split: only wake or task-tagged messages are
//! worth interrupting a live session for; everything else stays `pending`
//! for [`poll_quiet_channel_events`] (the `check_inbox` tool) to drain on
//! Claude's own initiative.

use super::CLAUDE_AGENT_ID;
use crate::{HubError, HubStore, MessageKind, MessageRecord};

/// One authenticated Hub event ready to push into a Channel-connected
/// Claude Code session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelEvent {
    pub message_id: String,
    pub from_agent: String,
    pub session_id: Option<String>,
    pub kind: String,
    pub task_id: Option<String>,
    pub body: String,
}

/// Whether a message addressed to `claude` is important enough to interrupt
/// a live session: an explicit wake, or a task-tagged send. Everything else
/// (plain chat, handoff, system) is durably queued for Claude to check on
/// its own terms via the `check_inbox` tool, rather than pushed.
fn is_disturbing(kind: &str, task_id: &Option<String>) -> bool {
    kind == MessageKind::Wake.as_str() || task_id.is_some()
}

fn enrolled_senders(store: &HubStore) -> Result<std::collections::HashSet<String>, HubError> {
    Ok(store
        .list_team_members()?
        .into_iter()
        .map(|agent| agent.id)
        .collect())
}

fn to_channel_event(message: MessageRecord) -> ChannelEvent {
    ChannelEvent {
        session_id: session_id_from_subject(message.subject.as_deref()),
        message_id: message.id,
        from_agent: message.from_agent,
        kind: message.kind,
        task_id: message.task_id,
        body: message.body,
    }
}

/// Extracts a session id from the `channel:session:<id>:...` subject
/// convention already used elsewhere in the Hub (see
/// `store/agents/capture.rs`, `store/messages/mod.rs`). `pub(crate)` so
/// `reply`'s regression test can assert a reply's subject round-trips back
/// to the session it was sent in.
pub(crate) fn session_id_from_subject(subject: Option<&str>) -> Option<String> {
    subject
        .and_then(|subject| subject.strip_prefix("channel:session:"))
        .and_then(|rest| rest.split(':').next())
        .map(str::to_string)
}

/// Authenticated sender gate + drain of the messages worth interrupting a
/// live session for: an explicit wake, or a task-tagged send, from an
/// enrolled team member. Everything else addressed to `claude` is left
/// `pending` — still visible in the Shared Hub chat, and retrievable on
/// Claude's own initiative via [`poll_quiet_channel_events`] — rather than
/// pushed and force-acked. Marks each *returned* message acked (same
/// semantics as `ca inbox watch` / `hub_poll_messages`) so a restarted
/// bridge does not replay history.
pub fn poll_channel_events(store: &HubStore) -> Result<Vec<ChannelEvent>, HubError> {
    let enrolled = enrolled_senders(store)?;
    let pending = store.poll_messages(CLAUDE_AGENT_ID, false)?;
    let disturbing_ids: Vec<String> = pending
        .iter()
        .filter(|message| {
            enrolled.contains(&message.from_agent) && is_disturbing(&message.kind, &message.task_id)
        })
        .map(|message| message.id.clone())
        .collect();
    let mut events = Vec::with_capacity(disturbing_ids.len());
    for id in disturbing_ids {
        if let Some(acked) = store.ack_message(&id)? {
            events.push(to_channel_event(acked));
        }
    }
    Ok(events)
}

/// Drains messages addressed to `claude` from an enrolled team member that
/// are *not* important enough to interrupt a live session (see
/// [`is_disturbing`]) — the `check_inbox` tool calls this so Claude can
/// catch up on quieter chat traffic on its own terms. Acks each returned
/// message, same as [`poll_channel_events`], so a repeat call doesn't
/// replay history.
pub fn poll_quiet_channel_events(store: &HubStore) -> Result<Vec<ChannelEvent>, HubError> {
    let enrolled = enrolled_senders(store)?;
    let pending = store.poll_messages(CLAUDE_AGENT_ID, false)?;
    let quiet_ids: Vec<String> = pending
        .iter()
        .filter(|message| {
            enrolled.contains(&message.from_agent)
                && !is_disturbing(&message.kind, &message.task_id)
        })
        .map(|message| message.id.clone())
        .collect();
    let mut events = Vec::with_capacity(quiet_ids.len());
    for id in quiet_ids {
        if let Some(acked) = store.ack_message(&id)? {
            events.push(to_channel_event(acked));
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn poll_only_returns_events_from_enrolled_senders() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();

        store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Wake,
                "from an enrolled sender",
                None,
                None,
                None,
            )
            .unwrap();
        store
            .send_message(
                "unknown-stray-id",
                CLAUDE_AGENT_ID,
                MessageKind::Wake,
                "from an unenrolled sender",
                None,
                None,
                None,
            )
            .unwrap();

        let events = poll_channel_events(&store).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].from_agent, "grok");
        assert_eq!(events[0].body, "from an enrolled sender");

        // Draining acks the messages; a second poll returns nothing new.
        assert!(poll_channel_events(&store).unwrap().is_empty());
    }

    #[test]
    fn poll_extracts_session_id_from_the_channel_subject_convention() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Wake,
                "session-scoped",
                Some("channel:session:abc-123:message"),
                None,
                None,
            )
            .unwrap();

        let events = poll_channel_events(&store).unwrap();
        assert_eq!(events[0].session_id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn poll_channel_events_ignores_plain_untagged_messages() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "just chatting, not urgent",
                None,
                None,
                None,
            )
            .unwrap();

        // A plain message with no task tag must never interrupt the
        // session — it stays pending for `poll_quiet_channel_events`.
        assert!(poll_channel_events(&store).unwrap().is_empty());
        let quiet = poll_quiet_channel_events(&store).unwrap();
        assert_eq!(quiet.len(), 1);
        assert_eq!(quiet[0].body, "just chatting, not urgent");
    }

    #[test]
    fn poll_channel_events_disturbs_for_task_tagged_plain_messages_too() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "[TASK] please do the thing",
                None,
                None,
                Some("task-1"),
            )
            .unwrap();

        let events = poll_channel_events(&store).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_id.as_deref(), Some("task-1"));
        assert!(poll_quiet_channel_events(&store).unwrap().is_empty());
    }

    #[test]
    fn poll_quiet_channel_events_only_returns_events_from_enrolled_senders() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "quiet from enrolled",
                None,
                None,
                None,
            )
            .unwrap();
        store
            .send_message(
                "unknown-stray-id",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "quiet from unenrolled",
                None,
                None,
                None,
            )
            .unwrap();

        let events = poll_quiet_channel_events(&store).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].from_agent, "grok");

        // Draining acks the messages; a second poll returns nothing new.
        assert!(poll_quiet_channel_events(&store).unwrap().is_empty());
    }
}
