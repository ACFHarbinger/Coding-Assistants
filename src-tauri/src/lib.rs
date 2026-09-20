mod agent;
mod bus;
mod client;
mod commands;
mod core;
mod harness;
mod invoke;
mod pty;
mod server;
mod tray;

use agent::{
    is_valid_task_id, next_task_id, remove_task_mcp_dir, AgentConfig, AgentSystem, TaskHandles,
    TaskLifecycleEvent, TaskRegistry,
};
use core::agent_resources::AgentResources;
use hub::bus::TOPIC_TASK_LIFECYCLE;
use hub::InProcessBus;
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
    /// Per-task handles keyed by task id (P2 / #331): concurrent tasks no
    /// longer share one cancellation flag or input channel.
    tasks: TaskRegistry,
    tcp_server: Mutex<Option<TcpServer>>,
}

/// Outcome of `run_agent_task`: the caller routes later `submit_user_input`
/// / `cancel_task` calls with `task_id`, and follows progress on the
/// `task-lifecycle` bus topic.
#[derive(serde::Serialize)]
struct RunTaskOutcome {
    task_id: String,
    result: String,
}

#[tauri::command]
async fn run_agent_task(
    config: AgentConfig,
    task: String,
    task_id: Option<String>,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    bus: State<'_, InProcessBus>,
) -> Result<RunTaskOutcome, String> {
    let task_id = match task_id.map(|id| id.trim().to_string()) {
        Some(id) if !id.is_empty() => {
            if !is_valid_task_id(&id) {
                return Err("task_id must match [A-Za-z0-9_-]+".into());
            }
            id
        }
        _ => next_task_id(),
    };
    if hub::sync::is_held(&hub::default_hub_home()) {
        return Err(hub::sync::LOCKED_MESSAGE.to_string());
    }
    let token = Arc::new(AtomicBool::new(false));
    let (input_tx, input_rx) = mpsc::channel(1);
    if !state.tasks.register(
        &task_id,
        TaskHandles {
            cancellation: token.clone(),
            input_tx,
        },
    ) {
        return Err(format!("task_id '{task_id}' is already active"));
    }

    let system = AgentSystem::new(config);
    let bus_handle = bus.inner().clone();
    bus_handle.emit(
        TOPIC_TASK_LIFECYCLE,
        TaskLifecycleEvent::new(&task_id, "started", None),
    );
    let outcome = system
        .run_task(&task_id, &task, &app_handle, &bus_handle, token, input_rx)
        .await;
    state.tasks.remove(&task_id);
    remove_task_mcp_dir(&hub::default_hub_home(), &task_id);

    let mut state_agents = state.agents.lock().unwrap();
    *state_agents = Some(system);

    match outcome {
        Ok(result) => {
            bus_handle.emit(
                TOPIC_TASK_LIFECYCLE,
                TaskLifecycleEvent::new(&task_id, "finished", None),
            );
            Ok(RunTaskOutcome { task_id, result })
        }
        Err(error) => {
            bus_handle.emit(
                TOPIC_TASK_LIFECYCLE,
                TaskLifecycleEvent::new(&task_id, "failed", Some(error.clone())),
            );
            Err(error)
        }
    }
}

#[tauri::command]
async fn submit_user_input(
    state: State<'_, AppState>,
    task_id: String,
    input: String,
) -> Result<(), String> {
    let handles = state.tasks.get(&task_id).ok_or_else(|| {
        format!("No active task '{task_id}' waiting for input (it may have finished)")
    })?;
    handles
        .input_tx
        .send(input)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cancel_task(
    state: State<'_, AppState>,
    task_id: String,
    bus: State<'_, InProcessBus>,
) -> Result<(), String> {
    // Idempotent: an unknown or finished id is already in the desired state.
    if let Some(handles) = state.tasks.get(&task_id) {
        handles.cancellation.store(true, Ordering::SeqCst);
        bus.inner().clone().emit(
            TOPIC_TASK_LIFECYCLE,
            TaskLifecycleEvent::new(&task_id, "cancelled", None),
        );
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
    let expanded = core::workspace::expand_tilde(&path);
    tokio::fs::read_to_string(expanded)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn validate_workspace_path(
    path: String,
) -> Result<core::workspace::WorkspaceValidation, String> {
    let expanded = core::workspace::expand_tilde(&path);
    Ok(core::workspace::validate_workspace(
        &expanded.to_string_lossy(),
    ))
}

#[tauri::command]
async fn bootstrap_workspace(work_dir: String, create_dir: Option<bool>) -> Result<(), String> {
    let expanded = core::workspace::expand_tilde(&work_dir);
    core::workspace::bootstrap_workspace_core(&expanded.to_string_lossy(), create_dir).await
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
    bus: State<'_, InProcessBus>,
) -> Result<String, String> {
    let mut server = TcpServer::new(app_handle.clone(), bus.inner().clone(), 5555);
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
        .manage(InProcessBus::new())
        .manage(AppState {
            agents: Mutex::new(None),
            tasks: TaskRegistry::new(),
            tcp_server: Mutex::new(None),
        })
        .manage(pty::PtySessions::default())
        .setup(|app| {
            tray::setup_tray(app)?;
            let bus = app.state::<InProcessBus>().inner().clone();
            bus::spawn_tauri_forwarder(bus, app.handle().clone());
            Ok(())
        })
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
