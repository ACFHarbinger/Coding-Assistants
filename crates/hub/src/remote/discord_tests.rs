use super::*;
use crate::store::HubStore;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

const FIXTURE: &str = r#"[{"id":"902","channel_id":"100","type":0,"author":{"id":"7","username":"pk","bot":false},"content":"/wakes"},{"id":"901","channel_id":"100","type":0,"author":{"id":"9","username":"ca-bot","bot":true},"content":"paired."},{"id":"900","channel_id":"100","type":0,"author":{"id":"7","username":"pk","bot":false},"content":"  "},{"id":"899","channel_id":"100","type":0,"author":{"id":"8","username":"rk","bot":false},"content":"/start ABC123"}]"#;

#[test]
fn parse_messages_skips_bots_blank_texts_and_orders_oldest_first() {
    let (inbound, max_seen) = parse_messages(FIXTURE, 100).unwrap();
    assert_eq!(max_seen, Some(902));
    assert_eq!(inbound.len(), 2);
    assert_eq!(inbound[0].0, 899);
    assert_eq!(inbound[0].2, "/start ABC123");
    assert_eq!(inbound[1].0, 902);
    assert_eq!(inbound[1].1, user(7));
    assert_eq!(inbound[1].2, "/wakes");
}

#[test]
fn snowflake_accepts_a_bare_number() {
    let (inbound, max) =
        parse_messages(r#"[{"id":42,"author":{"id":7},"content":"hi"}]"#, 100).unwrap();
    assert_eq!((max, inbound[0].0, &inbound[0].2[..]), (Some(42), 42, "hi"));
}

#[test]
fn truncate_caps_at_discord_limit() {
    let cut = truncate(&"x".repeat(3000));
    assert_eq!(cut.chars().count(), MAX_MESSAGE_CHARS);
    assert!(cut.ends_with('…'));
    assert_eq!(truncate("short"), "short");
}

fn user(id: u64) -> DiscordUser {
    DiscordUser {
        user_id: id,
        channel_id: 100,
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
    let code = bindings.issue_pairing(100).unwrap();
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
fn reload_error_fails_closed_and_denies_stale_authorized_user() {
    let dir = tempdir().unwrap();
    let hub = HubStore::open(dir.path()).unwrap();
    let mut bindings = BindingStore::open(dir.path()).unwrap();
    let code = bindings.issue_pairing(100).unwrap();
    bindings.redeem(7, 100, Some("pk".into()), &code).unwrap();
    assert!(bindings.is_bound(7));

    let wake = hub
        .request_wake("claude", Some("need you"), None, true)
        .unwrap();

    // Corrupt binding file so reload fails
    let binding_file = dir.path().join("discord_binding.json");
    fs::write(&binding_file, b"corrupted not json").unwrap();

    // Must fail closed: return Reply::Silent, do not approve wake
    let reply = handle_inbound(
        &hub,
        &mut bindings,
        &user(7),
        &format!("/approve {}", wake.id),
    );
    assert_eq!(reply, Reply::Silent, "must fail closed on reload error");
    assert_eq!(
        hub.list_wakes(None, true).unwrap().len(),
        1,
        "wake must not be approved on reload error"
    );
}

#[test]
fn process_channel_messages_aborts_on_reload_error_without_advancing_cursor() {
    let dir = tempdir().unwrap();
    let hub = HubStore::open(dir.path()).unwrap();
    let mut bindings = BindingStore::open(dir.path()).unwrap();
    let code = bindings.issue_pairing(100).unwrap();
    bindings.redeem(7, 100, Some("pk".into()), &code).unwrap();
    bindings.advance_cursor(100, 10).unwrap();

    // Corrupt binding file
    let binding_file = dir.path().join("discord_binding.json");
    fs::write(&binding_file, b"corrupted not json").unwrap();

    let inbound = vec![(999u64, user(7), "/wakes".into())];
    let res =
        process_channel_messages_to("http://127.0.0.1:1", &hub, &mut bindings, "token", inbound);
    assert!(res.is_err(), "must fail when reload fails");

    // Must not have advanced cursor or overwritten corrupted file with stale data
    let raw = fs::read_to_string(&binding_file).unwrap();
    assert_eq!(
        raw, "corrupted not json",
        "corrupted file must not be overwritten"
    );
}

#[test]
fn process_channel_messages_persists_cursor_before_reply_failure() {
    let dir = tempdir().unwrap();
    let hub = HubStore::open(dir.path()).unwrap();
    let mut bindings = BindingStore::open(dir.path()).unwrap();
    let code = bindings.issue_pairing(100).unwrap();
    bindings.redeem(7, 100, Some("pk".into()), &code).unwrap();

    let wake = hub
        .request_wake("worker", Some("need you"), None, true)
        .unwrap();
    let (base, _handle) = serve_once(500, r#"{"message":"error"}"#);
    let inbound = vec![(999u64, user(7), format!("/approve {}", wake.id))];

    let result = process_channel_messages_to(&base, &hub, &mut bindings, "token", inbound);
    assert!(result.is_err(), "must report send_message failure");
    assert!(
        hub.list_wakes(None, true).unwrap().is_empty(),
        "wake was approved"
    );
    assert_eq!(bindings.cursor(100), Some(999), "cursor advanced in memory");

    let reopened = BindingStore::open(dir.path()).unwrap();
    assert_eq!(reopened.cursor(100), Some(999), "cursor persisted to disk");
}

fn serve_once(status: u16, body: &str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut raw = Vec::new();
        let mut chunk = [0u8; 2048];
        while let Ok(n) = stream.read(&mut chunk) {
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&chunk[..n]);
            if request_complete(&raw) {
                break;
            }
        }
        let reply = format!(
            "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(reply.as_bytes());
        String::from_utf8_lossy(&raw).to_string()
    });
    (base, handle)
}

fn request_complete(raw: &[u8]) -> bool {
    let text = String::from_utf8_lossy(raw);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    let wanted = head
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(n, _)| n.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    body.len() >= wanted
}

#[test]
fn fetch_and_send_hit_the_rest_endpoints_with_bot_auth() {
    let (base, get) = serve_once(200, FIXTURE);
    let (inbound, max_seen) = fetch_messages_from(&base, "secret", 100, Some(900)).unwrap();
    let request = get.join().unwrap();
    assert!(request.starts_with("GET /channels/100/messages?limit=100&after=900 "));
    assert!(request.contains("\r\nauthorization: Bot secret\r\n"));
    assert_eq!(max_seen, Some(902));
    assert_eq!(inbound.len(), 2);

    let (base, post) = serve_once(200, r#"{"id": "903"}"#);
    send_message_to(&base, "secret", 100, "hello").unwrap();
    let request = post.join().unwrap();
    assert!(request.starts_with("POST /channels/100/messages "));
    assert!(request.contains("\r\nauthorization: Bot secret\r\n"));
    assert!(request.contains("hello"));
}

#[test]
fn send_retries_once_after_a_429_then_succeeds() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        for (status, body) in [
            (429, r#"{"message":"rate limited","retry_after":0.01}"#),
            (200, r#"{"id":"903"}"#),
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = [0u8; 4096];
            assert!(stream.read(&mut raw).unwrap() > 0);
            let reply = format!(
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
        }
    });
    send_message_to(&base, "secret", 100, "hello").unwrap();
    handle.join().unwrap();
}
