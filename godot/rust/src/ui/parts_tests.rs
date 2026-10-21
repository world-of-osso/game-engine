use super::*;
use ui_toolkit::frame::{Dimension, Frame, WidgetType};
use ui_toolkit::widgets::texture::TextureData;

fn button(data: ButtonData) -> Frame {
    let mut frame = Frame::new(1, Some("Button".into()), WidgetType::Button);
    frame.widget_data = Some(WidgetData::Button(data));
    frame
}

fn atlas(name: &str) -> Option<TextureSource> {
    Some(TextureSource::Atlas(name.into()))
}

fn button_atlas(name: &str) -> Option<ButtonTexture> {
    Some(ButtonTexture::new(TextureSource::Atlas(name.into())))
}

#[test]
fn authored_state_regions_project_crop_tint_alpha_and_checked_independently() {
    use ui_toolkit::{attrs::apply_attribute, registry::FrameRegistry};
    for state in [
        ButtonState::Normal,
        ButtonState::Pushed,
        ButtonState::Disabled,
    ] {
        let mut registry = FrameRegistry::new(800.0, 600.0);
        let id = registry.create_frame("AuthoredControl", None);
        registry.get_mut(id).unwrap().widget_data = Some(WidgetData::Button(Default::default()));
        for (attr, value) in [
            ("button_default_skin", "false"),
            ("button_fdid_up", "132152"),
            ("button_fdid_pressed", "132152"),
            ("button_fdid_disabled", "132152"),
            ("button_normal_tex_coords", "0.125,0.875,0.25,0.75"),
            ("button_pushed_tex_coords", "0.125,0.875,0.25,0.75"),
            ("button_disabled_tex_coords", "0.125,0.875,0.25,0.75"),
            ("button_normal_vertex_color", "0.2,0.4,0.8,0.5"),
            ("button_pushed_vertex_color", "0.2,0.4,0.8,0.5"),
            ("button_disabled_vertex_color", "0.2,0.4,0.8,0.5"),
            ("button_normal_alpha", "0.6"),
            ("button_pushed_alpha", "0.6"),
            ("button_disabled_alpha", "0.6"),
            ("button_checked", "true"),
            ("button_fdid_checked", "130724"),
            ("button_checked_blend_mode", "ADD"),
            ("button_fdid_disabled_checked", "130718"),
            ("button_disabled_checked_alpha", "0.25"),
        ] {
            apply_attribute(&mut registry, id, attr, value);
        }
        let frame = registry.get_mut(id).unwrap();
        let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
            unreachable!()
        };
        button.state = state;
        let parts = project_images(frame, 36.0, 34.0);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].crop, Crop::Normalized([0.125, 0.875, 0.25, 0.75]));
        assert_eq!(parts[0].color, [0.2, 0.4, 0.8, 0.3]);
        if state == ButtonState::Disabled {
            assert_eq!(parts[1].source, Some(TextureSource::FileDataId(130718)));
            assert_eq!(parts[1].color[3], 0.25);
            assert!(!parts[1].additive);
        } else {
            assert_eq!(parts[1].source, Some(TextureSource::FileDataId(130724)));
            assert!(parts[1].additive);
        }
    }
}

#[test]
fn ui_template_blend_metadata_only_states_do_not_replace_existing_normal_art() {
    use ui_toolkit::{attrs::apply_attribute, registry::FrameRegistry};
    for (state, mode_attribute) in [
        (ButtonState::Normal, "button_highlight_blend_mode"),
        (ButtonState::Pushed, "button_pushed_blend_mode"),
        (ButtonState::Disabled, "button_disabled_blend_mode"),
    ] {
        let mut registry = FrameRegistry::new(800.0, 600.0);
        let id = registry.create_frame("PanelControl", None);
        registry.get_mut(id).unwrap().widget_data = Some(WidgetData::Button(Default::default()));
        apply_attribute(&mut registry, id, "button_fdid_up", "132152");
        apply_attribute(&mut registry, id, mode_attribute, "ADD");
        let frame = registry.get_mut(id).unwrap();
        let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
            unreachable!();
        };
        button.state = state;
        button.hovered = true;
        let parts = project_images(frame, 48.0, 48.0);
        assert_eq!(parts.len(), 1);
        assert_eq!(
            parts[0].source,
            Some(TextureSource::FileDataId(132152)),
            "{state:?}"
        );
        assert!(
            !parts[0].additive,
            "source-less state mode must not change normal art"
        );
    }
}

