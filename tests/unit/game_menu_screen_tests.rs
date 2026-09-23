use super::escape_stack::{
    InWorldEscapePanel, close_bag_panel, close_inspect_panel, close_topmost_tracked_panel,
};
use super::*;
use crate::scenes::game_menu::options::HudDraft;
use game_engine::status::InspectStatusSnapshot;
use game_engine::ui::{event::EventBus, plugin::UiState};

#[test]
fn no_ui_preserves_numeric_fps_and_keeps_marker_hidden_after_options() {
    let mut world = World::new();
    world.insert_resource(client_options::UiDisabled);
    world.insert_resource(FpsOverlayConfig::default());
    let marker = world
        .spawn((crate::target::TargetMarker, Visibility::Visible))
        .id();
    apply_fps_overlay_snapshot(&mut world, false);
    apply_target_marker_visibility(&mut world, true);
    let fps = world.resource::<FpsOverlayConfig>();
    assert!(fps.enabled);
    assert!(!fps.frame_time_graph_config.enabled);
    assert_eq!(world.get::<Visibility>(marker), Some(&Visibility::Hidden));
}

#[test]
fn initial_modal_offset_defaults_to_center() {
    let reg = FrameRegistry::new(1920.0, 1080.0);
    let state = ClientOptionsUiState {
        modal_offset: None,
        legacy_modal_position: None,
    };

    assert_eq!(initial_modal_offset(&state, &reg), [0.0, 0.0]);
}

#[test]
fn initial_modal_offset_migrates_legacy_top_left_position() {
    let reg = FrameRegistry::new(1920.0, 1080.0);
    let state = ClientOptionsUiState {
        modal_offset: None,
        legacy_modal_position: Some([100.0, 120.0]),
    };

    assert_eq!(initial_modal_offset(&state, &reg), [-430.0, 130.0]);
}

#[test]
fn clamp_modal_offset_keeps_center_anchor_offset_on_screen() {
    let reg = FrameRegistry::new(1920.0, 1080.0);
    let clamped = clamp_modal_offset(Vec2::new(900.0, -900.0), &reg);

    assert_eq!(clamped, [530.0, -250.0]);
}

#[test]
fn charselect_options_do_not_show_inworld_hud_frames() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let minimap = reg.create_frame("MinimapCluster", None);
    let action_bar = reg.create_frame("MainActionBar", None);
    let hud = HudDraft {
        show_minimap: true,
        show_action_bars: true,
        show_nameplates: true,
        nameplate_distance: 40.0,
        nameplate_health_thickness: client_options::NameplateBarThickness::Thick,
        nameplate_spellbar_thickness: client_options::NameplateBarThickness::Thin,
        show_health_bars: true,
        show_target_marker: true,
        show_fps_overlay: true,
        chat_font_size: 10.0,
    };

    apply_ui_hud_visibility_for_state(&mut reg, GameState::CharSelect, &hud);

    let minimap = reg.get(minimap).expect("minimap frame");
    let action_bar = reg.get(action_bar).expect("action bar frame");
    assert!(minimap.hidden);
    assert!(!minimap.visible);
    assert!(action_bar.hidden);
    assert!(!action_bar.visible);
}

#[test]
fn inworld_options_can_show_inworld_hud_frames() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let minimap = reg.create_frame("MinimapCluster", None);
    let action_bar = reg.create_frame("MainActionBar", None);
    let hud = HudDraft {
        show_minimap: true,
        show_action_bars: true,
        show_nameplates: true,
        nameplate_distance: 40.0,
        nameplate_health_thickness: client_options::NameplateBarThickness::Thick,
        nameplate_spellbar_thickness: client_options::NameplateBarThickness::Thin,
        show_health_bars: true,
        show_target_marker: true,
        show_fps_overlay: true,
        chat_font_size: 10.0,
    };

    apply_ui_hud_visibility_for_state(&mut reg, GameState::InWorld, &hud);

    let minimap = reg.get(minimap).expect("minimap frame");
    let action_bar = reg.get(action_bar).expect("action bar frame");
    assert!(!minimap.hidden);
    assert!(minimap.visible);
    assert!(!action_bar.hidden);
    assert!(action_bar.visible);
}

fn inworld_escape_app(ui: UiState) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_resource::<ui_toolkit::native_render::caret::UiCaretBlocked>();
    app.add_plugins(GameMenuScreenPlugin);
    app.add_plugins(crate::ui_input_mode::UiInputModePlugin);
    app.insert_state(GameState::InWorld);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(ui);
    app.insert_resource(CameraOptions::default());
    app.insert_resource(HudOptions::default());
    app.insert_resource(ClientOptionsUiState {
        modal_offset: None,
        legacy_modal_position: None,
    });
    app.init_resource::<CurrentTarget>();
    app.init_resource::<crate::scenes::character_frame::CharacterFrameOpen>();
    app.init_resource::<crate::scenes::mail_frame::MailFrameOpen>();
    app.update();
    app
}

