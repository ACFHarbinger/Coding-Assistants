use hub::HubStore;
use serde::Serialize;

const CHAR_BUDGET: usize = 6_000;

/// Structured, prompt-visible recall data for the desktop's future memory UI.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecallEvent {
    pub role: String,
    pub workspace: String,
    pub limit: u8,
    pub memories: Vec<RecalledMemory>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecalledMemory {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub scope: String,
    pub tier: String,
    pub score: f32,
}

pub async fn append_recalled_memories(
    prompt: &mut String,
    task: &str,
    workspace: &str,
) -> Option<(u8, Vec<RecalledMemory>)> {
    let workspace = workspace.to_string();
    let task = task.to_string();
    let outcome = tokio::task::spawn_blocking(move || {
        let settings = hub::SettingsStore::open(hub::default_hub_home());
        let policy = settings.effective(Some(&workspace)).orchestration;
        if !policy.memory_recall_enabled {
            return Ok::<Option<(u8, Vec<(hub::MemoryRecord, f32)>)>, String>(None);
        }
        let store = HubStore::open(hub::default_hub_home()).map_err(|error| error.to_string())?;
        let hits = store
            .search_memories_for_workspace_recall(
                &task,
                usize::from(policy.memory_recall_limit),
                &workspace,
            )
            .map_err(|error| error.to_string())?;
        Ok(Some((policy.memory_recall_limit, hits)))
    })
    .await;
    let Ok(Ok(Some((limit, hits)))) = outcome else {
        if let Ok(Err(error)) = outcome {
            eprintln!("Memory recall skipped: {error}");
        }
        return None;
    };
    let recalled = format_memories(hits, CHAR_BUDGET);
    if recalled.is_empty() {
        return None;
    }
    prompt.push_str("Recalled Shared Memory (workspace and global; relevance-ranked):\n");
    prompt.push_str(&format_prompt(&recalled));
    prompt.push_str("\n\n");
    Some((limit, recalled))
}

fn format_memories(hits: Vec<(hub::MemoryRecord, f32)>, budget: usize) -> Vec<RecalledMemory> {
    let mut remaining = budget;
    let mut recalled = Vec::new();
    for (record, score) in hits {
        let overhead = record
            .title
            .as_ref()
            .map_or(0, |title| title.chars().count())
            + 80;
        if remaining <= overhead {
            break;
        }
        let body = truncate(&record.body, remaining - overhead);
        remaining = remaining.saturating_sub(overhead + body.chars().count());
        recalled.push(RecalledMemory {
            id: record.id,
            title: record.title,
            body,
            scope: record.scope,
            tier: record.tier,
            score,
        });
    }
    recalled
}

fn format_prompt(memories: &[RecalledMemory]) -> String {
    memories
        .iter()
        .enumerate()
        .map(|(index, memory)| {
            format!(
                "{}. [{} / {}, score {:.3}] {}\n{}",
                index + 1,
                memory.scope,
                memory.tier,
                memory.score,
                memory.title.as_deref().unwrap_or("Untitled memory"),
                memory.body
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut truncated: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        truncated.push('…');
    }
    truncated
}
