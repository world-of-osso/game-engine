use std::path::Path;
use std::time::UNIX_EPOCH;

/// Names a cache source by its path relative to `data_dir`, so the recorded key does not
/// depend on how the caller spells the data root (`data`, `godot/rust/../../data`, ...).
pub fn source_key(data_dir: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(data_dir)
        .map(|relative| relative.to_string_lossy().into_owned())
        .map_err(|_| {
            format!(
                "cache source {} is outside {}",
                path.display(),
                data_dir.display()
            )
        })
}

pub fn csv_mtime(path: &Path) -> Result<i64, String> {
    let modified = std::fs::metadata(path)
        .map_err(|err| format!("stat {}: {err}", path.display()))?
        .modified()
        .map_err(|err| format!("mtime {}: {err}", path.display()))?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
        .as_secs() as i64)
}
