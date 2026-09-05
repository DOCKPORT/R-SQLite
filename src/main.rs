mod app;
mod db;
mod ui;

use crate::app::App;
use crate::db::Database;
use crate::ui::tui;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Read-only SQLite database viewer.
#[derive(Parser)]
#[command(name = "r-sqlite", version, about = "Read-only SQLite database viewer")]
struct Args {
    /// Path to the SQLite database file to open.
    #[arg(value_name = "DATABASE")]
    db_path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let db = Database::open_read_only(&args.db_path)?;
    let mut app = App::new(&db, args.db_path.display().to_string());

    let terminal = tui::init()?;
    app.run(terminal)
}
