use hub::HubStore;

pub(super) fn record(
    store: &HubStore,
    agent: &str,
    task: Option<&str>,
    objective: &str,
    reason: &str,
    delegate_to: Option<&str>,
) -> anyhow::Result<()> {
    let outcome = store.record_shutdown(agent, task, objective, reason, delegate_to)?;
    println!("{}", serde_json::to_string_pretty(&outcome)?);
    Ok(())
}
