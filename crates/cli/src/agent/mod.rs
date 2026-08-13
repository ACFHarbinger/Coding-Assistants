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
    }
    Ok(())
}
