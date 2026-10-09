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
        layout("Forever", LayoutSkin::Forever)
    );

    set_active_layout(&path, 17, "Modern").unwrap();
    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Modern", LayoutSkin::Modern)
    );
    fs::remove_file(path).unwrap();
}

/// A file written before layouts carried a skin or settings keeps its custom Modern
/// layout; characters without a choice use Forever. Window positions survive.
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
        layout("Forever", LayoutSkin::Forever)
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
fn forever_default_without_a_layout_file() {
    let path = layout_path("forever-default-missing");
    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    assert!(!path.exists());
}

#[test]
fn forever_default_migrates_builtin_modern_and_preserves_saved_data() {
    let path = layout_path("forever-default-migrate");
    fs::write(
        &path,
        r#"(
    window_positions: {"17": {"WorldMapFrame": (210.0, 104.0)}},
    edit_mode: (
        layouts: {
            "Raid": (
                skin: Modern,
                elements: {"PlayerFrame": (anchor: Center, offset: (-300.0, -200.0))},
                settings: (player_frame: (frame_size: Some(150)), chat: (width: Some(640))),
            ),
        },
        active_layout: {"17": "Modern", "18": "Raid", "19": "Modern", "20": "Forever"},
    ),
)"#,
    )
    .unwrap();
    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    assert_eq!(
        active_layout(&path, 19).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    assert_eq!(
        active_layout(&path, 20).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    let rewritten = fs::read_to_string(&path).unwrap();
    assert!(rewritten.contains("\"17\": \"Forever\""), "{rewritten}");
    assert!(rewritten.contains("\"19\": \"Forever\""), "{rewritten}");
    assert_eq!(
        window_position(&path, 17, "WorldMapFrame").unwrap(),
        Some([210.0, 104.0])
    );
    let raid = active_layout(&path, 18).unwrap();
    assert_eq!(raid.name, "Raid");
    assert_eq!(raid.skin, LayoutSkin::Modern);
    assert_eq!(
        raid.elements["PlayerFrame"],
        SavedElement {
            anchor: HudAnchor::Center,
            offset: [-300.0, -200.0]
        }
    );
    assert_eq!(raid.settings.player_frame.frame_size, Some(150));
    assert_eq!(raid.settings.chat.width, Some(640));
    fs::remove_file(path).unwrap();
}

#[test]
fn forever_default_keeps_custom_modern_skin_positions_and_settings() {
    let path = layout_path("forever-default-custom");
    fs::write(
        &path,
        r#"(edit_mode: (
        layouts: {"My Modern": (
            skin: Modern,
            elements: {"ChatFrame": (anchor: BottomLeft, offset: (40.0, -80.0))},
            settings: (show_micro_menu: Some(true)),
        )},
        active_layout: {"17": "My Modern"},
    ))"#,
    )
    .unwrap();
    let mut expected = layout("My Modern", LayoutSkin::Modern);
    expected.elements.insert(
        "ChatFrame".into(),
        SavedElement {
            anchor: HudAnchor::BottomLeft,
            offset: [40.0, -80.0],
        },
    );
    expected.settings.show_micro_menu = Some(true);
    assert_eq!(active_layout(&path, 17).unwrap(), expected);
    assert_eq!(active_layout(&path, 17).unwrap(), expected);
    fs::remove_file(path).unwrap();
}

#[test]
fn forever_default_second_load_does_not_rewrite_or_remigrate_modern_choice() {
    let path = layout_path("forever-default-once");
    fs::write(&path, r#"(edit_mode: (active_layout: {"17": "Modern"}))"#).unwrap();
    assert_eq!(active_layout(&path, 17).unwrap().name, "Forever");
    let first = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(active_layout(&path, 17).unwrap().name, "Forever");
    assert_eq!(fs::read(&path).unwrap(), first);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    set_active_layout(&path, 17, "Modern").unwrap();
    let selected = fs::read(&path).unwrap();
    let selected_modified = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Modern", LayoutSkin::Modern)
    );
    assert_eq!(fs::read(&path).unwrap(), selected);
    assert_eq!(
        fs::metadata(&path).unwrap().modified().unwrap(),
        selected_modified
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn unknown_layout_is_rejected_without_writing() {
    let path = layout_path("unknown");
    let error = set_active_layout(&path, 17, "Classic").unwrap_err();
    assert!(error.contains("unknown UI layout \"Classic\""), "{error}");
    assert!(!path.exists());
}
