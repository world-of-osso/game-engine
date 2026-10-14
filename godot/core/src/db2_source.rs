//! Identity of an exported DB2 snapshot, independent of the client installation.
use std::path::Path;

/// Read the actual source build from the export receipt. Missing receipts fail.
pub fn read_build(directory: &Path) -> Result<String, String> {
    let path = directory.join("receipt.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("read DB2 receipt {}: {error}", path.display()))?;
    let receipt: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("parse DB2 receipt {}: {error}", path.display()))?;
    receipt["identity"]["build"]
        .as_str()
        .filter(|build| !build.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("DB2 receipt {} has no build", path.display()))
}
