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
fn ready_check_portrait_keeps_source_slot_and_metal_ring_both_skins() {
    use ui_toolkit::atlas::{AtlasSource, resolve_region};
    use ui_toolkit::layout_values::Val;

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
            difficulty_name: None,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(screen).sync(&shared, &mut registry);
        let portrait = registry
            .get(
                registry
                    .get_by_name("ReadyCheckPortrait")
                    .expect("missing initiator portrait host"),
            )
            .unwrap();
        assert_eq!(
            (portrait.width.value(), portrait.height.value()),
            (54.0, 54.0)
        );
        assert_eq!(
            (portrait.position.left, portrait.position.top),
            (Val::Px(0.0), Val::Px(-3.0))
        );
        let ring = registry
            .get(
                registry
                    .get_by_name("ReadyCheckFramePortraitRing")
                    .expect("missing portrait metal ring"),
            )
            .unwrap();
        assert_eq!(
            (ring.position.left, ring.position.top),
            (Val::Px(-13.0), Val::Px(-16.0))
        );
        let region = resolve_region("UI-Frame-PortraitMetal-CornerTopLeft", skin).unwrap();
        assert_eq!(
            (ring.width.value(), ring.height.value()),
            (region.width / 2.0, region.height / 2.0)
        );
        let AtlasSource::FileDataId(fdid) = region.source else {
            panic!("source portrait ring must resolve to file art");
        };
        let Some(WidgetData::Texture(texture)) = &ring.widget_data else {
            panic!("portrait metal ring must draw source art");
        };
        assert_eq!(texture.source, TextureSource::FileDataId(fdid));
    }
}

#[test]
fn ready_check_title_keeps_source_bounds_color_and_shadow_both_skins() {
    use ui_toolkit::{layout_values::Val, widgets::font_string::GameFont};

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
            difficulty_name: None,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(screen).sync(&shared, &mut registry);
        let frame = registry
            .get(registry.get_by_name("ReadyCheckFrameTitle").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(title)) = &frame.widget_data else {
            panic!("ReadyCheck title must be text");
        };
        // ReadyCheck.xml:45–58: 58 left/24 right, container top1 + title top5.
        // Current 20px height is conserved, not qualified as intrinsic title height.
        assert_eq!((frame.width.value(), frame.height.value()), (241.0, 20.0));
        assert_eq!(
            (frame.position.left, frame.position.top),
            (Val::Px(58.0), Val::Px(6.0))
        );
        assert_eq!(title.text, "Ready Check");
        assert_eq!(title.font, GameFont::FrizQuadrata);
        assert_eq!(title.font_size, 12.0);
        // GameFontNormal colour: GlobalColor row1 ARGB0xFFFFD200.
        assert_eq!(title.color, [1.0, 210.0 / 255.0, 0.0, 1.0], "{skin:?}");
        // ReadyCheck.xml:53 → GameFontNormal → Roman SystemFont_Shadow_Med1.
        assert_eq!(title.shadow_color, Some([0.0, 0.0, 0.0, 1.0]), "{skin:?}");
        assert_eq!(title.shadow_offset, [1.0, -1.0], "{skin:?}");
    }
}

#[test]
fn ready_check_message_inherits_roman_color_and_shadow_both_skins() {
    use ui_toolkit::widgets::font_string::GameFont;

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
            difficulty_name: None,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(screen).sync(&shared, &mut registry);
        let frame = registry
            .get(registry.get_by_name("ReadyCheckFrameText").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(text)) = &frame.widget_data else {
            panic!("ReadyCheck message must be text");
        };
        // Body height follows host text metrics; native layout regression covers it.
        assert_eq!(
            text.justify_v,
            ui_toolkit::widgets::font_string::JustifyV::Middle
        );
        assert_eq!(text.font, GameFont::FrizQuadrata);
        assert_eq!(text.font_size, 12.0);
        // GlobalColor row1 NORMAL_FONT_COLOR = ARGB0xFFFFD200, not white.
        assert_eq!(text.color, [1.0, 210.0 / 255.0, 0.0, 1.0], "{skin:?}");
        assert_eq!(text.shadow_color, Some([0.0, 0.0, 0.0, 1.0]), "{skin:?}");
        assert_eq!(text.shadow_offset, [1.0, -1.0]);
    }
}

#[test]
fn ready_check_message_keeps_source_top_anchor_both_skins() {
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
            difficulty_name: None,
        });
        let mut registry = FrameRegistry::new(1920., 1080.);
        Screen::new(screen).sync(&shared, &mut registry);
        let frame = registry
            .get(registry.get_by_name("ReadyCheckFrameText").unwrap())
            .unwrap();
        assert_eq!(frame.width.value(), 240.);
        assert_eq!(
            frame.position.left,
            ui_toolkit::layout_values::Val::Px(61.5)
        );
        assert_eq!(frame.position.top, ui_toolkit::layout_values::Val::Px(37.));
        let Some(WidgetData::FontString(text)) = &frame.widget_data else {
            panic!("ReadyCheck message must be text");
        };
        assert_eq!(text.text, "Raidleader has initiated a ready check.");
    }
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
            difficulty_name: None,
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
