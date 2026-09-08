use crate::db::Database;
use crate::ui::Action;
use crate::ui::bindings::{self, Command};
use crate::ui::grid::TableGrid;
use crate::ui::picker::Picker;
use crate::ui::tables::TableList;
use crate::ui::tui::{self, TuiTerminal};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use std::path::Path;

/// The application shell. Before a database is open it shows the directory
/// picker; once a database is open it switches between the table list and the
/// row grid. Each screen holds its own state and drawing.
pub struct App {
    /// The open database while browsing; None while the picker is showing.
    db: Option<Database>,
    /// Display path of the open database, for the header.
    db_path: String,
    /// The opening directory and database picker.
    picker: Picker,
    /// The table list of the open database.
    list: TableList,
    /// The open row grid, if a table is being viewed.
    grid: Option<TableGrid>,
    should_quit: bool,
}

impl App {
    /// Build the application shell, starting at the directory picker.
    pub fn new() -> Self {
        Self {
            db: None,
            db_path: String::new(),
            picker: Picker::new(),
            list: TableList::new(Vec::new()),
            grid: None,
            should_quit: false,
        }
    }

    /// Open `path` now and show its table list. Used when a database path is
    /// given on the command line, skipping the picker.
    pub fn open_database(&mut self, path: &Path) -> Result<()> {
        let db = Database::open_read_only(path)?;
        self.enter_browse(db, path.display().to_string());
        Ok(())
    }

    /// Open a database chosen in the picker, or show the error back in it.
    fn open_from_picker(&mut self, path: &str) {
        match Database::open_read_only(Path::new(path)) {
            Ok(db) => self.enter_browse(db, path.to_string()),
            Err(err) => self
                .picker
                .report(format!("Cannot open database {}: {err:#}", path)),
        }
    }

    /// Move into the browse phase for an already opened database.
    fn enter_browse(&mut self, db: Database, path: String) {
        let tables = db.table_names().unwrap_or_default();
        self.list = TableList::new(tables);
        self.db_path = path;
        self.grid = None;
        self.db = Some(db);
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
        // Draw once, then redraw only when an event actually changed something.
        // This avoids rebuilding the frame (and re-allocating its text) for
        // ignored events such as dropped Ctrl/Alt keys or stray mouse moves.
        let mut needs_redraw = true;
        while !self.should_quit {
            if needs_redraw {
                terminal.draw(|frame| self.render(frame))?;
            }
            needs_redraw = self.handle_events()?;
        }
        Ok(())
    }

    /// Read one input event, map it to a command, and dispatch to the screen.
    /// Return whether the frame changed and must be redrawn.
    fn handle_events(&mut self) -> Result<bool> {
        match event::read()? {
            // A size change requires a full redraw to re-layout the screens.
            Event::Resize(_, _) => Ok(true),
            Event::Key(key) => {
                // Accept plain keys and shifted text (for example capital
                // letters, which carry the SHIFT modifier). Drop Ctrl/Alt
                // combinations so that only ordinary typing and navigation
                // reach the screens.
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    return Ok(false);
                }

                let Some(command) = self.map_command(key.code, key.modifiers) else {
                    return Ok(false);
                };

                let action = if let Some(db) = self.db.as_ref() {
                    match self.grid.as_mut() {
                        Some(grid) => grid.handle(db, command),
                        None => self.list.handle(command),
                    }
                } else {
                    self.picker.handle(command)
                };

                self.apply(action);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Choose which key mapping applies. While the picker is accepting a typed
    /// directory, characters and editing keys go through the text mapping so
    /// that, for example, `x` types a letter instead of quitting.
    fn map_command(&self, code: KeyCode, mods: KeyModifiers) -> Option<Command> {
        if self.db.is_none() && self.picker.is_typing() {
            bindings::command_for_text_key(code, mods)
        } else {
            bindings::command_for_key(code, mods)
        }
    }

    /// Carry out an action returned by the active screen.
    fn apply(&mut self, action: Option<Action>) {
        let Some(action) = action else {
            return;
        };
        match action {
            Action::Quit => self.should_quit = true,
            Action::BackToList => self.grid = None,
            Action::BackToPicker => {
                self.db = None;
                self.grid = None;
            }
            Action::OpenTable(name) => {
                if let Some(db) = self.db.as_ref() {
                    self.grid = Some(TableGrid::new(db, name));
                }
            }
            Action::PickDatabase(path) => self.open_from_picker(&path),
        }
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

        let header = match &self.db {
            Some(_) => format!(" r-sqlite  {}", self.db_path),
            None => "R-SQLite".to_string(),
        };
        frame.render_widget(Paragraph::new(Text::raw(header)), chunks[0]);

        if self.db.is_some() {
            match self.grid.as_mut() {
                Some(grid) => grid.draw(frame, chunks[1]),
                None => self.list.draw(frame, chunks[1]),
            }
        } else {
            self.picker.draw(frame, chunks[1]);
        }

        let footer = self.footer_text();
        frame.render_widget(Paragraph::new(Text::raw(footer)), chunks[2]);
    }

    fn footer_text(&self) -> String {
        match &self.db {
            None => format!(" {} ", self.picker.footer_help()),
            Some(_) => match &self.grid {
                Some(grid) => format!(
                    " table {} | row {}/{} | {} ",
                    grid.name(),
                    grid.cursor() + 1,
                    grid.total(),
                    bindings::help_for_grid()
                ),
                None => format!(
                    " {} object(s) | {} ",
                    self.list.len(),
                    bindings::help_for_list()
                ),
            },
        }
    }
}
