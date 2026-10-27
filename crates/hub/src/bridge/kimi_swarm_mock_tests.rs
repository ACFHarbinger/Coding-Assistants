//! Empirical harness for #363: a mock `kimi` executable driven through the
//! app's real delivery + capture path.
//!
//! The sandbox has no real Kimi Code CLI and no Moonshot credentials, so a
//! live `/swarm <task>` run is impossible here. Instead this test installs a
//! mock `kimi` binary under a temp `$KIMI_CODE_HOME` that, when invoked with
//! a `/swarm `-prefixed `-p` body:
//!
//! - emits a *multiplexed* stream-json sequence on stdout (interleaved
//!   main-agent and sub-agent events plus unknown event types) — the exact
//!   undocumented shape the owner flagged; and
//! - writes `agents/main/wire.jsonl` **and** sibling
//!   `agents/coder|explore/wire.jsonl` transcripts, matching the documented
//!   `agents/<name>/wire.jsonl` on-disk layout.
//!
//! The test then runs the real `deliver_kimi_task` (which spawns the mock
//! through `kimi_managed_spawn_args`) and the real
//! `capture_kimi_session_from` poller, and records what the pre-#363 code
//! would have dropped or misattributed. NOTHING here is live-CLI verified;
//! remaining live steps are listed in
//! `docs/moon/research/kimi-363-swarm-goal.md`.

use super::*;
use crate::{
    capture_kimi_session_from, kimi_session_log_path, recent_kimi_wire_texts, HarnessInjectRequest,
    KimiTaskMode,
};
use tempfile::tempdir;

/// POSIX-sh mock of the Kimi Code CLI. Logs every argv vector to
/// `$KIMI_CODE_HOME/mock_args.log`, answers `session list --json` with an
/// empty list, and writes session transcripts shaped by the `-p` body:
/// `/swarm ` → multiplexed stdout + three agent wires; `/goal ` →
/// goal-shaped unknown events appended to the main wire; anything else → a
/// plain main wire.
const MOCK_KIMI: &str = r##"#!/bin/sh
home="$KIMI_CODE_HOME"
{
  echo "== invocation"
  i=0
  for a in "$@"; do
    printf 'arg%s=%s\n' "$i" "$a"
    i=$((i + 1))
  done
} >> "$home/mock_args.log"

if [ "$1" = "session" ]; then
  echo '[]'
  exit 0
fi

body=""
sess=""
prev=""
for a in "$@"; do
  case "$prev" in
    -p) body="$a" ;;
    --session) sess="$a" ;;
  esac
  prev="$a"
done
[ -n "$sess" ] || sess="session_mock-1"

dir="$home/sessions/wd_mock/$sess"
mkdir -p "$dir/agents/main"
printf '{"id":"%s","cwd":"%s","updatedAt":1000,"archived":false}\n' "$sess" "$PWD" > "$dir/state.json"

case "$body" in
  "/swarm "*)
    # Multiplexed stream-json on stdout. The app pipes stdout to null, so
    # nothing parses this; emit it anyway to prove a changed event shape
    # cannot break delivery.
    printf '%s\n' \
      '{"type":"init","agentId":"main","session":"x"}' \
      '{"type":"swarm.dispatch","agentId":"main","workers":["coder","explore"]}' \
      '{"type":"assistant.delta","agentId":"coder","text":"coder stream chunk"}' \
      '{"type":"assistant.delta","agentId":"main","text":"main stream chunk"}' \
      '{"type":"subagent.unknown_future","agentId":"explore","payload":{"nested":[1,2]}}' \
      '{"type":"result","agentId":"main","status":"ok"}'
    mkdir -p "$dir/agents/coder" "$dir/agents/explore"
    cat > "$dir/agents/main/wire.jsonl" <<'WIRE'
{"type":"turn.prompt","agentId":"main","time":1}
{"type":"swarm.dispatch","agentId":"main","workers":["coder","explore"],"time":2}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"main swarm summary"}},"time":3}
{"type":"context.append_loop_event","agentId":"coder","event":{"type":"content.part","part":{"type":"text","text":"coder multiplexed status"}},"time":4}
{"type":"subagent.heartbeat","agentId":"explore","metrics":{"x":1},"time":5}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"think","think":"hidden reasoning"}},"time":6}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":
WIRE
    cat > "$dir/agents/coder/wire.jsonl" <<'WIRE'
{"type":"context.append_loop_event","event":{"type":"content.part","part":{"type":"text","text":"coder own transcript text"}},"time":1}
{"type":"coder.custom_event","agentId":"coder","time":2}
WIRE
    cat > "$dir/agents/explore/wire.jsonl" <<'WIRE'
{"type":"context.append_message","message":{"role":"assistant","content":[{"type":"text","text":"explore findings"}]},"time":1}
{"type":"context.append_message","message":{"role":"assistant","content":"system injection","origin":{"kind":"injected"}},"time":2}
WIRE
    ;;
  "/goal "*)
    cat >> "$dir/agents/main/wire.jsonl" <<'WIRE'
{"type":"goal.acknowledged","agentId":"main","time":7}
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"goal acknowledged, continuing"}},"time":8}
WIRE
    printf '%s\n' \
      '{"type":"goal.status","agentId":"main","state":"running"}' \
      '{"type":"result","agentId":"main","status":"ok","exit_code":0}'
    ;;
  *)
    cat > "$dir/agents/main/wire.jsonl" <<'WIRE'
{"type":"context.append_loop_event","agentId":"main","event":{"type":"content.part","part":{"type":"text","text":"plain response"}},"time":1}
WIRE
    printf '%s\n' '{"type":"result","agentId":"main","status":"ok"}'
    ;;
esac
exit 0
"##;

