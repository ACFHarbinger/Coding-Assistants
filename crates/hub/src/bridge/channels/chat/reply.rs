//! Routes Codex's captured turn output back into the Hub.
//!
//! Mirrors `bridge::channels::claude::reply::record_channel_reply`: address
//! the reply to the original sender of `in_reply_to` when known (falling
//! back to `human`), inside `session_id` when the delivery was session-scoped.

use super::CODEX_AGENT_ID;
use crate::{HubError, HubStore, MessageKind, MessageRecord};
use uuid::Uuid;

pub fn record_codex_reply(
    store: &HubStore,
    in_reply_to: Option<&str>,
    session_id: Option<&str>,
    body: &str,
) -> Result<MessageRecord, HubError> {
    if body.trim().is_empty() {
        return Err(HubError::Invalid(
            "Codex reply body must not be empty".into(),
        ));
    }
    let recipient = match in_reply_to {
        Some(id) => store
            .get_message(id)?
            .map(|message| message.from_agent)
            .unwrap_or_else(|| "human".to_string()),
        None => "human".to_string(),
    };
    // Same reasoning as the Claude Channel's reply routing: a fixed subject
    // per session collides with itself across multiple replies, collapsing
    // earlier ones out of the desktop Chat & Memory view's per-post dedup.
    let subject = session_id.map(|id| format!("channel:session:{id}:reply:{}", Uuid::new_v4()));
    if let Some(session_id) = session_id {
        let sent = store.send_session_message(
            CODEX_AGENT_ID,
            session_id,
            &[recipient],
            body,
            subject.as_deref(),
            None,
            None,
        )?;
        let _ = store.mark_read(
            CODEX_AGENT_ID,
            &format!("channel:session:{session_id}"),
            None,
        );
        sent.into_iter()
            .next()
            .ok_or_else(|| HubError::Invalid("Codex reply produced no recipient".into()))
    } else {
        store.send_message(
            CODEX_AGENT_ID,
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
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn reply_routes_to_the_original_sender_and_falls_back_to_human() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.upsert_agent("orchestrator", "Orchestrator").unwrap();
        store.set_team_member("orchestrator", true).unwrap();
        let original = store
            .send_message(
                "orchestrator",
                CODEX_AGENT_ID,
                MessageKind::Message,
                "question",
                None,
                None,
                None,
            )
            .unwrap();

        let reply = record_codex_reply(&store, Some(&original.id), None, "answer").unwrap();
        assert_eq!(reply.from_agent, CODEX_AGENT_ID);
        assert_eq!(reply.to_agent, "orchestrator");

        let fallback = record_codex_reply(&store, None, None, "unprompted note").unwrap();
        assert_eq!(fallback.to_agent, "human");
    }

    #[test]
    fn reply_gives_each_session_scoped_reply_a_distinct_subject() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let session = store.create_work_session("acceptance").unwrap();

        let first = record_codex_reply(&store, None, Some(&session.id), "first reply").unwrap();
        let second = record_codex_reply(&store, None, Some(&session.id), "second reply").unwrap();

        assert_ne!(first.subject, second.subject);
    }

    #[test]
    fn reply_rejects_an_empty_body() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(record_codex_reply(&store, None, None, "   ").is_err());
    }
}
