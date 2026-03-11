use bevy::log::{info, warn};
use notify::{RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;

fn is_wasm_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("wasm")
}

fn create_watcher(
    watch_path: &Path,
    tx: mpsc::Sender<PathBuf>,
) -> Result<notify::RecommendedWatcher, String> {
    let mut watcher = notify::recommended_watcher(
        move |event: Result<notify::Event, notify::Error>| {
            let Ok(event) = event else { return };
            for changed_path in event.paths {
                if is_wasm_path(&changed_path) {
                    info!("addon changed: {}", changed_path.display());
                    let _ = tx.send(changed_path);
                }
            }
        },
    )
    .map_err(|e| format!("failed to create addon watcher: {e}"))?;
    watcher
        .watch(watch_path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("failed to watch {}: {e}", watch_path.display()))?;
    Ok(watcher)
}

fn run_watcher_thread(watch_path: PathBuf, tx: mpsc::Sender<PathBuf>, ready: SyncSender<Result<(), String>>) {
    match create_watcher(&watch_path, tx) {
        Ok(_watcher) => {
            let _ = ready.send(Ok(()));
            loop { thread::park(); }
        }
        Err(err) => {
            warn!("addon watcher failed: {err}");
            let _ = ready.send(Err(err));
        }
    }
}

/// Watch a directory for .wasm file changes.
/// Returns a receiver that gets notified of changed file paths.
pub fn start_addon_watcher(path: &Path) -> Result<Receiver<PathBuf>, String> {
    if !path.is_dir() {
        return Err(format!("addon directory does not exist: {}", path.display()));
    }

    let watch_path = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);

    thread::Builder::new()
        .name("addon-watcher".to_string())
        .spawn(move || run_watcher_thread(watch_path, tx, ready_tx))
        .map_err(|e| format!("failed to spawn addon watcher thread: {e}"))?;

    ready_rx.recv().map_err(|e| format!("addon watcher init failed: {e}"))??;
    info!("addon watcher started: {}", path.display());
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
        if is_wasm_path(&p) {
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn test_dir(name: &str) -> PathBuf {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join("codex").join(format!(
            "game_engine_{name}_{}_{}",
            std::process::id(),
            id
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn watcher_fails_on_nonexistent_dir() {
        let result = start_addon_watcher(Path::new("/tmp/nonexistent_addon_dir_12345"));
        assert!(result.is_err());
    }

    #[test]
    fn watcher_emits_changed_wasm_paths() {
        let dir = test_dir("watcher");
        let rx = start_addon_watcher(&dir).unwrap();
        let addon = dir.join("test-addon.wasm");

        fs::write(&addon, [0u8]).unwrap();

        let changed = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("watcher should emit .wasm changes");
        assert_eq!(changed, addon);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn watcher_ignores_non_wasm_changes() {
        let dir = test_dir("watcher_ignore");
        let rx = start_addon_watcher(&dir).unwrap();

        fs::write(dir.join("readme.txt"), b"not wasm").unwrap();

        assert!(rx.recv_timeout(Duration::from_millis(500)).is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_finds_wasm_files() {
        let dir = test_dir("scan");
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
