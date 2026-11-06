//! Slash-command parser for the Telegram remote client (U25 / #319).

/// A parsed inbound chat line. Unknown text stays [`Inbound::Unknown`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inbound {
    Start {
        code: Option<String>,
    },
    Approve {
        wake_id: String,
    },
    Reject {
        wake_id: String,
    },
    Send {
        session_id: Option<String>,
        body: String,
    },
    Wakes,
    Help,
    Unknown(String),
}

/// Parse a Telegram (or Telegram-shaped) message body into an [`Inbound`].
pub fn parse_inbound(text: &str) -> Inbound {
    let text = text.trim();
    if text.is_empty() {
        return Inbound::Unknown(String::new());
    }
    let (command, rest) = split_command(text);
    match command {
        "/start" => Inbound::Start {
            code: nonempty(rest),
        },
        "/approve" => match nonempty(rest) {
            Some(wake_id) => Inbound::Approve { wake_id },
            None => Inbound::Help,
        },
        "/reject" => match nonempty(rest) {
            Some(wake_id) => Inbound::Reject { wake_id },
            None => Inbound::Help,
        },
        "/send" => parse_send(rest),
        "/wakes" => Inbound::Wakes,
        "/help" => Inbound::Help,
        _ => Inbound::Unknown(text.to_string()),
    }
}

fn split_command(text: &str) -> (&str, &str) {
    let (raw, rest) = match text.split_once(char::is_whitespace) {
        Some((command, rest)) => (command, rest.trim()),
        None => (text, ""),
    };
    let command = raw.split_once('@').map(|(head, _)| head).unwrap_or(raw);
    (command, rest)
}

fn parse_send(rest: &str) -> Inbound {
    let rest = rest.trim();
    if rest.is_empty() {
        return Inbound::Help;
    }
    match rest.split_once(char::is_whitespace) {
        Some((first, body)) if looks_like_session_id(first) => Inbound::Send {
            session_id: Some(first.to_string()),
            body: body.trim().to_string(),
        },
        _ => Inbound::Send {
            session_id: None,
            body: rest.to_string(),
        },
    }
}

fn looks_like_session_id(token: &str) -> bool {
    token.len() == 36
        && token.as_bytes().get(8) == Some(&b'-')
        && token.as_bytes().get(13) == Some(&b'-')
        && token.as_bytes().get(18) == Some(&b'-')
        && token.as_bytes().get(23) == Some(&b'-')
        && token.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pairing_approve_reject_and_help() {
        assert_eq!(
            parse_inbound("/start ABC123"),
            Inbound::Start {
                code: Some("ABC123".into())
            }
        );
        assert_eq!(
            parse_inbound("/start@mybot ABC123"),
            Inbound::Start {
                code: Some("ABC123".into())
            }
        );
        assert_eq!(
            parse_inbound("/approve wake-1"),
            Inbound::Approve {
                wake_id: "wake-1".into()
            }
        );
        assert_eq!(
            parse_inbound("/reject wake-1"),
            Inbound::Reject {
                wake_id: "wake-1".into()
            }
        );
        assert_eq!(parse_inbound("/wakes"), Inbound::Wakes);
        assert_eq!(parse_inbound("/help"), Inbound::Help);
        assert_eq!(parse_inbound("/approve"), Inbound::Help);
    }

    #[test]
    fn send_treats_a_uuid_prefix_as_the_session() {
        let session = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        assert_eq!(
            parse_inbound(&format!("/send {session} ship it")),
            Inbound::Send {
                session_id: Some(session.into()),
                body: "ship it".into(),
            }
        );
        assert_eq!(
            parse_inbound("/send hello team"),
            Inbound::Send {
                session_id: None,
                body: "hello team".into(),
            }
        );
    }

    #[test]
    fn unknown_plain_text_is_not_a_command() {
        assert_eq!(
            parse_inbound("just chatting"),
            Inbound::Unknown("just chatting".into())
        );
    }
}
