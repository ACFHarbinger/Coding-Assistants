use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, oneshot};

use crate::agent::AgentConfig;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    GetModels,
    StartTask { config: AgentConfig, task: String },
    CancelTask,
    SubmitInput { input: String },
    GetStatus,
    GetPendingWakes,
    ResolveWake { wake_id: String, approve: bool },
    GetAgentCards,
    GetAgentResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerResponse {
    ModelsList {
        models: HashMap<String, Vec<String>>,
    },
    TaskStarted,
    TaskEvent {
        source: String,
        event_type: String,
        content: String,
    },
    TaskComplete {
        result: String,
    },
    Status {
        running: bool,
        message: String,
    },
    Error {
        message: String,
    },
    PendingWakesList {
        wakes: Vec<hub::WakeRecord>,
    },
    WakeResolved {
        wake_id: String,
    },
    AgentCardsList {
        cards: Vec<hub::AgentRecord>,
    },
    AgentResourcesList {
        work_dir: String,
        prompts: Vec<String>,
        rules: Vec<String>,
        workflows: Vec<String>,
        skills: Vec<String>,
    },
}

pub struct TcpServer {
    app_handle: AppHandle,
    port: u16,
    listener: Option<Arc<TcpListener>>,
    broadcast_tx: broadcast::Sender<ServerResponse>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TcpServer {
    pub fn new(app_handle: AppHandle, port: u16) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            app_handle,
            port,
            listener: None,
            broadcast_tx: tx,
            shutdown_tx: None,
        }
    }

    pub async fn start(&mut self) -> Result<String, String> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .map_err(|e| format!("Failed to bind TCP server: {}", e))?;

        let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        let address = format!("{}:{}", local_ip, self.port);

        self.listener = Some(Arc::new(listener));

        Ok(address)
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.listener = None;
    }

    pub async fn accept_connections(&mut self) -> Result<(), String> {
        let listener = self.listener.as_ref().ok_or("Server not started")?.clone();

        let app_handle = self.app_handle.clone();
        let broadcast_tx = self.broadcast_tx.clone();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        // Forward Tauri events to TCP clients
        let app_clone = app_handle.clone();
        let tx_clone = broadcast_tx.clone();
        tokio::spawn(async move {
            use tauri::Listener;
            app_clone.listen_any("agent-event", move |event| {
                if let Ok(agent_event) =
                    serde_json::from_str::<crate::agent::AgentEvent>(event.payload())
                {
                    let _ = tx_clone.send(ServerResponse::TaskEvent {
                        source: agent_event.source,
                        event_type: agent_event.event_type,
                        content: agent_event.content,
                    });
                }
            });
        });

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, addr)) => {
                                println!("New connection from: {}", addr);
                                let app = app_handle.clone();
                                let b_tx = broadcast_tx.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_client(stream, app, b_tx).await {
                                        eprintln!("Error handling client: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        println!("TCP Server shutting down...");
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

async fn handle_client(
    stream: TcpStream,
    app_handle: AppHandle,
    broadcast_tx: broadcast::Sender<ServerResponse>,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut broadcast_rx = broadcast_tx.subscribe();

    loop {
        tokio::select! {
            // Read from client
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            line.clear();
                            continue;
                        }

                        println!("Received: {}", trimmed);

                        let response = match serde_json::from_str::<ClientRequest>(trimmed) {
                            Ok(request) => handle_request(request, &app_handle).await,
                            Err(e) => ServerResponse::Error {
                                message: format!("Invalid request: {}", e),
                            },
                        };

                        let response_json = serde_json::to_string(&response).unwrap();
                        if let Err(e) = writer
                            .write_all(format!("{}\n", response_json).as_bytes())
                            .await
                        {
                            eprintln!("Failed to write response: {}", e);
                            break;
                        }

                        line.clear();
                    }
                }
            }
            // Read from broadcast
            Ok(response) = broadcast_rx.recv() => {
                let response_json = serde_json::to_string(&response).unwrap();
                if let Err(e) = writer
                    .write_all(format!("{}\n", response_json).as_bytes())
                    .await
                {
                    eprintln!("Failed to write broadcast: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn handle_request(request: ClientRequest, app_handle: &AppHandle) -> ServerResponse {
    match request {
        ClientRequest::GetModels => {
            let client = crate::client::llm::LLMClient::new();
            match client.list_models().await {
                Ok(models_list) => {
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
                    ServerResponse::ModelsList { models: models_map }
                }
                Err(e) => ServerResponse::Error {
                    message: format!("Failed to get models: {}", e),
                },
            }
        }
        ClientRequest::StartTask { config, task } => {
            // Emit task to frontend - the actual execution happens through Tauri commands
            // For now, we return success - the Android app will need to listen for events
            app_handle
                .emit(
                    "android-task-request",
                    serde_json::json!({"config": config, "task": task}),
                )
                .ok();
            ServerResponse::TaskStarted
        }
        ClientRequest::CancelTask => {
            app_handle.emit("android-cancel-request", ()).ok();
            ServerResponse::Status {
                running: false,
                message: "Cancel request sent".to_string(),
            }
        }
        ClientRequest::SubmitInput { input } => {
            app_handle
                .emit("android-input", serde_json::json!({ "input": input }))
                .ok();
            ServerResponse::Status {
                running: true,
                message: "Input submitted".to_string(),
            }
        }
        ClientRequest::GetStatus => ServerResponse::Status {
            running: true,
            message: "Connected".to_string(),
        },
        ClientRequest::GetPendingWakes => match crate::commands::commands::store::open_store() {
            Ok(store) => match store.list_wakes(None, true) {
                Ok(wakes) => ServerResponse::PendingWakesList { wakes },
                Err(e) => ServerResponse::Error {
                    message: e.to_string(),
                },
            },
            Err(e) => ServerResponse::Error {
                message: e.to_string(),
            },
        },
        ClientRequest::ResolveWake { wake_id, approve } => {
            match crate::commands::commands::store::open_store() {
                Ok(store) => {
                    let status = if approve {
                        hub::WakeStatus::Delivered
                    } else {
                        hub::WakeStatus::Cancelled
                    };
                    match store.set_wake_status(&wake_id, status) {
                        Ok(_) => ServerResponse::WakeResolved { wake_id },
                        Err(e) => ServerResponse::Error {
                            message: e.to_string(),
                        },
                    }
                }
                Err(e) => ServerResponse::Error {
                    message: e.to_string(),
                },
            }
        }
        ClientRequest::GetAgentCards => match crate::commands::commands::store::open_store() {
            Ok(store) => match store.list_agents() {
                Ok(cards) => ServerResponse::AgentCardsList { cards },
                Err(e) => ServerResponse::Error {
                    message: e.to_string(),
                },
            },
            Err(e) => ServerResponse::Error {
                message: e.to_string(),
            },
        },
        ClientRequest::GetAgentResources => {
            let work_dir = crate::core::agent_resources::resolve_desktop_workspace();
            let resources = crate::core::agent_resources::list_agent_resources(&work_dir);
            ServerResponse::AgentResourcesList {
                work_dir,
                prompts: resources.prompts,
                rules: resources.rules,
                workflows: resources.workflows,
                skills: resources.skills,
            }
        }
    }
}

fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;

    // Connect to a public DNS to determine local IP
    // This doesn't actually send data
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}
