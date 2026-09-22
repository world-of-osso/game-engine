use super::*;
use bevy::input::{ButtonState as InputState, mouse::MouseButtonInput};
use bevy::prelude::*;
use ui_toolkit::plugin::UiState;

fn closed_choice(
    label: &str,
    first: Option<[u8; 3]>,
    second: Option<[u8; 3]>,
) -> CharCreateUiState {
    let mut selected = option(22, "Eye Color");
    selected.choices[0].label = label.into();
    selected.choices[0].swatch = first;
    selected.choices[0].secondary_swatch = second;
    CharCreateUiState {
        options: vec![selected],
        ..customize_state()
    }
}

#[test]
fn closed_color_value_centers_its_actual_single_or_dual_swatch_bounds() {
    for (first, second, expected_width) in [
        (Some([128, 32, 16]), None, 42.0),
        (Some([128, 32, 16]), Some([20, 80, 160]), 54.0),
        (None, Some([20, 80, 160]), 42.0),
    ] {
        let harness = ScreenHarness::new(closed_choice("", first, second));
        let button = rect(&harness.reg, "OptionToggle_22");
        let value = rect(&harness.reg, "OptionValue_22");
        let swatch = rect(&harness.reg, "OptionValue_22_Swatch");
        assert_eq!(value.width, expected_width);
        assert!((value.x - (button.x + (button.width - expected_width) / 2.0)).abs() < 0.02);
        assert_eq!(swatch.x, value.x);
        assert!(swatch.x >= button.x && swatch.x + swatch.width <= button.x + button.width);
        if second.is_some() && first.is_some() {
            let other = rect(&harness.reg, "OptionValue_22_SecondarySwatch");
            assert_eq!(other.x - swatch.x, 18.0);
            assert_eq!(other.x + other.width, value.x + value.width);
        }
    }
}

#[test]
fn closed_named_and_numeric_values_center_the_visible_text_without_overflow() {
    for label in ["Amber", "", "A very long named customization selection"] {
        let harness = ScreenHarness::new(closed_choice(label, None, None));
        let button = rect(&harness.reg, "OptionToggle_22");
        let value = rect(&harness.reg, "OptionValue_22");
        let text = rect(&harness.reg, "OptionValue_22_Text");
        assert!(value.width > 0.0 && value.width <= 126.0);
        assert!(
            (value.x - (button.x + (button.width - value.width) / 2.0)).abs() <= 0.5,
            "{label:?} button={button:?}, value={value:?}"
        );
        assert_eq!(text.x, value.x);
        assert_eq!(text.width, value.width);
        assert!(value.x >= button.x && value.x + value.width <= button.x + button.width);
    }
}

#[test]
fn centered_closed_value_preserves_the_native_toggle_hit_target() {
    let state = closed_choice("", Some([128, 32, 16]), Some([20, 80, 160]));
    let mut app = layout_support::layout_app(1920.0, 1080.0);
    app.finish();
    app.cleanup();
    let mut shared = SharedContext::new();
    shared.insert(state);
    ui_toolkit::screen::Screen::new(char_create_screen).sync(
        &shared,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    for _ in 0..3 {
        app.update();
    }
    let button = {
        let ui = app.world().resource::<UiState>();
        let button = rect(&ui.registry, "OptionToggle_22");
        assert_eq!(
            action(&ui.registry, "OptionToggle_22"),
            CharCreateAction::ToggleOption(22)
        );
        let x = button.x + button.width / 2.0;
        let y = button.y + button.height / 2.0;
        let hit = ui_toolkit::input::find_frame_at(&ui.registry, x, y).unwrap();
        assert_eq!(hit, ui.registry.get_by_name("OptionToggle_22").unwrap());
        Vec2::new(x, y)
    };
    let window_entity = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(window_entity)
        .get_mut::<Window>()
        .unwrap()
        .set_cursor_position(Some(button));
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Left,
        state: InputState::Pressed,
        window: window_entity,
    });
    app.update();
    let ui = app.world().resource::<UiState>();
    assert!(matches!(
        &frame(&ui.registry, "OptionToggle_22").widget_data,
        Some(WidgetData::Button(data)) if data.state == ButtonState::Pushed
    ));
}
