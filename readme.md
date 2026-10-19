
## Repo Overview
R-SQLite is a TUI SQLite database viewer written in Rust. The user will be able to view all contents of a database.


## Core Features
- Open a SQLite database file (read-only)
- List all tables (and other schema objects) in the database
- Browse the rows of a selected table in a read-only grid
- Show table structure in the grid (column names and types)
- Search across the content of the current table
- View most recent paths scanned


## Read-Only Constraint
The tool is a viewer and an analysis tool. It never can modify the database. SQLite is opened in read-only mode.


## Tech Stack 
| Component | Technology |
|---|---|
| Language | Rust |
| TUI Framework | ratatui |
| Terminal Backend | crossterm |
| SQLite Access | rusqlite |


## Screenshots

![R-SQLITE](screenshots/view1.2.png)

![R-SQLITE](screenshots/Screenshot_2026-09-09_00-48-48.png)


## How to Run

- Binary releases are currently available only for Linux x86_64 systems.
- Place the binary in a directory and make it executable.
- Optionally, create a desktop entry so you can launch it with a click.
- Alternatively, run it from a terminal in several ways.