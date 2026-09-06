mod app;
mod db;
mod ui;

use crate::app::App;
use crate::ui::tui;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Read-only SQLite database viewer.
#[derive(Parser)]
#[command(name = "r-sqlite", version, about = "Read-only SQLite database viewer")]
struct Args {
    /// Path to a SQLite database file to open directly, skipping the picker.
    #[arg(value_name = "DATABASE")]
    db_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut app = App::new();
    if let Some(path) = &args.db_path {
        app.open_database(path)?;
    }

    let terminal = tui::init()?;
    app.run(terminal)
}
