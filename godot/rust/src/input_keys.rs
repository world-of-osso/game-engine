use game_engine_core::input_bindings_data::{BindingKey, BindingMouseButton};
use godot::global::{Key, MouseButton};
use godot::obj::EngineEnum;

pub(crate) fn binding_key(key: Key) -> Option<BindingKey> {
    use BindingKey as B;
    Some(match key {
        Key::A => B::KeyA,
        Key::B => B::KeyB,
        Key::C => B::KeyC,
        Key::D => B::KeyD,
        Key::E => B::KeyE,
        Key::F => B::KeyF,
        Key::G => B::KeyG,
        Key::H => B::KeyH,
        Key::I => B::KeyI,
        Key::J => B::KeyJ,
        Key::K => B::KeyK,
        Key::L => B::KeyL,
        Key::M => B::KeyM,
        Key::N => B::KeyN,
        Key::O => B::KeyO,
        Key::P => B::KeyP,
        Key::Q => B::KeyQ,
        Key::R => B::KeyR,
        Key::S => B::KeyS,
        Key::T => B::KeyT,
        Key::U => B::KeyU,
        Key::V => B::KeyV,
        Key::W => B::KeyW,
        Key::X => B::KeyX,
        Key::Y => B::KeyY,
        Key::Z => B::KeyZ,
        Key::KEY_0 => B::Digit0,
        Key::KEY_1 => B::Digit1,
        Key::KEY_2 => B::Digit2,
        Key::KEY_3 => B::Digit3,
        Key::KEY_4 => B::Digit4,
        Key::KEY_5 => B::Digit5,
        Key::KEY_6 => B::Digit6,
        Key::KEY_7 => B::Digit7,
        Key::KEY_8 => B::Digit8,
        Key::KEY_9 => B::Digit9,
        Key::F1 => B::F1,
        Key::F2 => B::F2,
        Key::F3 => B::F3,
        Key::F4 => B::F4,
        Key::F5 => B::F5,
        Key::F6 => B::F6,
        Key::F7 => B::F7,
        Key::F8 => B::F8,
        Key::F9 => B::F9,
        Key::F10 => B::F10,
        Key::F11 => B::F11,
        Key::F12 => B::F12,
        Key::SPACE => B::Space,
        Key::TAB => B::Tab,
        Key::ESCAPE => B::Escape,
        Key::MINUS => B::Minus,
        Key::EQUAL => B::Equal,
        Key::BRACKETLEFT => B::BracketLeft,
        Key::BRACKETRIGHT => B::BracketRight,
        Key::LEFT => B::ArrowLeft,
        Key::RIGHT => B::ArrowRight,
        Key::UP => B::ArrowUp,
        Key::DOWN => B::ArrowDown,
        Key::PAGEUP => B::PageUp,
        Key::PAGEDOWN => B::PageDown,
        Key::NUMLOCK => B::NumLock,
        Key::HOME => B::Home,
        Key::END => B::End,
        Key::INSERT => B::Insert,
        Key::DELETE => B::Delete,
        Key::BACKSPACE => B::Backspace,
        Key::ENTER => B::Enter,
        Key::KP_ADD => B::NumpadAdd,
        Key::KP_SUBTRACT => B::NumpadSubtract,
        _ => return None,
    })
}

pub(crate) fn binding_mouse_button(button: MouseButton) -> Option<BindingMouseButton> {
    use BindingMouseButton as B;
    match button {
        MouseButton::LEFT => Some(B::Left),
        MouseButton::RIGHT => Some(B::Right),
        MouseButton::MIDDLE => Some(B::Middle),
        MouseButton::XBUTTON1 => Some(B::Back),
        MouseButton::XBUTTON2 => Some(B::Forward),
        MouseButton::NONE
        | MouseButton::WHEEL_UP
        | MouseButton::WHEEL_DOWN
        | MouseButton::WHEEL_LEFT
        | MouseButton::WHEEL_RIGHT => None,
        _ => u16::try_from(button.ord()).ok().map(B::Other),
    }
}

#[cfg(test)]
mod tests {
    use super::{binding_key, binding_mouse_button};
    use game_engine_core::input_bindings_data::{BindingKey, BindingMouseButton};
    use godot::global::{Key, MouseButton};
    use godot::obj::EngineEnum;

