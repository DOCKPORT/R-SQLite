use crate::db::{Column, Database};
use crate::ui::tui::{self, TuiTerminal};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, TableState};

/// How many rows the data view buffers around the cursor at once.
const PAGE_ROWS: i64 = 300;
/// Number of rows to keep loaded above the cursor as a scroll margin.
const PAGE_MARGIN: i64 = PAGE_ROWS / 2;

/// Which screen the application is currently showing.
enum View {
    /// The list of tables and views.
    Tables,
    /// The row grid of one opened table.
    Data(DataView),
}

/// State for browsing the rows of one table.
struct DataView {
    /// Name of the table being viewed.
    table: String,
    /// Column metadata shown in the header row.
    columns: Vec<Column>,
    /// Total number of rows in the table.
    total: i64,
    /// Absolute index of the highlighted row across the whole table.
    cursor: i64,
    /// Absolute index of the first row currently loaded in `buffer`.
    window_start: i64,
    /// The loaded window of rows (text cells).
    buffer: Vec<Vec<String>>,
    /// Index within `buffer` of the first visible row.
    scroll_top: usize,
    /// Rows that fit in the data area, used to keep the highlight visible.
    viewport_rows: usize,
}

/// Top-level application state and its event loop.
pub struct App {
    db: Database,
    tables: Vec<String>,
    list_index: usize,
    db_path: String,
    should_quit: bool,
    view: View,
}

impl App {
    /// Build the application state for the opened database.
    pub fn new(db: Database, db_path: String) -> Self {
        let tables = db.table_names().unwrap_or_default();
        Self {
            db,
            tables,
            list_index: 0,
            db_path,
            should_quit: false,
            view: View::Tables,
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
                match (&mut self.view, key.code) {
                    (View::Tables, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
                    (View::Tables, KeyCode::Down | KeyCode::Char('j')) => self.list_move(1),
                    (View::Tables, KeyCode::Up | KeyCode::Char('k')) => self.list_move(-1),
                    (View::Tables, KeyCode::Enter) => self.open_selected(),
                    (View::Data(_), KeyCode::Char('q')) => self.should_quit = true,
                    (View::Data(_), KeyCode::Esc | KeyCode::Backspace | KeyCode::Left) => {
                        self.view = View::Tables;
                    }
                    (View::Data(_), KeyCode::Down | KeyCode::Char('j')) => self.data_move(1),
                    (View::Data(_), KeyCode::Up | KeyCode::Char('k')) => self.data_move(-1),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn list_move(&mut self, delta: isize) {
        if self.tables.is_empty() {
            return;
        }
        let new_index = self.list_index as isize + delta;
        self.list_index = new_index.clamp(0, self.tables.len() as isize - 1) as usize;
    }

    /// Open the highlighted table into a row grid.
    fn open_selected(&mut self) {
        let Some(name) = self.tables.get(self.list_index).cloned() else {
            return;
        };
        let Ok(columns) = self.db.columns(&name) else {
            return;
        };
        let total = self.db.row_count(&name).unwrap_or(0);

        let mut view = DataView {
            table: name,
            columns,
            total,
            cursor: 0,
            window_start: 0,
            buffer: Vec::new(),
            scroll_top: 0,
            viewport_rows: 20,
        };
        reload_window(&self.db, &mut view);
        self.view = View::Data(view);
    }

    /// Move the row highlight by `delta`, paging in rows as needed.
    fn data_move(&mut self, delta: isize) {
        let View::Data(view) = &mut self.view else {
            return;
        };
        let next = (view.cursor + delta as i64).clamp(0, (view.total - 1).max(0));
        if next == view.cursor {
            return;
        }
        view.cursor = next;
        reload_window(&self.db, view);
        keep_cursor_visible(view);
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

        match &mut self.view {
            View::Tables => render_table_list(frame, chunks[1], &self.tables, self.list_index),
            View::Data(view) => render_data(frame, chunks[1], view),
        }

        let footer = match &self.view {
            View::Tables => format!(
                " {} object(s) | j/k: move | Enter: open | q: quit ",
                self.tables.len()
            ),
            View::Data(view) => format!(
                " table {} | row {}/{} | j/k: scroll | Esc: back | q: quit ",
                view.table,
                view.cursor + 1,
                view.total
            ),
        };
        frame.render_widget(Paragraph::new(Text::raw(footer)), chunks[2]);
    }
}

/// Reload the buffer when the cursor moves outside the loaded window.
fn reload_window(db: &Database, view: &mut DataView) {
    let buffer_len = view.buffer.len() as i64;
    let inside = view.cursor >= view.window_start && view.cursor < view.window_start + buffer_len;
    if view.total == 0 || (inside && !view.buffer.is_empty()) {
        return;
    }
    let max_start = (view.total - 1).max(0);
    let start = (view.cursor - PAGE_MARGIN).clamp(0, max_start);
    view.buffer = db.rows(&view.table, start, PAGE_ROWS).unwrap_or_default();
    view.window_start = start;
    view.scroll_top = 0;
}

/// Keep the highlighted row inside the visible part of the data area.
fn keep_cursor_visible(view: &mut DataView) {
    let height = view.viewport_rows.max(1);
    let local = (view.cursor - view.window_start) as usize;
    if local < view.scroll_top {
        view.scroll_top = local;
    } else if local >= view.scroll_top + height {
        view.scroll_top = local + 1 - height;
    }
}

/// Draw the list of tables.
fn render_table_list(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    tables: &[String],
    list_index: usize,
) {
    let items: Vec<ListItem> = tables
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let style = if index == list_index {
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

    let title = if tables.is_empty() {
        " Tables (no tables or views) "
    } else {
        " Tables "
    };
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, area);
}

/// Draw the row grid for an opened table.
fn render_data(frame: &mut Frame<'_>, area: ratatui::layout::Rect, view: &mut DataView) {
    // Rows available below the top border and the header row.
    view.viewport_rows = area.height.saturating_sub(3) as usize;

    let header_cells: Vec<String> = view
        .columns
        .iter()
        .map(|column| match &column.decl_type {
            Some(ty) if !ty.is_empty() => format!("{} ({})", column.name, ty),
            _ => column.name.clone(),
        })
        .collect();

    // Show the buffer from `scroll_top`, so the highlight stays in view.
    let view_start = view.scroll_top.min(view.buffer.len());
    let rows: Vec<Row> = view
        .buffer
        .iter()
        .skip(view_start)
        .map(|cells| Row::new(cells.clone()))
        .collect();

    let local = (view.cursor - view.window_start) as usize;
    let widths: Vec<Constraint> = std::iter::repeat(Constraint::Fill(1))
        .take(view.columns.len().max(1))
        .collect();

    let title = format!(" {} — {} rows ", view.table, view.total);

    let table = Table::new(rows, widths)
        .header(
            Row::new(header_cells).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
        .column_spacing(2)
        .block(Block::default().borders(Borders::ALL).title(title));

    let mut state = TableState::default();
    state.select(local.checked_sub(view_start));
    frame.render_stateful_widget(table, area, &mut state);
}
