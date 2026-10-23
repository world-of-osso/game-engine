use game_engine_ui_model::casting_bar_frame_component::CastingBarState;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, UnitFrameMenuState, UnitFrameState, inworld_unit_frames_screen,
};
use ui_toolkit::{
    atlas::ActiveSkin,
    frame::{Dimension, Frame, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::texture::{TextureData, TextureSource},
};

fn state(cast: Option<CastingBarState>) -> InWorldUnitFramesState {
    InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Player"),
        target: Some(UnitFrameState::named("Enemy")),
        target_cast: cast,
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: vec![],
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    }
}
fn cast() -> CastingBarState {
    CastingBarState {
        visible: true,
        spell_name: "Frostbolt".into(),
        icon_fdid: Some(135846),
        timer_text: "1.5".into(),
        progress: 0.25,
        ..Default::default()
    }
}
fn registry(skin: ActiveSkin, state: InWorldUnitFramesState) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(state);
    let mut r = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&ctx, &mut r);
    r
}
fn frame<'a>(r: &'a FrameRegistry, name: &str) -> &'a Frame {
    r.get(
        r.get_by_name(name)
            .unwrap_or_else(|| panic!("missing {name}")),
    )
    .unwrap()
}
fn rect(r: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(r, name);
    let (Val::Px(x), Val::Px(y), Dimension::Fixed(w), Dimension::Fixed(h)) =
        (f.position.left, f.position.top, f.width, f.height)
    else {
        panic!("not fixed {name}")
    };
    (x, y, w, h)
}
fn text(r: &FrameRegistry, name: &str) -> String {
    let Some(WidgetData::FontString(t)) = &frame(r, name).widget_data else {
        panic!("not text")
    };
    t.text.clone()
}
fn texture<'a>(registry: &'a FrameRegistry, name: &str) -> &'a TextureData {
    let Some(WidgetData::Texture(texture)) = &frame(registry, name).widget_data else {
        panic!("{name} must supply texture render input");
    };
    texture
}

#[test]
fn modern_target_cast_uses_source_regions_and_crops_each_status_fill() {
    for (channel, interruptible, interrupted, atlas, pip) in [
        (
            false,
            true,
            false,
            "ui-castingbar-filling-standard",
            "ui-castingbar-pip",
        ),
        (
            true,
            true,
            false,
            "ui-castingbar-filling-channel",
            "ui-castingbar-pip",
        ),
        (
            false,
            false,
            false,
            "ui-castingbar-uninterruptable",
            "ui-castingbar-pip",
        ),
        (
            false,
            true,
            true,
            "ui-castingbar-interrupted",
            "ui-castingbar-pip-red",
        ),
    ] {
        for progress in [0.0, 0.25, 1.0] {
            let mut sample = cast();
            sample.is_channel = channel;
            sample.is_interruptible = interruptible;
            sample.is_interrupted = interrupted;
            sample.progress = progress;
            let registry = registry(ActiveSkin::Modern, state(Some(sample)));
            for (name, source) in [
                ("TargetCastingBarBackgroundArt", "ui-castingbar-background"),
                ("TargetCastingBarTextBorder", "ui-castingbar-textbox"),
                ("TargetCastingBarFill", atlas),
                ("TargetCastingBarSpark", pip),
            ] {
                assert_eq!(
                    texture(&registry, name).source,
                    TextureSource::Atlas(source.into())
                );
            }
            assert_eq!(
                rect(&registry, "TargetCastingBarBackgroundArt"),
                (0.0, 0.0, 150.0, 10.0)
            );
            assert_eq!(
                rect(&registry, "TargetCastingBarTextBorder"),
                (0.0, 0.0, 150.0, 22.0)
            );
            assert_eq!(
                rect(&registry, "TargetCastingBarFill"),
                (0.0, 0.0, 150.0 * progress, 10.0)
            );
            assert_eq!(
                texture(&registry, "TargetCastingBarFill").tex_coords,
                [0.0, progress, 0.0, 1.0]
            );
            assert_eq!(
                rect(&registry, "TargetCastingBarSpark"),
                (150.0 * progress - 3.0, -3.0, 6.0, 16.0)
            );
            assert!(
                frame(&registry, "TargetCastingBarBackground")
                    .background_color
                    .is_none()
            );
        }
    }
}

