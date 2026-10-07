//! Edit Mode layout presets pair a layout with a UI skin; the active one is saved per
//! character in `ui_layout.ron`.

use std::{fs, path::PathBuf};

use game_engine_core::ui_layout_data::{
    ActiveLayout, HudAnchor, LayoutSettings, LayoutSkin, SavedElement, active_layout,
    set_active_layout, window_position,
};

fn layout_path(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ui-layout-presets-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{test}.ron"));
    let _ = fs::remove_file(&path);
    path
}

fn layout(name: &str, skin: LayoutSkin) -> ActiveLayout {
    ActiveLayout {
        name: name.to_string(),
        skin,
        settings: LayoutSettings::default(),
        elements: Default::default(),
    }
}

#[test]
fn preset_choice_persists_per_character_across_reload() {
    let path = layout_path("persist");
    assert_eq!(
        set_active_layout(&path, 17, "Forever").unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    assert_eq!(
        set_active_layout(&path, 18, "Modern").unwrap(),
        layout("Modern", LayoutSkin::Modern)
    );

    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("\"17\": \"Forever\""), "{saved}");
    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    assert_eq!(
        active_layout(&path, 18).unwrap(),
        layout("Modern", LayoutSkin::Modern)
    );
    assert_eq!(
        active_layout(&path, 19).unwrap(),
        layout("Modern", LayoutSkin::Modern)
    );

    set_active_layout(&path, 17, "Modern").unwrap();
    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Modern", LayoutSkin::Modern)
    );
    fs::remove_file(path).unwrap();
}

/// A file written before layouts carried a skin or settings: its custom layout and every
/// character load as Modern with the preset's values, and its window positions survive a preset save.
#[test]
fn old_layout_file_loads_as_modern() {
    let path = layout_path("old");
    fs::write(
        &path,
        r#"(
    window_positions: {"17": {"WorldMapFrame": (210.0, 104.0)}},
    edit_mode: (
        layouts: {"Raid": (elements: {"PlayerFrame": (anchor: Center, offset: (-300.0, -200.0))})},
        active_layout: {"18": "Raid"},
    ),
)"#,
    )
    .unwrap();

    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Modern", LayoutSkin::Modern)
    );
    // The mover position the old Edit Mode saved in "Raid" loads with the layout.
    let mut raid = layout("Raid", LayoutSkin::Modern);
    raid.elements.insert(
        "PlayerFrame".to_string(),
        SavedElement {
            anchor: HudAnchor::Center,
            offset: [-300.0, -200.0],
        },
    );
    assert_eq!(active_layout(&path, 18).unwrap(), raid);

    set_active_layout(&path, 17, "Forever").unwrap();
    assert_eq!(
        window_position(&path, 17, "WorldMapFrame").unwrap(),
        Some([210.0, 104.0])
    );
    assert_eq!(active_layout(&path, 18).unwrap(), raid);
    fs::remove_file(path).unwrap();
}

#[test]
fn unknown_layout_is_rejected_without_writing() {
    let path = layout_path("unknown");
    let error = set_active_layout(&path, 17, "Classic").unwrap_err();
    assert!(error.contains("unknown UI layout \"Classic\""), "{error}");
    assert!(!path.exists());
}
