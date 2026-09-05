# Project Brief: r-sqlite

## Project Overview
r-sqlite is a terminal (TUI) SQLite database viewer written in Rust. It opens a SQLite database file and lets the user explore its content without editing it. The user can list the tables, browse the rows of a selected table, and view all content of the database.

## Core Requirements
- Open a SQLite database file (read-only)
- List all tables (and other schema objects) in the database
- Browse the rows of a selected table in a read-only grid
- Show table structure in the grid (column names and types)
- Search across the content of the current table
- Handle large tables without loading everything into memory at once
- Quit, scroll, and resize cleanly

## Read-Only Constraint
The tool is a viewer and an analysis tool. It must never modify the database. SQLite must be opened in read-only mode.

## Analysis Direction (deferred, not committed)
The user wants to view and analyse database content. Candidate future features include a read-only ad-hoc SQL panel and schema detail views. These are deferred until the browse foundation is approved.

## Tech Stack (proposed, pending design approval)
| Component | Technology |
|---|---|
| Language | Rust (edition 2024) |
| TUI Framework | ratatui |
| Terminal Backend | crossterm |
| SQLite Access | rusqlite (bundled SQLite) |

## Project Structure (target)
```
r-sqlite/
├── Cargo.toml
├── .clinerules/MemoryBank.md   # Session-persistent documentation rules
├── memory-bank/                # Session-persistent documentation
├── docs/architecture.md        # Foundation design (pending review)
├── src/
│   ├── main.rs
│   └── ...                     # modules per approved architecture
```

## Project Status
- Greenfield: Cargo scaffold only, `main.rs` prints "Hello, world!"
- No TUI code written yet
- Next: review `docs/architecture.md`, then plan implementation
