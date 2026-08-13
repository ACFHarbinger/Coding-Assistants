//! Routes Claude's output from the `reply` MCP tool back into the Hub.

use super::CLAUDE_AGENT_ID;
use crate::{HubError, HubStore, MessageKind, MessageRecord};
use uuid::Uuid;

/// The `reply` MCP tool calls this: routes Claude's output back to the
/// Hub, addressed to the original sender of `in_reply_to` when known
/// (falling back to `human`), inside `session_id` when given.
pub fn record_channel_reply(
    store: &HubStore,
    in_reply_to: Option<&str>,
    session_id: Option<&str>,
    body: &str,
) -> Result<MessageRecord, HubError> {
    if body.trim().is_empty() {
        return Err(HubError::Invalid(
            "Channel reply body must not be empty".into(),
        ));
    }
    let recipient = match in_reply_to {
        Some(id) => store
            .get_message(id)?
            .map(|message| message.from_agent)
            .unwrap_or_else(|| "human".to_string()),
        None => "human".to_string(),
    };
    // Each reply needs its own unique subject — a fixed
    // "channel:session:<id>:reply" for every reply in the same session
    // collided with itself, and the desktop Chat & Memory view's per-post
    // dedup (designed to collapse team fan-out *copies* of one broadcast,
    // not distinct sends) collapsed every one of Claude's replies down to
    // just the latest, making earlier ones vanish as soon as a new one
    // arrived. Mirrors the uuid-suffixed default `send_session_message`
    // already generates when no explicit subject is given.
    let subject = session_id.map(|id| format!("channel:session:{id}:reply:{}", Uuid::new_v4()));
    if let Some(session_id) = session_id {
        let sent = store.send_session_message(
            CLAUDE_AGENT_ID,
            session_id,
            &[recipient],
            body,
            subject.as_deref(),
            None,
            None,
        )?;
        // Replying implies Claude has read the session up through now —
        // one less thing a human has to do manually for the read marker
        // this reply's own scope will show against.
        let _ = store.mark_read(
            CLAUDE_AGENT_ID,
            &format!("channel:session:{session_id}"),
            None,
        );
        sent.into_iter()
            .next()
            .ok_or_else(|| HubError::Invalid("Channel reply produced no recipient".into()))
    } else {
        store.send_message(
            CLAUDE_AGENT_ID,
            &recipient,
            MessageKind::Message,
            body,
            subject.as_deref(),
            None,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::events::session_id_from_subject;
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn reply_routes_to_the_original_senders_and_falls_back_to_human() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_team_member("grok", true).unwrap();
        let original = store
            .send_message(
                "grok",
                CLAUDE_AGENT_ID,
                MessageKind::Message,
                "question",
                None,
                None,
                None,
            )
            .unwrap();

        let reply = record_channel_reply(&store, Some(&original.id), None, "answer").unwrap();
        assert_eq!(reply.from_agent, CLAUDE_AGENT_ID);
        assert_eq!(reply.to_agent, "grok");

        let fallback = record_channel_reply(&store, None, None, "unprompted note").unwrap();
        assert_eq!(fallback.to_agent, "human");
    }

    #[test]
    fn reply_gives_each_session_scoped_reply_a_distinct_subject() {
        // Regression: a fixed "channel:session:<id>:reply" subject for
        // every reply in the same session collided with itself, and the
        // desktop chat's per-post dedup collapsed every earlier reply as
        // soon as a new one arrived — they appeared to vanish.
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let session = store.create_work_session("acceptance").unwrap();

        let first = record_channel_reply(&store, None, Some(&session.id), "first reply").unwrap();
        let second = record_channel_reply(&store, None, Some(&session.id), "second reply").unwrap();

        assert_ne!(first.subject, second.subject);
        assert_eq!(
            session_id_from_subject(first.subject.as_deref()).as_deref(),
            Some(session.id.as_str())
        );
        assert_eq!(
            session_id_from_subject(second.subject.as_deref()).as_deref(),
            Some(session.id.as_str())
        );
    }

    #[test]
    fn reply_rejects_an_empty_body() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(record_channel_reply(&store, None, None, "   ").is_err());
    }
}
