## Repo Overview
R-SQLite is a terminal (TUI) SQLite database viewer written in Rust. It opens a SQLite database file and lets the user explore its content. The user can list the tables, browse the rows of a selected table, and view all content of the database.

## Core Features
- Open a SQLite database file (read-only)
- List all tables (and other schema objects) in the database
- Browse the rows of a selected table in a read-only grid
- Show table structure in the grid (column names and types)
- Search across the content of the current table
- Handle large tables without loading everything into memory at once
- Quit, scroll, and resize cleanly

## Read-Only Constraint
The tool is a viewer and an analysis tool. It never can modify the database. SQLite is opened in read-only mode.


## Tech Stack 
| Component | Technology |
|---|---|
| Language | Rust |
| TUI Framework | ratatui |
| Terminal Backend | crossterm |
| SQLite Access | rusqlite |

