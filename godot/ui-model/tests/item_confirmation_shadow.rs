//! StaticPopup item-presenter caption shadow; existing typography/actions are controls.
use game_engine_ui_model::static_popup_component::{ItemConfirmation, item_confirmation};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
    widgets::button::ButtonState,
};

fn confirmation_screen(ctx: &SharedContext) -> Element {
    let &(pending, accept_enabled) = ctx.get::<(bool, bool)>().unwrap();
    item_confirmation(ItemConfirmation {
        name: "ShadowConfirm",
        pending,
        accept_enabled,
        prompt: ("Purchase this item?", ""),
        currency_icon: None,
        item: Some(("Sharp Shortsword", 132350, 2)),
        accept: ("Confirm", "shadow_confirm:accept"),
        cancel_action: "shadow_confirm:cancel",
        screen: (1920.0, 1080.0),
    })
}

#[test]
fn item_confirmation_captions_inherit_black_shadow_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for (pending, accept_enabled) in [(false, true), (false, false), (true, true)] {
            let mut shared = SharedContext::new();
            shared.insert((pending, accept_enabled));
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(confirmation_screen).sync(&shared, &mut registry);
            game_engine_ui_model::menu_template_art::sync_template_art(&mut registry);
            for (suffix, caption, action, disabled) in [
                (
                    "Button",
                    "Confirm",
                    "shadow_confirm:accept",
                    pending || !accept_enabled,
                ),
                ("CancelButton", "Cancel", "shadow_confirm:cancel", pending),
            ] {
                let button_name = format!("ShadowConfirm{suffix}");
                let button = registry
                    .get(registry.get_by_name(&button_name).unwrap())
                    .unwrap();
                assert_eq!((button.width.value(), button.height.value()), (128.0, 21.0));
                assert_eq!(button.onclick.as_deref(), Some(action));
                let Some(WidgetData::Button(data)) = &button.widget_data else {
                    panic!("{button_name} must retain button state");
                };
                assert_eq!(data.state == ButtonState::Disabled, disabled);
                let name = format!("{button_name}DialogButtonText");
                let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
                assert_eq!((frame.width.value(), frame.height.value()), (128.0, 21.0));
                assert_eq!(
                    (frame.position.left, frame.position.top),
                    (Val::Px(0.0), Val::Px(-1.0))
                );
                let Some(WidgetData::FontString(label)) = &frame.widget_data else {
                    panic!("{name} must retain caption text");
                };
                assert_eq!(label.text, caption);
                assert_eq!(label.font_size, 12.0);
                assert_eq!(
                    label.color,
                    if disabled {
                        [0.5, 0.5, 0.5, 1.0]
                    } else {
                        [1.0, 0.82, 0.0, 1.0]
                    }
                );
                assert_eq!(
                    label.shadow_color,
                    Some([0.0, 0.0, 0.0, 1.0]),
                    "{skin:?}/{pending}/{accept_enabled}/{suffix}"
                );
                assert_eq!(label.shadow_offset, [1.0, -1.0]);
            }
        }
    }
}
