//! InWorld automation input: `ClickFrame` / `PressKey` actions are injected as real
//! mouse/keyboard input (cursor position + `MouseButtonInput` / `KeyboardInput`
//! messages) so in-world panels react through their production input systems.
//!
//! Each input action spans three frames: press, release, then pop. Popping only after
//! the release has been processed keeps following actions (dumps, waits) ordered after
//! the input's effects.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::game_state_enum::GameState;
use crate::ui::automation::{KeyChord, UiAutomationAction, UiAutomationQueue, UiAutomationRunner};
use crate::ui::input::find_frame_at;
use crate::ui::plugin::UiState;
use crate::ui::registry::FrameRegistry;

#[derive(Resource, Default, Debug)]
pub(crate) enum InWorldAutomationInput {
    #[default]
    Idle,
    Pressed(HeldInput),
    Released,
}

#[derive(Debug, Clone)]
pub(crate) enum HeldInput {
    Mouse,
    Keys(KeyChord),
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<InWorldAutomationInput>();
    app.add_systems(
        Update,
        process_inworld_automation_input
            .run_if(in_state(GameState::InWorld))
            .run_if(inworld_input_pending),
    );
}

fn inworld_input_pending(
    queue: Res<UiAutomationQueue>,
    phase: Res<InWorldAutomationInput>,
) -> bool {
    !matches!(*phase, InWorldAutomationInput::Idle)
        || queue
            .peek()
            .is_some_and(UiAutomationAction::is_input_action)
}

#[derive(bevy::ecs::system::SystemParam)]
struct InputWriters<'w> {
    mouse: MessageWriter<'w, MouseButtonInput>,
    keys: MessageWriter<'w, KeyboardInput>,
}

fn process_inworld_automation_input(
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    ui: Res<UiState>,
    mut phase: ResMut<InWorldAutomationInput>,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
    mut writers: InputWriters,
) {
    let Ok((window_entity, mut window)) = windows.single_mut() else {
        fail(
            &mut queue,
            &mut runner,
            &mut phase,
            "InWorld automation requires a primary window".into(),
        );
        return;
    };
    match std::mem::take(&mut *phase) {
        InWorldAutomationInput::Pressed(held) => {
            write_release(&held, window_entity, &mut writers);
            *phase = InWorldAutomationInput::Released;
        }
        InWorldAutomationInput::Released => {
            queue.pop();
        }
        InWorldAutomationInput::Idle => {
            let Some(action) = queue.peek().cloned() else {
                return;
            };
            match press_action(
                &action,
                &ui.registry,
                window_entity,
                &mut window,
                &mut writers,
            ) {
                Ok(held) => *phase = InWorldAutomationInput::Pressed(held),
                Err(err) => fail(&mut queue, &mut runner, &mut phase, err),
            }
        }
    }
}

fn fail(
    queue: &mut UiAutomationQueue,
    runner: &mut UiAutomationRunner,
    phase: &mut InWorldAutomationInput,
    err: String,
) {
    error!("UI automation failed in InWorld: {err}");
    runner.last_error = Some(err);
    *phase = InWorldAutomationInput::Idle;
    queue.pop();
}

fn press_action(
    action: &UiAutomationAction,
    registry: &FrameRegistry,
    window_entity: Entity,
    window: &mut Window,
    writers: &mut InputWriters,
) -> Result<HeldInput, String> {
    match action {
        UiAutomationAction::ClickFrame(name) => {
            window.set_cursor_position(Some(click_window_position(registry, name)?));
            writers
                .mouse
                .write(mouse_input(ButtonState::Pressed, window_entity));
            Ok(HeldInput::Mouse)
        }
        UiAutomationAction::PressKey(chord) => {
            let shift = chord_has_shift(chord);
            for &key in chord.modifiers.iter().chain(std::iter::once(&chord.key)) {
                writers.keys.write(keyboard_input(
                    key,
                    shift,
                    ButtonState::Pressed,
                    window_entity,
                ));
            }
            Ok(HeldInput::Keys(chord.clone()))
        }
        UiAutomationAction::TypeText(_) => {
            Err("ui.type is not supported in InWorld; use ui.key".to_string())
        }
        other => Err(format!("{other:?} is not an input action")),
    }
}

fn write_release(held: &HeldInput, window_entity: Entity, writers: &mut InputWriters) {
    match held {
        HeldInput::Mouse => {
            writers
                .mouse
                .write(mouse_input(ButtonState::Released, window_entity));
        }
        HeldInput::Keys(chord) => {
            let shift = chord_has_shift(chord);
            for &key in std::iter::once(&chord.key).chain(chord.modifiers.iter().rev()) {
                writers.keys.write(keyboard_input(
                    key,
                    shift,
                    ButtonState::Released,
                    window_entity,
                ));
            }
        }
    }
}