fn install_mock_kimi(home: &Path) {
    let bin = home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let mock = bin.join("kimi");
    fs::write(&mock, MOCK_KIMI).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Bodies delivered via `-p` across all mock invocations, in order.
fn delivered_bodies(home: &Path) -> Vec<String> {
    let log = fs::read_to_string(home.join("mock_args.log")).unwrap();
    let mut bodies = Vec::new();
    let mut prev = String::new();
    for line in log.lines() {
        if let Some(value) = line
            .strip_prefix("arg")
            .and_then(|l| l.split_once('=').map(|p| p.1))
        {
            if prev == "-p" {
                bodies.push(value.to_string());
            }
            prev = value.to_string();
        }
    }
    bodies
}

fn deliver(store: &HubStore, workspace: &Path, mode: Option<KimiTaskMode>, body: &str) -> String {
    let request = HarnessInjectRequest {
        harness: "kimi".into(),
        workspace: workspace.to_path_buf(),
        session_id: Some("session_mock-1".into()),
        message_id: None,
        body: body.into(),
        is_task: true,
        is_wake: false,
        model: None,
        effort: None,
        kimi_mode: mode,
    };
    let result = deliver_kimi_task(store, &request).unwrap();
    assert_eq!(result.status, "delivered", "{}", result.detail);
    result.detail
}

#[test]
fn mock_kimi_swarm_and_goal_end_to_end() {
    // `bridge::kimi` and `harness::kimi_spawn` tests mutate `KIMI_CODE_HOME`
    // under the same lock; hold it for the whole run.
    let _env_guard = crate::KIMI_CODE_HOME_LOCK.lock().unwrap();

    let home = tempdir().unwrap();
    install_mock_kimi(home.path());
    std::env::set_var("KIMI_CODE_HOME", home.path());

    let hub_dir = tempdir().unwrap();
    let store = HubStore::open(hub_dir.path()).unwrap();
    let ws_dir = tempdir().unwrap();
    let workspace = ws_dir.path().canonicalize().unwrap();
    let workspace_key = workspace.to_string_lossy().into_owned();
    store
        .register_managed_harness_session_with_state(
            "kimi",
            &workspace_key,
            "session_mock-1",
            None,
            HarnessSessionState::Ready,
        )
        .unwrap();

    // --- 1. /swarm delivery through the real spawn path -----------------
    let detail = deliver(
        &store,
        &workspace,
        Some(KimiTaskMode::Swarm),
        "map the repo and summarize",
    );
    assert!(detail.contains("/swarm prefix applied"), "{detail}");

    // --- 2. /goal delivery; then an explicit slash body passes through --
    deliver(
        &store,
        &workspace,
        Some(KimiTaskMode::Goal),
        "keep the build green",
    );
    deliver(
        &store,
        &workspace,
        Some(KimiTaskMode::Swarm),
        "/goal status",
    );

    let bodies = delivered_bodies(home.path());
    assert_eq!(
        bodies,
        [
            "/swarm map the repo and summarize",
            "/goal keep the build green",
            "/goal status",
        ]
    );

    // --- 3. Swarm-aware capture over the mock's transcripts -------------
    let sessions_root = home.path().join("sessions");
    let out = capture_kimi_session_from(
        &sessions_root,
        &store,
        &workspace,
        Some("session_mock-1"),
        None,
    )
    .unwrap();
    assert!(out.transcript_found);

    let mut kimi: Vec<&str> = Vec::new();
    let mut coder: Vec<&str> = Vec::new();
    let mut explore: Vec<&str> = Vec::new();
    for record in &out.captured {
        match record.from_agent.as_str() {
            "kimi" => kimi.push(record.body.as_str()),
            "kimi-coder" => coder.push(record.body.as_str()),
            "kimi-explore" => explore.push(record.body.as_str()),
            other => panic!("unexpected capture agent {other}"),
        }
    }
    assert_eq!(
        kimi,
        ["main swarm summary", "goal acknowledged, continuing"]
    );
    assert_eq!(
        coder,
        ["coder multiplexed status", "coder own transcript text"]
    );
    assert_eq!(explore, ["explore findings"]);

    // Unknown event types (swarm.dispatch, subagent.heartbeat,
    // coder.custom_event, goal.acknowledged), the truncated tail line, and
    // thinking/injected parts were all tolerated: nothing errored and none
    // of them leaked into the transcript.

    // --- 4. What the pre-#363 code did with the same transcripts --------
    let main_wire =
        kimi_session_log_path(&sessions_root, "session_mock-1").expect("main wire located");
    let old_style = recent_kimi_wire_texts(&main_wire, 200, "main");
    let old_texts: Vec<&str> = old_style.iter().map(|c| c.text.as_str()).collect();
    // Old code read ONLY agents/main/wire.jsonl and recorded every text as
    // agent "kimi": sibling wires were dropped entirely and multiplexed
    // sub-agent events were misattributed to "kimi".
    assert!(old_texts.contains(&"coder multiplexed status"));
    assert!(!old_texts.contains(&"coder own transcript text"));
    assert!(!old_texts.contains(&"explore findings"));
    eprintln!(
        "#363 empirical (mock) summary: old capture saw {} main-wire text(s) all as `kimi` \
         (misattributing multiplexed sub-agent events) and dropped 2 sibling-wire text(s); \
         hardened capture records {} text(s) across agents [kimi, kimi-coder, kimi-explore].",
        old_texts.len(),
        out.captured.len()
    );

    // --- 5. Second poll dedups across every wire ------------------------
    let out2 = capture_kimi_session_from(
        &sessions_root,
        &store,
        &workspace,
        Some("session_mock-1"),
        None,
    )
    .unwrap();
    assert_eq!(out2.captured.len(), 0);

    std::env::remove_var("KIMI_CODE_HOME");
}
