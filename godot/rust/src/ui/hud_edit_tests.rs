//! Behavioral regressions for HUD Edit Mode persistence and authored placements.
use game_engine_core::ui_layout_data::{self, HudAnchor, LayoutSkin};

#[test]
fn hudeditmode_resets_old_saved_offsets_to_builtin_forever() {
    let path = std::env::temp_dir().join(format!("hudeditmode-red-{}.ron", std::process::id()));
    std::fs::write(&path, r#"(edit_mode: (layouts: {"Moved": (skin: Forever, elements: {"player_frame": (anchor: Bottom, offset: (96.0, -240.0))})}, active_layout: {"42": "Moved"}))"#).unwrap();
    let active = ui_layout_data::active_layout(&path, 42).unwrap();
    assert_eq!(active, ui_layout_data::ActiveLayout::default());
    assert_eq!(
        ui_layout_data::layout_names(&path).unwrap(),
        ["Modern", "Forever"]
    );
    assert!(ui_layout_data::set_active_layout(&path, 42, "Moved").is_err());
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
        hovered: false,
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
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
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
        let rect = baseline[&player].clone();
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

#[test]
fn hudeditmode_every_registered_root_moves_with_its_child_and_hidden_roots_have_no_mover() {
    use game_engine_ui_model::hud_edit_elements::EDIT_MODE_ELEMENTS;
    use ui_toolkit::{
        frame::Dimension,
        layout_values::{PositionType, Val},
    };
    let mut registry = FrameRegistry::new(1366.0, 768.0);
    let mut placements = Placements::new();
    for (index, element) in EDIT_MODE_ELEMENTS.iter().enumerate() {
        let id = registry.create_frame(element.frame_name, None);
        let frame = registry.get_mut(id).unwrap();
        frame.position_type = PositionType::Absolute;
        frame.position.left = Val::Px(100.0 + index as f32 * 10.0);
        frame.position.top = Val::Px(200.0);
        frame.width = Dimension::Fixed(80.0);
        frame.height = Dimension::Fixed(32.0);
        let child = registry.create_frame(&format!("{}Child", element.frame_name), Some(id));
        let frame = registry.get_mut(child).unwrap();
        frame.position_type = PositionType::Absolute;
        frame.position.left = Val::Px(4.0);
        frame.position.top = Val::Px(6.0);
        frame.width = Dimension::Fixed(20.0);
        frame.height = Dimension::Fixed(14.0);
        placements.insert(
            element.key.into(),
            save_top_left(
                element.default_anchor,
                [32.0, 24.0],
                [80.0, 32.0],
                [1366.0, 768.0],
            ),
        );
    }
    let authored =
        super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
    for (id, rect) in &authored {
        registry.get_mut(*id).unwrap().layout_rect = Some(rect.clone());
    }
    let boxes = super::hud_edit_layout::collect_selection_boxes(&registry, Some("player_frame"));
    assert_eq!(boxes.len(), EDIT_MODE_ELEMENTS.len());
    assert_eq!(boxes.iter().filter(|entry| entry.selected).count(), 1);
    for element in EDIT_MODE_ELEMENTS {
        assert_eq!(
            boxes
                .iter()
                .find(|entry| entry.key == element.key)
                .unwrap()
                .label,
            element.label
        );
    }
    let mut moved = authored.clone();
    super::hud_edit_layout::apply_placements(&registry, &mut moved, &placements);
    for element in EDIT_MODE_ELEMENTS {
        let id = registry.get_by_name(element.frame_name).unwrap();
        assert_eq!([moved[&id].x, moved[&id].y], [32.0, 24.0]);
        let child = registry
            .get_by_name(&format!("{}Child", element.frame_name))
            .unwrap();
        assert_eq!([moved[&child].x, moved[&child].y], [36.0, 30.0]);
    }
    let hidden = registry.get_by_name("PlayerFrame").unwrap();
    registry.get_mut(hidden).unwrap().visible = false;
    assert!(
        !super::hud_edit_layout::collect_selection_boxes(&registry, None)
            .iter()
            .any(|entry| entry.key == "player_frame")
    );
    registry.get_mut(hidden).unwrap().visible = true;
    let reset = super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
    assert_eq!(reset, authored);
}

#[test]
fn hudeditmode_action_bar_previews_follow_mode_not_saved_defaults() {
    use game_engine_ui_model::hud_edit::EditModeActive;
    use game_engine_ui_model::main_action_bar_component::{
        MainActionBarState, main_action_bar_screen,
    };
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(MainActionBarState::default());
    shared.insert(EditModeActive(false));
    let mut registry = FrameRegistry::new(1366.0, 768.0);
    let mut screen = Screen::new(main_action_bar_screen);
    screen.sync(&shared, &mut registry);
    assert!(registry.get_by_name("MultiBarBottomLeft").is_none());
    shared.insert(EditModeActive(true));
    screen.sync(&shared, &mut registry);
    assert!(registry.get_by_name("MultiBarBottomLeft").is_some());
    assert!(registry.get_by_name("MultiBarBottomRight").is_some());
    shared.insert(EditModeActive(false));
    screen.sync(&shared, &mut registry);
    assert!(registry.get_by_name("MultiBarBottomLeft").is_none());
    assert!(registry.get_by_name("MultiBarBottomRight").is_none());
}

#[test]
fn hudeditmode_side_bar_previews_are_disabled_by_default_in_both_skins() {
    use game_engine_ui_model::main_action_bar_component::{
        MainActionBarState, main_action_bar_screen,
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut shared = SharedContext::new();
        shared.insert(skin);
        let mut registry = FrameRegistry::new(1366.0, 768.0);
        shared.insert(MainActionBarState::default());
        Screen::new(main_action_bar_screen).sync(&shared, &mut registry);
        let bounds =
            super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        for (id, rect) in bounds {
            registry.get_mut(id).unwrap().layout_rect = Some(rect);
        }
        let boxes =
            super::hud_edit_layout::collect_selection_boxes(&registry, Some("action_bar_4"));
        assert!(
            !boxes
                .iter()
                .any(|entry| entry.key == "action_bar_4" || entry.key == "action_bar_5"),
            "disabled side bars must not register movers"
        );
    }
}

#[test]
fn hudeditmode_manager_border_projects_registered_static_popup_art() {
    use game_engine_ui_model::hud_edit_component::{
        EditModePanelState, PANEL_H, PANEL_W, edit_mode_panel_screen,
    };
    use ui_toolkit::widgets::texture::TextureSource;
    let mut registry = FrameRegistry::new(1366.0, 768.0);
    super::hud_edit_preview::register_edit_panel_style(&mut registry);
    let mut shared = SharedContext::new();
    shared.insert(EditModePanelState {
        layout_name: "Modern".into(),
        preset: true,
        ..Default::default()
    });
    Screen::new(edit_mode_panel_screen).sync(&shared, &mut registry);
    let id = registry.get_by_name("EditModeManagerFrameBorder").unwrap();
    let images = super::parts::project_images(registry.get(id).unwrap(), PANEL_W, PANEL_H);
    assert_eq!(images.len(), 9);
    assert!(
        images
            .iter()
            .all(|part| part.source == Some(TextureSource::FileDataId(6_795_680)))
    );
    assert!(
        images
            .iter()
            .all(|part| part.rect[2] > 0.0 && part.rect[3] > 0.0)
    );
}

#[test]
fn hudeditmode_selection_background_projects_selected_and_unselected_art() {
    use game_engine_ui_model::hud_edit_component::edit_mode_overlay_screen;
    use ui_toolkit::widgets::texture::TextureSource;
    for (selected, fdid) in [(false, 4_554_383), (true, 4_554_386)] {
        let mut entry = player_box();
        entry.selected = selected;
        let mut shared = SharedContext::new();
        shared.insert(EditModeOverlayState { boxes: vec![entry] });
        let mut registry = FrameRegistry::new(1366.0, 768.0);
        Screen::new(edit_mode_overlay_screen).sync(&shared, &mut registry);
        let id = registry
            .get_by_name("EditModeSelection_player_frameBackground")
            .unwrap();
        let frame = registry.get(id).unwrap();
        let images = super::parts::project_images(frame, 240.0, 60.0);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].source, Some(TextureSource::FileDataId(fdid)));
        assert_eq!(images[0].rect, [0.0, 0.0, 240.0, 60.0]);
        let projected_opacity = frame.effective_alpha * images[0].color[3];
        assert_eq!(
            projected_opacity, 0.7,
            "mover image must not be transparent"
        );
    }
}