#[test]
fn ui_template_blend_metadata_only_highlight_does_not_draw_an_absent_source() {
    use ui_toolkit::{attrs::apply_attribute, registry::FrameRegistry};
    let mut registry = FrameRegistry::new(800.0, 600.0);
    let id = registry.create_frame("UntexturedControl", None);
    registry.get_mut(id).unwrap().widget_data = Some(WidgetData::Button(Default::default()));
    apply_attribute(&mut registry, id, "button_default_skin", "false");
    apply_attribute(&mut registry, id, "button_highlight_blend_mode", "ADD");
    let frame = registry.get_mut(id).unwrap();
    let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
        unreachable!();
    };
    button.hovered = true;
    assert!(project_images(frame, 48.0, 48.0).is_empty());
}

#[test]
fn ui_template_art_named_shared_red_crops_caps_when_the_button_is_narrow() {
    let mut registry = ui_toolkit::registry::FrameRegistry::new(800.0, 600.0);
    let id = registry.create_frame("NarrowSharedButton", None);
    registry.get_mut(id).unwrap().widget_data = Some(WidgetData::Button(Default::default()));
    ui_toolkit::attrs::apply_attribute(
        &mut registry,
        id,
        "button_template",
        "SharedButtonTemplate",
    );
    let parts = project_images(registry.get(id).unwrap(), 40.0, 30.0);
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].rect, [0.0, 0.0, 20.0, 30.0]);
    assert_eq!(parts[1].rect, [20.0, 0.0, 0.0, 30.0]);
    assert_eq!(parts[2].rect, [20.0, 0.0, 20.0, 30.0]);
    let Crop::Normalized(left) = parts[0].crop else {
        panic!("cropped left cap")
    };
    let Crop::Normalized(right) = parts[2].crop else {
        panic!("cropped right cap")
    };
    assert!((left[1] - 20.0 / (114.0 * 30.0 / 128.0)).abs() < 0.0001);
    assert!((right[0] - (1.0 - 20.0 / (292.0 * 30.0 / 128.0))).abs() < 0.0001);
}

#[test]
fn ui_template_art_explicit_default_skin_policy_does_not_depend_on_artwork_names() {
    for source in [
        TextureSource::FileDataId(130869),
        TextureSource::Atlas("RedButton-Exit".into()),
        TextureSource::Atlas("128-redbutton-cart-add".into()),
    ] {
        let frame = button(ButtonData {
            normal_texture: Some(ButtonTexture::new(source.clone())),
            highlight_texture: Some(ButtonTexture::new(TextureSource::FileDataId(999))),
            use_default_skin: true,
            hovered: true,
            ..Default::default()
        });
        let parts = project_images(&frame, 24.0, 24.0);
        assert_eq!(parts.len(), 1, "explicit skin policy {source:?}");
        assert_eq!(parts[0].source, Some(TextureSource::FileDataId(999)));
    }
}

#[test]
fn ui_template_art_source_identity_does_not_choose_button_geometry() {
    for source in [
        TextureSource::Atlas("128-redbutton-left".into()),
        TextureSource::FileDataId(130828),
    ] {
        let frame = button(ButtonData {
            normal_texture: Some(ButtonTexture::new(source.clone())),
            use_default_skin: false,
            ..Default::default()
        });
        let parts = project_images(&frame, 180.0, 46.0);
        assert_eq!(
            parts.len(),
            1,
            "plain image {source:?} must not become slices"
        );
        assert_eq!(parts[0].rect, [0.0, 0.0, 180.0, 46.0]);
    }
}

