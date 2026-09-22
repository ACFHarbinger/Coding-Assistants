//! Public A2A wire compatibility (P11b / #335). Minimal std-only HTTP/1.1
//! listener speaking the A2A JSON-RPC envelope — no new dependencies.
//!
//! v1 methods: `message/send`, `tasks/get`, `tasks/cancel`, plus the public
//! agent card. Out of scope: streaming/SSE, push notifications, non-text
//! message parts.
//!
//! Security posture (see the P11b design note on the bus):
//! - Default-off: the listener only starts after the owner sets
//!   `orchestration.a2a_enabled`, and only via an explicit
//!   `start_a2a_server` call. Default bind is loopback.
//! - Auth required: every JSON-RPC call needs the P6 pairing token as
//!   Bearer, resolved and compared exactly like the TCP channel.
//! - No path leaks: the public (unauthenticated, for discovery) card
//!   carries names, descriptions, and skill tags only — never workspaces,
//!   file paths, or machine info. Pinned by test.

use hub::{AgentCard, AgentRecord, HubStore, WorkflowStep};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

pub const DEFAULT_A2A_PORT: u16 = 8766;
pub const DEFAULT_BIND: &str = "127.0.0.1";
pub const CARD_PATH: &str = "/.well-known/agent-card.json";
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Build the public agent card. Only team-enrolled agents appear, with
/// sanitized skill fields — this function never inserts paths, workspaces,
/// or machine info (pinned by `card_carries_no_paths`).
pub fn build_agent_card(base_url: &str, version: &str, agents: &[AgentRecord]) -> Value {
    let skills: Vec<Value> = agents
        .iter()
        .filter(|agent| agent.team_member)
        .map(|agent| {
            let card: Option<AgentCard> = agent
                .card_json
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok());
            json!({
                "id": agent.id,
                "name": agent.display_name,
                "description": card
                    .as_ref()
                    .map(|c| c.description.clone())
                    .filter(|d| !d.trim().is_empty())
                    .unwrap_or_else(|| agent.display_name.clone()),
                "tags": card.map(|c| c.specializations).unwrap_or_default(),
            })
        })
        .collect();
    json!({
        "name": "Coding Assistants",
        "description": "Coding Assistants Hub: multi-agent orchestration over A2A.",
        "url": format!("{base_url}/"),
        "version": version,
        "capabilities": {"streaming": false, "pushNotifications": false},
        "defaultInputModes": ["text"],
        "defaultOutputModes": ["text"],
        "authentication": {"schemes": ["bearer"]},
        "skills": skills,
    })
}

fn bearer_token(headers: &HashMap<String, String>) -> Option<String> {
    headers.get("authorization").and_then(|value| {
        value
            .strip_prefix("Bearer ")
            .or_else(|| value.strip_prefix("bearer "))
            .map(str::to_string)
    })
}

/// Bearer check reusing the P6 fail-closed comparison. `expected` is the
/// resolved pairing secret; anything missing or mismatched is refused.
pub fn check_bearer(headers: &HashMap<String, String>, expected: Option<&str>) -> bool {
    crate::server::tcp_auth::authorize(expected, bearer_token(headers).as_deref()).is_ok()
}

fn rpc_error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn rpc_ok(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Map Hub task states onto A2A task states. Unknown persists as `failed`
/// rather than claiming progress.
pub fn a2a_task_state(status: &str) -> &'static str {
    match status {
        "pending" => "submitted",
        "running" => "working",
        "done" => "completed",
        "cancelled" => "canceled",
        "failed" => "failed",
        _ => "failed",
    }
}

fn task_result(store: &HubStore, task: &hub::TaskRecord, context_id: &str) -> Value {
    let state = a2a_task_state(&task.status);
    let message = task
        .last_message_id
        .as_deref()
        .and_then(|id| store.get_message(id).ok())
        .flatten()
        .map(|msg| {
            json!({
                "role": "agent",
                "messageId": msg.id,
                "parts": [{"type": "text", "text": msg.body}],
            })
        });
    let mut status = json!({"state": state});
    if let Some(message) = message {
        status["message"] = message;
    }
    json!({
        "id": task.id,
        "contextId": context_id,
        "status": status,
        "artifacts": [],
    })
}

