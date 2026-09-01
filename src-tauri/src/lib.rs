mod agent;
mod client;
mod commands;
mod core;
mod harness;
mod pty;
mod server;
mod tray;

use agent::{AgentConfig, AgentSystem};
use core::agent_resources::AgentResources;
use server::tcp_server::TcpServer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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
            // Embedded PTY terminals (in-app "Resume in terminal")
            pty::pty_spawn,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_kill,
            pty::pty_session_status,
            // Shared hub (`hub`) — same store as the `ca` CLI
            commands::commands::store::hub_init,
            commands::commands::store::hub_get_data_dir,
            commands::commands::messaging::hub_data_dir,
            commands::commands::store::hub_list_agents,
            commands::commands::store::hub_upsert_agent_card,
            commands::commands::memory::hub_write_memory,
            commands::commands::memory::hub_update_memory,
            commands::commands::memory::hub_list_memories,
            commands::commands::memory::hub_search_memories,
            commands::commands::memory::hub_mark_memory_stale,
            commands::commands::memory::hub_delete_memory,
            commands::commands::memory::hub_promote_memory,
            commands::commands::memory::hub_compact_short_term,
            commands::commands::memory::hub_link_memories,
            commands::commands::memory::hub_unlink_memories,
            commands::commands::memory::hub_list_memory_links,
            commands::commands::memory::hub_related_memories,
            commands::commands::memory::hub_memories_for_topic,
            commands::commands::memory::hub_suggest_links_for_memory,
            commands::commands::memory::hub_apply_link_suggestions,
            commands::commands::messaging::hub_send_message,
            commands::commands::messaging::hub_send_session_message,
            commands::commands::messaging::hub_send_tagged_message,
            commands::commands::messaging::hub_list_tagged_send_outcomes,
            commands::commands::messaging::hub_poll_messages,
            commands::commands::messaging::hub_list_messages,
            commands::commands::messaging::hub_mark_read,
            commands::commands::messaging::hub_list_read_markers,
            commands::commands::messaging::hub_list_channels,
            commands::commands::messaging::hub_create_channel,
            commands::commands::messaging::hub_delete_channel,
            commands::commands::messaging::hub_list_channel_messages,
            commands::commands::messaging::hub_list_message_memories,
            commands::commands::messaging::hub_request_wake,
            commands::commands::messaging::hub_request_team_wakes,
            commands::commands::messaging::hub_list_team_members,
            commands::commands::messaging::hub_set_team_member,
            commands::commands::messaging::hub_create_work_session,
            commands::commands::messaging::hub_list_work_sessions,
            commands::commands::messaging::hub_add_work_session_member,
            harness::commands::hub_start_harness,
            // Moved to the relaunch submodule when commands/ split (#158);
            // the tauri macro-generated __cmd__ items live there too.
            harness::commands::relaunch::hub_start_managed_harness,
            harness::commands::relaunch::hub_relaunch_harness_in_terminal,
            harness::commands::relaunch::hub_relaunch_harness_embedded,
            harness::stop::hub_stop_managed_harness,
            harness::commands::hub_inject_harness,
            harness::commands::hub_register_harness_session,
            harness::commands::hub_list_harness_sessions,
            harness::presence::hub_workspace_agent_presence,
            harness::commands::hub_register_managed_harness_session,
            harness::commands::hub_record_harness_capture,
            harness::capture_commands::hub_capture_claude_session,
            harness::capture_commands::hub_capture_codex_session,
            harness::capture_commands::hub_capture_gemini_session,
            harness::capture_commands::hub_capture_grok_session,
            harness::commands::claude_channel_list_workspaces,
            harness::commands::claude_channel_rename_workspace,
            harness::commands::claude_channel_delete_workspace,
            harness::commands::claude_channel_is_connected,
            harness::commands::claude_channel_connect,
            harness::commands::hub_grok_leader_status,
            harness::commands::hub_grok_list_live_sessions,
            harness::commands::hub_grok_connect,
            commands::commands::messaging::hub_list_wakes,
            commands::commands::messaging::hub_export_markdown,
            commands::commands::messaging::hub_export_markdown_git,
            commands::commands::messaging::hub_append_journal,
            commands::commands::messaging::hub_purge_stale_memories,
            commands::commands::messaging::hub_age_out_short_term,
            commands::commands::messaging::hub_set_message_status,
            commands::commands::messaging::hub_update_message,
            commands::commands::messaging::hub_delete_message,
            commands::commands::messaging::hub_resolve_wake,
            commands::commands::messaging::hub_list_audit_events,
            commands::commands::messaging::hub_approve_audit,
            commands::commands::messaging::hub_quarantine_audit,
            commands::commands::messaging::hub_get_wake_policy,
            commands::commands::messaging::hub_set_wake_policy,
            commands::commands::workflow::hub_create_task,
            commands::commands::workflow::hub_list_tasks,
            commands::commands::workflow::hub_get_task,
            commands::commands::workflow::hub_advance_task,
            commands::commands::workflow::hub_cancel_task,
            commands::commands::workflow::hub_complete_parallel_member,
            commands::commands::workflow::hub_retry_task,
            commands::commands::workflow::hub_set_agent_budget,
            commands::commands::workflow::hub_get_budget,
            commands::commands::workflow::hub_list_agent_metrics,
            commands::commands::workflow::hub_record_agent_metrics,
            commands::commands::quotas::hub_get_provider_quotas,
            commands::commands::quotas::hub_refresh_provider_quota,
            commands::commands::workflow::hub_record_budget_usage,
            commands::commands::workflow::hub_consume_budget,
            commands::commands::workflow::hub_resume_agent,
            commands::commands::workflow::hub_pause_for_budget,
            commands::commands::workflow::hub_record_shutdown,
            commands::commands::settings::settings_get_effective,
            commands::commands::settings::settings_get_load_status,
            commands::commands::settings::settings_update,
            commands::commands::settings::settings_reset_field,
            commands::commands::settings::settings_set_default_workspace,
            commands::commands::settings::settings_set_default_session,
            commands::commands::settings::settings_list_audit_events,
            commands::commands::settings::settings_list_profiles,
            commands::commands::settings::settings_upsert_profile,
            commands::commands::settings::settings_rename_profile,
            commands::commands::settings::settings_remove_profile,
            commands::commands::settings::settings_set_workspace_default_profile,
            commands::commands::settings::settings_reset_workspace_default_profile,
            commands::commands::settings::settings_list_harnesses,
            commands::commands::settings::settings_update_harness,
            commands::commands::settings::settings_update_orchestration,
            commands::commands::settings::settings_set_retention_days,
            commands::commands::settings::settings_get_standing_policy,
            commands::commands::settings::settings_set_confirm_wakes,
            commands::commands::settings::settings_set_allow_auto_wake,
            commands::commands::settings::settings_list_agent_budgets,
            commands::commands::settings::settings_set_agent_budget,
            commands::commands::harness_models::settings_get_harness_model_options,
            commands::commands::harness_models::settings_get_all_harness_options,
            commands::commands::harness_models::settings_set_harness_model,
            commands::commands::harness_models::settings_set_harness_effort,
            commands::commands::harness_models::settings_set_workspace_harness_model,
            commands::commands::harness_models::settings_reset_workspace_harness_model,
            commands::commands::harness_models::settings_set_workspace_harness_effort,
            commands::commands::harness_models::settings_reset_workspace_harness_effort,
            commands::commands::creative_tools::creative_tools_status,
            commands::commands::creative_tools::creative_tools_set_enabled,
            commands::commands::creative_tools::creative_tools_reapply,
            commands::commands::creative_tools::creative_tools_codex_snippet,
            commands::commands::roles::hub_upsert_role,
            commands::commands::roles::hub_get_role,
            commands::commands::roles::hub_list_roles,
            commands::commands::roles::hub_delete_role,
            commands::commands::roles::hub_assign_agent_role,
            commands::commands::roles::hub_unassign_agent_role,
            commands::commands::roles::hub_list_agent_roles,
            commands::commands::roles::hub_effective_agent_permissions,
            commands::commands::roles::hub_gate_quota_used_today,
            commands::commands::roles::hub_set_role_provider_default,
            commands::commands::roles::hub_list_role_provider_defaults,
            commands::commands::roles::hub_list_pending_gate_approvals,
            commands::commands::roles::hub_resolve_gate_approval,
            commands::commands::attachments::hub_save_attachment,
            commands::commands::attachments::hub_get_attachment,
            commands::commands::avatar::hub_set_agent_avatar,
            commands::commands::avatar::hub_clear_agent_avatar,
            commands::commands::avatar::hub_read_avatar_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
