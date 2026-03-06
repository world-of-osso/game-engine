use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

static LISTFILE: OnceLock<Listfile> = OnceLock::new();

struct Listfile {
    by_fdid: HashMap<u32, String>,
    by_path: HashMap<String, u32>,
}

/// Get the global listfile, loading lazily from `data/community-listfile.csv`.
fn get() -> &'static Listfile {
    LISTFILE.get_or_init(
        || match load_listfile(Path::new("data/community-listfile.csv")) {
            Ok(lf) => {
                eprintln!("Loaded listfile: {} entries", lf.by_fdid.len());
                lf
            }
            Err(e) => {
                eprintln!("Failed to load listfile: {e}");
                Listfile {
                    by_fdid: HashMap::new(),
                    by_path: HashMap::new(),
                }
            }
        },
    )
}

/// Look up a WoW internal path by FileDataID.
pub fn lookup_fdid(fdid: u32) -> Option<&'static str> {
    get().by_fdid.get(&fdid).map(|s| s.as_str())
}

/// Look up a FileDataID by WoW internal path (case-insensitive).
pub fn lookup_path(path: &str) -> Option<u32> {
    get().by_path.get(&path.to_ascii_lowercase()).copied()
}

/// Parse `data/community-listfile.csv` (format: `FDID;path`).
fn load_listfile(csv_path: &Path) -> Result<Listfile, String> {
    let data = std::fs::read_to_string(csv_path)
        .map_err(|e| format!("read {}: {e}", csv_path.display()))?;

    let mut by_fdid = HashMap::with_capacity(1_500_000);
    let mut by_path = HashMap::with_capacity(1_500_000);

    for line in data.lines() {
        let Some((fdid_str, path)) = line.split_once(';') else {
            continue;
        };
        let Ok(fdid) = fdid_str.parse::<u32>() else {
            continue;
        };
        by_fdid.insert(fdid, path.to_owned());
        by_path.insert(path.to_ascii_lowercase(), fdid);
    }

    Ok(Listfile { by_fdid, by_path })
}
