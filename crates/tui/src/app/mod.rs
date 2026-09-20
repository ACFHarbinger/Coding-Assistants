//! `ca tui` main application runner and Ratatui rendering engine.

mod ambient;
pub mod approvals;
pub mod composer;
pub mod keymap;
pub mod runner;
pub mod session_ops;
pub mod state;
pub mod ui;
pub mod views;

pub use runner::{persist_requested_defaults, run};
pub use state::{AppState, TabIndex};
pub use ui::draw_ui;
