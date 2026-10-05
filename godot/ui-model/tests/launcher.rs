use game_engine_core::input_bindings_data::{BindingKey, InputBindingsData};
use game_engine_ui_model::launcher::{
    ACTION_CLOSE, ACTION_OPEN, LauncherKey, LauncherView, SEARCH_FIELD, entries, launcher_screen,
};
use game_engine_ui_model::micro_menu::{ACTION_PLAYER_SPELLS, MICRO_BUTTONS};
use game_engine_ui_model::minimap::{MinimapClusterState, minimap_cluster_screen};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn build(view: LauncherView) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(view);
    Screen::new(launcher_screen).sync(&shared, &mut registry);
    registry
}

#[test]
fn launcher_ctrl_space_toggles_and_escape_closes() {
    let bindings = InputBindingsData::default();
    let mut view = LauncherView::default();
    view.handle_key(BindingKey::Space, true, false, &bindings);
    assert!(view.open);
    view.handle_key(BindingKey::Space, true, false, &bindings);
    assert!(!view.open);
    view.handle_key(BindingKey::Space, true, false, &bindings);
    view.key(LauncherKey::Escape);
    assert!(!view.open);
}

#[test]
fn launcher_filters_any_word_prefix_case_insensitively() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.set_query("SpE");
    let visible = view.filtered_entries();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].action, ACTION_PLAYER_SPELLS);
    let registry = build(view);
    assert!(registry.get_by_name(SEARCH_FIELD).is_some());
    assert!(
        registry
            .get_by_name("LauncherEntryPlayerSpellsMicroButton")
            .is_some()
    );
    assert!(
        registry
            .get_by_name("LauncherEntryCharacterMicroButton")
            .is_none()
    );
}

#[test]
fn launcher_enter_dispatches_selected_micro_action_and_closes() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.set_query("spe");
    assert_eq!(
        view.key(LauncherKey::Enter).as_deref(),
        Some(ACTION_PLAYER_SPELLS)
    );
    assert!(!view.open);
}

#[test]
fn launcher_arrows_move_in_grid_and_click_dispatches_same_action() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.key(LauncherKey::Right);
    assert_eq!(view.selected, 1);
    view.key(LauncherKey::Down);
    assert_eq!(view.selected, 6);
    view.key(LauncherKey::Left);
    view.key(LauncherKey::Up);
    assert_eq!(view.selected, 0);
    let registry = build(view.clone());
    let id = registry
        .get_by_name("LauncherEntryPlayerSpellsMicroButton")
        .unwrap();
    let action = registry.get(id).unwrap().onclick.as_deref().unwrap();
    assert_eq!(view.action(action).as_deref(), Some(ACTION_PLAYER_SPELLS));
    assert!(!view.open);
}

#[test]
fn launcher_has_every_micro_entry_and_requested_shortcuts() {
    let list = entries();
    for micro in MICRO_BUTTONS {
        assert!(
            list.iter()
                .any(|entry| entry.action == format!("micro:{}", micro.name)),
            "{}",
            micro.title
        );
    }
    for label in ["Help", "Bags", "World Map", "Options", "Key Bindings"] {
        assert!(list.iter().any(|entry| entry.label == label), "{label}");
    }
}

#[test]
fn launcher_minimap_search_button_opens_and_close_action_dismisses() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(MinimapClusterState::default());
    Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);
    let id = registry.get_by_name("MinimapLauncherButton").unwrap();
    let mut view = LauncherView::default();
    view.action(registry.get(id).unwrap().onclick.as_deref().unwrap());
    assert!(view.open);
    view.action(ACTION_CLOSE);
    assert!(!view.open);
}

#[test]
fn launcher_empty_results_do_not_dispatch_or_leave_selection_out_of_range() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.set_query("not-a-window");
    view.key(LauncherKey::Down);
    assert_eq!(view.selected, 0);
    assert_eq!(view.key(LauncherKey::Enter), None);
    assert!(view.open);
    view.set_query("map");
    assert_eq!(view.filtered_entries().len(), 1);
}
