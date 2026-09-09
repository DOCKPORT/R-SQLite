use crate::ui::Action;
use crate::ui::bindings::Command;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem};

/// The sidebar list of tables and views.
pub struct TableList {
    tables: Vec<String>,
    list_index: usize,
}

impl TableList {
    /// Build the list from the given table names.
    pub fn new(tables: Vec<String>) -> Self {
        Self {
            tables,
            list_index: 0,
        }
    }

    /// Number of items shown in the list.
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    /// Handle one command. Returns an action only when the command leaves the
    /// screen. Key-to-command mapping lives in the bindings module.
    pub fn handle(&mut self, command: Command) -> Option<Action> {
        match command {
            Command::MoveUp => {
                self.move_selection(-1);
                None
            }
            Command::MoveDown => {
                self.move_selection(1);
                None
            }
            Command::Open => self
                .tables
                .get(self.list_index)
                .cloned()
                .map(Action::OpenTable),
            Command::Back => Some(Action::BackToPicker),
            Command::Type(_) | Command::EraseChar | Command::StartSearch => None,
            Command::SortColumnLeft
            | Command::SortColumnRight
            | Command::OrderToggle
            | Command::PageUp
            | Command::PageDown => None,
            Command::Quit => Some(Action::Quit),
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.tables.is_empty() {
            return;
        }
        let next = self.list_index as isize + delta;
        self.list_index = next.clamp(0, self.tables.len() as isize - 1) as usize;
    }

    /// Draw the list into the given area.
    pub fn draw(&self, frame: &mut Frame<'_>, area: Rect) {
        let items: Vec<ListItem> = self
            .tables
            .iter()
            .map(|name| ListItem::new(Text::raw(name.clone())))
            .collect();

        let title = if self.tables.is_empty() {
            " Tables (no tables or views) "
        } else {
            " Tables "
        };
        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).title(title));

        let mut state = ratatui::widgets::ListState::default();
        state.select(Some(self.list_index));
        frame.render_stateful_widget(list, area, &mut state);
    }
}
