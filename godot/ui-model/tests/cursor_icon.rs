use game_engine_ui_model::ui::screens::cursor_item_component::{
    CURSOR_ICON_NAME, CursorItemFrameState, cursor_item_screen,
};
use ui_toolkit::frame::{Dimension, WidgetData, WidgetType};
use ui_toolkit::layout_values::{PositionType, Val};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::strata::FrameStrata;
use ui_toolkit::widgets::texture::TextureSource;

#[test]
fn cursor_icon_tracks_pointer_and_texture_then_leaves_no_control() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(cursor_item_screen);
    shared.insert(CursorItemFrameState::default());
    screen.sync(&shared, &mut registry);
    assert!(registry.get_by_name(CURSOR_ICON_NAME).is_none());

    for (icon_fdid, position, top_left) in [
        (132_889, [400.0, 300.0], [384.0, 284.0]),
        (133_784, [8.0, 12.0], [-8.0, -4.0]),
    ] {
        shared.insert(CursorItemFrameState {
            icon_fdid: Some(icon_fdid),
            position,
        });
        screen.sync(&shared, &mut registry);
        let icon = registry
            .get(registry.get_by_name(CURSOR_ICON_NAME).expect("cursor icon"))
            .unwrap();
        assert_eq!(icon.widget_type, WidgetType::Texture);
        assert_eq!(icon.width, Dimension::Fixed(32.0));
        assert_eq!(icon.height, Dimension::Fixed(32.0));
        assert_eq!(icon.position_type, PositionType::Absolute);
        assert_eq!(icon.position.left, Val::Px(top_left[0]));
        assert_eq!(icon.position.top, Val::Px(top_left[1]));
        assert_eq!(icon.strata, FrameStrata::Tooltip);
        assert!(!icon.hidden);
        assert!(!icon.mouse_enabled);
        assert!(icon.onclick.is_none());
        match icon.widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => {
                assert_eq!(texture.source, TextureSource::FileDataId(icon_fdid));
            }
            other => panic!("cursor icon is not a texture: {other:?}"),
        }
    }

    shared.insert(CursorItemFrameState {
        icon_fdid: None,
        position: [600.0, 200.0],
    });
    screen.sync(&shared, &mut registry);
    assert!(registry.get_by_name(CURSOR_ICON_NAME).is_none());
}
