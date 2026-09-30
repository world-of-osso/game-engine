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
fn native_swatch_sampling_is_lazy_shared_and_limited_to_color_choices() {
    let db = load_customization_db(&data_root()).expect("local customization catalog");
    let skin = db.choice_by_id(1, 1, 85).expect("Human female Skin Color");
    let face = db.choice_by_id(1, 1, 107).expect("Human female Face");
    let skin_clone = skin.clone();
    let mut calls = 0;
    assert_eq!(
        skin.sample_swatch_color_with(|materials| {
            calls += 1;
            assert!(!materials.is_empty());
            Some([17, 42, 93])
        }),
        Some([17, 42, 93])
    );
    assert_eq!(
        skin_clone.sample_swatch_color_with(|_| {
            calls += 1;
            Some([1, 2, 3])
        }),
        Some([17, 42, 93])
    );
    assert_eq!(
        face.sample_swatch_color_with(|_| {
            calls += 1;
            Some([1, 2, 3])
        }),
        None
    );
    assert_eq!(calls, 1);
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
    assert!(error.contains("customization-v4.sqlite"), "{error}");
    let error = load_compositor(&missing).unwrap_err();
    assert!(error.contains("char_texture-v2.sqlite"), "{error}");
    assert!(!missing.exists());
}
