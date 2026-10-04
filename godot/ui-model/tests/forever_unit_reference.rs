//! Concrete Forever reference measurements; Modern's complete trees remain in
//! forever_flare_frames.rs. No image/texture fallback or live client needed.
use game_engine_ui_model::casting_bar_frame_component::{
    CastingBarState, casting_bar_frame_screen,
};
use game_engine_ui_model::faction_reaction::Reaction;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PetFrameState, PowerBarState, SmallUnitFrameState, TargetAuraIconState,
    UnitFrameMenuState, UnitFrameState, inworld_unit_frames_screen,
};
use shared::components::PowerType;
use std::path::PathBuf;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn unit(name: &str, reaction: Reaction) -> UnitFrameState {
    UnitFrameState {
        level_text: "60".into(),
        health_fraction: 0.75,
        reaction: Some(reaction),
        power: Some(PowerBarState {
            power: PowerType::Mana,
            current: 40,
            max: 100,
        }),
        target_buffs: (1..=7).map(aura).collect(),
        ..UnitFrameState::named(name)
    }
}
fn aura(spell_id: u32) -> TargetAuraIconState {
    TargetAuraIconState {
        spell_id,
        icon_fdid: 135812,
        stacks: 1,
        dispel_color: None,
        large: true,
        elapsed: None,
    }
}
fn units(reaction: Reaction) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let player = unit("Testpaladin", Reaction::Friendly);
    let target = unit("Target", reaction);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        player,
        target_of_target: Some(SmallUnitFrameState::from(&target)),
        focus: Some(SmallUnitFrameState::from(&target)),
        target: Some(target),
        pet: Some(PetFrameState {
            name: "Wolf".into(),
            health_fraction: 0.5,
            health_text: Default::default(),
            power: None,
            power_text: Default::default(),
        }),
        bosses: vec![],
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}
fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("missing {name}")),
        )
        .unwrap()
}
fn rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(registry, name);
    let (Val::Px(x), Val::Px(y), Dimension::Fixed(w), Dimension::Fixed(h)) =
        (f.position.left, f.position.top, f.width, f.height)
    else {
        panic!("{name} not fixed")
    };
    (x, y, w, h)
}
fn text<'a>(
    registry: &'a FrameRegistry,
    name: &str,
) -> &'a ui_toolkit::widgets::font_string::FontStringData {
    let Some(WidgetData::FontString(text)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name} not text")
    };
    text
}
#[test]
fn text_is_player_left_and_target_tot_focus_mirrored() {
    let r = units(Reaction::Hostile);
    for (name, expected, value, justify) in [
        ("PlayerLevelText", (8.0, 4.0, 20.0, 38.0), "60", "LEFT"),
        (
            "PlayerName",
            (31.0, 4.0, 157.0, 38.0),
            "Testpaladin",
            "LEFT",
        ),
        (
            "PlayerHealthBarText",
            (192.0, 4.0, 40.0, 38.0),
            "75%",
            "RIGHT",
        ),
        ("TargetLevelText", (212.0, 4.0, 20.0, 52.0), "60", "RIGHT"),
        ("TargetName", (52.0, 4.0, 157.0, 52.0), "Target", "RIGHT"),
        ("TargetHealthBarText", (8.0, 4.0, 40.0, 52.0), "75%", "LEFT"),
        (
            "TargetOfTargetLevelText",
            (92.0, 4.0, 20.0, 20.0),
            "60",
            "RIGHT",
        ),
        (
            "TargetOfTargetName",
            (52.0, 4.0, 37.0, 20.0),
            "Target",
            "RIGHT",
        ),
        (
            "TargetOfTargetHealthBarText",
            (8.0, 4.0, 40.0, 20.0),
            "75%",
            "LEFT",
        ),
        ("FocusLevelText", (132.0, 4.0, 20.0, 28.0), "60", "RIGHT"),
    ] {
        assert_eq!(rect(&r, name), expected, "{name}");
        assert_eq!(text(&r, name).text, value, "{name}");
        assert_eq!(text(&r, name).justify_h.as_str(), justify, "{name}");
    }
    assert_eq!(
        rect(&r, "TargetOfTargetHealthBarFill"),
        (28.0, 0.0, 84.0, 20.0)
    );
    assert_eq!(
        frame(&r, "TargetOfTargetHealthBarFill").background_color,
        Some([0.87, 0.27, 0.27, 1.0])
    );
}
#[test]
fn pet_is_below_player_right_and_tot_right_of_target() {
    let r = units(Reaction::Hostile);
    let pet = frame(&r, "PetFrame");
    let tot = frame(&r, "TargetOfTargetFrame");
    assert_eq!(
        (pet.width, pet.height),
        (Dimension::Fixed(160.0), Dimension::Fixed(28.0))
    );
    assert_eq!(
        (pet.margin.left, pet.margin.top),
        (Val::Px(-370.0), Val::Px(306.0))
    );
    assert_eq!(
        (tot.margin.left, tot.margin.top),
        (Val::Px(458.0), Val::Px(240.0))
    );
    for prefix in ["Player", "Target", "TargetOfTarget", "Focus", "PetFrame"] {
        assert!(r.get_by_name(&format!("{prefix}Portrait")).is_none());
    }
}
#[test]
fn existing_aura_icons_are_above_right_edge_growing_left() {
    let r = units(Reaction::Hostile);
    for prefix in ["Target"] {
        assert_eq!(
            rect(&r, &format!("{prefix}BuffIcon0")),
            (216.0, -24.0, 20.0, 20.0)
        );
        assert_eq!(
            rect(&r, &format!("{prefix}BuffIcon1")),
            (194.0, -24.0, 20.0, 20.0)
        );
    }
    assert_eq!(rect(&r, "TargetBuffIcon5"), (216.0, -46.0, 20.0, 20.0));
}
#[test]
fn cast_icon_name_remaining_time_and_dark_track() {
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(CastingBarState {
        visible: true,
        spell_name: "Fireball".into(),
        icon_fdid: Some(135812),
        timer_text: "1.5".into(),
        progress: 0.5,
        ..Default::default()
    });
    let mut r = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(casting_bar_frame_screen).sync(&shared, &mut r);
    assert_eq!(rect(&r, "CastingBarIcon"), (4.0, 4.0, 26.0, 26.0));
    assert_eq!(rect(&r, "CastingBarSpellName"), (4.0, 0.0, 240.0, 26.0));
    assert_eq!(rect(&r, "CastingBarTimer"), (248.0, 0.0, 40.0, 26.0));
    assert_eq!(text(&r, "CastingBarSpellName").text, "Fireball");
    assert_eq!(text(&r, "CastingBarSpellName").justify_h.as_str(), "LEFT");
    assert_eq!(text(&r, "CastingBarTimer").text, "1.5");
    assert_eq!(text(&r, "CastingBarTimer").justify_h.as_str(), "RIGHT");
    assert_eq!(
        frame(&r, "CastingBarBackground").background_color,
        Some([0.15, 0.15, 0.15, 0.9])
    );
}
