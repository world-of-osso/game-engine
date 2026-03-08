use bevy::prelude::*;
use bevy::{input::ButtonState, input::keyboard::KeyboardInput};
use lightyear::prelude::MessageSender;

use crate::targeting::CurrentTarget;
use crate::ui::dioxus_runtime::{DioxusUiRuntime, SpellbookAction, SpellbookKeyInput};
use crate::ui::event::EventBus;
use crate::ui::registry::FrameRegistry;
use crate::ui::wasm_host::WasmHost;
use shared::protocol::{CombatChannel, SpellCastIntent};

/// Central UI state, accessible as a Bevy Resource.
#[derive(Resource)]
pub struct UiState {
    pub registry: FrameRegistry,
    pub event_bus: EventBus,
    pub wasm_host: WasmHost,
    /// Currently focused frame (receives keyboard input).
    pub focused_frame: Option<u64>,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let state = UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: EventBus::new(),
            wasm_host: WasmHost::new(),
            focused_frame: None,
        };
        app.insert_resource(state);
        app.insert_non_send_resource(DioxusUiRuntime::new());
        app.add_systems(Startup, crate::ui::render::setup_ui_camera);
        app.add_systems(
            Update,
            (
                recompute_layout,
                crate::ui::render::sync_ui_quads,
                crate::ui::render::sync_ui_button_highlights,
                crate::ui::render_text::sync_ui_text,
                crate::ui::render_border::sync_ui_borders,
                crate::ui::render_nine_slice::sync_ui_nine_slices,
                crate::ui::render_tiled::sync_ui_tiled_textures,
                crate::ui::render_text_fx::sync_ui_text_shadows,
                crate::ui::render_text_fx::sync_ui_text_outlines,
            )
                .chain(),
        );
    }
}

pub fn sync_dioxus_ui(mut state: ResMut<UiState>, mut runtime: NonSendMut<DioxusUiRuntime>) {
    runtime.sync(&mut state.registry);
}

pub fn tick_spellbook_cooldowns(
    time: Option<Res<Time>>,
    mut state: ResMut<UiState>,
    mut runtime: NonSendMut<DioxusUiRuntime>,
) {
    let Some(time) = time else {
        return;
    };
    runtime.advance_cooldowns(&mut state.registry, time.delta_secs());
}

pub fn handle_spellbook_pointer(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut state: ResMut<UiState>,
    mut runtime: NonSendMut<DioxusUiRuntime>,
    current_target: Option<Res<CurrentTarget>>,
    mut spell_senders: Query<&mut MessageSender<SpellCastIntent>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let x = cursor.x;
    let y = window.height() - cursor.y;
    runtime.handle_pointer_move(&mut state.registry, x, y);

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
    mut runtime: NonSendMut<DioxusUiRuntime>,
) {
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

fn send_spellbook_action(
    action: SpellbookAction,
    current_target: Option<&CurrentTarget>,
    spell_senders: &mut Query<&mut MessageSender<SpellCastIntent>>,
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

fn recompute_layout(mut state: ResMut<UiState>) {
    crate::ui::layout::recompute_layouts(&mut state.registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_adds_ui_state() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        assert!(app.world().get_resource::<UiState>().is_some());
    }
}
