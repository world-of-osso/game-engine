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
    widgets::texture::TextureSource,
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
    assert_eq!(rect(&r, "TargetCastingBarIcon"), (4.0, 4.0, 16.0, 16.0));
    assert_eq!(
        rect(&r, "TargetCastingBarBackground"),
        (20.0, 4.0, 216.0, 16.0)
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
        assert_ne!(
            frame(&r, "TargetCastingBarFill").background_color,
            frame(&normal, "TargetCastingBarFill").background_color,
            "interrupted bars use the failed colour"
        );
    }
}
