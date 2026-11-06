//! Role-based permissions: a named, reusable bundle of capabilities and
//! responsibilities assignable to any team member (including the human,
//! via the protected `cto` role). An agent's *effective* permissions are
//! the union across every role currently assigned to it — see
//! [`crate::HubStore::effective_agent_permissions`].
//!
//! Split by concern: [`crud`] (role definitions, assignments, and the
//! per-provider default-role resolution used when an agent has none
//! assigned yet) and [`gate`] (the daily ungated-send quota, the
//! broadcast-recipient limit, and the durable human-approval queue those
//! two limits route into when exceeded).

mod crud;
mod defaults;
mod gate;

pub(crate) const CTO_ROLE_ID: &str = "cto";
