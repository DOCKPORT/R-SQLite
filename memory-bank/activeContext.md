# Active Context: r-sqlite

## Current Work Focus
Bootstrap the project foundation: memory bank and a written architecture plan. No TUI code is written yet.

## Recent Changes
- Created `.clinerules/MemoryBank.md` matching the convention used by sibling projects (TallyBook, halvora, silo)
- Created the `memory-bank/` folder with projectbrief, productContext, techContext, and this file
- Created the `docs/` folder for the architecture plan

## Active Decisions and Considerations
- The foundation design lives in `docs/architecture.md` (pending user review)
- Proposed stack: ratatui + crossterm + rusqlite (bundled), edition 2024
- Database must open read-only; the tool never edits
- Foundation scope: open file, table sidebar, row grid, text search
- Deferred: ad-hoc SQL panel, schema detail views
- Do NOT write TUI code until the user reviews and approves `docs/architecture.md`

## Next Steps
1. Finish `docs/architecture.md` and commit it
2. Ask the user to review and approve the architecture
3. Only after approval, plan implementation (crates, module layout, event loop, first view)

## Important Patterns and Preferences
- Do not add test code to the source tree
- Do not run `cargo fmt`; the user formats Rust code themselves
- Keep responses and documentation in plain, clear language
- Match the memory bank conventions of sibling projects
