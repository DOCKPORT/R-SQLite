# Product Context: r-sqlite

## Why This Project Exists
SQLite databases are common and often inspected with GUI tools or the sqlite3 command line. The user wants a fast, terminal-native way to open a database and look around inside it: see what tables exist, inspect rows, and analyse the content. r-sqlite provides that in a Ratatui TUI that runs in the terminal.

## Problems It Solves
- Quick visual inspection of a SQLite database without a GUI
- Browsing tables and rows directly in the terminal
- Viewing all content of a database in one place
- Searching within table content

## How It Should Work
- The user launches the tool with a database path: `r-sqlite path/to/database.db`
- The tool opens the database read-only and shows a sidebar of tables
- The user selects a table and browses its rows in a grid
- The user can search within the selected table
- The user quits cleanly, returning the terminal to its normal state

## User Experience Goals
- Terminal-native and fast
- Read-only by design; editing is never an option
- Clear navigation between tables and rows
- Works on large tables through virtualization, not by loading all rows at once
- Clean terminal state on exit

## Non-Goals
- Editing, inserting, or deleting database content
- Replacing the full sqlite3 CLI
- Schema migration tooling