#[test]
fn ui_template_blend_button_material_depends_on_authored_mode_not_source_or_geometry_router() {
    for source in [
        TextureSource::Atlas("transmog-itemcard-default".into()),
        TextureSource::Atlas("house-chest-open-drawer-button".into()),
        TextureSource::Atlas("perks-button-up".into()),
        TextureSource::Atlas("ui-hud-minimap-zoom-in".into()),
        TextureSource::Atlas("128-redbutton-left".into()),
        TextureSource::FileDataId(130828),
    ] {
        for blend_mode in [BlendMode::Blend, BlendMode::Additive] {
            for state in [
                ButtonState::Normal,
                ButtonState::Pushed,
                ButtonState::Disabled,
            ] {
                let art = Some(ButtonTexture {
                    source: source.clone(),
                    blend_mode,
                    ..Default::default()
                });
                let frame = button(ButtonData {
                    normal_texture: art.clone(),
                    pushed_texture: art.clone(),
                    disabled_texture: art.clone(),
                    highlight_texture: art,
                    hovered: true,
                    state,
                    use_default_skin: false,
                    ..Default::default()
                });
                let parts = project_images(&frame, 164.0, 48.0);
                assert!(!parts.is_empty(), "{source:?} {state:?}");
                assert!(
                    parts
                        .iter()
                        .all(|part| part.additive == (blend_mode == BlendMode::Additive)),
                    "{source:?} {state:?} {blend_mode:?}: {parts:?}"
                );
            }
        }
    }
}

#[test]
fn ui_template_blend_texture_names_do_not_select_materials() {
    for name in [
        "PerksProgramSelected4",
        "PerksProgramFrozenSelected",
        "UnrelatedRegion",
    ] {
        for blend_mode in [BlendMode::Blend, BlendMode::Additive] {
            let mut frame = Frame::new(1, Some(name.into()), WidgetType::Texture);
            frame.widget_data = Some(WidgetData::Texture(TextureData {
                source: TextureSource::Atlas("perks-list-selected".into()),
                blend_mode,
                ..Default::default()
            }));
            let parts = project_images(&frame, 40.0, 40.0);
            assert_eq!(parts.len(), 1);
            assert_eq!(
                parts[0].additive,
                blend_mode == BlendMode::Additive,
                "{name}"
            );
        }
    }
}

#[test]
fn guild_tabard_arrow_hover_keeps_source_base_under_additive_highlight() {
    use game_engine_ui_model::{guild_tabard_frame, guild_tabards::GuildTabards};
    use shared::protocol::{GuildTabardReply, GuildTabardVendorSnapshot};
    use ui_toolkit::{
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    ui_toolkit::atlas::set_thread_skin(ActiveSkin::Modern);
    let mut view = GuildTabards::default();
    view.begin_open(701);
    view.receive(GuildTabardReply::Vendor(GuildTabardVendorSnapshot {
        npc: 701,
        npc_name: "Jondor Steelbrow".into(),
        own_rank: Some(0),
        design: None,
        cost: 100000,
    }))
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(guild_tabard_frame::tabard_screen).sync(&shared, &mut registry);
    guild_tabard_frame::apply_postsetup(&mut registry);
    for (name, normal, pushed, highlight) in [
        (
            "TabardFrameCustomization1LeftButton",
            130869,
            130868,
            130757,
        ),
        (
            "TabardFrameCustomization1RightButton",
            130866,
            130865,
            130757,
        ),
        (
            "TabardCharacterModelRotateLeftButton",
            130845,
            130844,
            130717,
        ),
        (
            "TabardCharacterModelRotateRightButton",
            130847,
            130846,
            130717,
        ),
    ] {
        let id = registry.get_by_name(name).unwrap();
        let frame = registry.get_mut(id).unwrap();
        for (state, expected) in [(ButtonState::Normal, normal), (ButtonState::Pushed, pushed)] {
            let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
                panic!("source button")
            };
            button.state = state;
            button.hovered = true;
            let parts = project_images(frame, 32.0, 32.0);
            assert_eq!(
                parts[0].source,
                Some(TextureSource::FileDataId(expected)),
                "{name}"
            );
            assert!(!parts[0].additive);
            assert_eq!(parts.len(), 2, "{name} base plus highlight");
            assert_eq!(parts[1].source, Some(TextureSource::FileDataId(highlight)));
            assert!(parts[1].additive, "Retail HighlightTexture blendMode=ADD");
        }
    }
}

