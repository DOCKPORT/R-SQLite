use crossterm::event::KeyCode;

/// The single vocabulary of user actions. Keys map to these, and screens act
/// on them. This file is the one source of truth for keyboard commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Move the highlight up (or scroll up).
    MoveUp,
    /// Move the highlight down (or scroll down).
    MoveDown,
    /// Open the selected item.
    Open,
    /// Go back to the previous screen.
    Back,
    /// Exit the application.
    Quit,
}

/// Map one physical key to a [`Command`]. Keys not listed here do nothing.
pub fn command_for_key(code: KeyCode) -> Option<Command> {
    match code {
        KeyCode::Up => Some(Command::MoveUp),
        KeyCode::Down => Some(Command::MoveDown),
        KeyCode::Right => Some(Command::Open),
        KeyCode::Left => Some(Command::Back),
        KeyCode::Char('x') => Some(Command::Quit),
        _ => None,
    }
}

/// Help text for the table list footer.
pub fn help_for_list() -> &'static str {
    "Up/Down: move | Right: open | x: exit"
}

/// Help text for the row grid footer.
pub fn help_for_grid() -> &'static str {
    "Up/Down: scroll | Left: back | x: exit"
}
