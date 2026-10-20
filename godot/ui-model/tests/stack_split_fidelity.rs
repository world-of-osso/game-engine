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
fn stack_split_amount_labels_inherit_black_shadow_both_skins() {
    use ui_toolkit::{layout_values::Val, widgets::font_string::GameFont};

    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for bundles in [false, true] {
            let amount = if bundles { "4 Stack(s)" } else { "4" };
            let mut shared = SharedContext::new();
            shared.insert(StackSplitFrameState {
                visible: true,
                text: amount.into(),
                total_text: bundles.then(|| "20 Total".into()),
                left_enabled: true,
                right_enabled: true,
                ..Default::default()
            });
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(stack_split_frame_screen).sync(&shared, &mut registry);
            assert_eq!(
                registry.get_by_name("StackItemCountText").is_some(),
                bundles
            );
            let left = if bundles { 41.0 } else { 22.0 };
            let amount_label = ("StackSplitText", amount, 23.0, [1.0; 4]);
            let total_label = (
                "StackItemCountText",
                "20 Total",
                48.0,
                [1.0, 0.82, 0.0, 1.0],
            );
            for (name, expected_text, top, color) in
                [Some(amount_label), bundles.then_some(total_label)]
                    .into_iter()
                    .flatten()
            {
                let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
                assert_eq!((frame.width.value(), frame.height.value()), (100.0, 14.0));
                assert_eq!(frame.position.left, Val::Px(left));
                assert_eq!(frame.position.top, Val::Px(top));
                let Some(WidgetData::FontString(label)) = &frame.widget_data else {
                    panic!("{name} must project text");
                };
                assert_eq!(label.text, expected_text);
                assert_eq!(label.font, GameFont::FrizQuadrata);
                assert_eq!(label.font_size, 12.0);
                // Current colors are controls; this slice only qualifies inherited shadow.
                assert_eq!(label.color, color);
                assert_eq!(
                    label.shadow_color,
                    Some([0.0, 0.0, 0.0, 1.0]),
                    "{skin:?}/{bundles}/{name}"
                );
                assert_eq!(label.shadow_offset, [1.0, -1.0]);
            }
        }
    }
}

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
                        Some(if enabled { action } else { "" })
                    );
                    let Some(WidgetData::Button(button)) = &frame.widget_data else {
                        panic!("{name} must project native pressed/disabled states");
                    };
                    assert_eq!(
                        button.state,
                        if enabled {
                            ButtonState::Normal
                        } else {
                            ButtonState::Disabled
                        }
                    );
                    assert_eq!(
                        button
                            .normal_texture
                            .as_ref()
                            .map(|texture| texture.source.clone()),
                        Some(TextureSource::FileDataId(up))
                    );
                    assert_eq!(
                        button
                            .pushed_texture
                            .as_ref()
                            .map(|texture| texture.source.clone()),
                        Some(TextureSource::FileDataId(down))
                    );
                    assert_eq!(
                        button
                            .disabled_texture
                            .as_ref()
                            .map(|texture| texture.source.clone()),
                        Some(TextureSource::FileDataId(disabled))
                    );
                    assert_eq!(
                        button
                            .highlight_texture
                            .as_ref()
                            .map(|texture| texture.source.clone()),
                        None,
                        "XML has no hover overlay"
                    );
                }
            }
        }
    }
}
