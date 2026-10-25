//! CLIP STUDIO PAINT MCP bridge.
//!
//! The tool names follow the unofficial 5.1.2 production catalog. File
//! exchange is enforced against three configured roots. Window, screenshot,
//! and input tools fail closed: this bridge does not attest `CLIPStudioPaint.exe`
//! and does not inject pointer or keyboard input.

mod files;
mod policy;
mod tools;

use mcp_core::{McpServer, MemoryProvider, MemoryTools, ServerInfo, ToolProvider, ToolResult};
use policy::{
    business_params, load_config, Config, Tokens, ATOMIC_INPUT, NO_SESSION, PROFILE, SESSION_TOOLS,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct ClipStudio {
    config: Option<Config>,
    tokens: Mutex<Tokens>,
}

impl ClipStudio {
    pub fn new(config: Option<Config>) -> Self {
        Self {
            config,
            tokens: Mutex::new(Tokens::default()),
        }
    }

    fn health(&self) -> Value {
        json!({
            "profile": PROFILE,
            "session_attested": false,
            "input": "closed",
            "roots_configured": self.config.is_some(),
            "locale": "zh-CN",
            "dpi": 120,
            "csp_version": "5.1.2.0",
        })
    }

    fn capabilities(&self) -> Value {
        let tools: Vec<Value> = self
            .tools()
            .iter()
            .filter_map(|tool| tool.get("name").and_then(|v| v.as_str()))
            .map(|name| {
                let (available, reason) = self.availability(name);
                json!({ "name": name, "available": available, "reason": reason })
            })
            .collect();
        json!({ "profile": PROFILE, "tools": tools })
    }

    fn availability(&self, name: &str) -> (bool, &'static str) {
        if ATOMIC_INPUT.contains(&name) || SESSION_TOOLS.contains(&name) {
            return (false, NO_SESSION);
        }
        if matches!(
            name,
            "csp_validate_path"
                | "csp_list_assets"
                | "csp_file_info"
                | "csp_scan_exports"
                | "csp_wait_for_file"
                | "csp_open_document"
                | "csp_list_commands"
                | "csp_list_auto_actions"
        ) && self.config.is_none()
        {
            return (false, "set CSP_MCP_CONFIG or pass --config");
        }
        (true, "ready")
    }

    fn preflight(&self, args: &Value) -> Result<Value, String> {
        let operation = req_str(args, "operation")?;
        if operation != "input" {
            return Err("operation must be input".into());
        }
        let identifier = req_str(args, "identifier")?;
        if !ATOMIC_INPUT.contains(&identifier) {
            return Err("identifier is not an atomic input tool".into());
        }
        let parameters_json = req_str(args, "parameters_json")?;
        let params: Value = serde_json::from_str(parameters_json)
            .map_err(|err| format!("parameters_json: {err}"))?;
        let token = self
            .tokens
            .lock()
            .map_err(|_| "token store poisoned".to_string())?
            .issue(identifier, &params);
        Ok(json!({
            "confirmToken": token,
            "expires_in_secs": 45,
            "focused": false,
            "identifier": identifier,
        }))
    }

    fn consume(&self, name: &str, args: &Value) -> Result<(), String> {
        let token = req_str(args, "confirmToken")?;
        let params = business_params(args)?;
        self.tokens
            .lock()
            .map_err(|_| "token store poisoned".to_string())?
            .take(token, name, &params)
    }

    fn names(&self, commands: bool) -> Result<Value, String> {
        let config = self
            .config
            .as_ref()
            .ok_or("set CSP_MCP_CONFIG or pass --config")?;
        let values = if commands {
            &config.commands
        } else {
            &config.auto_actions
        };
        Ok(json!({ "ids": values }))
    }
}

impl ToolProvider for ClipStudio {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-clipstudio".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        tools::schemas()
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        let result = self.dispatch(name, arguments);
        match result {
            Ok(value) => ToolResult::Ok(value.to_string()),
            Err(err) => ToolResult::Err(err),
        }
    }
}

impl ClipStudio {
    fn dispatch(&self, name: &str, arguments: &Value) -> Result<Value, String> {
        if ATOMIC_INPUT.contains(&name) {
            self.consume(name, arguments)?;
            return Err(NO_SESSION.into());
        }
        if SESSION_TOOLS.contains(&name) {
            return Err(NO_SESSION.into());
        }
        let config = self.config.as_ref();
        match name {
            "csp_health" => Ok(self.health()),
            "csp_capabilities" => Ok(self.capabilities()),
            "csp_list_commands" => self.names(true),
            "csp_list_auto_actions" => self.names(false),
            "csp_preflight" => self.preflight(arguments),
            "csp_validate_path" => files::validate(need(config)?, arguments),
            "csp_list_assets" => files::list_assets(need(config)?, arguments),
            "csp_file_info" => files::file_info(need(config)?, arguments),
            "csp_scan_exports" => files::scan_exports(need(config)?, arguments),
            "csp_wait_for_file" => files::wait_for_file(need(config)?, arguments),
            "csp_open_document" => files::open_document(need(config)?, arguments),
            _ => Err(format!("unknown tool {name}")),
        }
    }
}

fn need(config: Option<&Config>) -> Result<&Config, String> {
    config.ok_or_else(|| "set CSP_MCP_CONFIG or pass --config".into())
}

fn req_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{key} is required"))
}

pub fn run(args: &[String]) {
    let mut config_path = std::env::var_os("CSP_MCP_CONFIG").map(PathBuf::from);
    let mut workspace = std::env::var("CA_MCP_WORKSPACE").ok();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                config_path = args.get(i).map(PathBuf::from);
            }
            "--workspace" => {
                i += 1;
                workspace = args.get(i).cloned();
            }
            other => eprintln!("ignoring unknown argument {other}"),
        }
        i += 1;
    }
    let config = match config_path {
        Some(path) => match load_config(&path) {
            Ok(config) => Some(config),
            Err(err) => {
                eprintln!("clipstudio config: {err}");
                None
            }
        },
        None => None,
    };
    let provider = ClipStudio::new(config);
    McpServer::new(Arc::new(MemoryProvider::new(
        provider,
        MemoryTools::new("clipstudio", workspace),
    )))
    .run();
}

#[cfg(test)]
mod tests;
