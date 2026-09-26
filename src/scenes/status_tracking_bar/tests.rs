use bevy::window::PrimaryWindow;
use game_engine::ui::event::EventBus;
use shared::protocol::PlayerXpUpdate;

use super::*;

#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;

const SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_state(GameState::Loading);
    app.insert_resource(UiState {
        registry: FrameRegistry::new(SCREEN.x, SCREEN.y),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.world_mut().spawn((
        Window {
            resolution: (SCREEN.x as u32, SCREEN.y as u32).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.add_plugins(StatusTrackingBarPlugin);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::InWorld);
    app.update();
    app
}

fn receive_xp(app: &mut App, xp: u32, next_level_xp: u32) {
    app.world_mut().resource_mut::<ExperienceState>().0 = Some(PlayerXpUpdate {
        xp,
        next_level_xp,
        rested_xp: 0,
    });
    app.update();
    layout_support::compute_layout(&mut app.world_mut().resource_mut::<UiState>().registry);
}

fn frame<'a>(app: &'a App, name: &str) -> &'a ui_toolkit::frame::Frame {
    let registry = &app.world().resource::<UiState>().registry;
    registry
        .get(registry.get_by_name(name).expect(name))
        .expect(name)
}

fn set_cursor(app: &mut App, cursor: Vec2) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .unwrap()
        .set_cursor_position(Some(cursor));
}

#[test]
fn bar_appears_and_advances_with_player_xp_updates() {
    let mut app = app();
    assert!(frame(&app, "MainStatusTrackingBarContainer").hidden);

    receive_xp(&mut app, 100, 400);
    assert!(!frame(&app, "MainStatusTrackingBarContainer").hidden);
    let fill = |app: &App| {
        frame(app, "MainStatusTrackingBarContainerExpBarFill")
            .layout_rect
            .clone()
            .unwrap()
            .width
    };
    // Layout snaps to whole pixels.
    assert!((fill(&app) - 565.0 * 0.25).abs() <= 0.5);

    receive_xp(&mut app, 300, 400);
    assert!((fill(&app) - 565.0 * 0.75).abs() <= 0.5);

    // Level cap: the server sends next_level_xp 0.
    receive_xp(&mut app, 0, 0);
    assert!(frame(&app, "MainStatusTrackingBarContainer").hidden);
}

#[test]
fn hovering_the_bar_shows_its_text() {
    let mut app = app();
    receive_xp(&mut app, 45, 400);
    assert!(frame(&app, "MainStatusTrackingBarContainerExpBarText").hidden);

    let bar = frame(&app, "MainStatusTrackingBarContainerExpBar")
        .layout_rect
        .clone()
        .unwrap();
    set_cursor(&mut app, Vec2::new(bar.x + 100.0, bar.y + 5.0));
    app.update();
    assert!(!frame(&app, "MainStatusTrackingBarContainerExpBarText").hidden);

    set_cursor(&mut app, Vec2::new(bar.x + 100.0, bar.y - 200.0));
    app.update();
    assert!(frame(&app, "MainStatusTrackingBarContainerExpBarText").hidden);
}