#[test]
fn hudeditmodepolish_idle_labels_are_hidden_and_selected_labels_are_readable() {
    use game_engine_ui_model::hud_edit_component::edit_mode_overlay_screen;
    for selected in [false, true] {
        let mut entry = player_box();
        entry.selected = selected;
        let mut shared = SharedContext::new();
        shared.insert(EditModeOverlayState { boxes: vec![entry] });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(edit_mode_overlay_screen).sync(&shared, &mut registry);
        let label = registry
            .get_by_name("EditModeSelection_player_frameLabel")
            .unwrap();
        assert_eq!(
            super::hud_edit_layout::frame_is_visible(&registry, label),
            selected
        );
    }
}

#[test]
fn hudeditmodepolish_default_manager_does_not_cover_error_text() {
    use game_engine_ui_model::hud_edit_component::{
        EditModePanelState, PANEL_H, PANEL_W, edit_mode_panel_screen,
    };
    let mut shared = SharedContext::new();
    let errors = [704.0, 122.0, 512.0, 60.0];
    let boxes = vec![EditModeSelectionBox {
        rect: errors,
        ..Default::default()
    }];
    let position =
        game_engine_ui_model::hud_edit_component::find_panel_position([1920.0, 1080.0], &boxes)
            .unwrap();
    shared.insert(EditModePanelState {
        position: Some(position),
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(edit_mode_panel_screen).sync(&shared, &mut registry);
    let bounds = super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
    let panel = &bounds[&registry.get_by_name("EditModeManagerFrame").unwrap()];
    assert!(!rects_overlap([panel.x, panel.y, PANEL_W, PANEL_H], errors));
}

fn rects_overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

#[test]
fn hudeditmode_extra_bar_options_toggle_gameplay_and_movers_without_moving_tracker() {
    use game_engine_core::client_options_data::HudOptionsFile;
    use game_engine_ui_model::game_menu_component::game_menu_screen;
    use game_engine_ui_model::main_action_bar_component::{
        MainActionBarState, main_action_bar_screen,
    };
    use game_engine_ui_model::options_menu_data as policy;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut options = super::hud_edit_preview::extra_bar_options_model(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(EditModeOverlayState::default());
        let mut registry = FrameRegistry::new(1366.0, 768.0);
        let mut screen = Screen::new(super::hud_edit_preview::preview_screen);
        screen.sync(&shared, &mut registry);
        publish_test_bounds(&mut registry);
        let before = super::hud_edit_layout::collect_selection_boxes(&registry, None);
        assert_eq!(before.len(), 19);
        let tracker = before
            .iter()
            .find(|entry| entry.key == "objective_tracker")
            .unwrap()
            .rect;
        let mut gameplay = FrameRegistry::new(1366.0, 768.0);
        let mut game_shared = SharedContext::new();
        game_shared.insert(skin);
        game_shared.insert(MainActionBarState::default());
        let mut game_screen = Screen::new(main_action_bar_screen);
        game_screen.sync(&game_shared, &mut gameplay);
        assert!(gameplay.get_by_name("MultiBarRight").is_none());
        assert!(gameplay.get_by_name("MultiBarLeft").is_none());
        let mut menu_shared = SharedContext::new();
        let mut menu = FrameRegistry::new(1366.0, 768.0);
        let mut menu_screen = Screen::new(game_menu_screen);
        let mut last_y = f32::NEG_INFINITY;
        menu_shared.insert(policy::build_view_model(&options));
        menu_screen.sync(&menu_shared, &mut menu);
        let menu_bounds =
            super::layout::compute_layout_with_intrinsics(&menu, &HashMap::new()).unwrap();
        for number in 2..=5 {
            let name = format!("ExtraActionBaraction_bar_{number}");
            let id = menu.get_by_name(&name).unwrap();
            let label = menu.get_by_name(&format!("{name}Text")).unwrap();
            assert!(
                matches!(&menu.get(label).unwrap().widget_data, Some(ui_toolkit::frame::WidgetData::FontString(data)) if data.text == format!("Action Bar {number}"))
            );
            let y = menu_bounds[&id].y;
            assert!(y > last_y, "Retail checkbox order");
            last_y = y;
        }
        for number in [4, 5] {
            let id = menu
                .get_by_name(&format!("ExtraActionBaraction_bar_{number}"))
                .unwrap();
            let action = menu.click_frame(id).unwrap();
            assert!(policy::apply_toggle(
                policy::parse_toggle_action(&action).unwrap(),
                &mut options
            ));
        }
        let snapshot = policy::apply_snapshot(&mut options);
        let mut file = HudOptionsFile::default();
        policy::apply_hud_file_snapshot(&mut file, &snapshot.hud);
        shared.insert(file.extra_action_bars);
        screen.sync(&shared, &mut registry);
        publish_test_bounds(&mut registry);
        let enabled = super::hud_edit_layout::collect_selection_boxes(&registry, None);
        assert_eq!(enabled.len(), before.len() + 2);
        assert_eq!(
            enabled
                .iter()
                .find(|entry| entry.key == "objective_tracker")
                .unwrap()
                .rect,
            tracker
        );
        game_shared.insert(MainActionBarState {
            extra_action_bars: file.extra_action_bars,
            ..Default::default()
        });
        game_screen.sync(&game_shared, &mut gameplay);
        publish_test_bounds(&mut gameplay);
        let game_boxes = super::hud_edit_layout::collect_selection_boxes(&gameplay, None);
        let expected = if skin == ActiveSkin::Modern {
            [[1315.0, 103.0, 45.0, 562.0], [1268.0, 103.0, 45.0, 562.0]]
        } else {
            [[1312.0, 86.0, 48.0, 596.0], [1265.0, 86.0, 48.0, 596.0]]
        };
        for (key, rect) in ["action_bar_4", "action_bar_5"].into_iter().zip(expected) {
            let editor_rect = enabled.iter().find(|entry| entry.key == key).unwrap().rect;
            assert_eq!(editor_rect, rect, "existing {skin:?} {key} defaults");
            assert_eq!(
                game_boxes
                    .iter()
                    .find(|entry| entry.key == key)
                    .unwrap()
                    .rect,
                rect
            );
            println!("{skin:?} {key} rect={rect:?} tracker={tracker:?}");
        }
        for key in ["action_bar_4", "action_bar_5"] {
            assert!(policy::apply_toggle(key, &mut options));
        }
        let snapshot = policy::apply_snapshot(&mut options);
        policy::apply_hud_file_snapshot(&mut file, &snapshot.hud);
        shared.insert(file.extra_action_bars);
        screen.sync(&shared, &mut registry);
        publish_test_bounds(&mut registry);
        let disabled = super::hud_edit_layout::collect_selection_boxes(&registry, None);
        assert_eq!(disabled.len(), 19);
        assert_eq!(
            disabled
                .iter()
                .find(|entry| entry.key == "objective_tracker")
                .unwrap()
                .rect,
            tracker
        );
        game_shared.insert(MainActionBarState {
            extra_action_bars: file.extra_action_bars,
            ..Default::default()
        });
        game_screen.sync(&game_shared, &mut gameplay);
        assert!(gameplay.get_by_name("MultiBarRight").is_none());
        assert!(gameplay.get_by_name("MultiBarLeft").is_none());
    }
}

fn publish_test_bounds(registry: &mut FrameRegistry) {
    let bounds = super::layout::compute_layout_with_intrinsics(registry, &HashMap::new()).unwrap();
    for (id, rect) in bounds {
        registry.set_computed_layout(id, rect).unwrap();
    }
}

#[path = "hud_edit_polish_tests.rs"]
mod polish;
