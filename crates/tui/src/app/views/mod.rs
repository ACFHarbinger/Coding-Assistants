//! View modules for the Ratatui TUI client (T4 / #138).

pub mod chat;
pub mod modals;
pub mod orchestrate;
pub mod session_modals;
pub mod settings;
pub mod shared_hub;

pub use chat::draw_chat_view;
pub use modals::{draw_composer_modal, draw_confirmation_modal, draw_delivery_outcomes_modal};
pub use orchestrate::draw_orchestrate_view;
pub use session_modals::{draw_create_session_modal, draw_session_switcher_modal};
pub use settings::draw_settings_view;
pub use shared_hub::draw_shared_hub_view;