/// Dispatch one JSON-RPC call against the store. Pure logic — no sockets —
/// so every method is unit-testable against a temp `HubStore`.
pub fn handle_rpc(store: &HubStore, method: &str, params: &Value, id: &Value) -> Value {
    match method {
        "message/send" => {
            let message = params.get("message");
            let text = message
                .and_then(|m| m.get("parts"))
                .and_then(|p| p.as_array())
                .map(|parts| {
                    parts
                        .iter()
                        .filter(|part| part.get("type").and_then(|t| t.as_str()) == Some("text"))
                        .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            if text.trim().is_empty() {
                return rpc_error(id, -32602, "message/send needs message.parts[] with text");
            }
            let agent = params
                .get("metadata")
                .and_then(|m| m.get("ca_agent"))
                .and_then(|a| a.as_str())
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let Some(agent) = agent else {
                return rpc_error(id, -32602, "message/send needs metadata.ca_agent");
            };
            let context_id = params
                .get("contextId")
                .and_then(|c| c.as_str())
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string);
            let title: String = text
                .lines()
                .next()
                .unwrap_or("a2a task")
                .chars()
                .take(80)
                .collect();
            let steps = vec![WorkflowStep {
                agent: agent.to_string(),
                role: None,
                instruction: text,
                max_retries: 0,
                parallel_group: None,
                peer: None,
            }];
            let task = match store
                .create_task(&title, None, &steps)
                .and_then(|task| store.advance_task(&task.id, Some(agent), None))
            {
                Ok(task) => task,
                Err(error) => return rpc_error(id, -32000, &error.to_string()),
            };
            let context = context_id.unwrap_or_else(|| task.id.clone());
            rpc_ok(id, task_result(store, &task, &context))
        }
        "tasks/get" => {
            let task_id = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if task_id.is_empty() {
                return rpc_error(id, -32602, "tasks/get needs params.id");
            }
            match store.get_task(task_id) {
                Ok(Some(task)) => {
                    let context = params
                        .get("contextId")
                        .and_then(|c| c.as_str())
                        .unwrap_or(&task.id)
                        .to_string();
                    rpc_ok(id, task_result(store, &task, &context))
                }
                Ok(None) => rpc_error(id, -32001, "task not found"),
                Err(error) => rpc_error(id, -32000, &error.to_string()),
            }
        }
        "tasks/cancel" => {
            let task_id = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if task_id.is_empty() {
                return rpc_error(id, -32602, "tasks/cancel needs params.id");
            }
            match store.cancel_task(task_id) {
                Ok(task) => rpc_ok(id, task_result(store, &task, &task.id)),
                Err(error) => rpc_error(id, -32000, &error.to_string()),
            }
        }
        _ => rpc_error(id, -32601, "method not found"),
    }
}

/// Minimal HTTP/1.1 request head + body split for the three v1 routes.
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub fn parse_http_request(head: &str, body: &str) -> Result<HttpRequest, String> {
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or("empty request")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("bad request line")?.to_string();
    let path = parts.next().ok_or("bad request line")?.to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or("bad header line")?;
        headers.insert(name.trim().to_lowercase(), value.trim().to_string());
    }
    Ok(HttpRequest {
        method,
        path,
        headers,
        body: body.to_string(),
    })
}

