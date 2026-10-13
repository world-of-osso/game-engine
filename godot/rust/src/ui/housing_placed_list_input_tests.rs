use super::UiInput;
use super::projection::placed_list_context_input;
use godot::global::MouseButton;
use godot::prelude::Vector2;

#[test]
fn housing_placed_list_context_uses_only_actual_released_right_button() {
    let point = Vector2::new(723.0, 318.0);
    assert!(placed_list_context_input(42, MouseButton::RIGHT, true, false, point).is_none());
    assert!(placed_list_context_input(42, MouseButton::LEFT, false, false, point).is_none());
    for shift in [false, true] {
        let input = placed_list_context_input(42, MouseButton::RIGHT, false, shift, point);
        assert!(matches!(input, Some(UiInput::FrameAltClick {
            id: 42, right: true, shift: actual_shift, at,
        }) if actual_shift == shift && at == point));
    }
}
