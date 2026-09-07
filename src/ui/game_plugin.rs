use bevy::prelude::*;
use bevy::{input::ButtonState, input::keyboard::KeyboardInput};
use game_engine::network_runtime::messages::MessageSenders;

use crate::targeting::CurrentTarget;
use crate::ui::plugin::UiState;
use crate::ui::spellbook_runtime::{SpellbookAction, SpellbookKeyInput, SpellbookUiRuntime};
use shared::protocol::{CombatChannel, SpellCastIntent};

pub fn sync_screen_ui(mut state: ResMut<UiState>, runtime: Option<NonSendMut<SpellbookUiRuntime>>) {
    if let Some(mut runtime) = runtime
        && (state.is_changed() || runtime.is_changed())
    {
        runtime.sync(&mut state.registry);
    }
}

pub fn spellbook_cooldowns_active(runtime: Option<NonSend<SpellbookUiRuntime>>) -> bool {
    runtime.is_some_and(|runtime| runtime.has_active_cooldowns())
}

pub fn tick_spellbook_cooldowns(
    mut state: ResMut<UiState>,
    mut runtime: NonSendMut<SpellbookUiRuntime>,
) {
    runtime.advance_cooldowns(
        &mut state.registry,
        1.0 / game_engine::network_tick::NETWORK_TICKS_PER_SECOND as f32,
    );
}

pub fn handle_spellbook_pointer(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
    current_target: Option<Res<CurrentTarget>>,
    mut spell_senders: MessageSenders<SpellCastIntent>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let (Ok(window), Some(mut runtime)) = (windows.single(), runtime) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *last_cursor = None;
        return;
    };

    let x = cursor.x;
    let y = window.height() - cursor.y;
    let position = Vec2::new(x, y);
    if *last_cursor != Some(position) || state.is_changed() || runtime.is_changed() {
        runtime.handle_pointer_move(&mut state.registry, x, y);
        *last_cursor = Some(position);
    }

    let Some(mouse) = mouse else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        let _ = runtime.handle_pointer_button(&mut state.registry, true, x, y);
    }
    if mouse.just_released(MouseButton::Left)
        && let Some(action) = runtime.handle_pointer_button(&mut state.registry, false, x, y)
    {
        send_spellbook_action(action, current_target.as_deref(), &mut spell_senders);
    }
}

pub fn handle_spellbook_keyboard(
    mut key_events: Option<MessageReader<KeyboardInput>>,
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
) {
    let Some(mut runtime) = runtime else { return };
    if !runtime.has_focus() {
        return;
    }
    let Some(mut key_events) = key_events.take() else {
        return;
    };

    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        if let bevy::input::keyboard::Key::Character(text) = &event.logical_key {
            for ch in text.chars() {
                let _ =
                    runtime.handle_key_input(&mut state.registry, SpellbookKeyInput::Character(ch));
            }
            continue;
        }

        let key = match event.key_code {
            KeyCode::ArrowLeft => Some(SpellbookKeyInput::PreviousTab),
            KeyCode::ArrowRight => Some(SpellbookKeyInput::NextTab),
            KeyCode::PageUp => Some(SpellbookKeyInput::PreviousPage),
            KeyCode::PageDown => Some(SpellbookKeyInput::NextPage),
            KeyCode::Backspace => Some(SpellbookKeyInput::Backspace),
            KeyCode::Escape => Some(SpellbookKeyInput::Clear),
            _ => None,
        };

        if let Some(key) = key {
            let _ = runtime.handle_key_input(&mut state.registry, key);
        }
    }
}

#[cfg(test)]
mod idle_tests {
    use super::*;
    use crate::ui::{event::EventBus, frame::WidgetData, registry::FrameRegistry};
    use game_engine::network_runtime::messages::ConnectionSender;

    #[derive(Resource, Default)]
    struct Changes(usize);

