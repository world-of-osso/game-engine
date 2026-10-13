use super::UiInput;
use super::projection::buff_cancel_input as placed_list_context_input;
use godot::global::MouseButton;

#[test]
fn housing_placed_list_context_uses_only_actual_released_right_button() {
    assert!(placed_list_context_input(42, MouseButton::RIGHT, true, false).is_none());
    assert!(placed_list_context_input(42, MouseButton::LEFT, false, false).is_none());
    assert!(matches!(
        placed_list_context_input(42, MouseButton::RIGHT, false, false),
        Some(UiInput::AltClick {
            id: 42,
            right: true,
            shift: false
        })
    ));
    assert!(matches!(
        placed_list_context_input(43, MouseButton::RIGHT, false, true),
        Some(UiInput::AltClick {
            id: 43,
            right: true,
            shift: true
        })
    ));
}
