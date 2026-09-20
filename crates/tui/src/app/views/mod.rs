//! View modules for the Ratatui TUI client (T4 / #138, T5 / #139).

pub mod chat;
pub mod danger_modal;
pub mod modals;
pub mod orchestrate;
pub mod recovery_modal;
pub mod session_modals;
pub mod settings;
pub mod shared_hub;

pub use chat::draw_chat_view;
pub use danger_modal::draw_danger_modal;
pub use modals::{draw_composer_modal, draw_confirmation_modal, draw_delivery_outcomes_modal};
pub use orchestrate::draw_orchestrate_view;
pub use recovery_modal::draw_recovery_modal;
pub use session_modals::{draw_create_session_modal, draw_session_switcher_modal};
pub use settings::draw_settings_view;
pub use shared_hub::{draw_shared_hub_view, HubViewMode};
