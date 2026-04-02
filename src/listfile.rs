use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[path = "listfile_cache.rs"]
mod listfile_cache;

static LISTFILE: OnceLock<Listfile> = OnceLock::new();

#[derive(Default)]
struct CachedListfile {
    by_fdid: HashMap<u32, &'static str>,
    by_path: HashMap<String, u32>,
}

struct Listfile {
    community_path: PathBuf,
    community_cache_path: PathBuf,
    local_cache_path: PathBuf,
    local: Mutex<CachedListfile>,
}

/// Get the global listfile index. Loads only the small local cache immediately;
/// falls back to the full community listfile on first unresolved lookup.
fn get() -> &'static Listfile {
    LISTFILE.get_or_init(|| {
        Listfile::new(
            crate::paths::resolve_data_path("community-listfile.csv"),
            listfile_cache::cache_path(),
            crate::paths::shared_data_path("local-listfile-cache.sqlite"),
        )
    })
}

impl Listfile {
    fn new(
        community_path: PathBuf,
        community_cache_path: PathBuf,
        local_cache_path: PathBuf,
    ) -> Self {
        Self {
            community_path,
            community_cache_path,
            local_cache_path,
            local: Mutex::new(CachedListfile::default()),
        }
    }

    fn lookup_fdid(&self, fdid: u32) -> Option<&'static str> {
        if let Some(path) = self.local.lock().unwrap().by_fdid.get(&fdid).copied() {
            return Some(path);
        }
        if let Some(path) = self.lookup_local_fdid(fdid) {
            return Some(path);
        }
        self.lookup_community_fdid(fdid)
    }

    fn lookup_community_fdid(&self, fdid: u32) -> Option<&'static str> {
        let path = match listfile_cache::lookup_fdid(
            &self.community_cache_path,
            &self.community_path,
            fdid,
        ) {
            Ok(path) => path?,
            Err(err) => {
                eprintln!("Failed listfile fdid lookup {fdid}: {err}");
                return None;
            }
        };
        let leaked = Box::leak(path.into_boxed_str()) as &'static str;
        self.remember(fdid, leaked);
        Some(leaked)
    }

    fn lookup_path(&self, path: &str) -> Option<u32> {
        let normalized = path.to_ascii_lowercase();
        if let Some(fdid) = self.local.lock().unwrap().by_path.get(&normalized).copied() {
            return Some(fdid);
        }
        if let Some(fdid) = self.lookup_local_path(path) {
            return Some(fdid);
        }
        self.lookup_community_path(path)
    }

    fn lookup_community_path(&self, path: &str) -> Option<u32> {
        let (fdid, resolved_path) = self.resolve_community_path(path)?;
        let path = Box::leak(resolved_path.into_boxed_str()) as &'static str;
        self.remember(fdid, path);
        Some(fdid)
    }

    fn resolve_community_path(&self, path: &str) -> Option<(u32, String)> {
        match listfile_cache::lookup_path(&self.community_cache_path, &self.community_path, path) {
            Ok(row) => row,
            Err(err) => {
                eprintln!("Failed listfile path lookup `{path}`: {err}");
                None
            }
        }
    }

    fn lookup_local_fdid(&self, fdid: u32) -> Option<&'static str> {
        let path = match listfile_cache::lookup_local_fdid(&self.local_cache_path, fdid) {
            Ok(path) => path?,
            Err(err) => {
                eprintln!("Failed local listfile fdid lookup {fdid}: {err}");
                return None;
            }
        };
        let leaked = Box::leak(path.into_boxed_str()) as &'static str;
        self.remember_in_memory(fdid, leaked);
        Some(leaked)
    }

    fn lookup_local_path(&self, path: &str) -> Option<u32> {
        let (fdid, resolved_path) =
            match listfile_cache::lookup_local_path(&self.local_cache_path, path) {
                Ok(row) => row?,
                Err(err) => {
                    eprintln!("Failed local listfile path lookup `{path}`: {err}");
                    return None;
                }
            };
        let leaked = Box::leak(resolved_path.into_boxed_str()) as &'static str;
        self.remember_in_memory(fdid, leaked);
        Some(fdid)
    }

    fn remember(&self, fdid: u32, path: &'static str) {
        if self.remember_in_memory(fdid, path) {
            return;
        }
        if let Err(err) =
            listfile_cache::remember_local_cache_entry(&self.local_cache_path, fdid, path)
        {
            eprintln!("Failed to persist listfile cache entry {fdid}: {err}");
        }
    }

    fn remember_in_memory(&self, fdid: u32, path: &'static str) -> bool {
        let normalized = path.to_ascii_lowercase();
        let mut local = self.local.lock().unwrap();
        let known_fdid = local.by_fdid.contains_key(&fdid);
        let known_path = local.by_path.contains_key(&normalized);
        if known_fdid && known_path {
            return true;
        }
        local.by_fdid.entry(fdid).or_insert(path);
        local.by_path.entry(normalized).or_insert(fdid);
        false
    }
}

/// Look up a WoW internal path by FileDataID.
pub fn lookup_fdid(fdid: u32) -> Option<&'static str> {
    get().lookup_fdid(fdid)
}

/// Look up a FileDataID by WoW internal path (case-insensitive).
pub fn lookup_path(path: &str) -> Option<u32> {
    get().lookup_path(path)
}

#[cfg(test)]
#[path = "../tests/unit/listfile_tests.rs"]
mod tests;
