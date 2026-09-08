use crate::db::{Column, Database};
use crate::ui::Action;
use crate::ui::bindings::Command;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

/// How many rows the grid buffers around the cursor at once.
const PAGE_ROWS: i64 = 300;
/// Number of rows to keep loaded above the cursor as a scroll margin.
const PAGE_MARGIN: i64 = PAGE_ROWS / 2;
/// How many rows one page key jumps at a time.
const PAGE_JUMP: isize = 50;

/// Direction of the current sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    /// Smallest values first (oldest first for a time column).
    Asc,
    /// Largest values first (newest first for a time column).
    Desc,
}

impl Order {
    fn marker(self) -> &'static str {
        match self {
            Order::Asc => " ^",
            Order::Desc => " v",
        }
    }
}

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
    /// Index of the column chosen as the sort key.
    sort_col: usize,
    /// The active sort direction, if sorting is on.
    order: Option<Order>,
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
            sort_col: 0,
            order: None,
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
            Command::SortColumnLeft => {
                self.move_sort_col(db, -1);
                None
            }
            Command::SortColumnRight => {
                self.move_sort_col(db, 1);
                None
            }
            Command::OrderAsc => {
                self.set_order(db, Order::Asc);
                None
            }
            Command::OrderDesc => {
                self.set_order(db, Order::Desc);
                None
            }
            Command::PageUp => {
                self.page_up(db);
                None
            }
            Command::PageDown => {
                self.page_down(db);
                None
            }
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

    /// Move the highlight by one row and keep it inside the visible window.
    fn move_rows(&mut self, db: &Database, delta: isize) {
        let next = (self.cursor + delta as i64).clamp(0, (self.total - 1).max(0));
        if next == self.cursor {
            return;
        }
        self.cursor = next;
        self.reload(db);
        self.keep_cursor_visible();
    }

    /// Jump 50 rows up and pin the highlight to the top row of the view.
    fn page_up(&mut self, db: &Database) {
        if self.total <= 0 {
            return;
        }
        self.cursor = (self.cursor - PAGE_JUMP as i64).max(0);
        self.reload(db);
        let local = self.local_row();
        self.scroll_top = local.min(self.buffer.len().saturating_sub(1));
    }

    /// Jump 50 rows down and pin the highlight to the bottom row of the view.
    fn page_down(&mut self, db: &Database) {
        if self.total <= 0 {
            return;
        }
        self.cursor = (self.cursor + PAGE_JUMP as i64).min(self.total - 1);
        self.reload(db);
        let height = self.viewport_rows.max(1);
        let local = self.local_row();
        let max_scroll = self.buffer.len().saturating_sub(1);
        self.scroll_top = (local + 1).saturating_sub(height).min(max_scroll);
    }

    /// Index of the highlight within the loaded buffer, clamped to the buffer.
    fn local_row(&self) -> usize {
        let local = (self.cursor - self.window_start).max(0) as usize;
        local.min(self.buffer.len().saturating_sub(1))
    }

    /// Move the chosen sort column left or right. Re-sorts if sorting is on.
    fn move_sort_col(&mut self, db: &Database, delta: isize) {
        if self.columns.is_empty() {
            return;
        }
        let next = self.sort_col as isize + delta;
        self.sort_col = next.clamp(0, self.columns.len() as isize - 1) as usize;
        if self.order.is_some() {
            self.reset_to_top(db);
        }
    }

    /// Set the sort direction for the chosen column and show the sorted top.
    fn set_order(&mut self, db: &Database, direction: Order) {
        if self.columns.is_empty() {
            return;
        }
        self.order = Some(direction);
        self.reset_to_top(db);
    }

    /// Clear the loaded window and go back to the first sorted row.
    fn reset_to_top(&mut self, db: &Database) {
        self.cursor = 0;
        self.window_start = 0;
        self.scroll_top = 0;
        self.buffer.clear();
        self.reload(db);
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
        self.buffer = self.query_rows(db, start).unwrap_or_default();
        self.window_start = start;
        self.scroll_top = 0;
    }

    /// Fetch one window, ordered by the active sort when one is set.
    fn query_rows(&self, db: &Database, start: i64) -> anyhow::Result<Vec<Vec<String>>> {
        match self.order {
            Some(Order::Asc) | Some(Order::Desc) => {
                let column = self
                    .columns
                    .get(self.sort_col)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                let ascending = self.order == Some(Order::Asc);
                db.rows_sorted(&self.table, column, ascending, start, PAGE_ROWS)
            }
            None => db.rows(&self.table, start, PAGE_ROWS),
        }
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

        let base_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let chosen_style = base_style.bg(Color::LightBlue);

        let header_cells: Vec<Cell> = self
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let mut text = column.name.clone();
                if index == self.sort_col {
                    if let Some(direction) = self.order {
                        text.push_str(direction.marker());
                    }
                    Cell::from(text).style(chosen_style)
                } else {
                    Cell::from(text).style(base_style)
                }
            })
            .collect();

        // Show the buffer from `scroll_top`, so the highlight stays in view.
        // Build Row objects only for the visible slice; rows below the fold
        // would be clipped anyway, so cloning them is wasted work.
        let view_start = self.scroll_top.min(self.buffer.len());
        let view_end = (view_start + self.viewport_rows.max(1)).min(self.buffer.len());
        let rows: Vec<Row> = self.buffer[view_start..view_end]
            .iter()
            .map(|cells| Row::new(cells.clone()))
            .collect();

        let local = (self.cursor - self.window_start) as usize;
        let widths: Vec<Constraint> =
            std::iter::repeat_n(Constraint::Fill(1), self.columns.len().max(1)).collect();

        let title = format!(" {} — {} rows ", self.table, self.total);

        let table = Table::new(rows, widths)
            .header(Row::new(header_cells))
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
