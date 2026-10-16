//! StackSplitFrame.xml arrow art and Lua endpoint enablement; no purchase/persistence claim.
use game_engine_ui_model::stack_split_frame_component::{
    StackSplitFrameState, stack_split_frame_screen,
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::{button::ButtonState, texture::TextureSource},
};

#[test]
fn stack_split_arrows_keep_source_states_geometry_and_actions_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for bundles in [false, true] {
            for enabled in [false, true] {
                let mut shared = SharedContext::new();
                shared.insert(StackSplitFrameState {
                    visible: true,
                    text: "4".into(),
                    total_text: bundles.then(|| "20 Total".into()),
                    left_enabled: enabled,
                    right_enabled: enabled,
                    ..Default::default()
                });
                let mut registry = FrameRegistry::new(1920., 1080.);
                Screen::new(stack_split_frame_screen).sync(&shared, &mut registry);
                for (key, action, up, down, disabled, left) in [
                    ("Left", "stack_split:left", 136487, 136486, 136485, 11.),
                    ("Right", "stack_split:right", 136490, 136489, 136488, 150.),
                ] {
                    let name = format!("StackSplitFrame{key}Button");
                    let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
                    assert_eq!((frame.width.value(), frame.height.value()), (16., 16.));
                    assert_eq!(
                        frame.position.left,
                        ui_toolkit::layout_values::Val::Px(left)
                    );
                    let top = if bundles { 34. } else { 22. };
                    assert_eq!(frame.position.top, ui_toolkit::layout_values::Val::Px(top));
                    assert_eq!(
                        frame.onclick.as_deref(),
                        if enabled { Some(action) } else { None }
                    );
                    let Some(WidgetData::Button(button)) = &frame.widget_data else {
                        panic!("{name} must project native pressed/disabled states");
                    };
                    assert_eq!(button.enabled, enabled);
                    assert_eq!(
                        button.state,
                        if enabled {
                            ButtonState::Normal
                        } else {
                            ButtonState::Disabled
                        }
                    );
                    assert_eq!(button.normal_texture, Some(TextureSource::FileDataId(up)));
                    assert_eq!(button.pushed_texture, Some(TextureSource::FileDataId(down)));
                    assert_eq!(
                        button.disabled_texture,
                        Some(TextureSource::FileDataId(disabled))
                    );
                    assert_eq!(button.highlight_texture, None, "XML has no hover overlay");
                }
            }
        }
    }
}
