//! Edit Mode layout presets pair a layout with a UI skin; the active one is saved per
//! character in `ui_layout.ron`.

use std::{fs, path::PathBuf};

use game_engine_core::ui_layout_data::{
    ActiveLayout, HudAnchor, LayoutSettings, LayoutSkin, SavedElement, active_layout,
    create_layout, layout_names, save_layout_settings, set_active_layout, window_position,
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

/// Legacy custom layouts without a skin are reset too. Window positions are separate.
#[test]
fn old_layout_file_resets_to_forever() {
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
    assert_eq!(
        active_layout(&path, 18).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    assert_eq!(layout_names(&path).unwrap(), ["Modern", "Forever"]);
    set_active_layout(&path, 17, "Forever").unwrap();
    assert_eq!(
        window_position(&path, 17, "WorldMapFrame").unwrap(),
        Some([210.0, 104.0])
    );
    assert_eq!(
        active_layout(&path, 18).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
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
fn forever_default_resets_all_characters_and_preserves_window_positions() {
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
    for character in [17, 18, 19, 20] {
        assert_eq!(
            active_layout(&path, character).unwrap(),
            layout("Forever", LayoutSkin::Forever)
        );
    }
    let rewritten = fs::read_to_string(&path).unwrap();
    assert!(rewritten.contains("\"17\": \"Forever\""), "{rewritten}");
    assert!(rewritten.contains("\"19\": \"Forever\""), "{rewritten}");
    assert_eq!(
        window_position(&path, 17, "WorldMapFrame").unwrap(),
        Some([210.0, 104.0])
    );
    assert_eq!(layout_names(&path).unwrap(), ["Modern", "Forever"]);
    assert!(set_active_layout(&path, 18, "Raid").is_err());
    fs::remove_file(path).unwrap();
}

fn assert_custom_layout_reset(test: &str, fixture: &str, old_name: &str) {
    let path = layout_path(test);
    fs::write(&path, fixture).unwrap();
    assert_eq!(
        active_layout(&path, 17).unwrap(),
        layout("Forever", LayoutSkin::Forever)
    );
    assert_eq!(layout_names(&path).unwrap(), ["Modern", "Forever"]);
    assert!(set_active_layout(&path, 17, old_name).is_err());
    fs::remove_file(path).unwrap();
}

#[test]
fn forever_default_removes_custom_modern_skin_positions_and_settings() {
    assert_custom_layout_reset(
        "forever-default-custom-modern",
        r#"(edit_mode: (
        layouts: {"My Modern": (
            skin: Modern,
            elements: {"ChatFrame": (anchor: BottomLeft, offset: (40.0, -80.0))},
            settings: (show_micro_menu: Some(true)),
        )},
        active_layout: {"17": "My Modern"},
    ))"#,
        "My Modern",
    );
}

#[test]
fn forever_default_removes_custom_forever_skin_positions_and_settings() {
    assert_custom_layout_reset(
        "forever-default-custom-forever",
        r#"(edit_mode: (
        layouts: {"My Forever": (
            skin: Forever,
            elements: {"ChatFrame": (anchor: BottomLeft, offset: (40.0, -80.0))},
            settings: (show_micro_menu: Some(true)),
        ), "Unselected": (skin: Modern)},
        active_layout: {"17": "My Forever"},
    ))"#,
        "My Forever",
    );
}

#[test]
fn forever_default_layout_created_after_migration_survives_reload() {
    let path = layout_path("forever-default-new-custom");
    fs::write(&path, r#"(edit_mode: (active_layout: {"17": "Modern"}))"#).unwrap();
    assert_eq!(active_layout(&path, 17).unwrap().name, "Forever");
    let elements = [(
        "ChatFrame".into(),
        SavedElement {
            anchor: HudAnchor::BottomLeft,
            offset: [40.0, -80.0],
        },
    )]
    .into();
    let created = create_layout(&path, 17, "After migration", elements).unwrap();
    let settings = LayoutSettings {
        show_micro_menu: Some(true),
        ..Default::default()
    };
    let saved = save_layout_settings(&path, 17, settings).unwrap();
    assert_eq!(saved.elements, created.elements);
    assert_eq!(saved.settings, settings);
    assert_eq!(saved.skin, LayoutSkin::Forever);
    assert_eq!(saved.name, "After migration");
    let bytes = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(active_layout(&path, 17).unwrap(), saved);
    assert_eq!(active_layout(&path, 17).unwrap(), saved);
    assert_eq!(
        layout_names(&path).unwrap(),
        ["Modern", "Forever", "After migration"]
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
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