#[test]
fn cooldown_revert_projects_retail_panel_strips_for_each_button_state() {
    use crate::ui::{RegistryModel, ScreenPostsetup};
    use game_engine_ui_model::cooldown_assist::CooldownViewerView;
    use game_engine_ui_model::cooldown_viewer_component::cooldown_viewer_screen;
    use ui_toolkit::{
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
    let mut view = CooldownViewerView::default();
    view.settings.open = true;
    view.settings.can_undo = true;
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut model = RegistryModel {
        screen: Screen::new(cooldown_viewer_screen),
        shared,
        registry: FrameRegistry::new(1920.0, 1080.0),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    model.sync();
    let id = model.registry.get_by_name("CooldownSettingsUndo").unwrap();
    for (state, fdid) in [
        (ButtonState::Normal, 130828),
        (ButtonState::Pushed, 130825),
        (ButtonState::Disabled, 130824),
    ] {
        let frame = model.registry.get_mut(id).unwrap();
        let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
            panic!("Revert button");
        };
        button.state = state;
        let images = project_images(frame, 164.0, 22.0);
        assert_eq!(images.len(), 3);
        assert!(
            images
                .iter()
                .all(|image| image.source == Some(TextureSource::FileDataId(fdid)))
        );
        assert_eq!(images[0].rect, [0.0, 0.0, 12.0, 22.0]);
        assert_eq!(images[2].rect, [152.0, 0.0, 12.0, 22.0]);
    }
}

#[test]
fn retail_panel_button_preserves_12px_caps_and_crops_unused_sheet_area() {
    for (state, fdid) in [
        (ButtonState::Normal, 130828),
        (ButtonState::Pushed, 130825),
        (ButtonState::Disabled, 130824),
    ] {
        let frame = button(ButtonData {
            template: Some(ui_toolkit::button_art::ButtonTemplate::Panel),
            state,
            normal_texture: Some(ButtonTexture::new(TextureSource::FileDataId(130828))),
            pushed_texture: Some(ButtonTexture::new(TextureSource::FileDataId(130825))),
            disabled_texture: Some(ButtonTexture::new(TextureSource::FileDataId(130824))),
            ..Default::default()
        });
        let images = project_images(&frame, 164.0, 22.0);
        assert_eq!(images.len(), 3);
        for (image, rect, crop) in images
            .iter()
            .zip([
                ([0.0, 0.0, 12.0, 22.0], [0.0, 0.09375, 0.0, 0.6875]),
                ([12.0, 0.0, 140.0, 22.0], [0.09375, 0.53125, 0.0, 0.6875]),
                ([152.0, 0.0, 12.0, 22.0], [0.53125, 0.625, 0.0, 0.6875]),
            ])
            .map(|(image, (rect, crop))| (image, rect, crop))
        {
            assert_eq!(image.source, Some(TextureSource::FileDataId(fdid)));
            assert_eq!(image.rect, rect);
            assert_eq!(image.crop, Crop::Normalized(crop));
        }
    }
}

#[test]
fn shared_panel_art_keeps_xml_12px_caps_when_callers_change_height() {
    for height in [21.0, 24.0, 30.0] {
        let frame = button(ButtonData {
            template: Some(ui_toolkit::button_art::ButtonTemplate::Panel),
            normal_texture: Some(ButtonTexture::new(TextureSource::FileDataId(130828))),
            ..Default::default()
        });
        let images = project_images(&frame, 245.0, height);
        assert_eq!(images[0].rect, [0.0, 0.0, 12.0, height]);
        assert_eq!(images[1].rect, [12.0, 0.0, 221.0, height]);
        assert_eq!(images[2].rect, [233.0, 0.0, 12.0, height]);
    }
}

#[test]
fn portrait_party_desaturated_health_projects_sampled_image_desaturation() {
    let mut frame = Frame::new(
        1,
        Some("PartyMemberFrame3HealthBarFill".into()),
        WidgetType::Texture,
    );
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::Atlas("UI-HUD-UnitFrame-Party-PortraitOn-Bar-Health".into()),
        desaturated: true,
        ..Default::default()
    }));
    let images = project_images(&frame, 70.0, 10.0);
    assert_eq!(images.len(), 1);
    assert!(
        images[0].desaturated,
        "native shader must desaturate sampled green pixels, not white vertex tint"
    );
}

#[test]
fn default_skin_button_stretches_the_whole_state_region() {
    // Login button: 250x66 frame; the original 24px/500px margins scale to exactly 12px,
    // i.e. the region is stretched as one image with no base quad beneath it.
    let parts = project_images(&button(ButtonData::default()), 250.0, 66.0);
    assert_eq!(
        parts,
        vec![ImagePart {
            rect: [0.0, 0.0, 250.0, 66.0],
            source: atlas("defaultbutton-nineslice-up"),
            crop: Crop::Full,
            color: [1.0; 4],
            overlay: false,
            rotation: 0.0,
            additive: false,
            desaturated: false,
            tiling: [false; 2],
        }]
    );
}

