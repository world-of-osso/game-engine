use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, OpenFlags};

pub fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", path.display()))
}

/// Builds a cache in a private staging file and renames it over `cache_path`. Concurrent
/// rebuilders never write the same SQLite file, and readers only ever open a complete cache.
pub fn replace_atomically(
    cache_path: &Path,
    build: impl FnOnce(&Connection) -> Result<(), String>,
) -> Result<(), String> {
    let staging = staging_path(cache_path)?;
    let result = build_staged(&staging, build).and_then(|()| {
        std::fs::rename(&staging, cache_path).map_err(|err| {
            format!(
                "rename {} to {}: {err}",
                staging.display(),
                cache_path.display()
            )
        })
    });
    if let Err(err) = result {
        return Err(match std::fs::remove_file(&staging) {
            Ok(()) => err,
            Err(remove) if remove.kind() == std::io::ErrorKind::NotFound => err,
            Err(remove) => format!("{err}; remove {}: {remove}", staging.display()),
        });
    }
    Ok(())
}

fn staging_path(cache_path: &Path) -> Result<PathBuf, String> {
    static NEXT_STAGING_ID: AtomicU64 = AtomicU64::new(0);
    let (Some(parent), Some(name)) = (cache_path.parent(), cache_path.file_name()) else {
        return Err(format!(
            "cache path {} has no file name",
            cache_path.display()
        ));
    };
    std::fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    Ok(parent.join(format!(
        "{}.{}-{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed)
    )))
}

fn build_staged(
    staging: &Path,
    build: impl FnOnce(&Connection) -> Result<(), String>,
) -> Result<(), String> {
    let conn =
        Connection::open(staging).map_err(|err| format!("open {}: {err}", staging.display()))?;
    build(&conn)?;
    conn.close()
        .map_err(|(_, err)| format!("close {}: {err}", staging.display()))
}
