use game_engine_core::adventure_guide::Catalog;
use rusqlite::Connection;
use std::path::Path;

#[test]
fn db2_follow_install_adventure_cache_accepts_next_source_build() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let root = std::env::temp_dir().join(format!("db2-follow-adventure-{}", std::process::id()));
    let cache = root.join("cache");
    let active = root.join("db2/active");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::create_dir_all(&active).unwrap();
    for table in ["Difficulty.csv", "DungeonEncounter.csv"] {
        std::fs::copy(source.join("db2/active").join(table), active.join(table)).unwrap();
    }
    let path = cache.join("adventure_guide.sqlite");
    std::fs::copy(source.join("cache/adventure_guide.sqlite"), &path).unwrap();
    let db = Connection::open(&path).unwrap();
    db.execute("UPDATE source SET build=?1", ["fixture-before"])
        .unwrap();
    let before = Catalog::load(&path).unwrap();
    assert!(!before.encounters.is_empty());
    db.execute("UPDATE source SET build=?1", ["fixture-after"])
        .unwrap();
    let after = Catalog::load(&path).unwrap();
    assert_eq!(after.encounters.len(), before.encounters.len());
    assert_eq!(after.encounters[0].name, before.encounters[0].name);
    db.execute(
        "UPDATE source SET build=?1 WHERE rowid=(SELECT MIN(rowid) FROM source)",
        ["fixture-before"],
    )
    .unwrap();
    assert!(
        Catalog::load(&path).is_err(),
        "mixed source builds must fail"
    );
    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}
