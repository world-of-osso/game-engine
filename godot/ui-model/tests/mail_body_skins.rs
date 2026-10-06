//! Mail body layout in both skins. Its own test binary: it switches the
//! process-wide active skin, which would race other tests sharing the process.

use game_engine_ui_model::mail_frame_component::{BODY_BOX, MailFrameTab};

fn configure_assets() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
}

#[test]
fn uifixes_mail_body_is_multiline_and_spans_the_stationery_in_both_skins() {
    use game_engine_ui_model::mail_frame_component::{MailFrameState, mail_frame_screen};
    use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
    use ui_toolkit::frame::{Dimension, WidgetData};
    use ui_toolkit::layout_values::Val;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};
    use ui_toolkit::widgets::texture::TextureSource;
    configure_assets();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(MailFrameState {
            visible: true,
            tab: MailFrameTab::Send,
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(mail_frame_screen).sync(&shared, &mut registry);
        game_engine_ui_model::mail_frame_component::apply_mail_body_postsetup(&mut registry);
        let body = registry
            .get(registry.get_by_name(BODY_BOX).unwrap())
            .unwrap();
        let Some(WidgetData::EditBox(edit)) = &body.widget_data else {
            panic!("mail body missing")
        };
        assert!(edit.multi_line, "{skin:?}: mail body must retain newlines");
        assert_eq!(body.width, Dimension::Fixed(270.0));
        assert_eq!(body.height, Dimension::Fixed(134.0));
        assert_eq!(body.position.top, Val::Px(93.0));
        let paper = registry
            .get(
                registry
                    .get_by_name("SendStationeryBackgroundLeft")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&paper.widget_data, Some(WidgetData::Texture(t)) if t.source == TextureSource::FileDataId(136859))
        );
        assert_eq!(paper.height, Dimension::Fixed(154.0));
    }
    set_active_skin(ActiveSkin::Modern);
}
