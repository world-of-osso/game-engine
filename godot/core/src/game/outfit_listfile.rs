use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

static MODEL_PATHS: OnceLock<Mutex<HashMap<PathBuf, Arc<HashMap<u32, String>>>>> = OnceLock::new();

fn read_model_paths(data_root: &Path) -> Result<HashMap<u32, String>, String> {
    let path = data_root.join("community-listfile.csv");
    let file =
        std::fs::File::open(&path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let mut models = HashMap::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|err| format!("read {}: {err}", path.display()))?;
        let Some((id, name)) = line.split_once(';') else {
            continue;
        };
        let is_model = name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("m2"));
        if is_model {
            if let Ok(fdid) = id.parse() {
                models.insert(fdid, name.to_owned());
            }
        }
    }
    Ok(models)
}

fn load_model_paths(data_root: &Path) -> Result<Arc<HashMap<u32, String>>, String> {
    let cache = MODEL_PATHS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .map_err(|err| format!("lock model listfile: {err}"))?;
    if let Some(paths) = cache.get(data_root) {
        return Ok(Arc::clone(paths));
    }
    let paths = Arc::new(read_model_paths(data_root)?);
    cache.insert(data_root.to_path_buf(), Arc::clone(&paths));
    Ok(paths)
}

pub(crate) fn find_fdids_by_name(
    data_root: &Path,
    model_name: &str,
) -> Result<HashSet<u32>, String> {
    let paths = load_model_paths(data_root)?;
    Ok(paths
        .iter()
        .filter_map(|(&fdid, path)| {
            let name = Path::new(path).file_name()?.to_str()?;
            name.eq_ignore_ascii_case(model_name).then_some(fdid)
        })
        .collect())
}
