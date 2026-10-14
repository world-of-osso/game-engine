use game_engine_core::adventure_guide::Catalog;
use rusqlite::Connection;
use std::path::Path;

#[test]
fn db2_follow_install_adventure_cache_accepts_next_source_build() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cache/adventure_guide.sqlite");
    let path = std::env::temp_dir().join(format!(
        "db2-follow-adventure-{}.sqlite",
        std::process::id()
    ));
    std::fs::copy(source, &path).unwrap();
    let db = Connection::open(&path).unwrap();
    db.execute("UPDATE source SET build='12.1.0.69933'", [])
        .unwrap();
    let before = Catalog::load(&path).unwrap();
    assert!(!before.encounters.is_empty());
    db.execute("UPDATE source SET build='12.1.5.70391'", [])
        .unwrap();
    let after = Catalog::load(&path).unwrap();
    assert_eq!(after.encounters.len(), before.encounters.len());
    assert_eq!(after.encounters[0].name, before.encounters[0].name);
    db.execute(
        "UPDATE source SET build='12.1.0.69933' WHERE rowid=(SELECT MIN(rowid) FROM source)",
        [],
    )
    .unwrap();
    assert!(
        Catalog::load(&path).is_err(),
        "mixed source builds must fail"
    );
    drop(db);
    std::fs::remove_file(path).unwrap();
}
