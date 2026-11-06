//! Per-task runtime state (P2 / #331). Concurrent desktop tasks must not
//! clobber each other's cancellation flag, user-input channel, or MCP
//! configuration, so all three are keyed by task id instead of living in
//! single global slots.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;

/// Live handles for one running desktop task.
#[derive(Clone)]
pub struct TaskHandles {
    pub cancellation: Arc<AtomicBool>,
    pub input_tx: mpsc::Sender<String>,
}

/// Task-id-keyed registry replacing the old single-slot
/// `cancellation_token` / `user_input_tx` state.
pub struct TaskRegistry {
    inner: Mutex<HashMap<String, TaskHandles>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Registers a live task, returning false when its caller-supplied id is
    /// already active. Replacing an existing entry would let the first task's
    /// cleanup remove the second task's handles.
    pub fn register(&self, task_id: &str, handles: TaskHandles) -> bool {
        let mut tasks = self.inner.lock().unwrap();
        if tasks.contains_key(task_id) {
            return false;
        }
        tasks.insert(task_id.to_string(), handles);
        true
    }

    pub fn get(&self, task_id: &str) -> Option<TaskHandles> {
        self.inner.lock().unwrap().get(task_id).cloned()
    }

    pub fn remove(&self, task_id: &str) {
        self.inner.lock().unwrap().remove(task_id);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

static TASK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a process-unique task id (`task-<millis>-<counter>`).
pub fn next_task_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("task-{millis}-{seq}")
}

/// Caller-supplied ids must be path-safe: only letters, digits, `-`, `_`.
/// Anything else is rejected before it can reach the MCP config path.
pub fn is_valid_task_id(task_id: &str) -> bool {
    !task_id.is_empty()
        && task_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Task-scoped MCP config file: `<hub_home>/mcp-tasks/<task_id>/mcp.json`.
/// Concurrent tasks each get their own file instead of sharing one `mcp.json`.
pub fn task_mcp_file(hub_home: &Path, task_id: &str) -> PathBuf {
    hub_home.join("mcp-tasks").join(task_id).join("mcp.json")
}

/// Best-effort removal of a finished task's MCP config dir. Errors are
/// swallowed: a stale dir is harmless (ids are unique) and must never fail
/// the task outcome.
pub fn remove_task_mcp_dir(hub_home: &Path, task_id: &str) {
    if let Some(parent) = task_mcp_file(hub_home, task_id).parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

/// Lifecycle event published on the P1 bus (`task-lifecycle` topic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLifecycleEvent {
    pub task_id: String,
    pub status: String,
    pub detail: Option<String>,
}

impl TaskLifecycleEvent {
    pub fn new(
        task_id: impl Into<String>,
        status: impl Into<String>,
        detail: Option<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            status: status.into(),
            detail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_ids_are_unique() {
        assert_ne!(next_task_id(), next_task_id());
    }

    #[test]
    fn task_id_validation_rejects_path_traversal() {
        assert!(is_valid_task_id("task-1_2"));
        assert!(!is_valid_task_id(""));
        assert!(!is_valid_task_id("../evil"));
        assert!(!is_valid_task_id("a/b"));
        assert!(!is_valid_task_id("a b"));
    }

    #[test]
    fn mcp_files_are_scoped_per_task() {
        let home = Path::new("/tmp/ca-home");
        let first = task_mcp_file(home, "task-a");
        let second = task_mcp_file(home, "task-b");
        assert_ne!(first, second);
        assert!(first.starts_with(home.join("mcp-tasks")));
        assert_eq!(first.file_name().unwrap(), "mcp.json");
    }

    #[test]
    fn cancelling_one_task_leaves_the_other_armed() {
        let registry = TaskRegistry::new();
        let (tx_a, _rx_a) = mpsc::channel(1);
        let (tx_b, _rx_b) = mpsc::channel(1);
        assert!(registry.register(
            "task-a",
            TaskHandles {
                cancellation: Arc::new(AtomicBool::new(false)),
                input_tx: tx_a,
            },
        ));
        assert!(registry.register(
            "task-b",
            TaskHandles {
                cancellation: Arc::new(AtomicBool::new(false)),
                input_tx: tx_b,
            },
        ));
        registry
            .get("task-a")
            .unwrap()
            .cancellation
            .store(true, Ordering::SeqCst);
        assert!(registry
            .get("task-a")
            .unwrap()
            .cancellation
            .load(Ordering::SeqCst));
        assert!(!registry
            .get("task-b")
            .unwrap()
            .cancellation
            .load(Ordering::SeqCst));
        assert!(registry.get("task-unknown").is_none());
        registry.remove("task-a");
        assert_eq!(registry.len(), 1);
    }

    #[tokio::test]
    async fn input_routes_to_the_addressed_task_only() {
        let registry = TaskRegistry::new();
        let (tx_a, mut rx_a) = mpsc::channel(1);
        let (tx_b, mut rx_b) = mpsc::channel(1);
        assert!(registry.register(
            "task-a",
            TaskHandles {
                cancellation: Arc::new(AtomicBool::new(false)),
                input_tx: tx_a,
            },
        ));
        assert!(registry.register(
            "task-b",
            TaskHandles {
                cancellation: Arc::new(AtomicBool::new(false)),
                input_tx: tx_b,
            },
        ));
        registry
            .get("task-b")
            .unwrap()
            .input_tx
            .send("hello-b".into())
            .await
            .unwrap();
        assert_eq!(rx_b.recv().await.unwrap(), "hello-b");
        assert!(rx_a.try_recv().is_err());
    }

    #[test]
    fn duplicate_task_id_does_not_replace_live_handles() {
        let registry = TaskRegistry::new();
        let (first_tx, _first_rx) = mpsc::channel(1);
        let first_cancel = Arc::new(AtomicBool::new(false));
        assert!(registry.register(
            "task-a",
            TaskHandles {
                cancellation: first_cancel.clone(),
                input_tx: first_tx,
            },
        ));

        let (second_tx, _second_rx) = mpsc::channel(1);
        assert!(!registry.register(
            "task-a",
            TaskHandles {
                cancellation: Arc::new(AtomicBool::new(false)),
                input_tx: second_tx,
            },
        ));
        assert!(Arc::ptr_eq(
            &registry.get("task-a").unwrap().cancellation,
            &first_cancel
        ));
    }
}
