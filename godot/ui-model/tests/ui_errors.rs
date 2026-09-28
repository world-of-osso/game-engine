use game_engine_ui_model::UiErrorsModel;
use game_engine_ui_model::ui_errors_data::{ERROR_FADE_SECS, ERROR_HOLD_SECS, UiErrorsData};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::{PositionType, Val};
use ui_toolkit::widgets::font_string::{GameFont, JustifyH};

#[test]
fn errors_refresh_in_place_cap_newest_and_expire_after_fade() {
    let mut errors = UiErrorsData::default();
    for text in ["first", "second", "third", "fourth"] {
        errors.add(text);
    }
    assert_eq!(
        errors
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        ["fourth", "third", "second"]
    );
    errors.tick(2.0);
    errors.add("third");
    assert_eq!(errors.lines[1].age, 0.0);
    assert_eq!(errors.lines[0].age, 2.0);
    errors.tick(ERROR_HOLD_SECS - 2.0);
    assert_eq!(errors.lines[0].alpha(), 1.0);
    errors.tick(ERROR_FADE_SECS / 2.0);
    assert_eq!(errors.lines[0].alpha(), 0.5);
    errors.tick(ERROR_FADE_SECS / 2.0);
    assert_eq!(
        errors
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        ["third"]
    );
}

#[test]
fn authored_overlay_keeps_geometry_text_color_and_fade_in_registry() {
    let mut model = UiErrorsModel::new(1920.0, 1080.0);
    model.errors.add("Out of range.");
    model.errors.add("Invalid target");
    model.sync();
    let root = model
        .registry
        .get(model.registry.get_by_name("UIErrorsFrame").unwrap())
        .unwrap();
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(512.0), Dimension::Fixed(60.0))
    );
    assert_eq!(root.position_type, PositionType::Absolute);
    assert_eq!(root.position.left, Val::Percent(50.0));
    assert_eq!(root.position.top, Val::Px(122.0));
    assert_eq!(root.translation.x, Val::Percent(-50.0));
    for (index, text) in ["Invalid target", "Out of range."].iter().enumerate() {
        let frame = model
            .registry
            .get(
                model
                    .registry
                    .get_by_name(&format!("UIErrorsFrameLine{}", index + 1))
                    .unwrap(),
            )
            .unwrap();
        assert!(!frame.hidden);
        assert_eq!(frame.position.top, Val::Px(index as f32 * 20.0));
        let Some(WidgetData::FontString(font)) = &frame.widget_data else {
            panic!("not a fontstring")
        };
        assert_eq!(font.text, *text);
        assert_eq!(font.font, GameFont::FrizQuadrata);
        assert_eq!(font.font_size, 16.0);
        assert_eq!(font.justify_h, JustifyH::Center);
        assert_eq!(font.color, [1.0, 0.1, 0.1, 1.0]);
    }
    let third = model
        .registry
        .get(model.registry.get_by_name("UIErrorsFrameLine3").unwrap())
        .unwrap();
    assert!(third.hidden);
    model.tick(3.25);
    let first = model
        .registry
        .get(model.registry.get_by_name("UIErrorsFrameLine1").unwrap())
        .unwrap();
    let Some(WidgetData::FontString(font)) = &first.widget_data else {
        panic!("not a fontstring")
    };
    assert_eq!(font.color, [1.0, 0.1, 0.1, 0.5]);
    model.tick(0.25);
    assert!(
        model
            .registry
            .get(model.registry.get_by_name("UIErrorsFrameLine1").unwrap())
            .unwrap()
            .hidden
    );
}
