# System Patterns: r-sqlite

## Status
The foundation architecture is being designed in `docs/architecture.md`. This file records patterns once the design is approved and implemented.

## Planned Architecture (to be confirmed)
A Ratatui application following its standard view/update (event loop) model, layered over a read-only rusqlite connection. Proposed high-level responsibilities:

- **App state**: current database path, selected table, current view focus
- **Data layer**: read-only DB access, table listing, schema metadata, paged row queries, text search
- **UI layer**: terminal setup/teardown, Ratatui widget rendering, key input handling
- **Views**: table sidebar list, row grid with column headers, search input
- **Event loop**: handle input, update state, redraw

## Design Patterns (intended)
- Separate the pure DB query logic from the TUI rendering so the data layer is testable without a terminal
- Read-only connection enforced at open time
- Virtualized row grid that queries only the rows currently visible
- Single application state struct that the event loop mutates and the renderer reads

Refer to `docs/architecture.md` for the authoritative, pending design.
