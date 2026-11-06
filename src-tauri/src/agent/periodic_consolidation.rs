use crate::client::llm::ModelConfig;
use crate::commands::commands::memory::{hub_consolidate_memories, ConsolidateMemoriesArgs};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static LAST_ATTEMPT: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn gate_allows_attempt(
    enabled: bool,
    min_clusters: usize,
    cluster_count: usize,
    last_attempt: Option<Instant>,
    cooldown: Duration,
) -> bool {
    enabled
        && min_clusters > 0
        && cluster_count >= min_clusters
        && last_attempt.is_none_or(|last| last.elapsed() >= cooldown)
}

/// Opportunistic end-of-task consolidation. It never fails the completed task:
/// provider/offline failures are handled as non-destructive skips by M3.
pub async fn maybe_consolidate(
    app: &tauri::AppHandle,
    model_config: ModelConfig,
    workspace: String,
    enabled: bool,
    min_clusters: usize,
    cooldown_minutes: u64,
) {
    if !enabled || min_clusters == 0 {
        return;
    }
    let cooldown = Duration::from_secs(cooldown_minutes.saturating_mul(60));
    let attempts = LAST_ATTEMPT.get_or_init(|| Mutex::new(HashMap::new()));
    let last_attempt = attempts
        .lock()
        .ok()
        .and_then(|attempts| attempts.get(&workspace).copied());
    if last_attempt.is_some_and(|last| last.elapsed() < cooldown) {
        return;
    }
    let cluster_count = tauri::async_runtime::spawn_blocking(|| {
        hub::HubStore::open(hub::default_hub_home())?
            .consolidation_clusters()
            .map(|clusters| clusters.len())
    })
    .await
    .ok()
    .and_then(Result::ok)
    .unwrap_or_default();
    if !gate_allows_attempt(enabled, min_clusters, cluster_count, last_attempt, cooldown) {
        return;
    }
    if let Ok(mut attempts) = attempts.lock() {
        attempts.insert(workspace.clone(), Instant::now());
    }
    let _ = hub_consolidate_memories(
        app.clone(),
        ConsolidateMemoriesArgs {
            model_config,
            workspace: Some(workspace),
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_requires_opt_in_and_enough_clusters() {
        let cooldown = Duration::from_secs(60);
        assert!(!gate_allows_attempt(false, 2, 2, None, cooldown));
        assert!(!gate_allows_attempt(true, 0, 3, None, cooldown));
        assert!(!gate_allows_attempt(true, 2, 1, None, cooldown));
        assert!(gate_allows_attempt(true, 2, 2, None, cooldown));
    }

    #[test]
    fn gate_respects_per_workspace_cooldown() {
        let cooldown = Duration::from_secs(60);
        assert!(!gate_allows_attempt(
            true,
            2,
            2,
            Some(Instant::now()),
            cooldown
        ));
        assert!(gate_allows_attempt(
            true,
            2,
            2,
            Some(Instant::now() - Duration::from_secs(60)),
            cooldown
        ));
    }
}
