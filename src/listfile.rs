use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const COMMUNITY_LISTFILE_PATH: &str = "data/community-listfile.csv";
const LOCAL_CACHE_PATH: &str = "data/local-listfile-cache.csv";

static LISTFILE: OnceLock<Listfile> = OnceLock::new();

#[derive(Default)]
struct CachedListfile {
    by_fdid: HashMap<u32, &'static str>,
    by_path: HashMap<String, u32>,
}

struct Listfile {
    community_path: PathBuf,
    local_cache_path: PathBuf,
    local: Mutex<CachedListfile>,
    community: OnceLock<CachedListfile>,
}

/// Get the global listfile index. Loads only the small local cache immediately;
/// falls back to the full community listfile on first unresolved lookup.
fn get() -> &'static Listfile {
    LISTFILE.get_or_init(|| {
        Listfile::new(
            PathBuf::from(COMMUNITY_LISTFILE_PATH),
            PathBuf::from(LOCAL_CACHE_PATH),
        )
    })
}

impl Listfile {
    fn new(community_path: PathBuf, local_cache_path: PathBuf) -> Self {
        let local = match load_listfile(&local_cache_path) {
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
            local_cache_path,
            local: Mutex::new(local),
            community: OnceLock::new(),
        }
    }

    fn community(&self) -> &CachedListfile {
        self.community
            .get_or_init(|| match load_listfile(&self.community_path) {
                Ok(listfile) => {
                    eprintln!("Loaded listfile: {} entries", listfile.by_fdid.len());
                    listfile
                }
                Err(err) => {
                    eprintln!("Failed to load listfile: {err}");
                    CachedListfile::default()
                }
            })
    }

    fn lookup_fdid(&self, fdid: u32) -> Option<&'static str> {
        if let Some(path) = self.local.lock().unwrap().by_fdid.get(&fdid).copied() {
            return Some(path);
        }
        self.community().by_fdid.get(&fdid).copied()
    }

    fn lookup_path(&self, path: &str) -> Option<u32> {
        let normalized = path.to_ascii_lowercase();
        if let Some(fdid) = self.local.lock().unwrap().by_path.get(&normalized).copied() {
            return Some(fdid);
        }
        let fdid = self.community().by_path.get(&normalized).copied()?;
        let path = self
            .community()
            .by_fdid
            .get(&fdid)
            .copied()
            .unwrap_or_else(|| {
                let leaked = Box::leak(path.to_owned().into_boxed_str());
                leaked as &'static str
            });
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
        if let Err(err) = append_cache_entry(&self.local_cache_path, fdid, path) {
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

fn load_listfile(csv_path: &Path) -> Result<CachedListfile, String> {
    match std::fs::read_to_string(csv_path) {
        Ok(data) => Ok(parse_listfile(&data)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(CachedListfile::default()),
        Err(err) => Err(format!("read {}: {err}", csv_path.display())),
    }
}

fn parse_listfile(data: &str) -> CachedListfile {
    let mut by_fdid = HashMap::new();
    let mut by_path = HashMap::new();

    for line in data.lines() {
        let Some((fdid_str, path)) = line.split_once(';') else {
            continue;
        };
        let Ok(fdid) = fdid_str.parse::<u32>() else {
            continue;
        };
        let leaked = Box::leak(path.to_owned().into_boxed_str()) as &'static str;
        by_fdid.insert(fdid, leaked);
        by_path.insert(path.to_ascii_lowercase(), fdid);
    }

    CachedListfile { by_fdid, by_path }
}

fn append_cache_entry(cache_path: &Path, fdid: u32, path: &str) -> Result<(), String> {
    let Some(parent) = cache_path.parent() else {
        return Err(format!("missing parent for {}", cache_path.display()));
    };
    std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cache_path)
        .map_err(|e| format!("open {}: {e}", cache_path.display()))?;
    writeln!(file, "{fdid};{path}").map_err(|e| format!("write {}: {e}", cache_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_lookup_results_in_local_cache() {
        let test_dir = Path::new("target/test-artifacts/listfile");
        std::fs::create_dir_all(test_dir).unwrap();
        let community = test_dir.join("community.csv");
        let local = test_dir.join("local-cache.csv");
        let _ = std::fs::remove_file(&local);
        std::fs::write(&community, "123;world/maps/test/test_1_2.adt\n").unwrap();

        let listfile = Listfile::new(community.clone(), local.clone());
        assert_eq!(
            listfile.lookup_path("world/maps/test/test_1_2.adt"),
            Some(123)
        );
        assert_eq!(
            listfile.lookup_fdid(123),
            Some("world/maps/test/test_1_2.adt")
        );

        let persisted = std::fs::read_to_string(&local).unwrap();
        assert_eq!(persisted, "123;world/maps/test/test_1_2.adt\n");

        std::fs::remove_file(&community).unwrap();
        let cached_only = Listfile::new(community, local.clone());
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
    fn lookup_fdid_does_not_persist_broad_reverse_scans() {
        let test_dir = Path::new("target/test-artifacts/listfile");
        std::fs::create_dir_all(test_dir).unwrap();
        let community = test_dir.join("community-fdid-only.csv");
        let local = test_dir.join("local-cache-fdid-only.csv");
        let _ = std::fs::remove_file(&local);
        std::fs::write(&community, "456;creature/test/test.m2\n").unwrap();

        let listfile = Listfile::new(community.clone(), local.clone());
        assert_eq!(listfile.lookup_fdid(456), Some("creature/test/test.m2"));
        assert!(
            !local.exists(),
            "reverse lookups should not grow the local cache"
        );

        let _ = std::fs::remove_file(community);
        let _ = std::fs::remove_file(local);
    }
}
