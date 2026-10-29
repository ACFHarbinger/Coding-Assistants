//! Clients for external model-provider services.

pub(crate) mod endpoint_auth;
pub(crate) mod endpoint_health;
pub(crate) mod endpoint_link;
mod events;
mod grok_bot_turn;
pub mod http;
pub mod llm;
mod muse_http;
mod openrouter_turn;
pub mod providers;
mod stream;