#[test]
fn default_skin_follows_hover_pushed_and_disabled_state() {
    for (state, hovered, expected) in [
        (
            ButtonState::Normal,
            true,
            "defaultbutton-nineslice-highlight",
        ),
        (
            ButtonState::Pushed,
            false,
            "defaultbutton-nineslice-pressed",
        ),
        (
            ButtonState::Disabled,
            true,
            "defaultbutton-nineslice-disabled",
        ),
    ] {
        let data = ButtonData {
            state,
            hovered,
            ..Default::default()
        };
        let parts = project_images(&button(data), 200.0, 32.0);
        assert_eq!(
            parts[0].source,
            atlas(expected),
            "{state:?} hovered={hovered}"
        );
    }
}

#[test]
fn retail_shared_red_button_preserves_authored_caps_in_each_state() {
    for (state, suffix) in [
        (ButtonState::Normal, ""),
        (ButtonState::Pushed, "-pressed"),
        (ButtonState::Disabled, "-disabled"),
    ] {
        let frame = button(ButtonData {
            template: Some(ui_toolkit::button_art::ButtonTemplate::SharedRed),
            normal_texture: button_atlas("128-redbutton-left"),
            state,
            use_default_skin: false,
            ..Default::default()
        });
        let parts = project_images(&frame, 200.0, 28.0);
        assert_eq!(parts.len(), 3);
        assert_eq!(
            parts[0].source,
            atlas(&format!("128-redbutton-left{suffix}"))
        );
        assert_eq!(
            parts[1].source,
            atlas(&format!("_128-redbutton-center{suffix}"))
        );
        assert_eq!(
            parts[2].source,
            atlas(&format!("128-redbutton-right{suffix}"))
        );
        assert_eq!(parts[0].rect[2], 114.0 * 28.0 / 128.0);
        assert_eq!(parts[2].rect[2], 292.0 * 28.0 / 128.0);
        assert_eq!(parts[2].rect[0] + parts[2].rect[2], 200.0);
    }
}

#[test]
fn trading_post_rotate_button_projects_additive_hover_and_offset_pressed_art() {
    for state in [ButtonState::Normal, ButtonState::Pushed] {
        let frame = button(ButtonData {
            normal_texture: button_atlas("perks-button-up"),
            pushed_texture: Some(ButtonTexture {
                source: TextureSource::Atlas("perks-button-down".into()),
                offset: [1.0, 1.0],
                ..Default::default()
            }),
            highlight_texture: Some(ButtonTexture {
                source: TextureSource::Atlas("perks-button-up".into()),
                blend_mode: BlendMode::Additive,
                alpha: 0.6,
                ..Default::default()
            }),
            hovered: true,
            state,
            use_default_skin: false,
            ..Default::default()
        });
        let parts = project_images(&frame, 48., 48.);
        assert_eq!(
            parts.len(),
            2,
            "Retail normal/pushed plus additive highlight"
        );
        let base = &parts[0];
        assert_eq!(
            base.rect,
            if state == ButtonState::Pushed {
                [1., 1., 48., 48.]
            } else {
                [0., 0., 48., 48.]
            }
        );
        let hover = &parts[1];
        assert_eq!(hover.rect, [0., 0., 48., 48.]);
        assert!(hover.additive);
        assert_eq!(hover.color, [1., 1., 1., 0.6]);
    }
}

#[test]
fn authored_button_textures_replace_the_default_skin() {
    let data = ButtonData {
        normal_texture: button_atlas("128-redbutton-up"),
        pushed_texture: button_atlas("128-redbutton-pressed"),
        state: ButtonState::Pushed,
        ..Default::default()
    };
    let parts = project_images(&button(data), 235.0, 64.0);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].source, atlas("128-redbutton-pressed"));
}

#[test]
fn unskinned_button_without_textures_draws_nothing() {
    let data = ButtonData {
        use_default_skin: false,
        ..Default::default()
    };
    assert!(project_images(&button(data), 79.0, 79.0).is_empty());
}