#[test]
fn target_cast_modern_tree_has_retail_track_icon_shield_name_timer() {
    let r = registry(ActiveSkin::Modern, state(Some(cast())));
    let root = frame(&r, "TargetFrameSpellBar");
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(150.0), Dimension::Fixed(10.0))
    );
    assert_eq!(root.margin.left, Val::Px(343.0));
    assert_eq!(root.position.bottom, Val::Px(245.0));
    assert_eq!(rect(&r, "TargetCastingBarIcon"), (-22.0, 0.0, 20.0, 20.0));
    let Some(WidgetData::Texture(icon)) = &frame(&r, "TargetCastingBarIcon").widget_data else {
        panic!("not icon")
    };
    assert_eq!(
        icon.source,
        ui_toolkit::widgets::texture::TextureSource::FileDataId(135846)
    );
    assert_eq!(
        (
            frame(&r, "TargetCastingBarFill").width,
            frame(&r, "TargetCastingBarFill").height
        ),
        (Dimension::Fixed(37.5), Dimension::Fixed(10.0))
    );
    assert_eq!(text(&r, "TargetCastingBarSpellName"), "Frostbolt");
    assert_eq!(text(&r, "TargetCastingBarTimer"), "1.5");
    assert!(frame(&r, "TargetCastingBarShield").hidden);
    let mut c = cast();
    c.is_interruptible = false;
    let r = registry(ActiveSkin::Modern, state(Some(c)));
    assert!(!frame(&r, "TargetCastingBarShield").hidden);
    assert!(!frame(&r, "TargetCastingBarIcon").hidden);
}
#[test]
fn target_cast_forever_tree_has_bronze_holder_icon_overlay_and_timer() {
    let r = registry(ActiveSkin::Forever, state(Some(cast())));
    let root = frame(&r, "TargetFrameSpellBar");
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(240.0), Dimension::Fixed(24.0))
    );
    assert_eq!(
        (root.margin.left, root.margin.top),
        (Val::Px(210.0), Val::Px(300.0))
    );
    // The icon sits left of the track, on the same row, both inside the holder.
    let (ix, iy, iw, ih) = rect(&r, "TargetCastingBarIcon");
    let (bx, by, bw, bh) = rect(&r, "TargetCastingBarBackground");
    assert!(ix >= 0.0 && ix + iw <= bx, "icon left of the track");
    assert_eq!((iy, ih), (by, bh), "icon and track share a row");
    assert!(
        by >= 0.0 && by + bh <= 24.0 && bx + bw <= 240.0,
        "inside the holder"
    );
    assert_eq!(
        frame(&r, "TargetCastingBarFill").background_color,
        Some([0.80, 0.60, 0.36, 1.0])
    );
    assert_eq!(text(&r, "TargetCastingBarSpellName"), "Frostbolt");
    assert_eq!(text(&r, "TargetCastingBarTimer"), "1.5");
    let fill = frame(&r, "TargetCastingBarFill");
    assert!(frame(&r, "TargetFrameSpellBarBorder").frame_level + 1 >= fill.frame_level);
    assert!(frame(&r, "TargetCastingBarSpellName").frame_level >= fill.frame_level);
}
#[test]
fn target_cast_absent_hidden_or_targetless_adds_no_frames() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for s in [
            state(None),
            state(Some(CastingBarState::default())),
            {
                let mut s = state(Some(cast()));
                s.target = None;
                s
            },
            {
                let mut s = state(Some(cast()));
                s.show_target_frame = false;
                s
            },
        ] {
            let r = registry(skin, s);
            assert!(r.get_by_name("TargetFrameSpellBar").is_none());
        }
    }
}

#[test]
fn modern_target_cast_moves_below_auras_or_tot_as_retail_does() {
    let mut units = state(Some(cast()));
    units.target_of_target = Some(
        game_engine_ui_model::inworld_unit_frames_component::SmallUnitFrameState::from(
            units.target.as_ref().unwrap(),
        ),
    );
    let r = registry(ActiveSkin::Modern, units);
    assert_eq!(
        frame(&r, "TargetFrameSpellBar").position.bottom,
        Val::Px(194.0)
    );
    let mut units = state(Some(cast()));
    units.target.as_mut().unwrap().target_buffs = vec![
        game_engine_ui_model::inworld_unit_frames_component::TargetAuraIconState {
            spell_id: 1,
            icon_fdid: 135846,
            stacks: 1,
            dispel_color: None,
            large: true,
            elapsed: None,
        },
    ];
    let r = registry(ActiveSkin::Modern, units);
    assert_eq!(
        frame(&r, "TargetFrameSpellBar").position.bottom,
        Val::Px(234.5)
    );
}

