use crate::db::discover::sqlite_files;
use crate::store::RecentPaths;
use crate::ui::Action;
use crate::ui::bindings::{self, Command};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::path::{Path, PathBuf};

/// How many rows one page key jumps in the database list.
const PAGE_STEP: isize = 50;

/// Where the picker is in its flow.
enum PickerState {
    /// Waiting for the user to type a directory path.
    DirectoryInput,
    /// Showing the sqlite databases found in the entered directory.
    DatabaseList,
}

/// The opening screen: enter a directory, then choose a database to view.
pub struct Picker {
    state: PickerState,
    /// The directory path the user typed.
    input: String,
    /// Databases found in the current directory.
    results: Vec<PathBuf>,
    /// Relative display labels for `results`, built once when the scan runs.
    labels: Vec<String>,
    /// Index of the highlighted database in `results`.
    selected: usize,
    /// Index of the first row shown in the database list (its scroll position).
    scroll_offset: usize,
    /// Number of rows that fit in the list area, refreshed on every draw.
    list_height: usize,
    /// Message shown under the input (guidance, errors, or counts).
    status: Option<String>,
    /// The recent directories, loaded from disk and saved on each new scan.
    recents: RecentPaths,
    /// Index of the highlighted recent directory, when one is selected.
    recent_selected: Option<usize>,
}

impl Picker {
    /// Build a fresh picker in the directory input state.
    pub fn new() -> Self {
        Self {
            state: PickerState::DirectoryInput,
            input: String::new(),
            results: Vec::new(),
            labels: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            list_height: 10,
            status: Some("Paste a directory path, then press Enter.".to_string()),
            recents: RecentPaths::load(),
            recent_selected: None,
        }
    }

    /// True while the user is editing the directory path.
    pub fn is_typing(&self) -> bool {
        matches!(self.state, PickerState::DirectoryInput)
    }

