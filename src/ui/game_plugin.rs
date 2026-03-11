use bevy::prelude::*;
use bevy::{input::ButtonState, input::keyboard::KeyboardInput};
use lightyear::prelude::MessageSender;

use crate::targeting::CurrentTarget;
use crate::ui::spellbook_runtime::{SpellbookUiRuntime, SpellbookAction, SpellbookKeyInput};
use crate::ui::plugin::UiState;
use shared::protocol::{CombatChannel, SpellCastIntent};

pub fn sync_screen_ui(mut state: ResMut<UiState>, runtime: Option<NonSendMut<SpellbookUiRuntime>>) {
    if let Some(mut runtime) = runtime {
        runtime.sync(&mut state.registry);
    }
}

pub fn tick_spellbook_cooldowns(
    time: Option<Res<Time>>,
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
) {
    let (Some(time), Some(mut runtime)) = (time, runtime) else {
        return;
    };
    runtime.advance_cooldowns(&mut state.registry, time.delta_secs());
}

pub fn handle_spellbook_pointer(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut state: ResMut<UiState>,
    runtime: Option<NonSendMut<SpellbookUiRuntime>>,
    current_target: Option<Res<CurrentTarget>>,
    mut spell_senders: Query<&mut MessageSender<SpellCastIntent>>,
) {
    let (Ok(window), Some(mut runtime)) = (windows.single(), runtime) else {
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
