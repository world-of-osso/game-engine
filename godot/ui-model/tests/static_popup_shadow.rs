//! Ordinary PARTY_INVITE body shadow; effective user font scale/full-popup parity excluded.
use game_engine_ui_model::{
    popup::{PopupSpec, PopupStack},
    static_popup_component::{StaticPopupState, static_popup_screen},
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::font_string::GameFont,
};

#[test]
fn party_invite_body_inherits_black_shadow_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut stack = PopupStack::default();
        stack.push(PopupSpec {
            key: "PARTY_INVITE".into(),
            text: "Adriel invites you to a group.".into(),
            accept_label: "Accept".into(),
            cancel_label: Some("Decline".into()),
            timeout: Some(std::time::Duration::from_secs(60)),
            confirm_text: None,
        });
        let mut shared = SharedContext::new();
        shared.insert(StaticPopupState {
            popups: stack.visible(),
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(static_popup_screen).sync(&shared, &mut registry);
        let frame = registry
            .get(registry.get_by_name("StaticPopup1Text").unwrap())
            .unwrap();
        assert_eq!((frame.width.value(), frame.height.value()), (290.0, 18.0));
        assert_eq!(frame.position.left, Val::Px(15.0));
        assert_eq!(frame.position.top, Val::Px(16.0));
        let Some(WidgetData::FontString(label)) = &frame.widget_data else {
            panic!("PARTY_INVITE body must project text");
        };
        assert_eq!(label.text, "Adriel invites you to a group.");
        assert_eq!(label.font, GameFont::FrizQuadrata);
        // Current typography/color are conservation controls, not effective-scale parity.
        assert_eq!(label.font_size, 14.0);
        assert_eq!(label.color, [1.0; 4]);
        for (name, action) in [
            ("StaticPopup1Button1", "static_popup_accept:1"),
            ("StaticPopup1Button2", "static_popup_cancel:1"),
        ] {
            let button = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert_eq!((button.width.value(), button.height.value()), (128.0, 26.0));
            assert_eq!(button.onclick.as_deref(), Some(action));
        }
        assert_eq!(label.shadow_color, Some([0.0, 0.0, 0.0, 1.0]), "{skin:?}");
        assert_eq!(label.shadow_offset, [1.0, -1.0]);
    }
}
