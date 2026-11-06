use super::super::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationCluster {
    pub memories: Vec<MemoryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // Constructed by the desktop command crate over this public IPC type.
pub struct ConsolidationReport {
    pub clusters: usize,
    pub consolidated: usize,
    pub skipped: usize,
    pub notice: Option<String>,
}

impl HubStore {
    /// Plans conservative same-scope clusters from shared tag/token overlap.
    /// A cluster must contain at least two still-live short-term memories.
    pub fn consolidation_clusters(&self) -> Result<Vec<ConsolidationCluster>, HubError> {
        let memories = self.list_memories(None, Some(MemoryTier::ShortTerm), None, false)?;
        let mut clusters: Vec<Vec<MemoryRecord>> = Vec::new();
        for memory in memories {
            if let Some(cluster) = clusters.iter_mut().find(|cluster| {
                cluster
                    .iter()
                    .any(|other| same_scope(other, &memory) && overlap(other, &memory) >= 2)
            }) {
                cluster.push(memory);
            } else {
                clusters.push(vec![memory]);
            }
        }
        Ok(clusters
            .into_iter()
            .filter(|cluster| cluster.len() >= 2)
            .map(|memories| ConsolidationCluster { memories })
            .collect())
    }

    /// Writes an episodic summary, links each source, then marks sources stale.
    pub fn apply_consolidation(
        &self,
        cluster: &ConsolidationCluster,
        summary: &str,
    ) -> Result<MemoryRecord, HubError> {
        let first = cluster
            .memories
            .first()
            .ok_or_else(|| HubError::Invalid("empty consolidation cluster".into()))?;
        let scope = MemoryScope::parse(&first.scope)?;
        let tags = cluster
            .memories
            .iter()
            .flat_map(tags)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let summary_record = self.write_memory(
            MemoryTier::Episodic,
            scope,
            Some("system:consolidation"),
            first.workspace_path.as_deref(),
            Some("Consolidated memory"),
            summary,
            &tags,
        )?;
        for source in &cluster.memories {
            self.link_memories(
                &source.id,
                &summary_record.id,
                Some("consolidated_into"),
                "system:consolidation",
            )?;
            self.mark_memory_stale(&source.id, true)?;
        }
        Ok(summary_record)
    }
}

fn same_scope(left: &MemoryRecord, right: &MemoryRecord) -> bool {
    left.scope == right.scope && left.workspace_path == right.workspace_path
}

fn tags(memory: &MemoryRecord) -> Vec<String> {
    serde_json::from_str(&memory.tags_json).unwrap_or_default()
}

fn overlap(left: &MemoryRecord, right: &MemoryRecord) -> usize {
    let left_tokens = words(left).into_iter().collect::<BTreeSet<_>>();
    words(right)
        .into_iter()
        .filter(|token| left_tokens.contains(token))
        .count()
}

fn words(memory: &MemoryRecord) -> Vec<String> {
    let text = format!(
        "{} {} {}",
        memory.title.as_deref().unwrap_or_default(),
        memory.body,
        memory.tags_json
    );
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| word.len() > 2)
        .map(str::to_ascii_lowercase)
        .collect()
}