#[test]
fn transmog_hover_borders_add_light_without_covering_the_preview() {
    for (atlas, size) in [
        ("transmog-itemcard-default", [114.0, 146.0]),
        ("transmog-setcard-default", [194.0, 234.0]),
        ("transmog-setcard-incomplete", [194.0, 234.0]),
        ("transmog-gearslot-default", [59.0, 59.0]),
        ("transmog-gearslot-transmogrified", [59.0, 59.0]),
        ("transmog-gearslot-pending", [76.0, 76.0]),
        ("transmog-gearslot-default-small", [42.0, 42.0]),
        ("transmog-gearslot-transmogrified-small", [42.0, 42.0]),
    ] {
        let frame = button(ButtonData {
            use_default_skin: false,
            highlight_texture: Some(ButtonTexture {
                source: TextureSource::Atlas(atlas.into()),
                blend_mode: BlendMode::Additive,
                size: Some(size),
                ..Default::default()
            }),
            hovered: true,
            ..Default::default()
        });
        let parts = project_images(&frame, size[0] - 14.0, size[1] - 14.0);
        let hover = parts.last().expect("hover border");
        assert!(hover.additive, "{atlas} hover must add light to the model");
        assert_eq!(hover.rect, [-7.0, -7.0, size[0], size[1]]);
    }
}

#[test]
fn housing_storage_hover_overlays_add_half_alpha_with_source_handle_alignment() {
    for (atlas, size, left) in [
        ("house-chest-open-drawer-button", [64.0, 72.0], -4.0),
        ("house-chest-close-drawer-button", [32.0, 74.0], 0.0),
    ] {
        let mut frame = button(ButtonData {
            use_default_skin: false,
            highlight_texture: Some(ButtonTexture {
                source: TextureSource::Atlas(atlas.into()),
                blend_mode: BlendMode::Additive,
                alpha: 0.5,
                offset: [left, 0.0],
                ..Default::default()
            }),
            hovered: true,
            ..Default::default()
        });
        frame.width = Dimension::Fixed(size[0]);
        frame.height = Dimension::Fixed(size[1]);
        let parts = project_images(&frame, size[0], size[1]);
        let hover = parts.last().expect("source drawer hover image");
        assert!(hover.overlay);
        assert!(hover.additive);
        assert_eq!(hover.color[3], 0.5);
        assert_eq!(hover.rect, [left, 0.0, size[0], size[1]]);
        if let Some(WidgetData::Button(button)) = &mut frame.widget_data {
            button.hovered = false;
        }
        assert!(project_images(&frame, size[0], size[1]).is_empty());
    }
}

#[test]
fn unskinned_button_with_authored_slice_overlays_its_highlight() {
    let glow = TextureSource::File("data/ui/glow.ktx2".into());
    let mut frame = button(ButtonData {
        use_default_skin: false,
        highlight_texture: Some(ButtonTexture {
            source: glow.clone(),
            alpha: 0.7,
            size: Some([100.0, 100.0]),
            ..Default::default()
        }),
        hovered: true,
        ..Default::default()
    });
    frame.three_slice = Some(ThreeSlice::default());
    let parts = project_images(&frame, 80.0, 80.0);
    let highlight = parts.last().unwrap();
    assert_eq!(
        (
            highlight.rect,
            &highlight.source,
            highlight.color[3],
            highlight.overlay
        ),
        ([-10.0, -10.0, 100.0, 100.0], &Some(glow), 0.7, true)
    );
}

#[test]
fn authored_nine_slice_keeps_frame_background_behind_the_corners() {
    let mut frame = Frame::new(1, None, WidgetType::Frame);
    frame.background_color = Some([0.1, 0.2, 0.3, 1.0]);
    frame.nine_slice = Some(NineSlice {
        edge_size: 8.0,
        ..Default::default()
    });
    let parts = project_images(&frame, 100.0, 40.0);
    assert_eq!(parts.len(), 5 + 9);
    assert_eq!(parts[2].rect, [8.0, 8.0, 84.0, 24.0]);
    assert_eq!(parts[2].color, [0.1, 0.2, 0.3, 1.0]);
    assert!(parts[..5].iter().all(|part| part.rect[..2] != [0.0, 0.0]));
}

