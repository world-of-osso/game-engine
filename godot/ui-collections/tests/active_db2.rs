//! A separate process isolates the host's once-only data-root and scene-flags state.
use std::{fs, path::PathBuf};

#[test]
fn trading_post_reads_scene_flags_from_active_snapshot_without_historical_directory() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let root = std::env::temp_dir().join(format!("osso-active-db2-{}", std::process::id()));
    let snapshot = root.join("db2/installed-next");
    fs::create_dir_all(&snapshot).unwrap();
    for table in [
        "UiTextureAtlas",
        "UiTextureAtlasElement",
        "UiTextureAtlasMember",
    ] {
        std::os::unix::fs::symlink(
            source.join(format!("db2/active/{table}.csv")),
            snapshot.join(format!("{table}.csv")),
        )
        .unwrap();
    }
    fs::write(
        snapshot.join("UiModelScene.csv"),
        "ID,Flags\n430,0\n418,4\n",
    )
    .unwrap();
    std::os::unix::fs::symlink("installed-next", root.join("db2/active")).unwrap();
    std::os::unix::fs::symlink(
        source.join("db2/1.60.1.69913"),
        root.join("db2/1.60.1.69913"),
    )
    .unwrap();
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    assert!(!game_engine_ui_collections::trading_post::load_scene_autodress(430).unwrap());
    assert!(game_engine_ui_collections::trading_post::load_scene_autodress(418).unwrap());
    assert!(game_engine_ui_collections::trading_post::load_scene_autodress(999).is_err());
    fs::remove_dir_all(root).unwrap();
}