    fn record_changes(state: Res<UiState>, mut changes: ResMut<Changes>) {
        if state.is_changed() {
            changes.0 += 1;
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: EventBus::new(),
            focused_frame: None,
        });
        app.insert_non_send(SpellbookUiRuntime::new());
        app.init_resource::<Changes>();
        app.init_resource::<ConnectionSender>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app
    }

    fn first_spell(app: &App) -> String {
        let registry = &app.world().resource::<UiState>().registry;
        let id = registry.get_by_name("SpellBookSpellName1").unwrap();
        let Some(WidgetData::FontString(text)) = &registry.get(id).unwrap().widget_data else {
            panic!("spell name must be a font string");
        };
        text.text.clone()
    }

    #[test]
    fn screen_sync_preserves_initial_output_without_idle_changes() {
        let mut app = app();
        app.add_systems(Update, (sync_screen_ui, record_changes).chain());
        app.update();
        assert_eq!(first_spell(&app), "Avenger's Shield");
        let initial = app.world().resource::<Changes>().0;
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Changes>().0, initial);
        app.world_mut().resource_mut::<UiState>().focused_frame = Some(0);
        app.update();
        assert_eq!(first_spell(&app), "Avenger's Shield");
        app.world_mut().insert_non_send(SpellbookUiRuntime::new());
        app.update();
        assert_eq!(first_spell(&app), "Avenger's Shield");
    }

    fn pointer_app() -> (App, Entity) {
        let mut app = app();
        let mut runtime = app
            .world_mut()
            .remove_non_send::<SpellbookUiRuntime>()
            .unwrap();
        runtime.sync(&mut app.world_mut().resource_mut::<UiState>().registry);
        app.insert_non_send(runtime);
        let mut window = Window::default();
        window.resolution.set(1920.0, 1080.0);
        window.set_cursor_position(Some(Vec2::new(1900.0, 1000.0)));
        let window = app
            .world_mut()
            .spawn((window, bevy::window::PrimaryWindow))
            .id();
        app.add_systems(Update, (handle_spellbook_pointer, record_changes).chain());
        (app, window)
    }

    fn point_at_holy_tab(app: &mut App, window: Entity) {
        let registry = &app.world().resource::<UiState>().registry;
        let id = registry.get_by_name("SpellBookTabPanel4").unwrap();
        let rect = registry.get(id).unwrap().layout_rect.as_ref().unwrap();
        let cursor = Vec2::new(
            rect.x + rect.width / 2.0,
            1080.0 - rect.y - rect.height / 2.0,
        );
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(cursor));
    }

    #[test]
    fn stationary_pointer_is_idle_but_buttons_still_select_tabs() {
        let (mut app, window) = pointer_app();
        point_at_holy_tab(&mut app, window);
        app.update();
        let initial = app.world().resource::<Changes>().0;
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Changes>().0, initial);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().non_send::<SpellbookUiRuntime>().has_focus());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert_eq!(first_spell(&app), "Holy Shock");
    }

    #[test]
    fn pointer_rechecks_cursor_and_ui_changes() {
        let (mut app, window) = pointer_app();
        app.update();
        let initial = app.world().resource::<Changes>().0;
        point_at_holy_tab(&mut app, window);
        app.update();
        assert!(app.world().resource::<Changes>().0 > initial);
        let before = app.world().resource::<Changes>().0;
        app.world_mut().resource_mut::<UiState>().focused_frame = Some(0);
        app.update();
        assert!(app.world().resource::<Changes>().0 > before);
        app.world_mut().insert_non_send(SpellbookUiRuntime::new());
        app.update();
        assert!(app.world().resource::<Changes>().0 > before + 1);
    }
}

fn send_spellbook_action(
    action: SpellbookAction,
    current_target: Option<&CurrentTarget>,
    spell_senders: &mut MessageSenders<SpellCastIntent>,
) {
    let SpellbookAction::CastSpell {
        spell_id,
        spell_name,
    } = action;
    let target_entity = current_target
        .and_then(|target| target.0)
        .map(Entity::to_bits);
    let intent = SpellCastIntent {
        spell_id: Some(spell_id),
        spell: spell_name,
        target_entity,
    };

    for mut sender in spell_senders.iter_mut() {
        sender.send::<CombatChannel>(intent.clone());
    }
}
