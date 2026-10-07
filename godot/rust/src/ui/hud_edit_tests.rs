//! Behavioral regression: persisted placements were parsed but never returned to the client.
use game_engine_core::ui_layout_data::{self, HudAnchor, LayoutSkin};

#[test]
fn hudeditmode_loads_saved_offsets_without_changing_presets() {
    let path = std::env::temp_dir().join(format!("hudeditmode-red-{}.ron", std::process::id()));
    std::fs::write(&path, r#"(edit_mode: (layouts: {"Moved": (skin: Forever, elements: {"player_frame": (anchor: Bottom, offset: (96.0, -240.0))})}, active_layout: {"42": "Moved"}))"#).unwrap();
    let active = ui_layout_data::active_layout(&path, 42).unwrap();
    assert_eq!(active.skin, LayoutSkin::Forever);
    assert_eq!(active.elements["player_frame"].anchor, HudAnchor::Bottom);
    assert_eq!(active.elements["player_frame"].offset, [96.0, -240.0]);
    assert!(
        ui_layout_data::active_layout(&path, 43)
            .unwrap()
            .elements
            .is_empty()
    );
    assert!(
        ui_layout_data::set_active_layout(&path, 42, "Forever")
            .unwrap()
            .elements
            .is_empty()
    );
    std::fs::remove_file(path).unwrap();
}

use game_engine_ui_model::hud_edit::{
    EditDraft, Placements, save_top_left, snap_position, top_left,
};
use game_engine_ui_model::hud_edit_component::{EditModeOverlayState, EditModeSelectionBox};
use std::collections::HashMap;
use ui_toolkit::{
    atlas::ActiveSkin,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn player_box() -> EditModeSelectionBox {
    EditModeSelectionBox {
        key: "player_frame".into(),
        label: "Player Frame".into(),
        rect: [100.0, 200.0, 240.0, 60.0],
        selected: false,
    }
}

#[test]
fn hudeditmode_enter_drag_exit_discards_unsaved_and_reset_removes_only_selected() {
    let layout = ui_layout_data::ActiveLayout::default();
    let mut draft = EditDraft::default();
    draft.start_drag(&[player_box()], [110.0, 215.0]);
    assert!(draft.drag.is_none());
    draft.enter(&layout);
    assert!(draft.active);
    draft.start_drag(&[player_box()], [110.0, 215.0]);
    draft.move_drag([215.0, 324.0], [240.0, 60.0], [1366.0, 768.0]);
    let saved = draft.working["player_frame"];
    assert_eq!(
        top_left(saved, [240.0, 60.0], [1366.0, 768.0]),
        [208.0, 312.0]
    );
    draft.working.insert(
        "chat_frame".into(),
        save_top_left(
            HudAnchor::BottomLeft,
            [0.0, 400.0],
            [500.0, 280.0],
            [1366.0, 768.0],
        ),
    );
    draft.reset_selected();
    assert!(!draft.working.contains_key("player_frame"));
    assert!(draft.working.contains_key("chat_frame"));
    draft.exit();
    assert!(!draft.active);
    draft.enter(&layout);
    assert!(draft.working.is_empty());
}

#[test]
fn hudeditmode_grid_edge_snap_and_clamp_include_oversized_frames() {
    assert_eq!(
        snap_position([105.0, 203.0], [240.0, 60.0], [1366.0, 768.0]),
        [104.0, 200.0]
    );
    assert_eq!(
        snap_position([1120.0, 705.0], [240.0, 60.0], [1366.0, 768.0]),
        [1126.0, 708.0]
    );
    assert_eq!(
        snap_position([-500.0, 9000.0], [240.0, 60.0], [1366.0, 768.0]),
        [0.0, 708.0]
    );
    assert_eq!(
        snap_position([400.0, 500.0], [2000.0, 1000.0], [1366.0, 768.0]),
        [0.0, 0.0]
    );
}

#[test]
fn hudeditmode_save_round_trip_layout_settings_characters_rename_delete() {
    let path =
        std::env::temp_dir().join(format!("hudeditmode-roundtrip-{}.ron", std::process::id()));
    for skin in ["Modern", "Forever"] {
        ui_layout_data::set_active_layout(&path, 17, skin).unwrap();
        let original = ui_layout_data::active_layout(&path, 17).unwrap();
        let elements = Placements::from([(
            "player_frame".into(),
            save_top_left(
                HudAnchor::Bottom,
                [104.0, 312.0],
                [240.0, 60.0],
                [1366.0, 768.0],
            ),
        )]);
        let saved = ui_layout_data::save_layout_elements(&path, 17, elements.clone()).unwrap();
        assert_ne!(saved.name, skin);
        assert_eq!(saved.skin, original.skin);
        assert_eq!(saved.elements, elements);
        assert_eq!(ui_layout_data::active_layout(&path, 17).unwrap(), saved);
        assert!(
            ui_layout_data::active_layout(&path, 18)
                .unwrap()
                .elements
                .is_empty()
        );
        let settings = ui_layout_data::LayoutSettings {
            show_micro_menu: Some(true),
            ..Default::default()
        };
        let updated = ui_layout_data::save_layout_settings(&path, 17, settings).unwrap();
        assert_eq!(updated.elements, elements);
        assert_eq!(updated.settings, settings);
        ui_layout_data::rename_layout(&path, &updated.name, "Renamed").unwrap();
        assert_eq!(
            ui_layout_data::active_layout(&path, 17).unwrap().name,
            "Renamed"
        );
        ui_layout_data::delete_layout(&path, "Renamed").unwrap();
        assert_eq!(ui_layout_data::active_layout(&path, 17).unwrap(), original);
        assert!(ui_layout_data::rename_layout(&path, skin, "Forbidden").is_err());
        assert!(ui_layout_data::delete_layout(&path, skin).is_err());
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn hudeditmode_both_skins_reset_scale_and_unmoved_authored_defaults() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(EditModeOverlayState::default());
        let mut screen = Screen::new(super::hud_edit_preview::preview_screen);
        let mut registry = FrameRegistry::new(1366.0, 768.0);
        screen.sync(&shared, &mut registry);
        let player = registry.get_by_name("PlayerFrame").unwrap();
        let child = registry.get(player).unwrap().children[0];
        let baseline =
            super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        let mut untouched = baseline.clone();
        super::hud_edit_layout::apply_placements(&registry, &mut untouched, &Placements::new());
        for (id, rect) in &baseline {
            assert_eq!(untouched[id], *rect, "unmoved frame {id}");
        }
        let rect = baseline[&player];
        let saved = save_top_left(
            HudAnchor::Bottom,
            [104.0, 312.0],
            [rect.width, rect.height],
            [1366.0, 768.0],
        );
        let placements = Placements::from([("player_frame".into(), saved)]);
        let mut moved = baseline.clone();
        super::hud_edit_layout::apply_placements(&registry, &mut moved, &placements);
        assert_eq!([moved[&player].x, moved[&player].y], [104.0, 312.0]);
        assert_eq!(
            moved[&child].x - baseline[&child].x,
            moved[&player].x - rect.x
        );
        assert_eq!(
            moved[&child].y - baseline[&child].y,
            moved[&player].y - rect.y
        );
        let reset =
            super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        assert_eq!(reset[&player], rect);
        registry.screen_width = 1600.0;
        registry.screen_height = 900.0;
        registry.ui_scale = 1.2;
        let mut scaled =
            super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        super::hud_edit_layout::apply_placements(&registry, &mut scaled, &placements);
        assert_eq!([scaled[&player].x, scaled[&player].y], [221.0, 444.0]);
        assert_eq!(scaled[&player].width, rect.width);
        assert_eq!(scaled[&player].height, rect.height);
    }
}
