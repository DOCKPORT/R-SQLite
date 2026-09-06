use anyhow::{Result, bail};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Magic bytes at the start of every SQLite database file.
const SQLITE_HEADER: [u8; 16] = *b"SQLite format 3\x00";

/// Walk `dir` and its subfolders for sqlite database files.
///
/// A file counts as a sqlite database when its first 16 bytes match the
/// SQLite file header magic. This ignores companion files such as `-journal`,
/// `-wal`, and `-shm`, and it works regardless of the file extension.
/// Symbolic links are not followed, so directory cycles cannot occur.
pub fn sqlite_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        bail!("cannot read directory {}", dir.display());
    }

    let walker = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.file_type().is_dir() || entry.file_type().is_file());

    let mut found = Vec::new();
    for entry in walker {
        let Ok(entry) = entry else {
            // Skip folders we cannot read; keep scanning the rest.
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        if name.ends_with("-journal") || name.ends_with("-wal") || name.ends_with("-shm") {
            continue;
        }
        if is_sqlite_file(path) {
            found.push(path.to_path_buf());
        }
    }

    found.sort();
    Ok(found)
}

/// Whether `path` starts with the SQLite file header magic.
///
/// A file that cannot be opened is treated as not a database so that one
/// unreadable file does not abort the whole scan.
fn is_sqlite_file(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut header = [0u8; 16];
    if file.read(&mut header).unwrap_or(0) != SQLITE_HEADER.len() {
        return false;
    }
    header == SQLITE_HEADER
}