#[test]
fn texture_coordinates_crop_and_mirror_the_source_region() {
    let mut frame = Frame::new(1, None, WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::File("data/ui/icons.blp".into()),
        tex_coords: [0.5, 0.25, 0.0, 0.5],
        ..Default::default()
    }));
    let part = project_images(&frame, 32.0, 32.0).remove(0);
    let (rect, flip_x, flip_y) = crop_rect(&part.crop, [0.0, 0.0, 256.0, 128.0]);
    assert_eq!(
        (rect, flip_x, flip_y),
        ([64.0, 0.0, 64.0, 64.0], true, false)
    );
}

#[test]
fn slice_edges_select_part_within_atlas_region() {
    let crop = Crop::SliceEdges {
        index: 5,
        edges: [24.0, 20.0, 16.0, 12.0],
    };
    assert_eq!(
        crop_rect(&crop, [10.0, 100.0, 500.0, 132.0]).0,
        [494.0, 120.0, 16.0, 100.0]
    );
}

#[test]
fn button_text_color_follows_state() {
    for (state, color) in [
        (ButtonState::Normal, [1.0, 0.82, 0.0, 1.0]),
        (ButtonState::Pushed, [0.8, 0.65, 0.0, 1.0]),
        (ButtonState::Disabled, [0.5, 0.5, 0.5, 1.0]),
    ] {
        let text = project_button_text(&button(ButtonData {
            state,
            text: "Login".into(),
            font_size: 16.0,
            ..Default::default()
        }))
        .unwrap();
        assert_eq!((text.content.as_str(), text.color), ("Login", color));
    }
}

#[test]
fn css_border_draws_four_inset_edges_above_the_fill() {
    let mut frame = Frame::new(1, Some("TabBox".into()), WidgetType::Frame);
    frame.background_color = Some([0.02, 0.015, 0.01, 0.82]);
    frame.border = Some(ui_toolkit::frame::Border {
        width: 1.0,
        color: [1.0, 0.82, 0.0, 0.9],
    });
    let solid = |rect, color| ImagePart {
        rect,
        source: None,
        crop: Crop::Full,
        color,
        overlay: false,
        rotation: 0.0,
        additive: false,
        desaturated: false,
        tiling: [false; 2],
    };
    let gold = [1.0, 0.82, 0.0, 0.9];
    assert_eq!(
        project_images(&frame, 80.0, 34.0),
        vec![
            solid([0.0, 0.0, 80.0, 34.0], [0.02, 0.015, 0.01, 0.82]),
            solid([0.0, 0.0, 80.0, 1.0], gold),
            solid([79.0, 0.0, 1.0, 34.0], gold),
            solid([0.0, 33.0, 80.0, 1.0], gold),
            solid([0.0, 0.0, 1.0, 34.0], gold),
        ]
    );
}

#[test]
fn talent_missing_icon_projects_no_fake_white_art() {
    let mut frame = Frame::new(1, Some("TalentIcon".into()), WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData::default()));
    assert!(project_images(&frame, 36.0, 36.0).is_empty());
}

#[test]
fn talent_solid_edge_keeps_color_and_rotation() {
    let color = [0.35, 0.35, 0.35, 1.0];
    let mut frame = Frame::new(1, Some("TalentEdge127166".into()), WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::SolidColor(color),
        rotation: -std::f32::consts::FRAC_PI_4,
        ..Default::default()
    }));
    let images = project_images(&frame, 45.0, 2.0);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].color, color);
    assert_eq!(images[0].rotation, -std::f32::consts::FRAC_PI_4);
}

#[test]
fn texture_rotation_reaches_its_image_part() {
    let mut frame = Frame::new(1, Some("Arrow".into()), WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::FileDataId(803_894),
        rotation: 1.25,
        ..Default::default()
    }));
    let parts = project_images(&frame, 32.0, 32.0);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].rotation, 1.25);
}

#[test]
fn additive_texture_blend_reaches_its_image_part() {
    let mut frame = Frame::new(1, Some("Highlight".into()), WidgetType::Texture);
    frame.widget_data = Some(WidgetData::Texture(TextureData {
        source: TextureSource::FileDataId(137_138),
        blend_mode: BlendMode::Additive,
        ..Default::default()
    }));
    assert!(project_images(&frame, 64.0, 64.0)[0].additive);
}