    /// Help text for the current sub-state, shown in the app footer.
    pub fn footer_help(&self) -> &'static str {
        match self.state {
            PickerState::DirectoryInput => bindings::help_for_picker_input(),
            PickerState::DatabaseList => bindings::help_for_picker_list(),
        }
    }

    /// Show a message (for example when opening a database fails).
    pub fn report(&mut self, message: String) {
        self.status = Some(message);
    }

    /// Handle one command. Returns an action only when the command leaves this
    /// screen. Key-to-command mapping lives in the bindings module.
    pub fn handle(&mut self, command: Command) -> Option<Action> {
        match command {
            Command::Type(ch) => {
                self.input.push(ch);
                self.status = None;
                // Typing returns focus from a recent entry back to the input.
                self.recent_selected = None;
                None
            }
            Command::EraseChar => {
                self.input.pop();
                self.status = None;
                self.recent_selected = None;
                None
            }
            Command::Open => match self.state {
                PickerState::DirectoryInput => self.scan_directory(),
                PickerState::DatabaseList => self
                    .results
                    .get(self.selected)
                    .map(|path| Action::PickDatabase(path.display().to_string())),
            },
            Command::OpenRecent => match self.state {
                PickerState::DirectoryInput => self.open_recent(),
                PickerState::DatabaseList => None,
            },
            Command::Back => match self.state {
                PickerState::DatabaseList => {
                    self.state = PickerState::DirectoryInput;
                    self.status = None;
                    None
                }
                PickerState::DirectoryInput => None,
            },
            Command::MoveUp => {
                match self.state {
                    PickerState::DirectoryInput => self.move_recent(-1),
                    PickerState::DatabaseList => self.move_selection(-1),
                }
                None
            }
            Command::MoveDown => {
                match self.state {
                    PickerState::DirectoryInput => self.move_recent(1),
                    PickerState::DatabaseList => self.move_selection(1),
                }
                None
            }
            Command::SortColumnLeft
            | Command::SortColumnRight
            | Command::OrderToggle
            | Command::StartSearch => None,
            Command::PageUp => {
                self.jump_selection(-PAGE_STEP, true);
                None
            }
            Command::PageDown => {
                self.jump_selection(PAGE_STEP, false);
                None
            }
            Command::Quit => Some(Action::Quit),
        }
    }

    /// Scan the typed directory and move to the list when it finds databases.
    fn scan_directory(&mut self) -> Option<Action> {
        let dir = self.input.trim();
        if dir.is_empty() {
            self.status = Some("Enter a directory path first.".to_string());
            return None;
        }
        match sqlite_files(Path::new(dir)) {
            Ok(found) if found.is_empty() => {
                self.status = Some(format!("No sqlite databases found in {}.", dir));
                None
            }
            Ok(found) => {
                self.results = found;
                // Build the relative display labels once. The list reuses them
                // on every draw, so it never strips prefixes again.
                let root = Path::new(dir);
                self.labels = self
                    .results
                    .iter()
                    .map(|path| {
                        path.strip_prefix(root)
                            .unwrap_or(path)
                            .display()
                            .to_string()
                    })
                    .collect();
                self.selected = 0;
                self.scroll_offset = 0;
                self.state = PickerState::DatabaseList;
                self.status = None;
                // Remember the scanned directory so it can be reused later.
                self.recents.add(dir);
                self.recent_selected = None;
                None
            }
            Err(err) => {
                self.status = Some(format!("{err:#}"));
                None
            }
        }
    }

    /// Reopen the highlighted recent directory by rescanning it. When no
    /// recent is highlighted, nothing happens.
    fn open_recent(&mut self) -> Option<Action> {
        let index = self.recent_selected?;
        let path = self.recents.entries().get(index).cloned()?;
        self.input = path;
        self.scan_directory()
    }

    /// Move the recent-directory highlight by `delta`, clamped to the list.
    fn move_recent(&mut self, delta: isize) {
        let len = self.recents.entries().len();
        if len == 0 {
            self.recent_selected = None;
            return;
        }
        let current = self.recent_selected.unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, len as isize - 1);
        self.recent_selected = Some(next as usize);
    }

    fn move_selection(&mut self, delta: isize) {
        if self.results.is_empty() {
            return;
        }
        let next = self.selected as isize + delta;
        self.selected = next.clamp(0, self.results.len() as isize - 1) as usize;
        self.keep_selected_visible();
    }

    /// Jump the selection by a page and anchor it to the top or bottom row.
    fn jump_selection(&mut self, delta: isize, anchor_top: bool) {
        if self.results.is_empty() {
            return;
        }
        let len = self.results.len();
        let next = self.selected as isize + delta;
        self.selected = next.clamp(0, len as isize - 1) as usize;
        let height = self.list_height.max(1);
        let max_offset = len.saturating_sub(height);
        self.scroll_offset = if anchor_top {
            self.selected.min(max_offset)
        } else {
            (self.selected + 1).saturating_sub(height).min(max_offset)
        };
    }

    /// Move the list so the highlight stays inside the visible area, scrolling
    /// only when the highlight would leave it. This mirrors the row grid.
    fn keep_selected_visible(&mut self) {
        if self.results.is_empty() {
            self.scroll_offset = 0;
            return;
        }
        let height = self.list_height.max(1);
        let max_offset = self.results.len().saturating_sub(height);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + height {
            self.scroll_offset = self.selected + 1 - height;
        }
        self.scroll_offset = self.scroll_offset.clamp(0, max_offset);
    }

    /// Draw the recent directories as a selectable list. The highlighted entry
    /// is reopened with Right.
    fn draw_recents(&mut self, area: Rect, frame: &mut Frame<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let entries = self.recents.entries();
        let items: Vec<ListItem> = entries
            .iter()
            .map(|path| ListItem::new(Text::raw(path.clone())))
            .collect();
        let title = format!(" Recent directories ({}) ", entries.len());
        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).title(title));
        let mut state = ListState::default();
        state.select(self.recent_selected);
        frame.render_stateful_widget(list, area, &mut state);
    }

    /// Draw the plain-text logo, centred in the available empty area.
    fn draw_logo(&mut self, area: Rect, frame: &mut Frame<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let art = crate::ui::logo::logo();
        let art_rows = art.lines.len() as u16;
        let top_pad = if art_rows < area.height {
            (area.height - art_rows) / 2
        } else {
            0
        };
        let left_pad = if (art.width as u16) < area.width {
            (area.width - art.width as u16) / 2
        } else {
            0
        };

        let mut lines: Vec<Line> = (0..top_pad).map(|_| Line::raw("")).collect();
        for line in &art.lines {
            lines.push(Line::raw(format!(
                "{}{}",
                " ".repeat(left_pad as usize),
                line
            )));
        }
        frame.render_widget(Paragraph::new(Text::from(lines)), area);
    }

    /// Draw the picker into the given area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let title = match self.state {
            PickerState::DatabaseList => format!(" Databases in {} ", self.input),
            PickerState::DirectoryInput => " Open SQLite database ".to_string(),
        };

        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        frame.render_widget(Paragraph::new(" Database directory:"), rows[0]);

        let input_style = Style::default().fg(Color::Cyan);
        frame.render_widget(
            Paragraph::new(self.input.clone()).style(input_style),
            rows[1],
        );
        if self.is_typing() {
            let x = rows[1].x + (self.input.chars().count() as u16);
            frame.set_cursor(x.min(rows[1].right().saturating_sub(1)), rows[1].y);
        }

        let status_style = Style::default().fg(Color::Yellow);
        frame.render_widget(
            Paragraph::new(self.status.clone().unwrap_or_default()).style(status_style),
            rows[2],
        );

        match self.state {
            PickerState::DirectoryInput => {
                // Keep the logo. When recent directories exist, reserve a
                // bordered list at the bottom and give the rest to the logo.
                let count = self.recents.entries().len();
                let list_height = if count == 0 {
                    0
                } else {
                    (count + 2).min(rows[3].height as usize) as u16
                };
                let parts = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(list_height)])
                    .split(rows[3]);
                self.draw_logo(parts[0], frame);
                if list_height > 0 {
                    self.draw_recents(parts[1], frame);
                }
            }
            PickerState::DatabaseList => {
                if self.results.is_empty() {
                    frame.render_widget(
                        Paragraph::new("No sqlite databases found in this directory."),
                        rows[3],
                    );
                    return;
                }
                let list_title = format!(" {} database(s) found ", self.results.len());
                let inner_height = rows[3].height.saturating_sub(2) as usize;
                self.list_height = inner_height.max(1);
                let len = self.results.len();
                let start = self.scroll_offset.min(len);
                let take = inner_height.min(len - start);
                let visible: Vec<ListItem> = self
                    .labels
                    .iter()
                    .skip(start)
                    .take(take)
                    .map(|label| ListItem::new(Text::raw(label.clone())))
                    .collect();
                let list = List::new(visible)
                    .highlight_symbol("> ")
                    .highlight_style(
                        Style::default()
                            .bg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    )
                    .block(Block::default().title(list_title));
                let mut state = ListState::default();
                if let Some(local) = self.selected.checked_sub(start) {
                    state.select(Some(local));
                }
                frame.render_stateful_widget(list, rows[3], &mut state);
            }
        }
    }
}
