use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, oneshot};

use crate::agent::AgentConfig;
use crate::server::tcp_auth::{self, RejectReason};
use hub::bus::{EventBus, InProcessBus, TOPIC_AGENT_EVENT};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    Authenticate { token: String },
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
    bus: InProcessBus,
    port: u16,
    listener: Option<Arc<TcpListener>>,
    broadcast_tx: broadcast::Sender<ServerResponse>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TcpServer {
    pub fn new(app_handle: AppHandle, bus: InProcessBus, port: u16) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            app_handle,
            bus,
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

        // Forward hub bus agent-events to TCP clients (Tauri is another subscriber).
        let rx = self.bus.subscribe();
        let tx_clone = broadcast_tx.clone();
        std::thread::Builder::new()
            .name("ca-bus-tcp".into())
            .spawn(move || {
                while let Ok(event) = rx.recv() {
                    if event.topic != TOPIC_AGENT_EVENT {
                        continue;
                    }
                    if let Ok(agent_event) =
                        serde_json::from_value::<crate::agent::AgentEvent>(event.payload)
                    {
                        let _ = tx_clone.send(ServerResponse::TaskEvent {
                            source: agent_event.source,
                            event_type: agent_event.event_type,
                            content: agent_event.content,
                        });
                    }
                }
            })
            .expect("ca-bus-tcp forwarder thread");

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
                                    if let Err(e) = handle_client(stream, addr, app, b_tx).await {
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
    peer: SocketAddr,
    app_handle: AppHandle,
    broadcast_tx: broadcast::Sender<ServerResponse>,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut broadcast_rx = broadcast_tx.subscribe();
    let mut authenticated = false;

    loop {
        tokio::select! {
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            line.clear();
                            continue;
                        }

                        let (response, grant_auth, drop_after) =
                            dispatch_client_line(trimmed, peer, authenticated, &app_handle).await;
                        if grant_auth {
                            authenticated = true;
                        }

                        let response_json = serde_json::to_string(&response).unwrap();
                        if let Err(e) = writer
                            .write_all(format!("{}\n", response_json).as_bytes())
                            .await
                        {
                            eprintln!("Failed to write response: {}", e);
                            break;
                        }

                        line.clear();
                        if drop_after {
                            break;
                        }
                    }
                }
            }
            Ok(response) = broadcast_rx.recv() => {
                if !authenticated {
                    continue;
                }
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

async fn dispatch_client_line(
    trimmed: &str,
    peer: SocketAddr,
    authenticated: bool,
    app_handle: &AppHandle,
) -> (ServerResponse, bool, bool) {
    let request = match serde_json::from_str::<ClientRequest>(trimmed) {
        Ok(request) => request,
        Err(_) if !authenticated => {
            return reject_client(peer, RejectReason::UnauthenticatedCommand);
        }
        Err(error) => {
            return (
                ServerResponse::Error {
                    message: format!("Invalid request: {error}"),
                },
                false,
                false,
            );
        }
    };

    match request {
        ClientRequest::Authenticate { token } => {
            let expected = hub::secret::resolve(tcp_auth::TOKEN_VAULT_KEY);
            match tcp_auth::authorize(
                expected.as_ref().map(|secret| secret.expose()),
                Some(&token),
            ) {
                Ok(()) => (
                    ServerResponse::Status {
                        running: true,
                        message: "Authenticated".into(),
                    },
                    true,
                    false,
                ),
                Err(reason) => reject_client(peer, reason),
            }
        }
        _ if !authenticated => reject_client(peer, RejectReason::UnauthenticatedCommand),
        other => (handle_request(other, app_handle).await, false, false),
    }
}

fn reject_client(peer: SocketAddr, reason: RejectReason) -> (ServerResponse, bool, bool) {
    tcp_auth::record_reject(&peer.to_string(), reason);
    (
        ServerResponse::Error {
            message: "authentication required".into(),
        },
        false,
        true,
    )
}

async fn handle_request(request: ClientRequest, app_handle: &AppHandle) -> ServerResponse {
    match request {
        ClientRequest::Authenticate { .. } => ServerResponse::Error {
            message: "authentication required".into(),
        },
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
        ClientRequest::StartTask { mut config, task } => {
            // Kept for existing listeners: the Android app shows "Task Started..."
            // off TaskStarted + the TaskEvent stream.
            app_handle
                .emit(
                    "android-task-request",
                    serde_json::json!({"config": config, "task": task}),
                )
                .ok();
            // P11a / #334: paired peers get headless execution, not just a GUI
            // request. The task runs in the peer's own workspace (never the
            // caller-supplied path) under P2 per-task isolation, and the
            // result returns as TaskComplete. Auth already gated pre-dispatch.
            config.work_dir = crate::core::agent_resources::resolve_desktop_workspace();
            let state = app_handle.state::<crate::AppState>();
            let bus = app_handle.state::<hub::InProcessBus>();
            match crate::run_agent_task(config, task, None, state, app_handle.clone(), bus).await {
                Ok(outcome) => ServerResponse::TaskComplete {
                    result: outcome.result,
                },
                Err(error) => ServerResponse::Error { message: error },
            }
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
