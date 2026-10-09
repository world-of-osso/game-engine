use super::{
    ActiveSkin, EditDraft, FrameRegistry, HashMap, HudAnchor, Screen, SharedContext, player_box,
    rects_overlap, save_top_left, ui_layout_data,
};
use game_engine_ui_model::hud_edit_component::*;

#[test]
fn hudeditmodepolish_default_19_movers_clear_manager_and_use_shared_label_policy() {
    use game_engine_ui_model::hud_edit_elements::EDIT_MODE_ELEMENTS;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        for size in [[1920.0, 1080.0], [1366.0, 768.0]] {
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(EditModeOverlayState::default());
            let mut registry = FrameRegistry::new(size[0], size[1]);
            let mut screen = Screen::new(super::super::hud_edit_preview::preview_screen);
            screen.sync(&shared, &mut registry);
            let bounds =
                super::super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new())
                    .unwrap();
            for (id, rect) in bounds {
                registry.set_computed_layout(id, rect).unwrap();
            }
            let mut boxes = super::super::hud_edit_layout::collect_selection_boxes(&registry, None);
            assert_eq!(
                boxes.len(),
                EDIT_MODE_ELEMENTS.len() - 2,
                "{skin:?} {size:?}"
            );
            let at = find_panel_position(size, &boxes).expect("clear default manager space");
            let manager = [at[0], at[1], PANEL_W, PANEL_H];
            for entry in &boxes {
                assert!(
                    !rects_overlap(manager, entry.rect),
                    "manager covers {} {skin:?} {size:?}",
                    entry.key
                );
            }
            for selected in 0..boxes.len() {
                boxes[selected].selected = true;
                shared.insert(EditModeOverlayState {
                    boxes: boxes.clone(),
                });
                shared.insert(EditModePanelState {
                    position: Some(at),
                    ..Default::default()
                });
                screen.sync(&shared, &mut registry);
                let bounds = super::super::layout::compute_layout_with_intrinsics(
                    &registry,
                    &HashMap::new(),
                )
                .unwrap();
                for (index, entry) in boxes.iter().enumerate() {
                    let name = selection_box_name(&entry.key);
                    let label = registry.get_by_name(&format!("{name}Label")).unwrap();
                    assert_eq!(
                        super::super::hud_edit_layout::frame_is_visible(&registry, label),
                        index == selected
                    );
                    let rect = &bounds[&label];
                    assert!(!rects_overlap(
                        manager,
                        [rect.x, rect.y, rect.width, rect.height]
                    ));
                    let backing = registry
                        .get(
                            registry
                                .get_by_name(&format!("{name}LabelBacking"))
                                .unwrap(),
                        )
                        .unwrap();
                    assert_eq!(
                        super::super::hud_edit_layout::frame_is_visible(&registry, backing.id),
                        index == selected
                    );
                    if index == selected {
                        let parts =
                            super::super::parts::project_images(backing, rect.width, rect.height);
                        assert!(
                            parts.iter().any(|part| part.color == [0.0, 0.0, 0.0, 1.0]),
                            "opaque label backing {}",
                            entry.key
                        );
                    }
                }
                boxes[selected].selected = false;
            }
        }
    }
}

#[test]
fn hudeditmodepolish_title_drag_clamps_manager_and_preserves_hud_placements() {
    let mut draft = EditDraft::default();
    let layout = ui_layout_data::ActiveLayout::default();
    let panel = [493.0, 190.0, PANEL_W, PANEL_H];
    assert!(!draft.start_panel_drag(panel, [503.0, 200.0]));
    draft.enter(&layout);
    assert!(
        !draft.start_panel_drag(panel, [503.0, 240.0]),
        "controls do not drag the manager"
    );
    assert!(draft.start_panel_drag(panel, [503.0, 200.0]));
    assert!(draft.move_panel_drag([1000.0, 500.0], [1366.0, 768.0]));
    assert_eq!(draft.panel_position, Some([856.0, 490.0]));
    assert!(draft.move_panel_drag([-100.0, 1000.0], [1366.0, 768.0]));
    assert_eq!(draft.panel_position, Some([0.0, 508.0]));
    assert_eq!(draft.working, layout.elements);
    draft.panel_grab.take();
    assert!(!draft.move_panel_drag([500.0, 500.0], [1366.0, 768.0]));
    draft.exit();
    assert_eq!(draft.panel_position, None);
}

