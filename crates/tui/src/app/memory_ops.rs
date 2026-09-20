use hub::{HubStore, MemoryRecord, MemoryScope};
use std::path::Path;

#[derive(Debug, Default)]
pub struct MemorySearchState {
    pub query: String,
    pub is_active: bool,
    pub results: Vec<MemoryRecord>,
    pub selected_index: usize,
    pub scope_filter: Option<String>,
    pub status_message: Option<String>,
}

impl MemorySearchState {
    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.results.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn execute_search(
        &mut self,
        store: &HubStore,
        workspace: Option<&Path>,
    ) -> Result<usize, anyhow::Error> {
        let trimmed = self.query.trim();
        let ws_str = workspace.map(|p| p.display().to_string());

        let items = if trimmed.is_empty() {
            let scope = self
                .scope_filter
                .as_deref()
                .and_then(|s| MemoryScope::parse(s).ok());
            store.list_memories(scope, None, ws_str.as_deref(), true)?
        } else {
            let mut list = store.search_memories(trimmed)?;
            if let Some(scope) = &self.scope_filter {
                list.retain(|m| &m.scope == scope);
            }
            if let Some(ws) = &ws_str {
                list.retain(|m| m.scope == "global" || m.workspace_path.as_deref() == Some(ws));
            }
            list
        };

        let count = items.len();
        self.results = items;
        self.selected_index = 0;
        self.status_message = Some(format!("Found {count} memory records."));
        Ok(count)
    }

    pub fn cycle_scope(&mut self) {
        self.scope_filter = match self.scope_filter.as_deref() {
            None => Some("global".to_string()),
            Some("global") => Some("workspace".to_string()),
            _ => None,
        };
    }
}
