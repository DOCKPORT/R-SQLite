use crate::db::Database;
use crate::ui::tui::{self, TuiTerminal};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

/// Top-level application state and its event loop.
pub struct App {
    /// Names of the tables and views in the opened database.
    tables: Vec<String>,
    /// Currently highlighted row in the table list.
    selected: usize,
    /// Database path, shown in the header.
    db_path: String,
    /// Set when the user asks to quit.
    should_quit: bool,
}

impl App {
    /// Build the application state for the opened database.
    pub fn new(db: &Database, db_path: String) -> Self {
        let tables = db.table_names().unwrap_or_default();
        Self {
            tables,
            selected: 0,
            db_path,
            should_quit: false,
        }
    }

    /// Run the draw / input loop until the user quits, then restore the terminal.
    pub fn run(&mut self, mut terminal: TuiTerminal) -> Result<()> {
        let result = self.run_loop(&mut terminal);
        let restore = tui::restore(terminal);
        match (result, restore) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(err), _) => Err(err),
            (Ok(()), Err(err)) => Err(err),
        }
    }

    fn run_loop(&mut self, terminal: &mut TuiTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    /// Read one input event and update the state.
    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.modifiers == KeyModifiers::NONE {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                    KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn move_down(&mut self) {
        if !self.tables.is_empty() && self.selected + 1 < self.tables.len() {
            self.selected += 1;
        }
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Draw the current frame.
    fn render(&mut self, frame: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.size());

        let header = format!(" r-sqlite  {}", self.db_path);
        frame.render_widget(Paragraph::new(Text::raw(header)), chunks[0]);

        let items: Vec<ListItem> = self
            .tables
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let style = if index == self.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Text::raw(name.clone())).style(style)
            })
            .collect();

        let title = if self.tables.is_empty() {
            " Tables (no tables or views) "
        } else {
            " Tables "
        };
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(list, chunks[1]);

        let footer = format!(
            " {} object(s) | j/k or arrows: move | q: quit ",
            self.tables.len()
        );
        frame.render_widget(Paragraph::new(Text::raw(footer)), chunks[2]);
    }
}
