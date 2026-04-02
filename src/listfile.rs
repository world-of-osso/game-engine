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
        let local = match listfile_cache::load_local_cache(&local_cache_path) {
            Ok(cache) => cache,
            Err(err) => {
                eprintln!("Failed to load local listfile cache: {err}");
                CachedListfile::default()
            }
        };
        eprintln!(
            "Loaded local listfile cache: {} entries",
            local.by_fdid.len()
        );
        Self {
            community_path,
            community_cache_path,
            local_cache_path,
            local: Mutex::new(local),
        }
    }

    fn lookup_fdid(&self, fdid: u32) -> Option<&'static str> {
        if let Some(path) = self.local.lock().unwrap().by_fdid.get(&fdid).copied() {
            return Some(path);
        }
        let path = match listfile_cache::lookup_fdid(
            &self.community_cache_path,
            &self.community_path,
            fdid,
        ) {
            Ok(Some(path)) => Box::leak(path.into_boxed_str()) as &'static str,
            Ok(None) => return None,
            Err(err) => {
                eprintln!("Failed listfile fdid lookup {fdid}: {err}");
                return None;
            }
        };
        self.remember(fdid, path);
        Some(path)
    }

    fn lookup_path(&self, path: &str) -> Option<u32> {
        let normalized = path.to_ascii_lowercase();
        if let Some(fdid) = self.local.lock().unwrap().by_path.get(&normalized).copied() {
            return Some(fdid);
        }
        let (fdid, resolved_path) = match listfile_cache::lookup_path(
            &self.community_cache_path,
            &self.community_path,
            path,
        ) {
            Ok(Some(row)) => row,
            Ok(None) => return None,
            Err(err) => {
                eprintln!("Failed listfile path lookup `{path}`: {err}");
                return None;
            }
        };
        let path = Box::leak(resolved_path.into_boxed_str()) as &'static str;
        self.remember(fdid, path);
        Some(fdid)
    }

    fn remember(&self, fdid: u32, path: &'static str) {
        let normalized = path.to_ascii_lowercase();
        let mut local = self.local.lock().unwrap();
        let known_fdid = local.by_fdid.contains_key(&fdid);
        let known_path = local.by_path.contains_key(&normalized);
        if known_fdid && known_path {
            return;
        }
        local.by_fdid.entry(fdid).or_insert(path);
        local.by_path.entry(normalized).or_insert(fdid);
        if let Err(err) =
            listfile_cache::remember_local_cache_entry(&self.local_cache_path, fdid, path)
        {
            eprintln!("Failed to persist listfile cache entry {fdid}: {err}");
        }
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
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn persists_lookup_results_in_local_cache() {
        let test_dir = Path::new("target/test-artifacts/listfile");
        std::fs::create_dir_all(test_dir).unwrap();
        let community = test_dir.join("community.csv");
        let local = test_dir.join("local-cache.sqlite");
        let _ = std::fs::remove_file(&local);
        std::fs::write(&community, "123;world/maps/test/test_1_2.adt\n").unwrap();

        let community_cache = test_dir.join("community.sqlite");
        let listfile = Listfile::new(community.clone(), community_cache.clone(), local.clone());
        assert_eq!(
            listfile.lookup_path("world/maps/test/test_1_2.adt"),
            Some(123)
        );
        assert_eq!(
            listfile.lookup_fdid(123),
            Some("world/maps/test/test_1_2.adt")
        );

        let persisted = listfile_cache::load_local_cache(&local).unwrap();
        assert_eq!(
            persisted.by_fdid.get(&123).copied(),
            Some("world/maps/test/test_1_2.adt")
        );

        std::fs::remove_file(&community).unwrap();
        let cached_only = Listfile::new(community, community_cache, local.clone());
        assert_eq!(
            cached_only.lookup_path("world/maps/test/test_1_2.adt"),
            Some(123)
        );
        assert_eq!(
            cached_only.lookup_fdid(123),
            Some("world/maps/test/test_1_2.adt")
        );

        let _ = std::fs::remove_file(local);
    }

    #[test]
    fn lookup_fdid_persists_reverse_lookup_results_in_local_cache() {
        let test_dir = Path::new("target/test-artifacts/listfile");
        std::fs::create_dir_all(test_dir).unwrap();
        let community = test_dir.join("community-fdid-only.csv");
        let local = test_dir.join("local-cache-fdid-only.sqlite");
        let _ = std::fs::remove_file(&local);
        std::fs::write(&community, "456;creature/test/test.m2\n").unwrap();

        let community_cache = test_dir.join("community-fdid-only.sqlite");
        let listfile = Listfile::new(community.clone(), community_cache.clone(), local.clone());
        assert_eq!(listfile.lookup_fdid(456), Some("creature/test/test.m2"));
        let persisted = listfile_cache::load_local_cache(&local).unwrap();
        assert_eq!(
            persisted.by_fdid.get(&456).copied(),
            Some("creature/test/test.m2")
        );

        std::fs::remove_file(&community).unwrap();
        let cached_only = Listfile::new(community, community_cache, local.clone());
        assert_eq!(cached_only.lookup_fdid(456), Some("creature/test/test.m2"));
        assert_eq!(cached_only.lookup_path("creature/test/test.m2"), Some(456));

        let _ = std::fs::remove_file(local);
    }
}
