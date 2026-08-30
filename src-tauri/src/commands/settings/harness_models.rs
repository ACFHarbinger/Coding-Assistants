//! Dynamic model discovery, caching, and model/effort configuration per harness.

use hub::{EffectiveHarnessSettings, SettingsStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessModelCatalog {
    pub harness: String,
    pub models: Vec<String>,
    pub effort_options: Vec<String>,
}

/// Process-local cache for the optional CLI probes. Opening the Settings tab
/// must remain fast when a provider CLI is absent, unauthenticated, or slow.
/// `refresh: true` deliberately bypasses this cache.
static CATALOG_CACHE: OnceLock<RwLock<BTreeMap<String, HarnessModelCatalog>>> = OnceLock::new();

fn catalog_cache() -> &'static RwLock<BTreeMap<String, HarnessModelCatalog>> {
    CATALOG_CACHE.get_or_init(|| RwLock::new(BTreeMap::new()))
}

fn open_settings_store() -> SettingsStore {
    SettingsStore::open(hub::default_hub_home())
}

fn record_settings_audit(field: &str, scope: &str, action: &str) -> Result<(), String> {
    crate::commands::commands::store::open_store()?
        .record_settings_audit_event(field, scope, action)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn fallback_models_for_harness(harness: &str) -> Vec<String> {
    match harness {
        "opencode" => vec![
            "opencode-go/glm-5.3".into(),
            "opencode-go/glm-4-plus".into(),
            "anthropic/claude-3-7-sonnet".into(),
            "openai/gpt-4o".into(),
        ],
        "deepseek" => vec![
            "deepseek/deepseek-v4-flash".into(),
            "deepseek/deepseek-chat".into(),
            "deepseek/deepseek-reasoner".into(),
        ],
        "claude" => vec![
            "claude-3-7-sonnet-20250219".into(),
            "claude-3-5-sonnet-20241022".into(),
            "claude-3-5-haiku-20241022".into(),
            "claude-3-opus-20240229".into(),
        ],
        "chat" | "codex" => vec![
            "gpt-4o".into(),
            "gpt-4o-mini".into(),
            "o1".into(),
            "o3-mini".into(),
            "gpt-4-turbo".into(),
        ],
        "gemini" => vec![
            "gemini-3.7-flash-medium".into(),
            "gemini-3.7-flash-high".into(),
            "gemini-2.5-flash".into(),
            "gemini-2.5-pro".into(),
            "gemini-1.5-pro".into(),
            "gemini-1.5-flash".into(),
        ],
        "grok" => vec![
            "grok-4.6".into(),
            "grok-4".into(),
            "grok-3".into(),
            "grok-3-mini".into(),
            "grok-2".into(),
        ],
        "vibe" => vec![
            "mistral-medium-3.5".into(),
            "devstral-small".into(),
            "mistral-large-latest".into(),
            "codestral-latest".into(),
            "local".into(),
        ],
        _ => vec![],
    }
}

pub fn effort_options_for_harness(harness: &str) -> Vec<String> {
    match harness {
        "claude" => vec!["low".into(), "medium".into(), "high".into(), "max".into()],
        "chat" | "codex" | "gemini" | "grok" | "opencode" | "deepseek" => {
            vec!["low".into(), "medium".into(), "high".into()]
        }
        "vibe" => vec![],
        _ => vec![],
    }
}

async fn query_cli_models(harness: &str) -> Option<Vec<String>> {
    let timeout_dur = Duration::from_millis(2500);
    match harness {
        "opencode" => {
            let fut = Command::new("opencode").arg("models").output();
            if let Ok(Ok(output)) = timeout(timeout_dur, fut).await {
                if output.status.success() {
                    let content = String::from_utf8_lossy(&output.stdout);
                    let list: Vec<String> =
                        crate::client::providers::parse_opencode_models(&content);
                    if !list.is_empty() {
                        return Some(list);
                    }
                }
            }
        }
        "deepseek" => {
            let fut = Command::new("opencode").arg("models").output();
            if let Ok(Ok(output)) = timeout(timeout_dur, fut).await {
                if output.status.success() {
                    let content = String::from_utf8_lossy(&output.stdout);
                    let list: Vec<String> =
                        crate::client::providers::parse_opencode_models(&content)
                            .into_iter()
                            .filter(|m| m.starts_with("deepseek/"))
                            .collect();
                    if !list.is_empty() {
                        return Some(list);
                    }
                }
            }
        }
        "gemini" => {
            let fut = Command::new("agy").arg("models").output();
            if let Ok(Ok(output)) = timeout(timeout_dur, fut).await {
                if output.status.success() {
                    let content = String::from_utf8_lossy(&output.stdout);
                    let list: Vec<String> = content
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(String::from)
                        .collect();
                    if !list.is_empty() {
                        return Some(list);
                    }
                }
            }
        }
        "grok" => {
            let fut = Command::new("grok").arg("models").output();
            if let Ok(Ok(output)) = timeout(timeout_dur, fut).await {
                if output.status.success() {
                    let content = String::from_utf8_lossy(&output.stdout);
                    let list: Vec<String> = content
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(String::from)
                        .collect();
                    if !list.is_empty() {
                        return Some(list);
                    }
                }
            }
        }
        _ => {}
    }
    None
}

pub async fn get_catalog_for_harness(harness: &str, refresh: bool) -> HarnessModelCatalog {
    if !refresh {
        if let Some(cached) = catalog_cache()
            .read()
            .expect("model catalog cache lock poisoned")
            .get(harness)
            .cloned()
        {
            return cached;
        }
    }

    let mut models = query_cli_models(harness).await.unwrap_or_default();
    let fallback = fallback_models_for_harness(harness);
    for fb in fallback {
        if !models.contains(&fb) {
            models.push(fb);
        }
    }
    let effort_options = effort_options_for_harness(harness);
    let catalog = HarnessModelCatalog {
        harness: harness.to_string(),
        models,
        effort_options,
    };
    catalog_cache()
        .write()
        .expect("model catalog cache lock poisoned")
        .insert(harness.to_string(), catalog.clone());
    catalog
}

#[tauri::command]
pub async fn settings_get_harness_model_options(
    harness: String,
    refresh: Option<bool>,
) -> Result<HarnessModelCatalog, String> {
    Ok(get_catalog_for_harness(&harness, refresh.unwrap_or(false)).await)
}

#[tauri::command]
pub async fn settings_get_all_harness_options(
    refresh: Option<bool>,
) -> Result<BTreeMap<String, HarnessModelCatalog>, String> {
    // Run independent probes together: a missing CLI then costs at most one
    // timeout window, not the sum of every provider's timeout.
    let refresh = refresh.unwrap_or(false);
    let (opencode, deepseek, claude, chat, gemini, grok, vibe) = tokio::join!(
        get_catalog_for_harness("opencode", refresh),
        get_catalog_for_harness("deepseek", refresh),
        get_catalog_for_harness("claude", refresh),
        get_catalog_for_harness("chat", refresh),
        get_catalog_for_harness("gemini", refresh),
        get_catalog_for_harness("grok", refresh),
        get_catalog_for_harness("vibe", refresh),
    );
    Ok(BTreeMap::from([
        ("opencode".to_string(), opencode),
        ("deepseek".to_string(), deepseek),
        ("claude".to_string(), claude),
        ("chat".to_string(), chat),
        ("gemini".to_string(), gemini),
        ("grok".to_string(), grok),
        ("vibe".to_string(), vibe),
    ]))
}

#[tauri::command]
pub async fn settings_set_harness_model(
    harness: String,
    model: Option<String>,
) -> Result<EffectiveHarnessSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        settings_set_harness_model_blocking(harness, model)
    })
    .await
    .map_err(|error| format!("settings_set_harness_model worker panic: {error}"))?
}

