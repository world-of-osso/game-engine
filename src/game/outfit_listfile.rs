use std::io::{BufRead, BufReader};
use std::path::Path;

/// Read an authored model path from the caller's local listfile.
pub(crate) fn lookup_fdid(data_root: &Path, fdid: u32) -> Result<Option<String>, String> {
    let path = data_root.join("community-listfile.csv");
    let file =
        std::fs::File::open(&path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let prefix = format!("{fdid};");
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|err| format!("read {}: {err}", path.display()))?;
        if let Some(name) = line.strip_prefix(&prefix) {
            return Ok(Some(name.to_owned()));
        }
    }
    Ok(None)
}
