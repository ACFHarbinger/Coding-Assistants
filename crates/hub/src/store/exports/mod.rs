use super::*;
impl HubStore {
    pub fn export_markdown(&self, out_dir: Option<&Path>) -> Result<PathBuf, HubError> {
        let out = out_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.data_dir.join("markdown"));
        fs::create_dir_all(&out)?;

        let episodic = self.list_memories(None, Some(MemoryTier::Episodic), None, false)?;
        let semantic = self.list_memories(None, Some(MemoryTier::Semantic), None, false)?;
        let handoffs = self.list_messages(None, None)?;
        let handoffs: Vec<_> = handoffs
            .into_iter()
            .filter(|m| m.kind == MessageKind::Handoff.as_str())
            .collect();

        let mut body = String::from("# Coding-Assistants Shared Memory Export\n\n");
        body.push_str(&format!("Generated: {}\n\n", Utc::now().to_rfc3339()));

        body.push_str("## Episodic\n\n");
        for m in &episodic {
            body.push_str(&format!(
                "### {} ({})\n\n- id: `{}`\n- scope: {}\n- agent: {}\n\n{}\n\n",
                m.title.as_deref().unwrap_or("(untitled)"),
                m.created_at,
                m.id,
                m.scope,
                m.agent_id.as_deref().unwrap_or("-"),
                m.body
            ));
        }

        body.push_str("## Semantic\n\n");
        for m in &semantic {
            body.push_str(&format!(
                "### {} ({})\n\n- id: `{}`\n- scope: {}\n- agent: {}\n\n{}\n\n",
                m.title.as_deref().unwrap_or("(untitled)"),
                m.created_at,
                m.id,
                m.scope,
                m.agent_id.as_deref().unwrap_or("-"),
                m.body
            ));
        }

        body.push_str("## Handoffs\n\n");
        if handoffs.is_empty() {
            body.push_str("_No handoff messages._\n\n");
        }
        for m in &handoffs {
            body.push_str(&format!(
                "### {} → {} ({})\n\n- id: `{}`\n- status: {}\n- task: {}\n\n{}\n\n",
                m.from_agent,
                m.to_agent,
                m.created_at,
                m.id,
                m.status,
                m.task_id.as_deref().unwrap_or("-"),
                m.body
            ));
        }

        let path = out.join("shared_memory.md");
        fs::write(&path, body)?;
        Ok(path)
    }
}