fn settings_set_harness_model_blocking(
    harness: String,
    model: Option<String>,
) -> Result<EffectiveHarnessSettings, String> {
    let mut store = open_settings_store();
    store
        .set_harness_default_model(&harness, model.as_deref())
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(&format!("harness.{harness}.model"), "global", "update")?;
    store
        .effective_harness(None, &harness)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_set_harness_effort(
    harness: String,
    effort: Option<String>,
) -> Result<EffectiveHarnessSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        settings_set_harness_effort_blocking(harness, effort)
    })
    .await
    .map_err(|error| format!("settings_set_harness_effort worker panic: {error}"))?
}

fn settings_set_harness_effort_blocking(
    harness: String,
    effort: Option<String>,
) -> Result<EffectiveHarnessSettings, String> {
    let mut store = open_settings_store();
    store
        .set_harness_default_effort(&harness, effort.as_deref())
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(&format!("harness.{harness}.effort"), "global", "update")?;
    store
        .effective_harness(None, &harness)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_set_workspace_harness_model(
    workspace: String,
    harness: String,
    model: String,
) -> Result<EffectiveHarnessSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        settings_set_workspace_harness_model_blocking(workspace, harness, model)
    })
    .await
    .map_err(|error| format!("settings_set_workspace_harness_model worker panic: {error}"))?
}

