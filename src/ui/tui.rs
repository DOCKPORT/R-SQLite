use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

/// The concrete terminal backend used by this application.
pub type Backend = CrosstermBackend<Stdout>;
/// The ratatui terminal handle used for drawing.
pub type TuiTerminal = Terminal<Backend>;

/// Enter raw mode and the alternate screen, returning a ready terminal.
///
/// Call [`restore`] when done so the user's terminal is put back exactly as it
/// was before the application started.
pub fn init() -> Result<TuiTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Leave the alternate screen and restore normal terminal behaviour.
pub fn restore(mut terminal: TuiTerminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
