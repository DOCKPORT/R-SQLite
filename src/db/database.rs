use anyhow::{Context, Result};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, params, params_from_iter};
use std::path::Path;

/// A single column of a table: its name.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
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
            .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")?;
        let cols = stmt.query_map(params![table], |row| Ok(Column { name: row.get(0)? }))?;
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
        self.fetch_window(&sql, offset, limit)
    }

    /// Fetch a window of rows ordered by one column, ascending or descending.
    ///
    /// The ORDER BY applies to the whole table, so the offset indexes into the
    /// sorted result. Only the requested window is materialized.
    pub fn rows_sorted(
        &self,
        table: &str,
        column: &str,
        ascending: bool,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Vec<String>>> {
        let dir = if ascending { "ASC" } else { "DESC" };
        let sql = format!(
            "SELECT * FROM {} ORDER BY {} {} LIMIT ?1 OFFSET ?2",
            quote_ident(table),
            quote_ident(column),
            dir
        );
        self.fetch_window(&sql, offset, limit)
    }

    /// Number of rows whose text contains `term` in any column.
    ///
    /// The match is a case-insensitive substring scan done by SQLite `LIKE`.
    /// When `term` is empty every row matches, mirroring an unfiltered view.
    pub fn search_count(&self, table: &str, term: &str) -> Result<i64> {
        if term.is_empty() {
            return self.row_count(table);
        }
        let (condition, column_count) = self.like_condition(table)?;
        if column_count == 0 {
            return self.row_count(table);
        }
        let sql = format!(
            "SELECT count(*) FROM {} WHERE {}",
            quote_ident(table),
            condition
        );
        let params = vec![like_pattern(term); column_count];
        self.conn
            .query_row(&sql, params_from_iter(params.iter()), |row| {
                row.get::<_, i64>(0)
            })
            .map_err(Into::into)
    }

    /// Fetch a window of rows that contain `term`, in natural order.
    pub fn search_rows(
        &self,
        table: &str,
        term: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Vec<String>>> {
        let (condition, column_count) = self.like_condition(table)?;
        if column_count == 0 || term.is_empty() {
            return self.rows(table, offset, limit);
        }
        let sql = format!(
            "SELECT * FROM {} WHERE {} LIMIT ?{} OFFSET ?{}",
            quote_ident(table),
            condition,
            column_count + 1,
            column_count + 2
        );
        self.fetch_filtered(&sql, term, column_count, offset, limit)
    }

    /// Fetch a window of matching rows ordered by one column.
    ///
    /// The `WHERE` filters first and the `ORDER BY` sorts the filtered set, so
    /// the offset indexes into the matching rows only.
    pub fn search_rows_sorted(
        &self,
        table: &str,
        term: &str,
        column: &str,
        ascending: bool,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Vec<String>>> {
        let (condition, column_count) = self.like_condition(table)?;
        if column_count == 0 || term.is_empty() {
            return self.rows_sorted(table, column, ascending, offset, limit);
        }
        let dir = if ascending { "ASC" } else { "DESC" };
        let sql = format!(
            "SELECT * FROM {} WHERE {} ORDER BY {} {} LIMIT ?{} OFFSET ?{}",
            quote_ident(table),
            condition,
            quote_ident(column),
            dir,
            column_count + 1,
            column_count + 2
        );
        self.fetch_filtered(&sql, term, column_count, offset, limit)
    }

    /// Run a filtered LIMIT/OFFSET window query and render each row as text.
    ///
    /// The filter patterns fill the leading `?` placeholders, then `offset`
    /// and `limit` fill the trailing ones.
    fn fetch_filtered(
        &self,
        sql: &str,
        term: &str,
        column_count: usize,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Vec<String>>> {
        let pattern = like_pattern(term);
        let mut values: Vec<rusqlite::types::Value> = Vec::with_capacity(column_count + 2);
        for _ in 0..column_count {
            values.push(pattern.clone().into());
        }
        values.push(limit.into());
        values.push(offset.into());

        let mut stmt = self.conn.prepare(sql)?;
        let columns = stmt.column_count();
        let mut query = stmt.query(params_from_iter(values.iter()))?;

        let mut out = Vec::with_capacity(limit as usize);
        while let Some(row) = query.next()? {
            let mut cells = Vec::with_capacity(columns);
            for index in 0..columns {
                cells.push(cell_text(row.get_ref(index)?));
            }
            out.push(cells);
        }
        Ok(out)
    }

    /// Build a `col LIKE ? OR col LIKE ? ...` clause for every column of `table`.
    ///
    /// Returns the clause plus the number of columns (one bound pattern each).
    fn like_condition(&self, table: &str) -> Result<(String, usize)> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")?;
        let names = stmt
            .query_map(params![table], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let column_count = names.len();
        let parts: Vec<String> = names
            .iter()
            .map(|name| format!("{} LIKE ? ESCAPE '\\'", quote_ident(name)))
            .collect();
        Ok((parts.join(" OR "), column_count))
    }

    /// Run a LIMIT/OFFSET window query and render each row as text.
    fn fetch_window(&self, sql: &str, offset: i64, limit: i64) -> Result<Vec<Vec<String>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let column_count = stmt.column_count();
        let mut query = stmt.query(params![limit, offset])?;

        // Reserve the full window up front so a complete page avoids repeated
        // reallocation as rows are pushed. `limit` is a small bounded page size.
        let mut out = Vec::with_capacity(limit as usize);
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

/// Wrap a search term as a `LIKE` pattern, escaping `%`, `_`, and `\` so the
/// term is matched literally rather than as wildcards.
fn like_pattern(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len() + 2);
    for ch in term.chars() {
        match ch {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            other => escaped.push(other),
        }
    }
    format!("%{escaped}%")
}
