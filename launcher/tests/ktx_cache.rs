#[path = "../../vendor/ktx2-rw/cache.rs"]
mod cache;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn directory() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ktx-cache-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir(&root).unwrap();
    root
}

#[test]
fn concurrent_slots_publish_once_and_distinct_keys_do_not_share() {
    let root = directory();
    let count = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    cache::populate_locked(
                        &root,
                        "4.4.0-linux-release",
                        &["libktx.a", "bindings.rs"],
                        |entry| {
                            count.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            fs::write(entry.join("libktx.a"), b"library").unwrap();
                            fs::write(entry.join("bindings.rs"), b"bindings").unwrap();
                        },
                    )
                })
            })
            .collect();
        for handle in handles {
            let entry = handle.join().unwrap();
            assert_eq!(fs::read(entry.join("libktx.a")).unwrap(), b"library");
            assert_eq!(fs::read(entry.join("bindings.rs")).unwrap(), b"bindings");
        }
    });
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let other = cache::populate_locked(&root, "4.4.0-windows-release", &["ktx.lib"], |entry| {
        fs::write(entry.join("ktx.lib"), b"windows library").unwrap();
    });
    assert_eq!(fs::read(other.join("ktx.lib")).unwrap(), b"windows library");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_population_is_not_published_and_next_slot_rebuilds() {
    let root = directory();
    let failed = std::panic::catch_unwind(|| {
        cache::populate_locked(&root, "key", &["libktx.a"], |entry| {
            fs::write(entry.join("partial"), b"incomplete").unwrap();
            panic!("native build failed");
        });
    });
    assert!(failed.is_err());
    assert!(!root.join("key/complete").exists());
    let entry = cache::populate_locked(&root, "key", &["libktx.a"], |entry| {
        assert!(!entry.join("partial").exists());
        fs::write(entry.join("libktx.a"), b"complete library").unwrap();
    });
    assert_eq!(
        fs::read(entry.join("libktx.a")).unwrap(),
        b"complete library"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_artifact_in_completed_cache_fails_loudly() {
    let root = directory();
    let entry = cache::populate_locked(&root, "key", &["bindings.rs"], |entry| {
        fs::write(entry.join("bindings.rs"), b"bindings").unwrap();
    });
    fs::remove_file(entry.join("bindings.rs")).unwrap();
    let result = std::panic::catch_unwind(|| {
        cache::populate_locked(&root, "key", &["bindings.rs"], |_| {
            panic!("must not rebuild a corrupt completed entry")
        });
    });
    assert!(result.is_err());
    fs::remove_dir_all(root).unwrap();
}
