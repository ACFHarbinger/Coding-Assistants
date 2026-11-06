//! Terminal lifecycle management and safe restoration guard.

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::panic;

/// Alias for the standard Ratatui Crossterm terminal type.
pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Initializes the terminal in raw mode and alternate screen, installing a panic hook
/// to ensure the terminal state is cleanly restored if a panic occurs.
///
/// Deliberately does *not* enable crossterm's mouse capture: capturing the
/// mouse steals it from the terminal emulator, which is exactly what broke
/// native click-and-drag text selection/copy in this TUI. Tab switching and
/// scrolling are already fully reachable from the keyboard (1-4, Tab/h/l,
/// j/k, g/G), so nothing is lost by leaving the mouse alone.
pub fn init_terminal() -> anyhow::Result<TuiTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    // Install panic hook to restore terminal mode before printing the panic stack trace
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal_raw();
        original_hook(panic_info);
    }));

    Ok(terminal)
}

/// Helper function to reset terminal raw mode and screen state.
fn restore_terminal_raw() -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Restores the terminal to standard canonical mode upon normal exit.
pub fn restore_terminal(mut terminal: TuiTerminal) -> anyhow::Result<()> {
    restore_terminal_raw()?;
    terminal.show_cursor()?;
    Ok(())
}
