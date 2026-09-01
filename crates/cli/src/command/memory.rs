//! ca memory subcommand dispatch (split from command/mod.rs for
//! the 500-LoC cap, #158).

use crate::app::MemoryCommand;
use hub::{HubStore, LinkSuggestionMode, MemoryScope, MemoryTier};

pub(super) fn run(store: &HubStore, action: MemoryCommand) -> anyhow::Result<()> {
    match action {
        MemoryCommand::Write {
            tier,
            scope,
            agent,
            workspace,
            title,
            tags,
            body,
        } => {
            let tier = MemoryTier::parse(&tier)?;
            let scope = MemoryScope::parse(&scope)?;
            let record = store.write_memory(
                tier,
                scope,
                agent.as_deref(),
                workspace.as_deref(),
                title.as_deref(),
                &body,
                &tags,
            )?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        MemoryCommand::List {
            scope,
            tier,
            workspace,
            include_stale,
        } => {
            let scope = scope.map(|s| MemoryScope::parse(&s)).transpose()?;
            let tier = tier.map(|t| MemoryTier::parse(&t)).transpose()?;
            let records = store.list_memories(scope, tier, workspace.as_deref(), include_stale)?;
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        MemoryCommand::Search { query } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.search_memories(&query)?)?
            );
        }
        MemoryCommand::Stale { id, unstale } => {
            store.mark_memory_stale(&id, !unstale)?;
            println!("ok");
        }
        MemoryCommand::Promote { id, to } => {
            let to = MemoryTier::parse(&to)?;
            let record = store.promote_memory(&id, to)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        MemoryCommand::Delete { id } => {
            store.delete_memory(&id)?;
            println!("ok");
        }
        MemoryCommand::Compact { keep } => {
            let report = store.compact_short_term(keep)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        MemoryCommand::PurgeStale => {
            let n = store.purge_stale_memories()?;
            println!("{{\"purged\":{n}}}");
        }
        MemoryCommand::AgeOut { hours } => {
            let n = store.mark_short_term_stale_older_than(hours)?;
            println!("{{\"aged_out\":{n}}}");
        }
        MemoryCommand::Link {
            from,
            to,
            relation,
            created_by,
        } => {
            let record = store.link_memories(&from, &to, relation.as_deref(), &created_by)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        MemoryCommand::Unlink { link_id } => {
            store.unlink_memories(&link_id)?;
            println!("ok");
        }
        MemoryCommand::Links { memory_id } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.list_memory_links(&memory_id)?)?
            );
        }
        MemoryCommand::Related { memory_id, depth } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.related_memories(&memory_id, depth)?)?
            );
        }
        MemoryCommand::Topic { query } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.memories_for_topic(&query)?)?
            );
        }
        MemoryCommand::SuggestLinks { memory_id, limit } => {
            let suggestions = store.suggest_links_for_memory(&memory_id, limit)?;
            println!("{}", serde_json::to_string_pretty(&suggestions)?);
        }
        MemoryCommand::ApplySuggestions {
            memory_id,
            mode,
            limit,
        } => {
            let mode = LinkSuggestionMode::parse(&mode).ok_or_else(|| {
                anyhow::anyhow!("unknown link-suggestion mode {mode:?} (expected off|suggest|auto)")
            })?;
            let suggestions = store.apply_link_suggestions(&memory_id, mode, limit)?;
            println!("{}", serde_json::to_string_pretty(&suggestions)?);
        }
        MemoryCommand::SemanticSearch {
            query,
            limit,
            scope,
            tier,
            workspace,
        } => {
            let scope = scope.map(|s| MemoryScope::parse(&s)).transpose()?;
            let tier = tier.map(|t| MemoryTier::parse(&t)).transpose()?;
            let results = store.search_memories_semantic(
                &query,
                limit,
                scope,
                tier,
                workspace.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        MemoryCommand::HybridSearch {
            query,
            limit,
            scope,
            tier,
            workspace,
        } => {
            let scope = scope.map(|s| MemoryScope::parse(&s)).transpose()?;
            let tier = tier.map(|t| MemoryTier::parse(&t)).transpose()?;
            let results = store.search_memories_hybrid(
                &query,
                limit,
                scope,
                tier,
                workspace.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        MemoryCommand::Reindex => {
            let count = store.reindex_memory_vectors()?;
            println!("{{\"reindexed\":{count}}}");
        }
    }
    Ok(())
}
