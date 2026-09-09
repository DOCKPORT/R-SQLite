use crate::db::discover::sqlite_files;
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
                None
            }
            Command::EraseChar => {
                self.input.pop();
                self.status = None;
                None
            }
            Command::Open => match self.state {
                PickerState::DirectoryInput => self.scan_directory(),
                PickerState::DatabaseList => self
                    .results
                    .get(self.selected)
                    .map(|path| Action::PickDatabase(path.display().to_string())),
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
                self.move_selection(-1);
                None
            }
            Command::MoveDown => {
                self.move_selection(1);
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
                None
            }
            Err(err) => {
                self.status = Some(format!("{err:#}"));
                None
            }
        }
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
            PickerState::DirectoryInput => self.draw_logo(rows[3], frame),
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
