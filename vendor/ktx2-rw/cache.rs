//! Publish complete native artifacts once, independently of Cargo's OUT_DIR.
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub fn populate_locked(
    root: &Path,
    key: &str,
    required: &[&str],
    populate: impl FnOnce(&Path),
) -> PathBuf {
    fs::create_dir_all(root).expect("Cannot create KTX cache root");
    let lock = File::create(root.join(format!("{key}.lock"))).expect("Cannot open KTX cache lock");
    lock.lock().expect("Cannot lock KTX cache");
    let directory = root.join(key);
    if !directory.join("complete").exists() {
        if directory.exists() {
            fs::remove_dir_all(&directory).expect("Cannot remove incomplete KTX cache entry");
        }
        fs::create_dir(&directory).expect("Cannot create KTX cache entry");
        populate(&directory);
        assert_artifacts(&directory, required);
        fs::write(directory.join("complete"), b"complete\n")
            .expect("Cannot publish KTX cache entry");
    }
    assert_artifacts(&directory, required);
    directory
}

fn assert_artifacts(directory: &Path, required: &[&str]) {
    for name in required {
        let path = directory.join(name);
        assert!(
            path.is_file(),
            "KTX cache artifact missing: {}",
            path.display()
        );
    }
}
