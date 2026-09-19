//! Map a parsed inbound command onto existing HubStore writes (U25 / #319).

use super::binding::BindingStore;
use super::command::{parse_inbound, Inbound};
use crate::store::{HubStore, MessageKind, WakeStatus};

/// Telegram (or Telegram-shaped) sender identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatUser {
    pub user_id: i64,
    pub chat_id: i64,
    pub username: Option<String>,
}

/// Reply sent back to the bound chat. [`Reply::Silent`] is for strangers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Silent,
    Text(String),
}

const HELP: &str = "\
/start <code> — pair this chat to the Hub
/approve <wake-id> — approve a pending wake
/reject <wake-id> — reject a pending wake
/send <text> — post to the team
/send <session-id> <text> — post to a work session
/wakes — list pending wakes
/help";

/// Handle one inbound line. Unbound senders get silence except `/start`.
pub fn handle_inbound(
    hub: &HubStore,
    bindings: &mut BindingStore,
    from: &ChatUser,
    text: &str,
) -> Reply {
    let inbound = parse_inbound(text);
    match inbound {
        Inbound::Start { code } => redeem(bindings, from, code.as_deref()),
        _ if !bindings.is_bound(from.user_id) => Reply::Silent,
        Inbound::Approve { wake_id } => resolve_wake(hub, &wake_id, WakeStatus::Delivered),
        Inbound::Reject { wake_id } => resolve_wake(hub, &wake_id, WakeStatus::Cancelled),
        Inbound::Send { session_id, body } => send(hub, session_id.as_deref(), &body),
        Inbound::Wakes => list_wakes(hub),
        Inbound::Help | Inbound::Unknown(_) => Reply::Text(HELP.into()),
    }
}

fn redeem(bindings: &mut BindingStore, from: &ChatUser, code: Option<&str>) -> Reply {
    let Some(code) = code else {
        return Reply::Text("send /start <code> from `ca telegram pair`".into());
    };
    match bindings.redeem(from.user_id, from.chat_id, from.username.clone(), code) {
        Ok(()) => Reply::Text("paired. /help for commands.".into()),
        Err(error) => Reply::Text(error.to_string()),
    }
}

fn resolve_wake(hub: &HubStore, wake_id: &str, status: WakeStatus) -> Reply {
    match hub.set_wake_status(wake_id, status) {
        Ok(()) => Reply::Text(format!("{wake_id} → {}", status.as_str())),
        Err(error) => Reply::Text(error.to_string()),
    }
}

fn send(hub: &HubStore, session_id: Option<&str>, body: &str) -> Reply {
    let result = match session_id {
        Some(session_id) => {
            let session = match hub.get_work_session(session_id) {
                Ok(Some(session)) => session,
                Ok(None) => return Reply::Text(format!("work session {session_id} not found")),
                Err(error) => return Reply::Text(error.to_string()),
            };
            let recipients: Vec<String> = session
                .member_ids
                .into_iter()
                .filter(|id| id != "human" && id != "system")
                .collect();
            hub.send_session_message("human", session_id, &recipients, body, None, None, None)
                .map(|rows| rows.len())
        }
        None => hub
            .send_message_to_team("human", MessageKind::Message, body, None, None, None)
            .map(|rows| rows.len()),
    };
    match result {
        Ok(n) => Reply::Text(format!("sent ({n})")),
        Err(error) => Reply::Text(error.to_string()),
    }
}

fn list_wakes(hub: &HubStore) -> Reply {
    match hub.list_wakes(None, true) {
        Ok(wakes) if wakes.is_empty() => Reply::Text("no pending wakes".into()),
        Ok(wakes) => {
            let lines: Vec<String> = wakes
                .into_iter()
                .map(|wake| {
                    format!(
                        "{} {} {}",
                        wake.id,
                        wake.target_agent,
                        wake.reason.unwrap_or_default()
                    )
                })
                .collect();
            Reply::Text(lines.join("\n"))
        }
        Err(error) => Reply::Text(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::binding::BindingStore;
    use crate::store::HubStore;
    use tempfile::tempdir;

    fn user(id: i64) -> ChatUser {
        ChatUser {
            user_id: id,
            chat_id: id,
            username: Some("pk".into()),
        }
    }

    #[test]
    fn strangers_are_silent_and_pairing_then_approve_works() {
        let dir = tempdir().unwrap();
        let hub = HubStore::open(dir.path()).unwrap();
        let mut bindings = BindingStore::open(dir.path()).unwrap();
        assert_eq!(
            handle_inbound(&hub, &mut bindings, &user(1), "/wakes"),
            Reply::Silent
        );
        let code = bindings.issue_pairing().unwrap();
        assert!(matches!(
            handle_inbound(&hub, &mut bindings, &user(1), &format!("/start {code}")),
            Reply::Text(text) if text.starts_with("paired")
        ));
        let wake = hub
            .request_wake("claude", Some("need you"), None, true)
            .unwrap();
        let reply = handle_inbound(
            &hub,
            &mut bindings,
            &user(1),
            &format!("/approve {}", wake.id),
        );
        assert!(matches!(reply, Reply::Text(text) if text.contains("delivered")));
        assert!(hub.list_wakes(None, true).unwrap().is_empty());
    }

    #[test]
    fn bound_send_posts_a_team_message_from_human() {
        let dir = tempdir().unwrap();
        let hub = HubStore::open(dir.path()).unwrap();
        hub.upsert_agent("human", "Human").unwrap();
        hub.upsert_agent("claude", "Claude").unwrap();
        hub.set_team_member("human", true).unwrap();
        hub.set_team_member("claude", true).unwrap();
        let mut bindings = BindingStore::open(dir.path()).unwrap();
        let code = bindings.issue_pairing().unwrap();
        handle_inbound(&hub, &mut bindings, &user(9), &format!("/start {code}"));
        let reply = handle_inbound(&hub, &mut bindings, &user(9), "/send ship it");
        assert_eq!(reply, Reply::Text("sent (1)".into()));
        let inbox = hub.list_messages(Some("claude"), None).unwrap();
        assert_eq!(inbox[0].from_agent, "human");
        assert_eq!(inbox[0].body, "ship it");
    }
}
