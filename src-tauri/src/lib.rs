mod agent;
mod client;
mod commands;
mod core;
mod harness;
mod invoke;
mod pty;
mod server;
mod tray;

use agent::{AgentConfig, AgentSystem};
use core::agent_resources::AgentResources;
use server::tcp_server::TcpServer;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Manager, State};
use tokio::sync::mpsc;

struct AppState {
    agents: Mutex<Option<AgentSystem>>,
    cancellation_token: Mutex<Option<Arc<AtomicBool>>>,
    user_input_tx: Mutex<Option<mpsc::Sender<String>>>,
    tcp_server: Mutex<Option<TcpServer>>,
}

#[tauri::command]
async fn run_agent_task(
    config: AgentConfig,
    task: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let token = Arc::new(AtomicBool::new(false));

    let (input_tx, input_rx) = mpsc::channel(1);

    *state.cancellation_token.lock().unwrap() = Some(token.clone());
    *state.user_input_tx.lock().unwrap() = Some(input_tx);

    let system = AgentSystem::new(config);
    // run_task will now consume input_rx
    let result = system.run_task(&task, &app_handle, token, input_rx).await?;

    let mut state_agents = state.agents.lock().unwrap();
    *state_agents = Some(system);

    Ok(result)
}

#[tauri::command]
async fn submit_user_input(state: State<'_, AppState>, input: String) -> Result<(), String> {
    let tx = {
        let tx_guard = state.user_input_tx.lock().unwrap();
        tx_guard.clone()
    };

    if let Some(tx) = tx {
        tx.send(input).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No active agent waiting for input".to_string())
    }
}

#[tauri::command]
fn cancel_task(state: State<'_, AppState>) -> Result<(), String> {
    let token_guard = state.cancellation_token.lock().unwrap();
    if let Some(token) = token_guard.as_ref() {
        token.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
async fn get_agent_resources(work_dir: String) -> Result<AgentResources, String> {
    Ok(core::agent_resources::list_agent_resources(&work_dir))
}

/// `ps` process-table scan — offload so Orchestrate discovery does not
/// freeze the window while the table is read (#163).
#[tauri::command]
async fn detect_agent_processes() -> Result<Vec<core::process_detector::DetectedProcess>, String> {
    harness::blocking::run_blocking("detect_agent_processes", || {
        core::process_detector::detect_agent_processes()
    })
    .await
}

#[tauri::command]
async fn get_resource_content(work_dir: String, path: String) -> Result<String, String> {
    // path is the full relative path from work_dir, e.g. ".agent/prompts/test_planner.md"
    let full_path = std::path::Path::new(&work_dir).join(&path);

    // Security check: ensure the resolved path starts with .agent
    if !path.starts_with(".agent") {
        return Err("Invalid path: must be within .agent directory".to_string());
    }

    tokio::fs::read_to_string(full_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_file_absolute(path: String) -> Result<String, String> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn bootstrap_workspace(work_dir: String, create_dir: Option<bool>) -> Result<(), String> {
    let trimmed = work_dir.trim();
    if trimmed.is_empty() {
        return Err("Workspace path cannot be empty".to_string());
    }

    let work_path = std::path::Path::new(trimmed);
    if !work_path.is_absolute() {
        return Err("Workspace root must be an absolute path".to_string());
    }

    if !work_path.exists() {
        if create_dir != Some(true) {
            return Err(format!("Workspace directory '{}' does not exist", trimmed));
        }
        tokio::fs::create_dir_all(work_path)
            .await
            .map_err(|e| format!("Failed to create workspace directory: {}", e))?;
    } else if !work_path.is_dir() {
        return Err(format!("Workspace path '{}' is not a directory", trimmed));
    }

    let base = work_path.join(".agent");
    if base.exists() {
        return Err("Workspace is already bootstrapped (.agent directory exists)".to_string());
    }

    tokio::fs::create_dir_all(base.join("rules"))
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(base.join("prompts"))
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(base.join("workflows"))
        .await
        .map_err(|e| e.to_string())?;

    let mcp_config = r#"{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    }
  }
}"#;

    tokio::fs::write(base.join("mcp_config.json"), mcp_config)
        .await
        .map_err(|e| e.to_string())?;

    let agents_md = r#"# AGENTS.md

Welcome to your new Coding Assistants workspace!
Place your instructions in this file or under the `rules/` directory.
"#;

    tokio::fs::write(base.join("AGENTS.md"), agents_md)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_available_models() -> Result<HashMap<String, Vec<String>>, String> {
    // Determine if we have an active agent system or need to create a temporary one (or just use a temporary LLMClient)
    // Since LLMClient::new() is cheap, we can just create one.
    // But list_models is on LLMClient.
    // Accessing state.agents might be empty if no task ran yet.
    // Better: LLMClient::new().list_models().await

    let client = crate::client::llm::LLMClient::new();
    let models_list = client.list_models().await?;
    let mut models_map: HashMap<String, Vec<String>> = HashMap::new();

    for model_line in models_list {
        if let Some((provider, model)) = model_line.split_once('/') {
            models_map
                .entry(provider.to_lowercase())
                .or_default()
                .push(model.to_string());
        } else {
            models_map
                .entry("opencode".to_string())
                .or_default()
                .push(model_line);
        }
    }
    Ok(models_map)
}

#[tauri::command]
async fn start_tcp_server(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let mut server = TcpServer::new(app_handle.clone(), 5555);
    let address = server.start().await?;

    // Start accepting connections in background
    server.accept_connections().await?;

    *state.tcp_server.lock().unwrap() = Some(server);
    Ok(address)
}

#[tauri::command]
async fn stop_tcp_server(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(mut server) = state.tcp_server.lock().unwrap().take() {
        server.stop();
    }
    Ok(())
}

#[tauri::command]
async fn get_server_ip() -> Result<String, String> {
    use std::net::UdpSocket;

    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.connect("8.8.8.8:80").map_err(|e| e.to_string())?;
    let local_addr = socket.local_addr().map_err(|e| e.to_string())?;
    Ok(local_addr.ip().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK 2.48+ enables DMA-BUF buffer sharing by default on Wayland.
    // On NVIDIA + Wayland this causes a GPU pipeline stall on every frame
    // transfer when the window surface is large (e.g. maximized): the DMA-BUF
    // import blocks the WebKit render thread until the NVIDIA driver flushes its
    // command queue, producing severe scroll jank. Browsers (Chrome, Firefox)
    // avoid this by running their own GPU process. Disabling DMA-BUF falls back
    // to the SHM path, which is non-blocking and has no visual quality impact.
    // Must be set before any WebView is created — process-level env var is the
    // only reliable way to pass it to WebKitGTK's internal renderer process.
    // See: https://bugs.webkit.org/show_bug.cgi?id=261874
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            // Only set if not already overridden by the caller.
            unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            agents: Mutex::new(None),
            cancellation_token: Mutex::new(None),
            user_input_tx: Mutex::new(None),
            tcp_server: Mutex::new(None),
        })
        .manage(pty::PtySessions::default())
        .setup(tray::setup_tray)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Only the main window is tray-resident (hide instead of
                // exit). Settings is a plain utility dialog: let it actually
                // close so its label frees up and `openSettingsWindow`'s
                // getByLabel/create dance doesn't depend on a hidden window
                // resurrecting correctly on reuse.
                if window.label() == "main" {
                    if let Some(settings) = window.app_handle().get_webview_window("settings") {
                        let _ = settings.close();
                    }
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(invoke::invoke_handler!())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
