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
    assert_eq!(button.hit_rect_insets, [6.0; 4]);
}

#[test]
fn name_input_uses_readable_retail_font_and_fits_allowed_names() {
    use bevy::prelude::*;
    use ui_toolkit::plugin::UiState;
    for scale in [1.0, 1.15] {
        for name in ["Donagh", "Wwmwmwmwmwmw"] {
            let mut app = layout_support::layout_app(1920.0, 1080.0);
            app.finish();
            app.cleanup();
            app.world_mut()
                .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
                .single_mut(app.world_mut())
                .unwrap()
                .resolution
                .set_scale_factor_override(Some(scale));
            let mut shared = SharedContext::new();
            shared.insert(CharCreateUiState {
                mode: CharCreateMode::Customize,
                name: name.into(),
                random_name_available: true,
                ..Default::default()
            });
            ui_toolkit::screen::Screen::new(char_create_screen).sync(
                &shared,
                &mut app.world_mut().resource_mut::<UiState>().registry,
            );
            for _ in 0..3 {
                app.update();
            }
            let id = app
                .world()
                .resource::<UiState>()
                .registry
                .get_by_name(CREATE_NAME_INPUT.0)
                .unwrap();
            let (_, text, font, layout) = app
                .world_mut()
                .query::<(
                    &ui_toolkit::native_render::RegistryText,
                    &Text,
                    &TextFont,
                    &bevy::text::TextLayoutInfo,
                )>()
                .iter(app.world())
                .find(|(text, _, _, _)| text.frame_id == id && text.key == 0)
                .expect("rendered name input");
            assert_eq!(text.0, name);
            assert_eq!(
                font.font_size,
                FontSize::Px(20.0),
                "Retail NumberFont_Shadow_Large"
            );
            assert!(!layout.glyphs.is_empty());
            assert!(layout.glyphs.iter().all(|glyph| glyph.line_index == 0));
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut max_x = 0.0_f32;
            for glyph in &layout.glyphs {
                let half = glyph.atlas_info.rect.size() / 2.0;
                min_y = min_y.min((glyph.position.y - half.y) / layout.scale_factor);
                max_y = max_y.max((glyph.position.y + half.y) / layout.scale_factor);
                max_x = max_x.max((glyph.position.x + half.x) / layout.scale_factor);
            }
            assert!(
                max_x <= 283.0,
                "12-letter name must fit the unchanged input width"
            );
            assert!(
                max_y - min_y <= 23.0,
                "name glyphs must fit the input height"
            );
        }
    }
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
