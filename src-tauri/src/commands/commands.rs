//! Stable Tauri-command facade for the durable shared hub.
//!
//! Commands are grouped by responsibility under `commands/`, with files
//! further organized by concern into `messager/` (chat + memory),
//! `hub/` (store bootstrap + workflow), `quota/`, `settings/` (settings +
//! roles), and `tests/` subdirectories. Module names (`commands::messaging`,
//! `commands::quotas`, etc.) are unchanged for callers — only the on-disk
//! file layout moved.

#[path = "hub/activity.rs"]
pub mod activity;
#[path = "messager/attachments.rs"]
pub mod attachments;
#[path = "hub/avatar.rs"]
pub mod avatar;
#[path = "settings/creative_tools.rs"]
pub mod creative_tools;
#[path = "settings/credentials.rs"]
pub mod credentials;
#[path = "settings/danger.rs"]
pub mod danger;
#[path = "settings/external_mcp.rs"]
pub mod external_mcp;
#[path = "github/board.rs"]
pub mod github_board;
#[path = "github/branches.rs"]
pub mod github_branches;
#[path = "settings/harness_models.rs"]
pub mod harness_models;
#[path = "health/health.rs"]
pub mod health;
#[path = "mcp_invoke.rs"]
pub mod mcp_invoke;
#[path = "messager/memory.rs"]
pub mod memory;
#[path = "messager/messaging.rs"]
pub mod messaging;
#[path = "os_tool.rs"]
pub mod os_tool;
#[path = "hub/oversight.rs"]
pub mod oversight;
// The quota adapters and the schedule probe classifier moved to the shared
// `quota` crate (S5c / #358) so `crates/cli` can probe with full fidelity;
// only the desktop IPC wrappers remain, in `quota/quotas.rs`.
#[path = "quota/quotas.rs"]
pub mod quotas;
#[path = "settings/roles.rs"]
pub mod roles;
#[path = "hub/schedule.rs"]
pub mod schedule;
#[path = "hub/schedule_poller.rs"]
pub mod schedule_poller;
#[path = "settings/schedule_trigger.rs"]
pub mod schedule_trigger;
#[path = "hub/scorecard.rs"]
pub mod scorecard;
#[path = "settings/settings.rs"]
pub mod settings;
#[path = "hub/store.rs"]
pub mod store;
#[path = "hub/sync.rs"]
pub mod sync;
#[path = "system/mod.rs"]
pub mod system;
#[path = "terminal_grid.rs"]
pub mod terminal_grid;
#[path = "hub/vtuber.rs"]
pub mod vtuber;
#[path = "hub/workflow.rs"]
pub mod workflow;
#[path = "hub/workspaces.rs"]
pub mod workspaces;

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
