use super::*;
use crate::ui::registry::FrameRegistry;
#[path = "../menu_character_layout_test_support.rs"]
mod layout_support;

#[test]
fn name_button_is_left_of_editbox_and_dispatches_distinct_action() {
    let state = CharCreateUiState {
        mode: CharCreateMode::Customize,
        random_name_available: true,
        ..Default::default()
    };
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut screen = ui_toolkit::screen::Screen::new(char_create_screen);
    screen.sync(&shared, &mut registry);
    layout_support::compute_layout(&mut registry);
    let button = registry
        .get(registry.get_by_name(RANDOM_NAME_BUTTON.0).unwrap())
        .unwrap();
    let input = registry
        .get(registry.get_by_name(CREATE_NAME_INPUT.0).unwrap())
        .unwrap();
    assert_eq!(
        CharCreateAction::parse(button.onclick.as_deref().unwrap()),
        Some(CharCreateAction::RandomizeName)
    );
    assert!(
        button.layout_rect.as_ref().unwrap().x + button.layout_rect.as_ref().unwrap().width
            <= input.layout_rect.as_ref().unwrap().x
    );
    assert_eq!(button.layout_rect.as_ref().unwrap().height, 48.0);
}

#[test]
fn missing_catalog_disables_random_name_button() {
    let state = CharCreateUiState {
        mode: CharCreateMode::Customize,
        random_name_available: false,
        ..Default::default()
    };
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut screen = ui_toolkit::screen::Screen::new(char_create_screen);
    screen.sync(&shared, &mut registry);
    let button = registry
        .get(registry.get_by_name(RANDOM_NAME_BUTTON.0).unwrap())
        .unwrap();
    assert!(
        matches!(&button.widget_data, Some(crate::ui::frame::WidgetData::Button(data)) if data.state == crate::ui::widgets::button::ButtonState::Disabled)
    );
    assert!(button.onclick.as_deref().unwrap_or("").is_empty());
}
