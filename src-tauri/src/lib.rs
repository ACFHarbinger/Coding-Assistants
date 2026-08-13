mod agents;
mod file_tools;
mod harness_c12;
mod harness_claude;
mod harness_cmds;
mod harness_codex;
mod harness_gemini;
mod harness_grok;
mod hub_cmds;
mod llm_client;
mod process_detector;
mod tcp_server;

use agents::{AgentConfig, AgentSystem};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};
use tcp_server::TcpServer;
use tokio::sync::mpsc;

struct AppState {
    agents: Mutex<Option<AgentSystem>>,
    cancellation_token: Mutex<Option<Arc<AtomicBool>>>,
    user_input_tx: Mutex<Option<mpsc::Sender<String>>>,
    tcp_server: Mutex<Option<TcpServer>>,
}

#[derive(serde::Serialize)]
struct AgentResources {
    prompts: Vec<String>,
    rules: Vec<String>,
    workflows: Vec<String>,
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

    // Store token and input_tx in state
    {
        let mut cancel_guard = state.cancellation_token.lock().unwrap();
        *cancel_guard = Some(token.clone());
        let mut input_guard = state.user_input_tx.lock().unwrap();
        *input_guard = Some(input_tx);
    }

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
    // Note: tools variable is unused in this logic but keeping structure for potential future use or removing if completely unneeded.
    // For now we just scan directories.

    let base_path = std::path::Path::new(&work_dir).join(".agent");
    let prompts_dir = base_path.join("prompts");
    let rules_dir = base_path.join("rules");
    let workflows_dir = base_path.join("workflows");

    async fn list_files(dir: &std::path::Path, prefix: &str) -> Vec<String> {
        let mut out = Vec::new();
        let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
            return out;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().is_file() {
                let filename = entry.file_name().to_string_lossy().to_string();
                out.push(format!("{}/{}", prefix, filename));
            }
        }
        out
    }

    Ok(AgentResources {
        prompts: list_files(&prompts_dir, ".agent/prompts").await,
        rules: list_files(&rules_dir, ".agent/rules").await,
        workflows: list_files(&workflows_dir, ".agent/workflows").await,
    })
}

#[tauri::command]
fn detect_agent_processes() -> Result<Vec<process_detector::DetectedProcess>, String> {
    process_detector::detect_agent_processes()
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
async fn bootstrap_workspace(work_dir: String) -> Result<(), String> {
    let base = std::path::Path::new(&work_dir).join(".agent");
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

    let client = crate::llm_client::LLMClient::new();
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

    // Store server instance in state
    {
        let mut server_guard = state.tcp_server.lock().unwrap();
        *server_guard = Some(server);
    }

    Ok(address)
}

#[tauri::command]
async fn stop_tcp_server(state: State<'_, AppState>) -> Result<(), String> {
    let mut server_guard = state.tcp_server.lock().unwrap();
    if let Some(mut server) = server_guard.take() {
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
        .setup(|app| {
            let quit_i = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let show_i = MenuItemBuilder::with_id("show", "Show").build(app)?;
            let menu = MenuBuilder::new(app).item(&show_i).item(&quit_i).build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            run_agent_task,
            cancel_task,
            submit_user_input,
            get_agent_resources,
            detect_agent_processes,
            get_resource_content,
            read_file_absolute,
            bootstrap_workspace,
            get_available_models,
            start_tcp_server,
            stop_tcp_server,
            get_server_ip,
            // Shared hub (ca-hub) — same store as the `ca` CLI
            hub_cmds::hub_init,
            hub_cmds::hub_data_dir,
            hub_cmds::hub_list_agents,
            hub_cmds::hub_upsert_agent_card,
            hub_cmds::hub_write_memory,
            hub_cmds::hub_update_memory,
            hub_cmds::hub_list_memories,
            hub_cmds::hub_search_memories,
            hub_cmds::hub_mark_memory_stale,
            hub_cmds::hub_delete_memory,
            hub_cmds::hub_promote_memory,
            hub_cmds::hub_compact_short_term,
            hub_cmds::hub_send_message,
            hub_cmds::hub_send_session_message,
            hub_cmds::hub_send_tagged_message,
            hub_cmds::hub_list_tagged_send_outcomes,
            hub_cmds::hub_poll_messages,
            hub_cmds::hub_list_messages,
            hub_cmds::hub_list_channel_messages,
            hub_cmds::hub_list_message_memories,
            hub_cmds::hub_request_wake,
            hub_cmds::hub_request_team_wakes,
            hub_cmds::hub_list_team_members,
            hub_cmds::hub_set_team_member,
            hub_cmds::hub_create_work_session,
            hub_cmds::hub_list_work_sessions,
            hub_cmds::hub_add_work_session_member,
            harness_cmds::hub_start_harness,
            harness_cmds::hub_inject_harness,
            harness_cmds::hub_record_harness_capture,
            harness_cmds::hub_capture_claude_session,
            harness_cmds::hub_capture_codex_session,
            harness_cmds::hub_capture_gemini_session,
            harness_cmds::hub_capture_grok_session,
            hub_cmds::hub_list_wakes,
            hub_cmds::hub_export_markdown,
            hub_cmds::hub_export_markdown_git,
            hub_cmds::hub_append_journal,
            hub_cmds::hub_purge_stale_memories,
            hub_cmds::hub_age_out_short_term,
            hub_cmds::hub_set_message_status,
            hub_cmds::hub_update_message,
            hub_cmds::hub_delete_message,
            hub_cmds::hub_resolve_wake,
            hub_cmds::hub_list_audit_events,
            hub_cmds::hub_approve_audit,
            hub_cmds::hub_quarantine_audit,
            hub_cmds::hub_get_wake_policy,
            hub_cmds::hub_set_wake_policy,
            hub_cmds::hub_create_task,
            hub_cmds::hub_list_tasks,
            hub_cmds::hub_get_task,
            hub_cmds::hub_advance_task,
            hub_cmds::hub_cancel_task,
            hub_cmds::hub_complete_parallel_member,
            hub_cmds::hub_retry_task,
            hub_cmds::hub_set_agent_budget,
            hub_cmds::hub_get_budget,
            hub_cmds::hub_list_agent_metrics,
            hub_cmds::hub_record_agent_metrics,
            hub_cmds::hub_get_provider_quotas,
            hub_cmds::hub_refresh_provider_quota,
            hub_cmds::hub_record_budget_usage,
            hub_cmds::hub_consume_budget,
            hub_cmds::hub_resume_agent,
            hub_cmds::hub_pause_for_budget,
            hub_cmds::hub_record_shutdown,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