fn settings_set_workspace_harness_model_blocking(
    workspace: String,
    harness: String,
    model: String,
) -> Result<EffectiveHarnessSettings, String> {
    let mut store = open_settings_store();
    store
        .set_workspace_default_model(&workspace, &harness, &model)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(&format!("harness.{harness}.model"), &workspace, "override")?;
    store
        .effective_harness(Some(&workspace), &harness)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_reset_workspace_harness_model(
    workspace: String,
    harness: String,
) -> Result<EffectiveHarnessSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        settings_reset_workspace_harness_model_blocking(workspace, harness)
    })
    .await
    .map_err(|error| format!("settings_reset_workspace_harness_model worker panic: {error}"))?
}

fn settings_reset_workspace_harness_model_blocking(
    workspace: String,
    harness: String,
) -> Result<EffectiveHarnessSettings, String> {
    let mut store = open_settings_store();
    store
        .reset_workspace_default_model(&workspace, &harness)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(&format!("harness.{harness}.model"), &workspace, "reset")?;
    store
        .effective_harness(Some(&workspace), &harness)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_set_workspace_harness_effort(
    workspace: String,
    harness: String,
    effort: String,
) -> Result<EffectiveHarnessSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        settings_set_workspace_harness_effort_blocking(workspace, harness, effort)
    })
    .await
    .map_err(|error| format!("settings_set_workspace_harness_effort worker panic: {error}"))?
}

fn settings_set_workspace_harness_effort_blocking(
    workspace: String,
    harness: String,
    effort: String,
) -> Result<EffectiveHarnessSettings, String> {
    let mut store = open_settings_store();
    store
        .set_workspace_default_effort(&workspace, &harness, &effort)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(&format!("harness.{harness}.effort"), &workspace, "override")?;
    store
        .effective_harness(Some(&workspace), &harness)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_reset_workspace_harness_effort(
    workspace: String,
    harness: String,
) -> Result<EffectiveHarnessSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        settings_reset_workspace_harness_effort_blocking(workspace, harness)
    })
    .await
    .map_err(|error| format!("settings_reset_workspace_harness_effort worker panic: {error}"))?
}

fn settings_reset_workspace_harness_effort_blocking(
    workspace: String,
    harness: String,
) -> Result<EffectiveHarnessSettings, String> {
    let mut store = open_settings_store();
    store
        .reset_workspace_default_effort(&workspace, &harness)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(&format!("harness.{harness}.effort"), &workspace, "reset")?;
    store
        .effective_harness(Some(&workspace), &harness)
        .map_err(|e| e.to_string())
}
