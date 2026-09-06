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
    /// Insert one typed character (text entry only).
    Type(char),
    /// Delete the last typed character (text entry only).
    EraseChar,
    /// Exit the application.
    Quit,
}

/// Map a navigation or action key to a [`Command`]. Keys not listed do nothing.
///
/// `Esc` is the single exit key in every screen. `Enter` is left unmapped here:
/// it is only used to submit the directory path while typing (see
/// [`command_for_text_key`]). Confirming or opening uses the Right arrow.
pub fn command_for_key(code: KeyCode) -> Option<Command> {
    match code {
        KeyCode::Esc => Some(Command::Quit),
        KeyCode::Up => Some(Command::MoveUp),
        KeyCode::Down => Some(Command::MoveDown),
        KeyCode::Right => Some(Command::Open),
        KeyCode::Left => Some(Command::Back),
        _ => None,
    }
}

/// Map a key to a [`Command`] while the user is typing free text (for example
/// a directory path). Esc still exits; every character, including lowercase
/// `x`, is inserted as text.
pub fn command_for_text_key(code: KeyCode) -> Option<Command> {
    match code {
        KeyCode::Esc => Some(Command::Quit),
        KeyCode::Char(c) if !c.is_control() => Some(Command::Type(c)),
        KeyCode::Backspace => Some(Command::EraseChar),
        KeyCode::Enter => Some(Command::Open),
        _ => None,
    }
}

/// Help text for the table list footer.
pub fn help_for_list() -> &'static str {
    "Up/Down: move | Right: open | Left: change database | Esc: exit"
}

/// Help text for the row grid footer.
pub fn help_for_grid() -> &'static str {
    "Up/Down: scroll | Left: back | Esc: exit"
}

/// Help text while typing a directory path in the picker.
pub fn help_for_picker_input() -> &'static str {
    "type a directory | Backspace: delete | Enter: scan | Esc: exit"
}

/// Help text while choosing a database in the picker.
pub fn help_for_picker_list() -> &'static str {
    "Up/Down: move | Right: open | Left: change directory | Esc: exit"
}
