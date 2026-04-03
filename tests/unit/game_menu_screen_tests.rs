use super::*;
use crate::scenes::game_menu::options::HudDraft;
use game_engine::ui::{event::EventBus, plugin::UiState};

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
        show_health_bars: true,
        show_target_marker: true,
        show_fps_overlay: true,
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
        show_health_bars: true,
        show_target_marker: true,
        show_fps_overlay: true,
    };

    apply_ui_hud_visibility_for_state(&mut reg, GameState::InWorld, &hud);

    let minimap = reg.get(minimap).expect("minimap frame");
    let action_bar = reg.get(action_bar).expect("action bar frame");
    assert!(!minimap.hidden);
    assert!(minimap.visible);
    assert!(!action_bar.hidden);
    assert!(action_bar.visible);
}

#[test]
fn escape_opens_game_menu_inworld() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(GameMenuScreenPlugin);
    app.insert_state(GameState::InWorld);
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(CameraOptions::default());
    app.insert_resource(HudOptions::default());
    app.insert_resource(ClientOptionsUiState {
        modal_offset: None,
        legacy_modal_position: None,
    });

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();

    assert!(app.world().contains_resource::<UiModalOpen>());
    let ui = app.world().resource::<UiState>();
    assert!(ui.registry.get_by_name(GAME_MENU_ROOT.0).is_some());
}
