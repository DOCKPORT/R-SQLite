# Foundation Architecture: r-sqlite

Status: pending review (2026-09-05). No code is written from this document until it is approved.

## 1. Goal
r-sqlite is a read-only terminal viewer for SQLite databases. The user opens a database file and explores its content: listing tables, browsing the rows of a selected table, and searching within that content. The tool never modifies the database.

## 2. Read-Only Constraint
Open SQLite with the read-only flags (`SQLITE_OPEN_READ_ONLY`). Design every query path on that assumption. Do not expose write operations.

## 3. Recommended Stack
| Component | Choice | Reason |
|---|---|---|
| Language | Rust, edition 2024 | Project default, matches siblings |
| TUI | ratatui | Mature, declarative widgets |
| Backend | crossterm | Standard input/event/backend for ratatui |
| SQLite | rusqlite (feature `bundled`) | Compiles SQLite in, no system dependency |
| Errors | anyhow | Low-friction context-rich errors |
| Args | clap | Clean `r-sqlite <path>` parsing |

## 4. Proposed Module Layout
```
src/
├── main.rs          # entry point: parse args, open DB, run the app
├── app.rs           # application state and event loop
├── db/
│   └── database.rs  # read-only connection, table listing, row queries, search
├── ui/
│   ├── tui.rs       # terminal setup/teardown, crossterm raw mode
│   ├── layout.rs    # split the screen into regions
│   ├── table_list.rs# sidebar list of tables
│   └── row_grid.rs  # main grid of rows for the selected table
```

### Responsibilities
- `db::database` — owns the rusqlite `Connection`; exposes functions to list tables, fetch schema metadata, and fetch a paged window of rows for a table, plus a text search over a table. Pure query logic, no terminal code.
- `app.rs` — holds current state (path, selected table, focus, scroll offsets, search term) and runs the event loop: read input, update state, request a redraw.
- `ui` — renders widgets from state and wires terminal setup/teardown.
- `main.rs` — reads the CLI path, builds the app, runs it.

## 5. Foundation Milestone Scope
Deliverables in order:

1. Read-only open: accept `r-sqlite <path>`; fail clearly if the file is not a valid SQLite database.
2. Table sidebar: list tables (and, minimally, views) from the schema. Keyboard navigation moves the selection.
3. Row grid: for the selected table, show a grid with column headers and types. Rows load in windows; the grid queries only the visible range, so large tables stay responsive.
4. Text search: a search input filters or locates rows within the current table.
5. Terminal hygiene: raw mode entered on start and exited on quit, correct restore on error, resize handled, clear quit key.
6. Read-only guarantee enforced by the connection flags.

## 6. Virtualization Approach
Do not `SELECT *` a whole table into memory. Fetch a small window around the current scroll position, sized to the visible rows plus a margin, using `LIMIT`/`OFFSET`. Re-query when the scroll position moves. The grid only ever holds a page of rows.

## 7. Deferred Features (out of foundation)
- Ad-hoc read-only SQL input panel for typed queries
- Schema detail pane (columns, types, indexes, primary and foreign keys)
- Multiple open databases / cross-table analysis
- Bookmarking, export, or formatting niceties

These stay out of the foundation so the browse core is solid first.

## 8. Risks and Open Questions
- **ratatui version and API** — pin a recent stable version; check edition 2024 compatibility.
- **rusqlite bundled build time** — the first compile compiles SQLite; acceptable for a desktop tool.
- **Search semantics** — plain substring scan over the selected table is the simplest first pass; decide whether it must filter the grid or only find the next match.
- **BLOB and large cell display** — cells must render truncated; decide a max display width.
- **Table without an `ORDER BY`** — SQLite row order is undefined without one; decide whether pagination uses `rowid`/primary key ordering.

## 9. Success Criteria for the Foundation
- Open any SQLite file and see its tables listed.
- Select a table and scroll through its rows with headers shown.
- Search within a table and move to results.
- Open a multi-gigabyte table file without freezing and without loading all rows.
- Quit always restores the terminal.
- No code path can write to the database.
