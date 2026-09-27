use std::path::PathBuf;

use game_engine_core::npc_appearance_assets::{load_compositor, load_customization_db};

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn loads_authored_human_catalog_and_layout_from_local_cache() {
    let db = load_customization_db(&data_root()).expect("local customization catalog");
    assert_eq!(db.chr_model_id(1, 1), Some(2));
    assert_eq!(db.layout_id(1, 1), Some(104));
    let choice = db.choice_by_id(1, 1, 85).expect("authored NPC choice 85");
    assert_eq!(choice.id, 85);
}

#[test]
fn loads_original_hd_compositor_dimensions() {
    let compositor = load_compositor(&data_root()).expect("local compositor catalog");
    let layout = compositor.layout(103).expect("HumanMaleHD layout");
    assert_eq!((layout.width, layout.height), (2048, 1024));
}

#[test]
fn missing_catalogs_report_the_required_path_without_creating_files() {
    let missing = data_root().join("missing-native-appearance-catalog-fixture");
    let error = load_customization_db(&missing).unwrap_err();
    assert!(error.contains("customization.sqlite"), "{error}");
    let error = load_compositor(&missing).unwrap_err();
    assert!(error.contains("char_texture.sqlite"), "{error}");
    assert!(!missing.exists());
}
