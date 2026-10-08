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
    #[cfg(test)]
    count_staged_build(cache_path);
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

#[cfg(test)]
static STAGED_BUILDS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
fn count_staged_build(cache_path: &Path) {
    STAGED_BUILDS.lock().unwrap().push(cache_path.to_path_buf());
}

/// How many times a cache at `cache_path` has been built into a staging file.
#[cfg(test)]
pub(crate) fn staged_builds(cache_path: &Path) -> usize {
    STAGED_BUILDS
        .lock()
        .unwrap()
        .iter()
        .filter(|built| built.as_path() == cache_path)
        .count()
}

#[cfg(test)]
type StaleBarriers = Vec<(PathBuf, std::sync::Arc<std::sync::Barrier>)>;

#[cfg(test)]
static STALE_BARRIERS: std::sync::Mutex<StaleBarriers> = std::sync::Mutex::new(Vec::new());

/// Makes the next `importers` callers that find the cache at `cache_path` missing or stale
/// wait for each other before any of them rebuilds it.
#[cfg(test)]
pub(crate) fn hold_stale_observers(cache_path: &Path, importers: usize) {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(importers));
    STALE_BARRIERS
        .lock()
        .unwrap()
        .push((cache_path.to_path_buf(), barrier));
}

#[cfg(test)]
pub(crate) fn release_stale_observers(cache_path: &Path) {
    STALE_BARRIERS
        .lock()
        .unwrap()
        .retain(|(held, _)| held.as_path() != cache_path);
}

/// Called by an importer right after it found the cache missing or stale.
#[cfg(test)]
pub(crate) fn observed_stale(cache_path: &Path) {
    let barrier = STALE_BARRIERS
        .lock()
        .unwrap()
        .iter()
        .find(|(held, _)| held.as_path() == cache_path)
        .map(|(_, barrier)| barrier.clone());
    if let Some(barrier) = barrier {
        barrier.wait();
    }
}
