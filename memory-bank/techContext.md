# Tech Context: r-sqlite

## Technologies Used (proposed, pending design approval)
| Technology | Purpose |
|---|---|
| Rust (edition 2024) | Core language |
| ratatui | Terminal UI framework |
| crossterm | Terminal backend and input handling |
| rusqlite (bundled SQLite) | SQLite access in read-only mode |

## Development Setup
- **Build**: `cargo build` / `cargo run`
- **Run**: `cargo run -- path/to/database.db`
- **Project root**: `/home/dockport/DOCK-HQ/DEV/r-sqlite`
- **Dependencies file**: `Cargo.toml` (currently empty)

## Technical Constraints
- **Rust edition 2024**: new edition; verify crate compatibility
- **rusqlite bundled**: compiles SQLite from source, so no system SQLite dependency
- **Read-only DB**: SQLite must open with `SQLITE_OPEN_READ_ONLY`
- **Terminal app**: design for resize and clean exit (restore terminal state)
- **Large tables**: use virtualized or paged row loading, never materialize whole tables

## Dependencies Needed (to add once design is approved)
- `ratatui` — TUI widgets and layout
- `crossterm` — terminal backend and events
- `rusqlite` with `bundled` feature — SQLite access
- `anyhow` (candidate) — error handling
- `clap` (candidate) — command line argument parsing for the database path

## Tool Usage Patterns
- Build commands: `cargo build`, `cargo run`, `cargo check`
- No test harness in the source tree; test new code in a separate scratch area
- Keep UI modules modular, one file per major view
