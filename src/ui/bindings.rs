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
    /// Sort the grid ascending (oldest first) by the chosen column.
    OrderAsc,
    /// Sort the grid descending (newest first) by the chosen column.
    OrderDesc,
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
/// Shift changes meaning: Shift+Left/Right pick the sort column and
/// Shift+Up/Down set the sort direction. PgUp/PgDn (with or without Shift) jump
/// 50 rows, since many terminals do not report Shift on those keys. `Enter` is
/// left unmapped here: it is only used to submit the directory path while typing.
pub fn command_for_key(code: KeyCode, mods: KeyModifiers) -> Option<Command> {
    if code == KeyCode::Esc {
        return Some(Command::Quit);
    }
    let shifted = mods.contains(KeyModifiers::SHIFT);
    match code {
        KeyCode::Up => {
            if shifted {
                Some(Command::OrderDesc)
            } else {
                Some(Command::MoveUp)
            }
        }
        KeyCode::Down => {
            if shifted {
                Some(Command::OrderAsc)
            } else {
                Some(Command::MoveDown)
            }
        }
        KeyCode::Right => {
            if shifted {
                Some(Command::SortColumnRight)
            } else {
                Some(Command::Open)
            }
        }
        KeyCode::Left => {
            if shifted {
                Some(Command::SortColumnLeft)
            } else {
                Some(Command::Back)
            }
        }
        KeyCode::PageUp => Some(Command::PageUp),
        KeyCode::PageDown => Some(Command::PageDown),
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
    "Up/Down: move | Right: open | Left: change database | Esc: exit"
}

/// Help text for the row grid footer.
pub fn help_for_grid() -> &'static str {
    "Up/Down: scroll | PgUp/PgDn: 50 rows | Shift+arrows: column+sort | Left: back | Esc: exit"
}

/// Help text while typing a directory path in the picker.
pub fn help_for_picker_input() -> &'static str {
    "type a directory | Backspace: delete | Enter: scan | Esc: exit"
}

/// Help text while choosing a database in the picker.
pub fn help_for_picker_list() -> &'static str {
    "Up/Down: move | PgUp/PgDn: 50 | Right: open | Left: change directory | Esc: exit"
}
