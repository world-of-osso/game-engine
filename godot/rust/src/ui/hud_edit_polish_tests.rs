use super::{
    ActiveSkin, EditDraft, FrameRegistry, HashMap, HudAnchor, Screen, SharedContext, player_box,
    rects_overlap, save_top_left, ui_layout_data,
};
use game_engine_ui_model::hud_edit_component::*;

#[test]
fn hudeditmodepolish_all_21_movers_clear_manager_and_use_shared_label_policy() {
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
            assert_eq!(boxes.len(), EDIT_MODE_ELEMENTS.len(), "{skin:?} {size:?}");
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
    assert_eq!(draft.panel_position, Some([986.0, 490.0]));
    assert!(draft.move_panel_drag([-100.0, 1000.0], [1366.0, 768.0]));
    assert_eq!(draft.panel_position, Some([0.0, 540.0]));
    assert_eq!(draft.working, layout.elements);
    draft.panel_grab.take();
    assert!(!draft.move_panel_drag([500.0, 500.0], [1366.0, 768.0]));
    draft.exit();
    assert_eq!(draft.panel_position, None);
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
