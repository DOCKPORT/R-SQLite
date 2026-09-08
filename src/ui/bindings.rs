use crossterm::event::{KeyCode, KeyModifiers};

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
    /// Pick the previous column as the sort key.
    SortColumnLeft,
    /// Pick the next column as the sort key.
    SortColumnRight,
    /// Toggle the sort direction on the chosen column.
    OrderToggle,
    /// Jump 50 rows up.
    PageUp,
    /// Jump 50 rows down.
    PageDown,
    /// Exit the application.
    Quit,
}

/// Map a key to a [`Command`]. Keys not listed do nothing.
///
/// `Esc` is the single exit key. Plain arrows move/scroll and confirm. Holding
/// Shift with Left/Right picks the sort column. `Space` toggles the sort
/// direction on the chosen column. Many terminals drop the Shift modifier on
/// Up/Down, so Space is used instead of Shift+Up/Down. PgUp/PgDn (with or
/// without Shift) jump 50 rows, since many terminals do not report Shift on
/// those keys. `Enter` is left unmapped here: it is only used to submit the
/// directory path while typing.
pub fn command_for_key(code: KeyCode, mods: KeyModifiers) -> Option<Command> {
    if code == KeyCode::Esc {
        return Some(Command::Quit);
    }
    let shifted = mods.contains(KeyModifiers::SHIFT);
    match (code, shifted) {
        (KeyCode::Up, _) => Some(Command::MoveUp),
        (KeyCode::Down, _) => Some(Command::MoveDown),
        (KeyCode::Right, false) => Some(Command::Open),
        (KeyCode::Right, true) => Some(Command::SortColumnRight),
        (KeyCode::Left, false) => Some(Command::Back),
        (KeyCode::Left, true) => Some(Command::SortColumnLeft),
        (KeyCode::Char(' '), false) => Some(Command::OrderToggle),
        (KeyCode::PageUp, _) => Some(Command::PageUp),
        (KeyCode::PageDown, _) => Some(Command::PageDown),
        _ => None,
    }
}

/// Map a key to a [`Command`] while the user is typing free text (for example
/// a directory path). Esc still exits; every character is inserted as text.
pub fn command_for_text_key(code: KeyCode, _mods: KeyModifiers) -> Option<Command> {
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
    "↑/↓: scroll | →: open | ←: change database | Esc: exit"
}

/// Help text for the row grid footer.
pub fn help_for_grid() -> &'static str {
    "↑/↓: scroll | PgUp/PgDn: 50 rows | Shift+←/→: sort column | Space: sort direction | ←: back | Esc: exit"
}

/// Help text while typing a directory path in the picker.
pub fn help_for_picker_input() -> &'static str {
    "type a directory | Backspace: delete | Enter: scan | Esc: exit"
}

/// Help text while choosing a database in the picker.
pub fn help_for_picker_list() -> &'static str {
    "↑/↓: scroll | PgUp/PgDn: 50 | →: open | ←: change directory | Esc: exit"
}