#[test]
fn hudeditshowlist_manager_has_retail_basic_labels_in_order_in_both_skins() {
    use ui_toolkit::frame::WidgetData;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(EditModePanelState::default());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(edit_mode_panel_screen).sync(&shared, &mut registry);
        let bounds =
            super::super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new())
                .unwrap();
        let entries = [
            ("target_and_focus", "Target and Focus"),
            ("raid_frames", "Raid Frames"),
            ("party_frames", "Party Frames"),
            ("buffs_and_debuffs", "Buffs and Debuffs"),
            ("cast_bar", "Cast Bar"),
        ];
        let mut positions = Vec::new();
        for (key, label) in entries {
            let name = format!("EditModeManagerShow_{key}");
            let id = registry
                .get_by_name(&name)
                .expect("manager category checkbox missing");
            let text = registry
                .get_by_name(&format!("{name}Label"))
                .expect("checkbox label missing");
            assert!(
                matches!(&registry.get(text).unwrap().widget_data, Some(WidgetData::FontString(data)) if data.text == label)
            );
            let rect = &bounds[&id];
            positions.push((rect.y, rect.x));
            assert!(
                registry.get_by_name(&format!("{name}Check")).is_some(),
                "initially checked"
            );
        }
        assert!(
            positions.windows(2).all(|pair| pair[0] < pair[1]),
            "Retail row-major basic order"
        );
    }
}

