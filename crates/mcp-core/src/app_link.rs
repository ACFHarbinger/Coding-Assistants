//! The client half of the app-side transport every real tool bridge uses.
//!
//! A `crates/mcp-<tool>` bridge does not touch its target application
//! directly — the application's native plugin (`plugins/<tool>/`) opens a
//! localhost line-JSON TCP server, and the bridge connects to it once per
//! `tools/call`:
//!
//! ```text
//! bridge ──connect── {"op": <tool>, "args": {...}}\n ──►  plugin
//! bridge ◄────────── {"ok": bool, "result"|"error": ...}\n ─┘
//! ```
//!
//! Per-call connections keep a bridge stateless and unbothered by the
//! plugin restarting. This module was lifted out of `mcp-blender` once
//! `mcp-krita` needed the identical thing (C-3).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// One request/response round-trip to a plugin. Implemented by
/// [`TcpAppLink`] for real use and by a mock in each bridge's tests.
pub trait AppLink: Send + Sync {
    /// `op` is the tool name; `args` is its `arguments` object. Returns the
    /// plugin's `result` payload, or a human-readable error string
    /// (surfaced to the agent as a tool error, never a panic).
    fn request(&self, op: &str, args: &Value) -> Result<Value, String>;
}

/// A fresh TCP connection to `127.0.0.1:<port>` per call. `app_label` is
/// only used to phrase the "is it running?" connection error.
pub struct TcpAppLink {
    pub port: u16,
    pub app_label: &'static str,
}

impl TcpAppLink {
    pub fn new(port: u16, app_label: &'static str) -> Self {
        Self { port, app_label }
    }
}

impl AppLink for TcpAppLink {
    fn request(&self, op: &str, args: &Value) -> Result<Value, String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let socket_addr = addr
            .parse()
            .map_err(|e| format!("bad address {addr}: {e}"))?;
        let stream = TcpStream::connect_timeout(&socket_addr, CONNECT_TIMEOUT).map_err(|e| {
            format!(
                "could not reach the {} plugin on {addr}: {e}. \
                 Is {} running with the Coding-Assistants plugin enabled?",
                self.app_label, self.app_label
            )
        })?;
        stream.set_read_timeout(Some(IO_TIMEOUT)).ok();
        stream.set_write_timeout(Some(IO_TIMEOUT)).ok();

        let mut writer = &stream;
        let payload = json!({ "op": op, "args": args });
        writeln!(writer, "{payload}").map_err(|e| format!("write to plugin failed: {e}"))?;
        writer.flush().ok();

        let mut line = String::new();
        BufReader::new(&stream)
            .read_line(&mut line)
            .map_err(|e| format!("read from plugin failed: {e}"))?;
        parse_response(&line)
    }
}

/// Interpret one `{ "ok": bool, ... }` response line.
pub fn parse_response(line: &str) -> Result<Value, String> {
    let line = line.trim();
    if line.is_empty() {
        return Err("empty response from the plugin".to_string());
    }
    let value: Value =
        serde_json::from_str(line).map_err(|e| format!("plugin sent invalid JSON: {e}"))?;
    match value.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(value.get("result").cloned().unwrap_or(Value::Null)),
        Some(false) => Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unspecified plugin error")
            .to_string()),
        None => Err("plugin response missing the `ok` field".to_string()),
    }
}

/// Render a plugin `result` value as tool-call text: strings pass through,
/// everything else is pretty JSON.
pub fn result_to_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_maps_ok_err_and_malformed() {
        assert_eq!(
            parse_response(r#"{"ok":true,"result":{"n":3}}"#).unwrap(),
            json!({ "n": 3 })
        );
        assert_eq!(
            parse_response(r#"{"ok":true}"#).unwrap(),
            Value::Null,
            "ok with no result is null"
        );
        assert_eq!(
            parse_response(r#"{"ok":false,"error":"no active document"}"#).unwrap_err(),
            "no active document"
        );
        assert!(parse_response("").is_err());
        assert!(parse_response("not json").is_err());
        assert!(parse_response(r#"{"result":1}"#).is_err(), "missing ok");
    }

    #[test]
    fn result_to_text_passes_strings_and_pretty_prints_the_rest() {
        assert_eq!(result_to_text(&Value::String("hi".into())), "hi");
        assert!(result_to_text(&json!({ "a": 1 })).contains("\"a\": 1"));
    }

    #[test]
    fn tcp_link_reports_a_helpful_error_when_nothing_is_listening() {
        let link = TcpAppLink::new(59_998, "Krita");
        let err = link.request("ping", &json!({})).unwrap_err();
        assert!(err.contains("Krita"));
        assert!(err.contains("Is Krita running"));
    }
}
