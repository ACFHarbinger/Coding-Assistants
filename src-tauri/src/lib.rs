mod agents;
mod file_tools;
mod llm_client;
mod tcp_server;

use agents::{AgentConfig, AgentSystem};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State};
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

    std::fs::create_dir_all(&prompts_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&rules_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&workflows_dir).map_err(|e| e.to_string())?;

    let list_files = |dir: &std::path::Path, prefix: &str| -> Vec<String> {
        std::fs::read_dir(dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .map(|e| {
                        let filename = e.file_name().to_string_lossy().to_string();
                        format!("{}/{}", prefix, filename)
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    Ok(AgentResources {
        prompts: list_files(&prompts_dir, ".agent/prompts"),
        rules: list_files(&rules_dir, ".agent/rules"),
        workflows: list_files(&workflows_dir, ".agent/workflows"),
    })
}

#[tauri::command]
async fn get_resource_content(work_dir: String, path: String) -> Result<String, String> {
    // path is the full relative path from work_dir, e.g. ".agent/prompts/test_planner.md"
    let full_path = std::path::Path::new(&work_dir).join(&path);

    // Security check: ensure the resolved path starts with .agent
    if !path.starts_with(".agent") {
        return Err("Invalid path: must be within .agent directory".to_string());
    }

    std::fs::read_to_string(full_path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_file_absolute(path: String) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
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

// Legacy function - keeping for backward compatibility
#[tauri::command]
async fn start_remote_server(window: tauri::Window) -> Result<(), String> {
    use std::process::Command;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    tokio::spawn(async move {
        let listener = TcpListener::bind("0.0.0.0:5555").await.unwrap();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 1024];
            if let Ok(n) = socket.read(&mut buf).await {
                let msg = String::from_utf8_lossy(&buf[..n]);
                if msg.trim() == "launch" {
                    let output =
                        Command::new("/home/pkhunter/Repositories/Coding-Assistants/app/task.sh")
                            .output();

                    let status = match output {
                        Ok(o) => format!("Task executed: {}", String::from_utf8_lossy(&o.stdout)),
                        Err(e) => format!("Error: {}", e),
                    };
                    let _ = window.emit("remote-status", status);
                }
            }
        }
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            run_agent_task,
            cancel_task,
            submit_user_input,
            get_agent_resources,
            get_resource_content,
            read_file_absolute,
            get_available_models,
            start_remote_server,
            start_tcp_server,
            stop_tcp_server,
            get_server_ip
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
