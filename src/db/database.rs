use anyhow::{Context, Result};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, params};
use std::path::Path;

/// A single column of a table: its name and declared SQLite type.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub decl_type: Option<String>,
}

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

    /// Return the columns of a table in declaration order.
    pub fn columns(&self, table: &str) -> Result<Vec<Column>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, type FROM pragma_table_info(?1) ORDER BY cid")?;
        let cols = stmt.query_map(params![table], |row| {
            Ok(Column {
                name: row.get(0)?,
                decl_type: row.get(1)?,
            })
        })?;
        Ok(cols.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Return the number of rows in a table.
    pub fn row_count(&self, table: &str) -> Result<i64> {
        let sql = format!("SELECT count(*) FROM {}", quote_ident(table));
        self.conn
            .query_row(&sql, [], |row| row.get::<_, i64>(0))
            .map_err(Into::into)
    }

    /// Fetch a window of `limit` rows starting at `offset`, rendered as text.
    ///
    /// Only this small window is materialized, so large tables stay cheap.
    pub fn rows(&self, table: &str, offset: i64, limit: i64) -> Result<Vec<Vec<String>>> {
        let sql = format!("SELECT * FROM {} LIMIT ?1 OFFSET ?2", quote_ident(table));
        let mut stmt = self.conn.prepare(&sql)?;
        let column_count = stmt.column_count();
        let mut query = stmt.query(params![limit, offset])?;

        let mut out = Vec::new();
        while let Some(row) = query.next()? {
            let mut cells = Vec::with_capacity(column_count);
            for index in 0..column_count {
                cells.push(cell_text(row.get_ref(index)?));
            }
            out.push(cells);
        }
        Ok(out)
    }
}

/// Render a single SQLite value as display text for the grid.
fn cell_text(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(b) => format!("<{} bytes>", b.len()),
    }
}

/// Quote an identifier for safe interpolation into a `FROM` clause.
fn quote_ident(name: &str) -> String {
    let mut quoted = String::with_capacity(name.len() + 2);
    quoted.push('"');
    for ch in name.chars() {
        if ch == '"' {
            quoted.push('"');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}