/// Window (logical) cursor position for clicking the named frame: its UI-space center
/// times `ui_scale`, the inverse of `ui_cursor_position`.
fn click_window_position(registry: &FrameRegistry, name: &str) -> Result<Vec2, String> {
    Ok(clickable_frame_center(registry, name)? * registry.ui_scale)
}

/// Center of the named frame in UI coordinates, verified to be the topmost hit
/// there (or covered only by its own descendants), as a real click would require.
fn clickable_frame_center(registry: &FrameRegistry, name: &str) -> Result<Vec2, String> {
    let id = registry
        .get_by_name(name)
        .ok_or_else(|| format!("unknown InWorld frame '{name}'"))?;
    let frame = registry
        .get(id)
        .ok_or_else(|| format!("frame '{name}' missing"))?;
    if !frame.visible {
        return Err(format!("InWorld frame '{name}' is hidden"));
    }
    let rect = frame
        .layout_rect
        .as_ref()
        .ok_or_else(|| format!("InWorld frame '{name}' has no layout"))?;
    let center = Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    match find_frame_at(registry, center.x, center.y) {
        Some(hit) if is_self_or_descendant(registry, hit, id) => Ok(center),
        Some(hit) => Err(format!(
            "InWorld frame '{name}' is covered by frame {:?} at its center",
            registry.get(hit).and_then(|f| f.name.clone())
        )),
        None => Err(format!("InWorld frame '{name}' is not mouse-enabled")),
    }
}

fn is_self_or_descendant(registry: &FrameRegistry, mut id: u64, ancestor: u64) -> bool {
    loop {
        if id == ancestor {
            return true;
        }
        match registry.get(id).and_then(|frame| frame.parent_id) {
            Some(parent) => id = parent,
            None => return false,
        }
    }
}

fn chord_has_shift(chord: &KeyChord) -> bool {
    chord
        .modifiers
        .iter()
        .any(|key| matches!(key, KeyCode::ShiftLeft | KeyCode::ShiftRight))
}

fn mouse_input(state: ButtonState, window: Entity) -> MouseButtonInput {
    MouseButtonInput {
        button: MouseButton::Left,
        state,
        window,
    }
}

fn keyboard_input(key: KeyCode, shift: bool, state: ButtonState, window: Entity) -> KeyboardInput {
    let (logical_key, text) = logical_key(key, shift);
    KeyboardInput {
        key_code: key,
        logical_key,
        state,
        text: text
            .filter(|_| state == ButtonState::Pressed)
            .map(|text| text.as_str().into()),
        repeat: false,
        window,
    }
}

/// Logical key and produced text for a US layout, matching what winit reports.
fn logical_key(key: KeyCode, shift: bool) -> (Key, Option<String>) {
    if let Some(label) = crate::input_bindings::key_alpha_numeric_label(key) {
        let text = if shift {
            label.to_string()
        } else {
            label.to_ascii_lowercase()
        };
        return (Key::Character(text.as_str().into()), Some(text));
    }
    let named = match key {
        KeyCode::Space => return (Key::Space, Some(" ".to_string())),
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Escape => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Shift,
        KeyCode::ControlLeft | KeyCode::ControlRight => Key::Control,
        KeyCode::AltLeft | KeyCode::AltRight => Key::Alt,
        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,
        _ => Key::Unidentified(NativeKey::Unidentified),
    };
    (named, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::menu_character_layout_test_support::compute_layout;
    use ui_toolkit::rsx;
    use ui_toolkit::screen::{Screen, SharedContext};
    use ui_toolkit::widget_def::Element;

    fn button_screen(_: &SharedContext) -> Element {
        rsx! {
            r#frame {
                name: "ClickTarget",
                width: 40.0,
                height: 20.0,
                onclick: "noop",
                pos_type: "absolute",
                left: 300.0,
                top: 150.0,
            }
        }
    }

    #[test]
    fn click_position_scales_ui_coordinates_to_the_window() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(button_screen).sync(&SharedContext::new(), &mut registry);
        compute_layout(&mut registry);
        registry.ui_scale = 2.0 / 3.0;

        let window = click_window_position(&registry, "ClickTarget").unwrap();

        assert!(window.distance(Vec2::new(320.0, 160.0) * (2.0 / 3.0)) < 0.01);
        // ui_cursor_position divides by ui_scale: the click lands on the frame center.
        assert!((window / registry.ui_scale).distance(Vec2::new(320.0, 160.0)) < 0.01);
    }
}