/// Forever's c60 metal border on a one-row `LootFrame`: the window is 92 tall and the
/// border frame 16 above and 8 below it (116), while its top edge is 95 and its bottom
/// edge 100 tall.
#[test]
fn forever_short_nine_slice_keeps_its_bottom_row_inside_the_frame() {
    let cuts = [0.0, 0.3, 0.7, 1.0];
    let uv_rects = std::array::from_fn(|part| {
        let (col, row) = (part % 3, part / 3);
        [cuts[col], cuts[col + 1], cuts[row], cuts[row + 1]]
    });
    let slice = NineSlice {
        edge_size: 95.0,
        edge_sizes: Some([95.0, 95.0, 95.0, 100.0]),
        texture: Some(TextureSource::File("data/ui/metal.ktx2".into())),
        uv_rects: Some(uv_rects),
        ..Default::default()
    };
    let height = 116.0;
    let mut parts = Vec::new();
    project_nine_slice(&slice, 232.0, height, true, &mut parts);
    let visible: Vec<_> = parts.iter().filter(|part| part.rect[3] > 0.0).collect();
    for part in &visible {
        assert!(
            part.rect[1] >= 0.0 && part.rect[1] + part.rect[3] <= height,
            "{part:?}"
        );
    }
    let [top_row, bottom_row] = [0, 6].map(|first| &parts[first..first + 3]);
    for (upper, lower) in top_row.iter().zip(bottom_row) {
        assert!(
            upper.rect[1] + upper.rect[3] <= lower.rect[1],
            "{upper:?} over {lower:?}"
        );
        assert_eq!(lower.rect[1] + lower.rect[3], height);
        // The kept part is the bottom of the source: the frame's bottom rule.
        let Crop::SliceRect([_, _, top, bottom]) = lower.crop else {
            panic!("{lower:?}")
        };
        assert_eq!(bottom, 1.0);
        assert!(
            (top - (1.0 - 0.3 * lower.rect[3] / 100.0)).abs() < 1e-6,
            "{top}"
        );
    }
}

#[test]
fn ui_template_blend_minimap_zoom_source_states_keep_full_opacity_and_disabled_grayscale() {
    use game_engine_ui_model::minimap::{MinimapClusterState, minimap_cluster_screen};
    use ui_toolkit::{
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        for zoom in [0, 2, 5] {
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(MinimapClusterState {
                zoom_buttons: true,
                zoom,
                ..Default::default()
            });
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);
            for (name, atlas_name, size, disabled) in [
                (
                    "MinimapZoomIn",
                    "ui-hud-minimap-zoom-in",
                    [17., 17.],
                    zoom == 5,
                ),
                (
                    "MinimapZoomOut",
                    "ui-hud-minimap-zoom-out",
                    [17., 9.],
                    zoom == 0,
                ),
            ] {
                let id = registry.get_by_name(name).unwrap();
                let frame = registry.get_mut(id).unwrap();
                assert_eq!(frame.alpha, 1.0, "source has no disabled alpha reduction");
                let Some(WidgetData::Button(data)) = &mut frame.widget_data else {
                    panic!("{name} must accept real button state");
                };
                assert_eq!(data.state == ButtonState::Disabled, disabled);
                for state in [ButtonState::Normal, ButtonState::Pushed] {
                    for hovered in [false, true] {
                        let Some(WidgetData::Button(data)) = &mut frame.widget_data else {
                            unreachable!()
                        };
                        data.state = if disabled {
                            ButtonState::Disabled
                        } else {
                            state
                        };
                        data.hovered = hovered;
                        let parts = project_images(frame, size[0], size[1]);
                        let suffix = if !disabled && state == ButtonState::Pushed {
                            "-down"
                        } else {
                            ""
                        };
                        assert_eq!(parts[0].source, atlas(&format!("{atlas_name}{suffix}")));
                        assert_eq!(parts[0].desaturated, disabled);
                        assert_eq!(parts[0].color, [1.; 4]);
                        let highlighted = hovered && !disabled;
                        assert_eq!(parts.len(), if highlighted { 2 } else { 1 });
                        if highlighted {
                            assert_eq!(parts[1].source, atlas(&format!("{atlas_name}-mouseover")));
                            // R Minimap.xml203/218 omit alphaMode; source highlights default BLEND.
                            assert!(!parts[1].additive);
                        }
                    }
                }
            }
        }
    }
}