    #[test]
    fn maps_all_bindable_letters_digits_and_function_keys() {
        let letters = [
            (Key::A, BindingKey::KeyA),
            (Key::B, BindingKey::KeyB),
            (Key::C, BindingKey::KeyC),
            (Key::D, BindingKey::KeyD),
            (Key::E, BindingKey::KeyE),
            (Key::F, BindingKey::KeyF),
            (Key::G, BindingKey::KeyG),
            (Key::H, BindingKey::KeyH),
            (Key::I, BindingKey::KeyI),
            (Key::J, BindingKey::KeyJ),
            (Key::K, BindingKey::KeyK),
            (Key::L, BindingKey::KeyL),
            (Key::M, BindingKey::KeyM),
            (Key::N, BindingKey::KeyN),
            (Key::O, BindingKey::KeyO),
            (Key::P, BindingKey::KeyP),
            (Key::Q, BindingKey::KeyQ),
            (Key::R, BindingKey::KeyR),
            (Key::S, BindingKey::KeyS),
            (Key::T, BindingKey::KeyT),
            (Key::U, BindingKey::KeyU),
            (Key::V, BindingKey::KeyV),
            (Key::W, BindingKey::KeyW),
            (Key::X, BindingKey::KeyX),
            (Key::Y, BindingKey::KeyY),
            (Key::Z, BindingKey::KeyZ),
        ];
        let digits = [
            (Key::KEY_0, BindingKey::Digit0),
            (Key::KEY_1, BindingKey::Digit1),
            (Key::KEY_2, BindingKey::Digit2),
            (Key::KEY_3, BindingKey::Digit3),
            (Key::KEY_4, BindingKey::Digit4),
            (Key::KEY_5, BindingKey::Digit5),
            (Key::KEY_6, BindingKey::Digit6),
            (Key::KEY_7, BindingKey::Digit7),
            (Key::KEY_8, BindingKey::Digit8),
            (Key::KEY_9, BindingKey::Digit9),
        ];
        let functions = [
            (Key::F1, BindingKey::F1),
            (Key::F2, BindingKey::F2),
            (Key::F3, BindingKey::F3),
            (Key::F4, BindingKey::F4),
            (Key::F5, BindingKey::F5),
            (Key::F6, BindingKey::F6),
            (Key::F7, BindingKey::F7),
            (Key::F8, BindingKey::F8),
            (Key::F9, BindingKey::F9),
            (Key::F10, BindingKey::F10),
            (Key::F11, BindingKey::F11),
            (Key::F12, BindingKey::F12),
        ];
        for (godot_key, expected) in letters.into_iter().chain(digits).chain(functions) {
            assert_eq!(binding_key(godot_key), Some(expected), "{godot_key:?}");
        }
    }

    #[test]
    fn maps_all_bindable_navigation_and_punctuation_keys() {
        let keys = [
            (Key::SPACE, BindingKey::Space),
            (Key::TAB, BindingKey::Tab),
            (Key::ESCAPE, BindingKey::Escape),
            (Key::MINUS, BindingKey::Minus),
            (Key::EQUAL, BindingKey::Equal),
            (Key::BRACKETLEFT, BindingKey::BracketLeft),
            (Key::BRACKETRIGHT, BindingKey::BracketRight),
            (Key::LEFT, BindingKey::ArrowLeft),
            (Key::RIGHT, BindingKey::ArrowRight),
            (Key::UP, BindingKey::ArrowUp),
            (Key::DOWN, BindingKey::ArrowDown),
            (Key::PAGEUP, BindingKey::PageUp),
            (Key::PAGEDOWN, BindingKey::PageDown),
            (Key::NUMLOCK, BindingKey::NumLock),
            (Key::HOME, BindingKey::Home),
            (Key::END, BindingKey::End),
            (Key::INSERT, BindingKey::Insert),
            (Key::DELETE, BindingKey::Delete),
            (Key::BACKSPACE, BindingKey::Backspace),
            (Key::ENTER, BindingKey::Enter),
            (Key::KP_ADD, BindingKey::NumpadAdd),
            (Key::KP_SUBTRACT, BindingKey::NumpadSubtract),
        ];
        for (godot_key, expected) in keys {
            assert_eq!(binding_key(godot_key), Some(expected), "{godot_key:?}");
        }
    }

    #[test]
    fn rejects_non_bindable_and_logical_only_keys() {
        for key in [
            Key::NONE,
            Key::UNKNOWN,
            Key::SHIFT,
            Key::CTRL,
            Key::ALT,
            Key::META,
            Key::F13,
            Key::KP_1,
            Key::KP_ENTER,
            Key::BACKTAB,
            Key::PLUS,
            Key::EXCLAM,
            Key::BRACELEFT,
        ] {
            assert_eq!(binding_key(key), None, "{key:?}");
        }
    }

    #[test]
    fn maps_mouse_buttons_without_treating_wheel_as_held_input() {
        let buttons = [
            (MouseButton::LEFT, Some(BindingMouseButton::Left)),
            (MouseButton::RIGHT, Some(BindingMouseButton::Right)),
            (MouseButton::MIDDLE, Some(BindingMouseButton::Middle)),
            (MouseButton::XBUTTON1, Some(BindingMouseButton::Back)),
            (MouseButton::XBUTTON2, Some(BindingMouseButton::Forward)),
            (MouseButton::NONE, None),
            (MouseButton::WHEEL_UP, None),
            (MouseButton::WHEEL_DOWN, None),
            (MouseButton::WHEEL_LEFT, None),
            (MouseButton::WHEEL_RIGHT, None),
            (
                MouseButton::try_from_ord(10).unwrap(),
                Some(BindingMouseButton::Other(10)),
            ),
            (
                MouseButton::try_from_ord(65535).unwrap(),
                Some(BindingMouseButton::Other(65535)),
            ),
            (MouseButton::try_from_ord(65536).unwrap(), None),
            (MouseButton::try_from_ord(-1).unwrap(), None),
        ];
        for (button, expected) in buttons {
            assert_eq!(binding_mouse_button(button), expected, "{button:?}");
        }
    }
}