fn target_aura_state(icon_fdid: u32) -> InWorldUnitFramesState {
    let mut units = state(None);
    units.target.as_mut().unwrap().target_buffs = vec![
        game_engine_ui_model::inworld_unit_frames_component::TargetAuraIconState {
            spell_id: 1_459,
            icon_fdid,
            stacks: 1,
            dispel_color: None,
            large: false,
            elapsed: None,
        },
    ];
    units
}

#[test]
fn ui_template_behavior_combat_target_aura_owns_tooltip_before_parent_unit_frame() {
    use ui_toolkit::atlas::set_thread_skin;
    use ui_toolkit::tooltip::{TooltipAnchor, tooltip_owner};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = registry(skin, target_aura_state(135932));
        let hit = registry.get_by_name("TargetBuffIcon0Texture").unwrap();
        let (owner, binding) = tooltip_owner(&registry, hit).expect("target aura owner");
        assert_eq!(owner.name.as_deref(), Some("TargetBuffIcon0"));
        assert_eq!(binding.provider.as_deref(), Some("target_aura"));
        assert_eq!(binding.anchor, TooltipAnchor::ByCenter);
        for name in ["PlayerFrame", "TargetFrame"] {
            let hit = registry.get_by_name(name).unwrap();
            let (owner, binding) = tooltip_owner(&registry, hit).expect(name);
            assert_eq!(owner.id, hit);
            assert_eq!(binding.provider.as_deref(), Some("hud_frame"));
            assert_eq!(binding.anchor, TooltipAnchor::Default);
        }
    }
}

fn drawn_fdids(r: &FrameRegistry) -> Vec<u32> {
    r.frames_iter()
        .filter_map(|frame| match frame.widget_data.as_ref()? {
            WidgetData::Texture(texture) => match texture.source {
                TextureSource::FileDataId(fdid) => Some(fdid),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A target aura replicated before the spell catalog has loaded has no icon yet
/// (`icon_fdid` 0, `aura_instance`): its button builds no icon texture under either skin;
/// once the catalog supplies the icon the same button draws it.
#[test]
fn target_aura_without_a_known_icon_builds_no_icon_texture() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut ctx = SharedContext::new();
        ctx.insert(skin);
        ctx.insert(target_aura_state(0));
        let mut r = registry(skin, state(None));
        let mut screen = Screen::new(inworld_unit_frames_screen);
        screen.sync(&ctx, &mut r);
        assert!(!drawn_fdids(&r).contains(&0), "{skin:?}");
        assert!(r.get_by_name("TargetBuffIcon0").is_some(), "{skin:?}");
        assert!(
            r.get_by_name("TargetBuffIcon0Texture").is_none(),
            "{skin:?}"
        );

        ctx.insert(target_aura_state(135_932));
        screen.sync(&ctx, &mut r);
        assert!(!drawn_fdids(&r).contains(&0), "{skin:?}");
        match frame(&r, "TargetBuffIcon0Texture").widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => {
                assert_eq!(
                    texture.source,
                    TextureSource::FileDataId(135_932),
                    "{skin:?}"
                )
            }
            other => panic!("{skin:?} aura icon is not a texture: {other:?}"),
        }
    }
}

#[test]
fn target_cast_interrupted_fading_bar_shows_its_text_at_its_alpha_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let interrupted = CastingBarState {
            spell_name: "Interrupted".into(),
            timer_text: String::new(),
            is_interrupted: true,
            alpha: 0.4,
            ..cast()
        };
        let r = registry(skin, state(Some(interrupted)));
        let root = frame(&r, "TargetFrameSpellBar");
        assert!(!root.hidden);
        assert_eq!(root.alpha, 0.4);
        assert_eq!(text(&r, "TargetCastingBarSpellName"), "Interrupted");
        let normal = registry(skin, state(Some(cast())));
        match skin {
            ActiveSkin::Modern => assert_ne!(
                texture(&r, "TargetCastingBarFill").source,
                texture(&normal, "TargetCastingBarFill").source,
                "interrupted bars use the failed source art"
            ),
            ActiveSkin::Forever => assert_ne!(
                frame(&r, "TargetCastingBarFill").background_color,
                frame(&normal, "TargetCastingBarFill").background_color,
                "interrupted bars use the failed colour"
            ),
        }
    }
}
