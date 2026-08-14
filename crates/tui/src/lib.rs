//! `crates/tui` — Ratatui TUI client library for Coding-Assistants (U7).

pub mod app;
pub mod model;
pub mod options;
pub mod terminal;
pub mod theme;

pub use app::run;
pub use model::HubReadModel;
pub use options::TuiOptions;
pub use theme::{Theme, ThemeName};
