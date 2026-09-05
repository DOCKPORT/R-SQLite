use crate::db::Database;
use crate::ui::Action;
use crate::ui::bindings;
use crate::ui::grid::TableGrid;
use crate::ui::tables::TableList;
use crate::ui::tui::{self, TuiTerminal};
use anyhow::Result;
use crossterm::event::{self, Event, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

/// The application shell: owns the database, switches between screens, and
/// runs the draw/input loop. Each screen holds its own state and drawing.
pub struct App {
    db: Database,
    db_path: String,
    /// The table list. It stays alive so its selection survives navigation.
    list: TableList,
    /// The open row grid, if a table is being viewed.
    grid: Option<TableGrid>,
    should_quit: bool,
}

impl App {
    /// Build the application shell for the opened database.
    pub fn new(db: Database, db_path: String) -> Self {
        let tables = db.table_names().unwrap_or_default();
        Self {
            db,
            db_path,
            list: TableList::new(tables),
            grid: None,
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

    /// Read one input event, map it to a command, and dispatch to the screen.
    fn handle_events(&mut self) -> Result<()> {
        let code = match event::read()? {
            Event::Key(key) if key.modifiers == KeyModifiers::NONE => key.code,
            _ => return Ok(()),
        };

        let Some(command) = bindings::command_for_key(code) else {
            return Ok(());
        };

        let action = match self.grid.as_mut() {
            Some(grid) => grid.handle(&self.db, command),
            None => self.list.handle(command),
        };

        self.apply(action);
        Ok(())
    }

    /// Carry out an action returned by the active screen.
    fn apply(&mut self, action: Option<Action>) {
        let Some(action) = action else {
            return;
        };
        match action {
            Action::Quit => self.should_quit = true,
            Action::BackToList => self.grid = None,
            Action::OpenTable(name) => {
                self.grid = Some(TableGrid::new(&self.db, name));
            }
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

        let header = format!(" r-sqlite  {}", self.db_path);
        frame.render_widget(Paragraph::new(Text::raw(header)), chunks[0]);

        match self.grid.as_mut() {
            Some(grid) => grid.draw(frame, chunks[1]),
            None => self.list.draw(frame, chunks[1]),
        }

        let footer = self.footer_text();
        frame.render_widget(Paragraph::new(Text::raw(footer)), chunks[2]);
    }

    fn footer_text(&self) -> String {
        match &self.grid {
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
        }
    }
}
