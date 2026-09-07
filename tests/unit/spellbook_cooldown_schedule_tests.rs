use super::*;
use game_engine::network_runtime::messages::ConnectionSender;
use game_engine::network_tick::{NetworkTick, NetworkTickPlugin};
use game_engine::ui::{
    event::EventBus, frame::WidgetData, plugin::UiState, registry::FrameRegistry,
    spellbook_runtime::SpellbookUiRuntime,
};
use std::time::Duration;

#[derive(Resource, Default)]
struct UiChanges(usize);

fn record_ui_changes(ui: Res<UiState>, mut changes: ResMut<UiChanges>) {
    if ui.is_changed() {
        changes.0 += 1;
    }
}

fn cooldown_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(GameState::InWorld);
    app.add_plugins(NetworkTickPlugin);
    app.init_resource::<Time>();
    app.init_resource::<ConnectionSender>();
    app.init_resource::<UiChanges>();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_non_send(SpellbookUiRuntime::new());
    register_in_world_systems(&mut app);
    app.add_systems(Last, record_ui_changes);
    app.update();
    app
}

fn cast_first_spell(app: &mut App) {
    let mut runtime = app
        .world_mut()
        .remove_non_send::<SpellbookUiRuntime>()
        .unwrap();
    let mut ui = app.world_mut().resource_mut::<UiState>();
    let id = ui.registry.get_by_name("SpellBookSpellName1").unwrap();
    let rect = ui.registry.get(id).unwrap().layout_rect.clone().unwrap();
    assert!(
        runtime
            .handle_click(
                &mut ui.registry,
                rect.x + rect.width / 2.0,
                rect.y + rect.height / 2.0
            )
            .is_some()
    );
    app.insert_non_send(runtime);
}

fn cooldown_text(app: &App) -> Option<String> {
    let registry = &app.world().resource::<UiState>().registry;
    let id = registry.get_by_name("SpellBookSpellCooldown1")?;
    let WidgetData::FontString(text) = registry.get(id)?.widget_data.as_ref()? else {
        panic!("cooldown label must be text");
    };
    Some(text.text.clone())
}

#[test]
fn cooldown_schedule_does_not_dirty_idle_ui_on_render_frames() {
    let mut app = cooldown_app();
    let initial = app.world().resource::<UiChanges>().0;
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(app.world().resource::<UiChanges>().0, initial);
    app.world_mut().remove_resource::<UiState>();
    app.world_mut().run_schedule(NetworkTick);
}

#[test]
fn cooldown_schedule_ignores_render_delta_and_advances_on_network_ticks() {
    let mut app = cooldown_app();
    cast_first_spell(&mut app);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(1));
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(cooldown_text(&app).as_deref(), Some("15.0"));
    for _ in 0..60 {
        app.world_mut().run_schedule(NetworkTick);
    }
    assert_eq!(cooldown_text(&app).as_deref(), Some("14.0"));
}

#[test]
fn cooldown_schedule_finishes_then_skips_idle_parameters() {
    let mut app = cooldown_app();
    cast_first_spell(&mut app);
    for _ in 0..901 {
        app.world_mut().run_schedule(NetworkTick);
    }
    assert_eq!(cooldown_text(&app), None);
    app.world_mut().remove_resource::<UiState>();
    app.world_mut().run_schedule(NetworkTick);
}