fn http_reply(status: u16, reason: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// Route one parsed request. Card building needs the store's agent list;
/// `expected_token` is the resolved pairing secret (fail closed when `None`).
pub fn route_request(
    store: &HubStore,
    request: &HttpRequest,
    base_url: &str,
    expected_token: Option<&str>,
) -> String {
    if request.method == "GET" && request.path == CARD_PATH {
        let agents = store.list_agents().unwrap_or_default();
        let card = build_agent_card(base_url, env!("CARGO_PKG_VERSION"), &agents);
        let body = serde_json::to_string(&card).unwrap_or_default();
        return http_reply(200, "OK", "application/json", &body);
    }
    if request.method == "POST" && request.path == "/" {
        if !check_bearer(&request.headers, expected_token) {
            let body =
                serde_json::to_string(&rpc_error(&Value::Null, -32001, "authentication required"))
                    .unwrap_or_default();
            return http_reply(401, "Unauthorized", "application/json", &body);
        }
        let envelope: Value = match serde_json::from_str(&request.body) {
            Ok(value) => value,
            Err(_) => {
                let body = serde_json::to_string(&rpc_error(&Value::Null, -32700, "parse error"))
                    .unwrap_or_default();
                return http_reply(400, "Bad Request", "application/json", &body);
            }
        };
        let id = envelope.get("id").cloned().unwrap_or(Value::Null);
        let method = envelope
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        let params = envelope.get("params").cloned().unwrap_or(Value::Null);
        let response = handle_rpc(store, method, &params, &id);
        let body = serde_json::to_string(&response).unwrap_or_default();
        return http_reply(200, "OK", "application/json", &body);
    }
    http_reply(
        404,
        "Not Found",
        "application/json",
        r#"{"error":"not found"}"#,
    )
}

/// Read one HTTP request (head + Content-Length body) from a connection.
fn read_request(stream: &TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut head = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).map_err(|e| e.to_string())?;
        head.push(byte[0]);
        if head.len() > 65536 {
            return Err("request head too large".into());
        }
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let head_text = String::from_utf8(head).map_err(|_| "request head is not UTF-8".to_string())?;
    let length = head_text
        .split("\r\n")
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0)
        .min(8 * 1024 * 1024);
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    let body_text = String::from_utf8(body).map_err(|_| "body is not UTF-8".to_string())?;
    let head_only = head_text.split("\r\n\r\n").next().unwrap_or("").to_string();
    parse_http_request(&head_only, &body_text)
}

/// Handle one connection: parse, route against a fresh store handle, reply.
fn serve_connection(stream: TcpStream, base_url: String, expected_token: Option<String>) {
    let reply = match read_request(&stream) {
        Ok(request) => match crate::commands::commands::store::open_store() {
            Ok(store) => route_request(&store, &request, &base_url, expected_token.as_deref()),
            Err(error) => http_reply(
                500,
                "Error",
                "application/json",
                &format!("{{\"error\":\"{}\"}}", error.replace('"', "'")),
            ),
        },
        Err(error) => http_reply(
            400,
            "Bad Request",
            "application/json",
            &format!("{{\"error\":\"{}\"}}", error.replace('"', "'")),
        ),
    };
    let mut stream = stream;
    let _ = stream.write_all(reply.as_bytes());
    let _ = stream.flush();
}

/// The listener. One thread accepts (nonblocking poll); each connection gets
/// its own thread. Never started unless the owner enabled it and called
/// `start_a2a_server`.
pub struct A2aServer {
    address: String,
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl A2aServer {
    pub fn start(bind: &str, port: u16) -> Result<Self, String> {
        let listener = TcpListener::bind(format!("{bind}:{port}"))
            .map_err(|error| format!("cannot bind A2A listener on {bind}:{port}: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let address = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .to_string();
        let base_url = format!("http://{address}");
        let running = Arc::new(AtomicBool::new(true));
        let flag = running.clone();
        let expected_token = hub::secret::resolve(crate::server::tcp_auth::TOKEN_VAULT_KEY)
            .map(|secret| secret.expose().to_string());
        let handle = std::thread::spawn(move || {
            while flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let base_url = base_url.clone();
                        let expected_token = expected_token.clone();
                        std::thread::spawn(move || {
                            serve_connection(stream, base_url, expected_token)
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            address,
            running,
            handle: Some(handle),
        })
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[path = "a2a_tests.rs"]
#[cfg(test)]
mod a2a_tests;
