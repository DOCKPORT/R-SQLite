use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// A read-only view into a SQLite database file.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open the database at `path` in read-only mode.
    ///
    /// The read-only flag makes any write attempt fail at the SQLite level, so
    /// this viewer can never modify the file.
    pub fn open_read_only(path: &Path) -> Result<Self> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .with_context(|| format!("cannot open database at {}", path.display()))?;

        // Force SQLite to read the file header so a non-SQLite file fails here
        // with a clear message instead of later during the first query.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
            row.get::<_, i64>(0)
        })
        .context("the file is not a valid SQLite database")?;

        Ok(Self { conn })
    }

    /// Return the names of all tables and views in the database.
    ///
    /// Internal SQLite objects whose names start with `sqlite_` are omitted.
    pub fn table_names(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master \
             WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(names)
    }
}
