use crate::db::{Column, Database};
use crate::ui::Action;
use crate::ui::bindings::Command;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Row, Table, TableState};

/// How many rows the grid buffers around the cursor at once.
const PAGE_ROWS: i64 = 300;
/// Number of rows to keep loaded above the cursor as a scroll margin.
const PAGE_MARGIN: i64 = PAGE_ROWS / 2;

/// The row grid for one opened table.
pub struct TableGrid {
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

impl TableGrid {
    /// Open a table into a row grid, loading the first window of rows.
    pub fn new(db: &Database, table: String) -> Self {
        let columns = db.columns(&table).unwrap_or_default();
        let total = db.row_count(&table).unwrap_or(0);
        let mut grid = Self {
            table,
            columns,
            total,
            cursor: 0,
            window_start: 0,
            buffer: Vec::new(),
            scroll_top: 0,
            viewport_rows: 20,
        };
        grid.reload(db);
        grid
    }

    /// Handle one command. Returns an action only when the command leaves the
    /// screen. Key-to-command mapping lives in the bindings module.
    pub fn handle(&mut self, db: &Database, command: Command) -> Option<Action> {
        match command {
            Command::MoveUp => {
                self.move_rows(db, -1);
                None
            }
            Command::MoveDown => {
                self.move_rows(db, 1);
                None
            }
            Command::Open => None,
            Command::Back => Some(Action::BackToList),
            Command::Type(_) | Command::EraseChar => None,
            Command::Quit => Some(Action::Quit),
        }
    }

    /// Name of the table being viewed.
    pub fn name(&self) -> &str {
        &self.table
    }

    /// Zero-based absolute row index of the highlight.
    pub fn cursor(&self) -> i64 {
        self.cursor
    }

    /// Total number of rows in the table.
    pub fn total(&self) -> i64 {
        self.total
    }

    fn move_rows(&mut self, db: &Database, delta: isize) {
        let next = (self.cursor + delta as i64).clamp(0, (self.total - 1).max(0));
        if next == self.cursor {
            return;
        }
        self.cursor = next;
        self.reload(db);
        self.keep_cursor_visible();
    }

    /// Reload the buffer when the cursor moves outside the loaded window.
    fn reload(&mut self, db: &Database) {
        let buffer_len = self.buffer.len() as i64;
        let inside =
            self.cursor >= self.window_start && self.cursor < self.window_start + buffer_len;
        if self.total == 0 || (inside && !self.buffer.is_empty()) {
            return;
        }
        let max_start = (self.total - 1).max(0);
        let start = (self.cursor - PAGE_MARGIN).clamp(0, max_start);
        self.buffer = db.rows(&self.table, start, PAGE_ROWS).unwrap_or_default();
        self.window_start = start;
        self.scroll_top = 0;
    }

    /// Keep the highlighted row inside the visible part of the data area.
    fn keep_cursor_visible(&mut self) {
        let height = self.viewport_rows.max(1);
        let local = (self.cursor - self.window_start) as usize;
        if local < self.scroll_top {
            self.scroll_top = local;
        } else if local >= self.scroll_top + height {
            self.scroll_top = local + 1 - height;
        }
    }

    /// Draw the row grid into the given area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        // Rows available below the top border and the header row.
        self.viewport_rows = area.height.saturating_sub(3) as usize;

        let header_cells: Vec<String> = self
            .columns
            .iter()
            .map(|column| match &column.decl_type {
                Some(ty) if !ty.is_empty() => format!("{} ({})", column.name, ty),
                _ => column.name.clone(),
            })
            .collect();

        // Show the buffer from `scroll_top`, so the highlight stays in view.
        let view_start = self.scroll_top.min(self.buffer.len());
        let rows: Vec<Row> = self
            .buffer
            .iter()
            .skip(view_start)
            .map(|cells| Row::new(cells.clone()))
            .collect();

        let local = (self.cursor - self.window_start) as usize;
        let widths: Vec<Constraint> =
            std::iter::repeat_n(Constraint::Fill(1), self.columns.len().max(1)).collect();

        let title = format!(" {} — {} rows ", self.table, self.total);

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
}
