use crate::app::AgentCommand;
use hub::HubStore;

pub(crate) fn run(store: &HubStore, action: AgentCommand) -> anyhow::Result<()> {
    match action {
        AgentCommand::List => {
            println!("{}", serde_json::to_string_pretty(&store.list_agents()?)?);
        }
        AgentCommand::Team => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.list_team_members()?)?
            );
        }
        AgentCommand::Enroll { id } => {
            let record = store.set_team_member(&id, true)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        AgentCommand::Unenroll { id } => {
            let record = store.set_team_member(&id, false)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        AgentCommand::RegisterCard { agent, path } => {
            let json = std::fs::read_to_string(&path)?;
            let card: hub::AgentCard = serde_json::from_str(&json)?;
            store.upsert_agent_card(&agent, &card)?;
            println!("registered card for {}", agent);
        }
        AgentCommand::SetAvatar { agent_id, path } => {
            let data = std::fs::read(&path)
                .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
            if data.is_empty() {
                anyhow::bail!("{} is empty", path.display());
            }
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("avatar");
            let mime = guess_image_mime(&path);
            let record = store.set_agent_avatar(&agent_id, filename, mime, &data)?;
            eprintln!(
                "set avatar for {agent_id} ({} bytes, {mime}, {})",
                data.len(),
                filename
            );
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        AgentCommand::ClearAvatar { agent_id } => {
            let record = store.clear_agent_avatar(&agent_id)?;
            eprintln!("cleared avatar for {agent_id}");
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
    }
    Ok(())
}

fn guess_image_mime(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}
