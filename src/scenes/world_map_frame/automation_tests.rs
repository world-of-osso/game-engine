//! JS UI automation driving the real InWorld world-map input systems.

use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::window::PrimaryWindow;
use game_engine::ui::automation::{UiAutomationPlugin, UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::event::EventBus;
use game_engine::ui::js_automation::run_js_to_actions;
use game_engine::ui::layout::LayoutRect;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::world_map_frame_component::WorldMapFrameState;
use game_engine::world_map_data::WorldMapState;

use super::WorldMapFramePlugin;
use crate::game_state::GameState;
use crate::networking::CurrentZone;
use crate::window_manager::{WindowId, WindowManager};

fn inworld_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin));
    let mut window = Window::default();
    window.resolution.set(1920.0, 1080.0);
    app.world_mut().spawn((window, PrimaryWindow));
    app.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.init_resource::<WorldMapState>();
    app.init_resource::<CurrentZone>();
    app.init_resource::<WindowManager>();
    app.init_resource::<game_engine::input_bindings::InputBindings>();
    app.insert_state(GameState::InWorld);
    app.add_plugins((
        UiAutomationPlugin,
        WorldMapFramePlugin,
        crate::ui_input_mode::UiInputModePlugin,
    ));
    app.update();
    app
}

fn queue_script(app: &mut App, script: &str) {
    let actions = run_js_to_actions(script).expect("script should parse");
    app.world_mut().resource_mut::<UiAutomationQueue>().0 = actions.into();
}

fn run_until_queue_drained(app: &mut App) {
    for _ in 0..20 {
        app.update();
        if app.world().resource::<UiAutomationQueue>().is_empty() {
            app.update();
            return;
        }
    }
    panic!("automation queue did not drain");
}

/// Stand-in for the native Bevy UI layout readback (`read_bounds`), which needs a
/// render pipeline these headless tests do not run.
fn simulate_layout_readback(app: &mut App, name: &str) {
    let mut ui = app.world_mut().resource_mut::<UiState>();
    let id = ui.registry.get_by_name(name).expect("frame exists");
    let rect = LayoutRect {
        x: 1500.0,
        y: 120.0,
        width: 24.0,
        height: 24.0,
    };
    ui.registry.set_computed_layout(id, rect).unwrap();
}

fn map_open(app: &App) -> bool {
    app.world()
        .resource::<WindowManager>()
        .is_open(WindowId::WorldMap)
}

fn frame_visible(app: &App, name: &str) -> bool {
    let registry = &app.world().resource::<UiState>().registry;
    let id = registry.get_by_name(name).expect("frame exists");
    registry.get(id).unwrap().visible
}

fn assert_no_automation_error(app: &App) {
    assert_eq!(
        app.world().resource::<UiAutomationRunner>().last_error,
        None
    );
}

#[test]
fn js_m_opens_world_map_and_click_close_button_closes_it() {
    let mut app = inworld_app();
    assert!(!map_open(&app));

    queue_script(&mut app, r#"ui.key("M");"#);
    run_until_queue_drained(&mut app);
    assert_no_automation_error(&app);
    assert!(map_open(&app));
    assert!(frame_visible(&app, "WorldMapCloseButton"));
    simulate_layout_readback(&mut app, "WorldMapCloseButton");

    queue_script(&mut app, r#"ui.click("WorldMapCloseButton");"#);
    run_until_queue_drained(&mut app);
    assert_no_automation_error(&app);
    assert!(!map_open(&app));
}

#[test]
fn js_m_with_focused_editbox_does_not_open_world_map() {
    let mut app = inworld_app();
    app.insert_resource(crate::ui_input_mode::tests::ui_with_focused_editbox());
    queue_script(&mut app, r#"ui.key("M");"#);
    run_until_queue_drained(&mut app);
    assert_no_automation_error(&app);
    assert!(!map_open(&app));
}

#[test]
fn js_click_on_hidden_inworld_frame_reports_error_and_advances() {
    let mut app = inworld_app();
    simulate_layout_readback(&mut app, "WorldMapCloseButton");
    queue_script(&mut app, r#"ui.click("WorldMapCloseButton"); ui.key("M");"#);
    run_until_queue_drained(&mut app);
    let error = app
        .world()
        .resource::<UiAutomationRunner>()
        .last_error
        .clone()
        .expect("hidden frame click must fail");
    assert!(error.contains("WorldMapCloseButton"), "{error}");
    assert!(map_open(&app), "queue must continue after the failed click");
}

/// Goldshire flight master (TaxiNodes 582) in engine axes.
const GOLDSHIRE: Vec3 = Vec3::new(-9433.99, 57.0, -85.149);

fn spawn_player_at_goldshire(app: &mut App) -> Entity {
    let mut terrain = crate::terrain::AdtManager::default();
    terrain.map_name = "azeroth".into();
    app.insert_resource(terrain);
    app.world_mut()
        .spawn((
            Transform::from_translation(GOLDSHIRE),
            crate::camera::CharacterFacing { yaw: 0.0 },
            crate::camera::Player,
            crate::networking::LocalPlayer,
        ))
        .id()
}

fn model(app: &App) -> &WorldMapFrameState {
    &app.world().resource::<super::WorldMapFrameModel>().0
}

#[test]
fn m_opens_the_players_zone_and_the_arrow_follows_movement() {
    let mut app = inworld_app();
    if !std::path::Path::new(super::DB2_DIR).exists() {
        return;
    }
    let player = spawn_player_at_goldshire(&mut app);
    queue_script(&mut app, r#"ui.key("M");"#);
    run_until_queue_drained(&mut app);
    let opened = model(&app).clone();
    assert!(opened.visible);
    assert_eq!(opened.map_name, "Elwynn Forest");
    let arrow = opened.player.clone().expect("player arrow on its zone");
    assert!((arrow.x - 0.4178).abs() < 0.002, "{arrow:?}");

    // Engine +Z is world -Y: east on the map.
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation
        .z += 100.0;
    app.update();
    let moved = model(&app).player.clone().unwrap();
    assert!(moved.x > arrow.x, "moving east: {moved:?}");
    assert!((moved.y - arrow.y).abs() < 0.0001);
}
