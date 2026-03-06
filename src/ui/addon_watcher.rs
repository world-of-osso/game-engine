use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

/// Watch a directory for .wasm file changes.
/// Returns a receiver that gets notified of changed file paths.
///
/// TODO: Add `notify` crate for real filesystem watching.
/// For now, returns a receiver with no sender (will never receive events).
pub fn start_addon_watcher(path: &Path) -> Result<Receiver<PathBuf>, String> {
    if !path.is_dir() {
        return Err(format!(
            "addon directory does not exist: {}",
            path.display()
        ));
    }
    let (_tx, rx) = mpsc::channel();
    Ok(rx)
}

/// Scan a directory for .wasm files (non-recursive).
pub fn scan_addon_dir(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.is_dir() {
        return Err(format!("not a directory: {}", path.display()));
    }
    let mut wasm_files = Vec::new();
    let entries = std::fs::read_dir(path).map_err(|e| format!("failed to read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("wasm") {
            wasm_files.push(p);
        }
    }
    wasm_files.sort();
    Ok(wasm_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn watcher_fails_on_nonexistent_dir() {
        let result = start_addon_watcher(Path::new("/tmp/nonexistent_addon_dir_12345"));
        assert!(result.is_err());
    }

    #[test]
    fn watcher_succeeds_on_real_dir() {
        let dir = std::env::temp_dir().join("game_engine_watcher_test");
        fs::create_dir_all(&dir).unwrap();
        let result = start_addon_watcher(&dir);
        assert!(result.is_ok());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_finds_wasm_files() {
        let dir = std::env::temp_dir().join("game_engine_scan_test");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("addon1.wasm"), &[0u8]).unwrap();
        fs::write(dir.join("addon2.wasm"), &[0u8]).unwrap();
        fs::write(dir.join("readme.txt"), &[0u8]).unwrap();

        let files = scan_addon_dir(&dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.extension().unwrap() == "wasm"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_nonexistent_dir_fails() {
        let result = scan_addon_dir(Path::new("/tmp/nonexistent_scan_dir_12345"));
        assert!(result.is_err());
    }
}
