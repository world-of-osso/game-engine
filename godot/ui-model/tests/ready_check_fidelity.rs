//! ReadyCheck.xml response buttons and inherited UIPanel art; not full dialog parity.
use game_engine_ui_model::ready_check_frame_component::{ReadyCheckFrameState, ready_check_frame};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
    widgets::texture::TextureSource,
};

fn screen(shared: &SharedContext) -> Element {
    ready_check_frame(shared.get::<ReadyCheckFrameState>().unwrap())
}

#[test]
fn ready_check_response_buttons_keep_source_anchors_and_art_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(ReadyCheckFrameState {
            visible: true,
            initiator: "Raidleader".into(),
        });
        let mut registry = FrameRegistry::new(1920., 1080.);
        Screen::new(screen).sync(&shared, &mut registry);
        for (suffix, left, caption, action) in [
            ("YesButton", 53.5, "Ready", "ready_check_yes"),
            ("NoButton", 183.5, "Not Ready", "ready_check_no"),
        ] {
            let name = format!("ReadyCheckFrame{suffix}");
            let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
            assert_eq!((frame.width.value(), frame.height.value()), (119., 24.));
            assert_eq!(
                frame.position.left,
                ui_toolkit::layout_values::Val::Px(left)
            );
            assert_eq!(frame.position.top, ui_toolkit::layout_values::Val::Px(60.));
            assert_eq!(frame.onclick.as_deref(), Some(action));
            let Some(WidgetData::Button(button)) = &frame.widget_data else {
                panic!("{name} must be a button");
            };
            assert_eq!(button.text, caption);
            assert_eq!(
                button.normal_texture,
                Some(TextureSource::FileDataId(130828))
            );
            assert_eq!(
                button.pushed_texture,
                Some(TextureSource::FileDataId(130825))
            );
            assert_eq!(
                button.highlight_texture,
                Some(TextureSource::FileDataId(130826))
            );
            assert_eq!(
                button.disabled_texture,
                Some(TextureSource::FileDataId(130824))
            );
        }
    }
}