fn empty_ui() -> UiState {
    UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    }
}

fn press_escape(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release_all();
    keys.clear();
    keys.press(KeyCode::Escape);
    app.update();
}

fn open_character_and_mail(app: &mut App) {
    app.world_mut()
        .resource_mut::<crate::scenes::character_frame::CharacterFrameOpen>()
        .0 = true;
    app.world_mut()
        .resource_mut::<crate::scenes::mail_frame::MailFrameOpen>()
        .0 = true;
    app.update();
}

fn panels_open(app: &App) -> (bool, bool) {
    (
        app.world()
            .resource::<crate::scenes::character_frame::CharacterFrameOpen>()
            .0,
        app.world()
            .resource::<crate::scenes::mail_frame::MailFrameOpen>()
            .0,
    )
}

fn game_menu_open(app: &App) -> bool {
    app.world()
        .resource::<UiState>()
        .registry
        .get_by_name(GAME_MENU_ROOT.0)
        .is_some()
}

#[test]
fn escape_opens_game_menu_inworld() {
    let mut app = inworld_escape_app(empty_ui());

    press_escape(&mut app);

    assert!(app.world().contains_resource::<UiModalOpen>());
    assert!(game_menu_open(&app));
}

#[test]
fn escape_closes_all_panels_then_clears_target_then_opens_menu() {
    let mut app = inworld_escape_app(empty_ui());
    let target = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<CurrentTarget>().0 = Some(target);
    open_character_and_mail(&mut app);

    press_escape(&mut app);
    assert_eq!(
        panels_open(&app),
        (false, false),
        "press 1 closes both panels"
    );
    assert_eq!(app.world().resource::<CurrentTarget>().0, Some(target));
    assert!(!game_menu_open(&app));

    press_escape(&mut app);
    assert_eq!(
        app.world().resource::<CurrentTarget>().0,
        None,
        "press 2 clears target"
    );
    assert!(!game_menu_open(&app));

    press_escape(&mut app);
    assert!(game_menu_open(&app), "press 3 opens the game menu");
}

#[test]
fn escape_with_focused_editbox_only_clears_focus() {
    let mut app = inworld_escape_app(crate::ui_input_mode::tests::ui_with_focused_editbox());
    let target = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<CurrentTarget>().0 = Some(target);
    open_character_and_mail(&mut app);

    press_escape(&mut app);

    let ui = app.world().resource::<UiState>();
    assert_eq!(ui.focused_frame, None);
    assert_eq!(ui.registry.focused_frame, None);
    assert_eq!(panels_open(&app), (true, true));
    assert_eq!(app.world().resource::<CurrentTarget>().0, Some(target));
    assert!(!game_menu_open(&app));
}

#[test]
fn escape_stack_tracks_open_order_and_reopen_promotion() {
    let mut stack = InWorldEscapeStack::default();

    stack.sync(InWorldEscapePanel::Character, true);
    stack.sync(InWorldEscapePanel::WorldMap, true);
    stack.sync(InWorldEscapePanel::Character, false);
    stack.sync(InWorldEscapePanel::Character, true);

    assert_eq!(
        stack.ordered_panels(),
        vec![InWorldEscapePanel::WorldMap, InWorldEscapePanel::Character]
    );
}

#[test]
fn close_topmost_tracked_panel_skips_stale_entries() {
    let mut stack = InWorldEscapeStack::with_open_order(&[
        InWorldEscapePanel::Character,
        InWorldEscapePanel::WorldMap,
        InWorldEscapePanel::Calendar,
    ]);
    let mut attempted = Vec::new();

    let closed = close_topmost_tracked_panel(&mut stack, |panel| {
        attempted.push(panel);
        panel == InWorldEscapePanel::WorldMap
    });

    assert_eq!(closed, Some(InWorldEscapePanel::WorldMap));
    assert_eq!(
        attempted,
        vec![InWorldEscapePanel::Calendar, InWorldEscapePanel::WorldMap]
    );
    assert_eq!(stack.ordered_panels(), vec![InWorldEscapePanel::Character]);
}

#[test]
fn close_bag_panel_clears_all_open_bags() {
    let mut open = crate::scenes::bag_frame::BagFrameOpenState::default();
    open.toggle(0);
    open.toggle(2);

    assert!(close_bag_panel(Some(&mut open)));
    assert!(!open.any_open());
}

#[test]
fn close_inspect_panel_resets_snapshot() {
    let mut snapshot = InspectStatusSnapshot {
        target_name: Some("Valeera".into()),
        last_server_message: Some("inspect ready".into()),
        ..Default::default()
    };

    assert!(close_inspect_panel(Some(&mut snapshot)));
    assert_eq!(snapshot, InspectStatusSnapshot::default());
}
