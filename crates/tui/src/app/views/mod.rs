//! View modules for the Ratatui TUI client (T4 / #138, T5 / #139).

pub mod chat;
pub mod conflict_banner;
pub mod danger_modal;
pub mod harness_launcher_modal;
pub mod harness_panes;
pub mod modals;
pub mod orchestrate;
pub mod recovery_modal;
pub mod session_modals;
pub mod settings;
pub mod shared_hub;

pub use chat::draw_chat_view;
pub use conflict_banner::draw_conflict_banner;
pub use danger_modal::draw_danger_modal;
pub use harness_launcher_modal::draw_harness_launcher_modal;
pub use harness_panes::draw_harness_workspace_view;
pub use modals::{draw_composer_modal, draw_confirmation_modal, draw_delivery_outcomes_modal};
pub use orchestrate::draw_orchestrate_view;
pub use recovery_modal::draw_recovery_modal;
pub use session_modals::{draw_create_session_modal, draw_session_switcher_modal};
pub use settings::draw_settings_view;
pub use shared_hub::{draw_shared_hub_view, HubViewMode};
