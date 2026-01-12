mod llm_client;
mod file_tools;
mod agents;

use agents::{AgentConfig, AgentSystem};
use tauri::State;
use std::sync::Mutex;

struct AppState {
    agents: Mutex<Option<AgentSystem>>,
}

#[tauri::command]
async fn run_agent_task(
    config: AgentConfig,
    task: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let system = AgentSystem::new(config);
    let result = system.run_task(&task).await?;
    
    let mut state_agents = state.agents.lock().unwrap();
    *state_agents = Some(system);
    
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            agents: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![run_agent_task])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
