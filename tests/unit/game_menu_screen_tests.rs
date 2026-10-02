#[path = "../../src/ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;

use super::*;
use crate::scenes::game_menu::options::HudDraft;
use crate::window_manager::WindowId;
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
        nameplate_style: client_options::NameplateStyle::default(),
        show_health_bars: true,
        show_target_marker: true,
        auto_loot: false,
        soft_target_interact: false,
        soft_target: client_options::SoftTargetOptions::default(),
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
        nameplate_style: client_options::NameplateStyle::default(),
        show_health_bars: true,
        show_target_marker: true,
        auto_loot: false,
        soft_target_interact: false,
        soft_target: client_options::SoftTargetOptions::default(),
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
    app.add_plugins(crate::window_manager::WindowManagerPlugin);
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
    let mut windows = app.world_mut().resource_mut::<WindowManager>();
    windows.open(WindowId::Character);
    windows.open(WindowId::Mail);
    app.update();
}

fn panels_open(app: &App) -> (bool, bool) {
    let windows = app.world().resource::<WindowManager>();
    (
        windows.is_open(WindowId::Character),
        windows.is_open(WindowId::Mail),
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
fn escape_cancels_only_the_popup_when_popup_and_panel_are_open() {
    let mut app = inworld_escape_app(empty_ui());
    app.init_resource::<PopupStack>();
    let popup =
        app.world_mut()
            .resource_mut::<PopupStack>()
            .push(game_engine::ui::popup::PopupSpec {
                key: "duel".into(),
                text: "Duel?".into(),
                accept_label: "Accept".into(),
                cancel_label: Some("Decline".into()),
                timeout: None,
                confirm_text: None,
            });
    open_character_and_mail(&mut app);

    press_escape(&mut app);

    let results = app.world_mut().resource_mut::<PopupStack>().drain_results();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, popup);
    assert_eq!(
        results[0].outcome,
        game_engine::ui::popup::PopupOutcome::Cancelled
    );
    assert!(!app.world().resource::<PopupStack>().is_open());
    assert_eq!(
        panels_open(&app),
        (true, true),
        "panels stay open on press 1"
    );
    assert!(!game_menu_open(&app));

    press_escape(&mut app);
    assert_eq!(
        panels_open(&app),
        (false, false),
        "press 2 closes the panels"
    );
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
fn escape_closes_bags_with_wide_window_in_one_press() {
    let mut app = inworld_escape_app(empty_ui());
    {
        let mut windows = app.world_mut().resource_mut::<WindowManager>();
        windows.open(WindowId::Bag(0));
        windows.open(WindowId::Bag(2));
        windows.open(WindowId::WorldMap);
    }
    app.update();

    press_escape(&mut app);

    assert!(!app.world().resource::<WindowManager>().any_open());
    assert!(!game_menu_open(&app));
}

fn click_at(app: &mut App, cursor: Vec2) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .unwrap()
        .set_cursor_position(Some(cursor));
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.clear();
    mouse.press(MouseButton::Left);
    app.update();
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.clear();
    mouse.release(MouseButton::Left);
    app.update();
}

#[test]
fn interface_reset_window_positions_button_clears_saved_positions() {
    use crate::ui_layout_store::UiLayoutStore;
    let path =
        std::env::temp_dir().join(format!("ui-layout-reset-button-{}.ron", std::process::id()));
    let mut app = inworld_escape_app(empty_ui());
    app.world_mut().spawn((
        Window {
            resolution: (1920, 1080).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.insert_resource(ButtonInput::<MouseButton>::default());
    let mut store = UiLayoutStore::load(path.clone());
    store.set_window_position("11", "CharacterFrame", Vec2::new(300.0, 300.0));
    store.save();
    app.insert_resource(store);
    app.insert_resource(crate::networking::SelectedCharacterId {
        character_id: Some(11),
        character_name: Some("Theron".into()),
    });
    press_escape(&mut app);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .resource_scope(|world, mut overlay: Mut<GameMenuOverlay>| {
            overlay.model.options.view = GameMenuView::Options;
            overlay.model.options.category = OptionsCategory::Interface;
            let mut ui = world.resource_mut::<UiState>();
            sync_overlay_model_only(&mut overlay, &mut ui.registry);
            layout_support::compute_layout(&mut ui.registry);
        });
    let button = {
        let registry = &app.world().resource::<UiState>().registry;
        let id = registry
            .get_by_name("ActionButtonreset_window_positions")
            .expect("reset button");
        registry
            .get(id)
            .unwrap()
            .layout_rect
            .clone()
            .expect("layout")
    };

    click_at(
        &mut app,
        Vec2::new(
            button.x + button.width / 2.0,
            button.y + button.height / 2.0,
        ),
    );

    let store = app.world().resource::<UiLayoutStore>();
    assert_eq!(store.window_position("11", "CharacterFrame"), None);
    assert_eq!(
        UiLayoutStore::load(path.clone()).window_position("11", "CharacterFrame"),
        None
    );
    std::fs::remove_file(&path).unwrap();
}