#[test]
fn hudeditshowlist_checkbox_actions_toggle_movers_without_changing_layout() {
    use super::super::hud_edit_layout::collect_selection_boxes;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let path =
        std::env::temp_dir().join(format!("hudeditshowlist-toggle-{}.ron", std::process::id()));
    let layout = ui_layout_data::set_active_layout(&path, 57, "Forever").unwrap();
    let mut draft = EditDraft::default();
    draft.enter(&layout);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(EditModeOverlayState::default());
    Screen::new(super::super::hud_edit_preview::preview_screen).sync(&shared, &mut registry);
    let bounds =
        super::super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
    for (id, rect) in bounds {
        registry.set_computed_layout(id, rect).unwrap();
    }
    let original = collect_selection_boxes(&registry, None);
    for (key, movers) in [
        (
            "target_and_focus",
            vec!["target_frame", "target_of_target", "focus_frame"],
        ),
        ("raid_frames", vec!["raid_frames"]),
        ("party_frames", vec!["party_frames"]),
        ("buffs_and_debuffs", vec!["buffs", "debuffs"]),
        ("cast_bar", vec!["cast_bar"]),
    ] {
        let action = click_manager_button(
            EditModePanelState::default(),
            &format!("EditModeManagerShow_{key}"),
        )
        .expect("checkbox emits action");
        assert_eq!(action, format!("edit_mode_show_{key}"));
        draft.selected = Some(movers[0].into());
        draft.hovered = Some(movers[0].into());
        draft.drag = Some(game_engine_ui_model::hud_edit::Drag {
            key: movers[0].into(),
            grab: [4.0, 8.0],
        });
        crate::hud_edit::apply_manager_action(&path, 57, &action, "", &layout, &mut draft).unwrap();
        let hidden = collect_selection_boxes(&registry, None);
        assert!(
            hidden
                .iter()
                .all(|entry| !movers.contains(&entry.key.as_str()))
        );
        assert_eq!(draft.selected, None);
        assert_eq!(draft.hovered, None);
        assert!(draft.drag.is_none());
        assert_eq!(ui_layout_data::active_layout(&path, 57).unwrap(), layout);
        crate::hud_edit::apply_manager_action(&path, 57, &action, "", &layout, &mut draft).unwrap();
        assert_eq!(
            collect_selection_boxes(&registry, None),
            original,
            "checkbox restores exact mover rectangles"
        );
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn hudeditshowlist_account_preferences_survive_reopen_character_layout_and_exit() {
    use super::super::hud_edit_layout::publish_account_settings;
    let path = std::env::temp_dir().join(format!(
        "hudeditshowlist-account-{}.ron",
        std::process::id()
    ));
    let other_account = path.with_extension("other.ron");
    let layout = ui_layout_data::set_active_layout(&path, 57, "Modern").unwrap();
    let mut draft = EditDraft::default();
    draft.enter(&layout);
    let action = click_manager_button(
        EditModePanelState::default(),
        "EditModeManagerShow_party_frames",
    )
    .unwrap();
    crate::hud_edit::apply_manager_action(&path, 57, &action, "", &layout, &mut draft).unwrap();
    crate::hud_edit::apply_manager_action(
        &path,
        57,
        ACTION_EDIT_MODE_REVERT,
        "",
        &layout,
        &mut draft,
    )
    .unwrap();
    crate::hud_edit::apply_manager_action(
        &path,
        57,
        ACTION_EDIT_MODE_EXIT,
        "",
        &layout,
        &mut draft,
    )
    .unwrap();
    assert!(!draft.active);
    let second = ui_layout_data::set_active_layout(&path, 58, "Forever").unwrap();
    assert_eq!(
        ui_layout_data::active_layout(&path, 57).unwrap().name,
        "Modern"
    );
    draft.enter(&second);
    ui_layout_data::create_layout(&path, 58, "Raid Night", Default::default()).unwrap();
    let reloaded = ui_layout_data::edit_mode_account_settings(&path).unwrap();
    assert!(!system_is_shown(&reloaded, "party_frames"));
    assert!(system_is_shown(
        &ui_layout_data::edit_mode_account_settings(&other_account).unwrap(),
        "party_frames"
    ));
    publish_account_settings(reloaded.clone());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(EditModePanelState {
            show_systems: reloaded.clone(),
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(edit_mode_panel_screen).sync(&shared, &mut registry);
        let check = registry
            .get_by_name("EditModeManagerShow_party_framesCheck")
            .unwrap();
        assert!(
            !super::super::hud_edit_layout::frame_is_visible(&registry, check),
            "reopened manager is unchecked"
        );
        let target = registry
            .get_by_name("EditModeManagerShow_target_and_focusCheck")
            .unwrap();
        assert!(super::super::hud_edit_layout::frame_is_visible(
            &registry, target
        ));
    }
    assert_eq!(
        ui_layout_data::layout_names(&path).unwrap(),
        ["Modern", "Forever", "Raid Night"]
    );
    publish_account_settings(Default::default());
    std::fs::remove_file(path).unwrap();
}

fn click_manager_button(state: EditModePanelState, button: &str) -> Option<String> {
    use super::super::{RegistryModel, ScreenPostsetup};
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut model = RegistryModel {
        screen: Screen::new(edit_mode_panel_screen),
        shared,
        registry: FrameRegistry::new(1920.0, 1080.0),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    model.sync();
    let id = model.registry.get_by_name(button).unwrap();
    let mut actions = std::collections::VecDeque::new();
    model.queue_click_action(&mut actions, id);
    assert!(actions.len() <= 1);
    actions.pop_front()
}

fn click_and_apply_manager(
    path: &std::path::Path,
    layout: &mut ui_layout_data::ActiveLayout,
    draft: &mut EditDraft,
    button: &str,
    expected: &str,
    name: &str,
) {
    let state = EditModePanelState {
        layout_name: layout.name.clone(),
        name_draft: name.into(),
        layout_names: ui_layout_data::layout_names(path).unwrap(),
        pending_delete: draft.pending_delete.clone(),
        preset: ui_layout_data::SYSTEM_PRESETS
            .iter()
            .any(|(name, _)| *name == layout.name),
        dirty: draft.working != layout.elements,
        ..Default::default()
    };
    let action = click_manager_button(state, button).expect("enabled button emits action");
    assert_eq!(action, expected);
    if let Some(updated) =
        crate::hud_edit::apply_manager_action(path, 57, &action, name, layout, draft).unwrap()
    {
        *layout = updated;
    }
}

#[test]
fn hudeditmodepolish_manager_clicks_drive_real_draft_and_persistence_transitions() {
    for skin in ["Modern", "Forever"] {
        let path = std::env::temp_dir().join(format!(
            "hudeditmodepolish-manager-{skin}-{}.ron",
            std::process::id()
        ));
        let mut layout = ui_layout_data::set_active_layout(&path, 57, skin).unwrap();
        let mut draft = EditDraft::default();
        draft.enter(&layout);
        let placement = save_top_left(
            HudAnchor::Bottom,
            [104.0, 312.0],
            [240.0, 60.0],
            [1366.0, 768.0],
        );
        draft.working.insert("player_frame".into(), placement);
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameNew",
            ACTION_EDIT_MODE_NEW,
            "Polished",
        );
        assert_eq!(layout.name, "Polished");
        assert_eq!(
            ui_layout_data::active_layout(&path, 57).unwrap().elements,
            draft.working
        );
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameRename",
            ACTION_EDIT_MODE_RENAME,
            "Renamed",
        );
        assert_eq!(layout.name, "Renamed");
        assert_eq!(
            ui_layout_data::layout_names(&path).unwrap(),
            ["Modern", "Forever", "Renamed"]
        );
        draft.selected = Some("player_frame".into());
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameReset",
            ACTION_EDIT_MODE_RESET,
            "",
        );
        assert!(draft.working.is_empty());
        assert_eq!(
            ui_layout_data::active_layout(&path, 57).unwrap().elements["player_frame"],
            placement
        );
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameRevert",
            ACTION_EDIT_MODE_REVERT,
            "",
        );
        assert_eq!(draft.working, layout.elements);
        draft.selected = Some("player_frame".into());
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameReset",
            ACTION_EDIT_MODE_RESET,
            "",
        );
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameSave",
            ACTION_EDIT_MODE_SAVE,
            "",
        );
        assert!(layout.elements.is_empty());
        assert!(
            ui_layout_data::active_layout(&path, 57)
                .unwrap()
                .elements
                .is_empty()
        );
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameNext",
            ACTION_EDIT_MODE_NEXT_LAYOUT,
            "",
        );
        assert_eq!(layout.name, "Modern");
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFramePrev",
            ACTION_EDIT_MODE_PREV_LAYOUT,
            "",
        );
        assert_eq!(layout.name, "Renamed");
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameDelete",
            ACTION_EDIT_MODE_DELETE,
            "",
        );
        assert_eq!(layout.name, "Renamed");
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeDeleteLayoutDialogYes",
            ACTION_EDIT_MODE_CONFIRM_DELETE,
            "",
        );
        assert_eq!(layout.name, skin);
        assert_eq!(
            ui_layout_data::layout_names(&path).unwrap(),
            ["Modern", "Forever"]
        );
        draft.working.insert("player_frame".into(), placement);
        click_and_apply_manager(
            &path,
            &mut layout,
            &mut draft,
            "EditModeManagerFrameExit",
            ACTION_EDIT_MODE_EXIT,
            "",
        );
        assert!(!draft.active);
        assert!(draft.working.is_empty());
        assert_eq!(ui_layout_data::active_layout(&path, 57).unwrap(), layout);
        assert!(
            ui_layout_data::active_layout(&path, 58)
                .unwrap()
                .elements
                .is_empty()
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn hudeditmodepolish_disabled_manager_buttons_do_not_emit_actions() {
    let state = EditModePanelState {
        preset: true,
        dirty: false,
        ..Default::default()
    };
    for button in [
        "EditModeManagerFrameRename",
        "EditModeManagerFrameDelete",
        "EditModeManagerFrameRevert",
        "EditModeManagerFrameSave",
    ] {
        assert_eq!(
            click_manager_button(state.clone(), button),
            None,
            "{button}"
        );
    }
}

#[test]
fn hudeditmodepolish_hover_uses_instructions_and_leaving_hides_unselected_label() {
    use ui_toolkit::frame::WidgetData;
    let mut draft = EditDraft::default();
    let mut entry = player_box();
    draft.update_hover(&[entry.clone()], [110.0, 210.0]);
    assert_eq!(draft.hovered.as_deref(), Some("player_frame"));
    entry.hovered = true;
    let mut shared = SharedContext::new();
    shared.insert(EditModeOverlayState { boxes: vec![entry] });
    let mut registry = FrameRegistry::new(1366.0, 768.0);
    Screen::new(edit_mode_overlay_screen).sync(&shared, &mut registry);
    let label = registry
        .get(
            registry
                .get_by_name("EditModeSelection_player_frameLabel")
                .unwrap(),
        )
        .unwrap();
    assert!(super::super::hud_edit_layout::frame_is_visible(
        &registry, label.id
    ));
    let Some(WidgetData::FontString(text)) = &label.widget_data else {
        panic!("label text");
    };
    assert_eq!(text.text, "Click to edit");
    draft.update_hover(&[player_box()], [600.0, 400.0]);
    assert_eq!(draft.hovered, None);
}

#[test]
fn hudeditmodepolish_tracker_selection_follows_retail_default_height_below_header() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    // Blizzard_ObjectiveTrackerContainer.lua:203-209: own-unit height = 1080 + offsetY.
    let forever_scale = 260.0 / 288.0;
    for (skin, top, height) in [
        (ActiveSkin::Modern, 275.0, 805.0),
        (
            ActiveSkin::Forever,
            // Layout pixel-rounds the top under the minimap, 268 + 4 * 260/288 = 271.61;
            // the box still ends at 1080 tracker units, 975 on screen.
            272.0,
            1080.0 * forever_scale - 272.0,
        ),
    ] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(EditModeOverlayState::default());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(super::super::hud_edit_preview::preview_screen).sync(&shared, &mut registry);
        let bounds =
            super::super::layout::compute_layout_with_intrinsics(&registry, &HashMap::new())
                .unwrap();
        for (id, rect) in &bounds {
            registry.set_computed_layout(*id, rect.clone()).unwrap();
        }
        let mut tracker = super::super::hud_edit_layout::collect_selection_boxes(
            &registry,
            Some("objective_tracker"),
        )
        .into_iter()
        .find(|entry| entry.key == "objective_tracker")
        .expect("tracker mover");
        let [_, y, _, h] = tracker.rect;
        assert!((y - top).abs() < 0.01, "{skin:?} top {y}");
        assert!((h - height).abs() < 0.01, "{skin:?} height {h}");
        let header = |name: &str| {
            let rect = &bounds[&registry.get_by_name(name).unwrap()];
            [rect.x, rect.y, rect.width, rect.height]
        };
        let headers = [
            header("ObjectiveTrackerFrameHeaderBackground"),
            header("ObjectiveTrackerFrameHeaderText"),
        ];
        tracker.selected = true;
        let mut overlay = SharedContext::new();
        overlay.insert(EditModeOverlayState {
            boxes: vec![tracker],
        });
        let mut overlay_registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(edit_mode_overlay_screen).sync(&overlay, &mut overlay_registry);
        let overlay_bounds = super::super::layout::compute_layout_with_intrinsics(
            &overlay_registry,
            &HashMap::new(),
        )
        .unwrap();
        let name = selection_box_name("objective_tracker");
        for part in ["Label", "LabelBacking"] {
            let id = overlay_registry
                .get_by_name(&format!("{name}{part}"))
                .unwrap();
            let rect = &overlay_bounds[&id];
            let rect = [rect.x, rect.y, rect.width, rect.height];
            assert!(
                (rect[1] + rect[3] / 2.0 - (top + height / 2.0)).abs() <= 1.0,
                "{skin:?} {part} {rect:?} not centred in the selection box"
            );
            for header in headers {
                assert!(
                    !rects_overlap(rect, header),
                    "{skin:?} {part} {rect:?} overlaps header {header:?}"
                );
            }
        }
    }
    ui_toolkit::atlas::set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn hudnames_new_rejects_empty_names_without_action_or_persistence() {
    let path = std::env::temp_dir().join(format!("hudnames-invalid-{}.ron", std::process::id()));
    let layout = ui_layout_data::set_active_layout(&path, 57, "Modern").unwrap();
    ui_layout_data::create_layout(&path, 58, "Raid Night", Default::default()).unwrap();
    for name in ["", "   ", "Modern", "Forever", "Raid Night", " Raid Night "] {
        let state = EditModePanelState {
            name_draft: name.into(),
            layout_names: ui_layout_data::layout_names(&path).unwrap(),
            ..Default::default()
        };
        assert_eq!(
            click_manager_button(state, "EditModeManagerFrameNew"),
            None,
            "{name:?}"
        );
        let mut draft = EditDraft::default();
        draft.enter(&layout);
        assert!(
            crate::hud_edit::apply_manager_action(
                &path,
                57,
                ACTION_EDIT_MODE_NEW,
                name,
                &layout,
                &mut draft
            )
            .is_err()
        );
        assert_eq!(
            ui_layout_data::layout_names(&path).unwrap(),
            ["Modern", "Forever", "Raid Night"]
        );
        assert_eq!(ui_layout_data::active_layout(&path, 57).unwrap(), layout);
    }
    let mut draft = EditDraft::default();
    let mut active = layout;
    draft.enter(&active);
    click_and_apply_manager(
        &path,
        &mut active,
        &mut draft,
        "EditModeManagerFrameNew",
        ACTION_EDIT_MODE_NEW,
        "Dungeon Night",
    );
    assert_eq!(active.name, "Dungeon Night");
    assert_eq!(ui_layout_data::active_layout(&path, 57).unwrap(), active);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn hudnames_delete_request_keeps_layout_and_active_selection() {
    let path = std::env::temp_dir().join(format!("hudnames-delete-{}.ron", std::process::id()));
    ui_layout_data::set_active_layout(&path, 57, "Forever").unwrap();
    let layout =
        ui_layout_data::create_layout(&path, 57, "Raid Night", Default::default()).unwrap();
    let mut draft = EditDraft::default();
    draft.enter(&layout);
    assert!(
        crate::hud_edit::apply_manager_action(
            &path,
            57,
            ACTION_EDIT_MODE_DELETE,
            "",
            &layout,
            &mut draft
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(ui_layout_data::active_layout(&path, 57).unwrap(), layout);
    assert_eq!(
        ui_layout_data::layout_names(&path).unwrap(),
        ["Modern", "Forever", "Raid Night"]
    );
    assert_eq!(draft.pending_delete.as_deref(), Some("Raid Night"));
    draft.selected = Some("player_frame".into());
    let mut active = layout;
    click_and_apply_manager(
        &path,
        &mut active,
        &mut draft,
        "EditModeDeleteLayoutDialogNo",
        ACTION_EDIT_MODE_CANCEL_DELETE,
        "",
    );
    assert_eq!(draft.pending_delete, None);
    assert_eq!(draft.selected.as_deref(), Some("player_frame"));
    assert_eq!(active.name, "Raid Night");
    assert_eq!(ui_layout_data::active_layout(&path, 57).unwrap(), active);
    click_and_apply_manager(
        &path,
        &mut active,
        &mut draft,
        "EditModeManagerFrameDelete",
        ACTION_EDIT_MODE_DELETE,
        "",
    );
    click_and_apply_manager(
        &path,
        &mut active,
        &mut draft,
        "EditModeDeleteLayoutDialogYes",
        ACTION_EDIT_MODE_CONFIRM_DELETE,
        "",
    );
    assert_eq!(draft.pending_delete, None);
    assert_eq!(active.name, "Forever");
    assert_eq!(ui_layout_data::active_layout(&path, 57).unwrap(), active);
    assert_eq!(
        ui_layout_data::layout_names(&path).unwrap(),
        ["Modern", "Forever"]
    );
    std::fs::remove_file(path).unwrap();
}
