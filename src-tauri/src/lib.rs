mod agents;
mod file_tools;
mod llm_client;

use agents::{AgentConfig, AgentSystem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

struct AppState {
    agents: Mutex<Option<AgentSystem>>,
    cancellation_token: Mutex<Option<Arc<AtomicBool>>>,
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
    window: tauri::Window,
) -> Result<String, String> {
    let token = Arc::new(AtomicBool::new(false));

    // Store token in state
    {
        let mut cancel_guard = state.cancellation_token.lock().unwrap();
        *cancel_guard = Some(token.clone());
    }

    let system = AgentSystem::new(config);
    let result = system.run_task(&task, &window, token).await?;

    let mut state_agents = state.agents.lock().unwrap();
    *state_agents = Some(system);

    Ok(result)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            agents: Mutex::new(None),
            cancellation_token: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            run_agent_task,
            cancel_task,
            get_agent_resources,
            get_resource_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
