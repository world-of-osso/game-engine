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
            reaction: None,
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
fn right(r: (f32, f32, f32, f32)) -> f32 {
    r.0 + r.2
}
#[test]
fn text_is_player_left_and_target_tot_focus_mirrored() {
    let r = units(Reaction::Hostile);
    for (name, value, justify) in [
        ("PlayerLevelText", "60", "LEFT"),
        ("PlayerName", "Testpaladin", "LEFT"),
        ("PlayerHealthBarText", "75%", "RIGHT"),
        ("TargetLevelText", "60", "RIGHT"),
        ("TargetName", "Target", "RIGHT"),
        ("TargetHealthBarText", "75%", "LEFT"),
        ("TargetOfTargetLevelText", "60", "RIGHT"),
        ("TargetOfTargetName", "Target", "RIGHT"),
        ("TargetOfTargetHealthBarText", "75%", "LEFT"),
        ("FocusLevelText", "60", "RIGHT"),
    ] {
        assert_eq!(text(&r, name).text, value, "{name}");
        assert_eq!(text(&r, name).justify_h.as_str(), justify, "{name}");
    }
    // Player: level at the left edge, health at the right edge, name between;
    // Target and TargetOfTarget mirror that.
    for (prefix, mirrored) in [
        ("Player", false),
        ("Target", true),
        ("TargetOfTarget", true),
    ] {
        let level = rect(&r, &format!("{prefix}LevelText"));
        let name = rect(&r, &format!("{prefix}Name"));
        let health = rect(&r, &format!("{prefix}HealthBarText"));
        let (first, last) = if mirrored {
            (health, level)
        } else {
            (level, health)
        };
        assert!(right(first) <= name.0, "{prefix}: name overlaps left text");
        assert!(right(name) <= last.0, "{prefix}: name overlaps right text");
        // Names are one line (no word wrap, FlareUI UnitFrames.lua:2101), centred on
        // the text band the level and health texts span.
        assert!(
            name.3 < 20.0,
            "{prefix}Name is taller than one line: {}",
            name.3
        );
        assert_eq!(
            name.1 + name.3 / 2.0,
            level.1 + level.3 / 2.0,
            "{prefix}Name is off the band's centre"
        );
        assert_eq!((level.1, level.3), (health.1, health.3), "{prefix} band");
    }
    // Player and Target share one width: each Target text is the Player text mirrored.
    let width = right(rect(&r, "PlayerHealthBarText")) + rect(&r, "PlayerLevelText").0;
    for (player, target) in [
        ("PlayerLevelText", "TargetLevelText"),
        ("PlayerName", "TargetName"),
        ("PlayerHealthBarText", "TargetHealthBarText"),
    ] {
        let (p, t) = (rect(&r, player), rect(&r, target));
        assert_eq!(
            (t.0, t.2),
            (width - p.0 - p.2, p.2),
            "{target} mirrors {player}"
        );
    }
    // TargetOfTarget's health fill is 75% of its bar, draining from the left.
    let fill = rect(&r, "TargetOfTargetHealthBarFill");
    let Dimension::Fixed(bar_width) = frame(&r, "TargetOfTargetHealthBar").width else {
        panic!("TargetOfTargetHealthBar not fixed")
    };
    assert_eq!((fill.2, right(fill)), (0.75 * bar_width, bar_width));
    assert_eq!(
        frame(&r, "TargetOfTargetHealthBarFill").background_color,
        // hostile REACTION 0.87,0.27,0.27 times the Flat bar texture's 143/255 grey
        Some([
            0.87 * (143.0 / 255.0),
            0.27 * (143.0 / 255.0),
            0.27 * (143.0 / 255.0),
            1.0
        ])
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
    let icon = |i: u32| rect(&r, &format!("TargetBuffIcon{i}"));
    let level = rect(&r, "TargetLevelText");
    assert!(icon(0).1 + icon(0).3 <= 0.0, "icons sit above the frame");
    assert!(
        right(icon(0)) >= right(level),
        "first icon is at the right edge"
    );
    assert_eq!(icon(1).1, icon(0).1);
    assert!(
        right(icon(1)) <= icon(0).0,
        "row grows left without overlap"
    );
    assert_eq!(icon(5).0, icon(0).0, "next row restarts at the right edge");
    assert!(icon(5).1 + icon(5).3 <= icon(0).1, "next row stacks upward");
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
    let icon = rect(&r, "CastingBarIcon");
    let name = rect(&r, "CastingBarSpellName");
    let timer = rect(&r, "CastingBarTimer");
    assert_eq!(icon.2, icon.3, "icon is square");
    assert_eq!(
        (name.1, name.3),
        (timer.1, timer.3),
        "name and timer share a row"
    );
    assert!(right(name) <= timer.0, "timer is right of the name");
    assert_eq!(text(&r, "CastingBarSpellName").text, "Fireball");
    assert_eq!(text(&r, "CastingBarSpellName").justify_h.as_str(), "LEFT");
    assert_eq!(text(&r, "CastingBarTimer").text, "1.5");
    assert_eq!(text(&r, "CastingBarTimer").justify_h.as_str(), "RIGHT");
    let Some([red, green, blue, alpha]) = frame(&r, "CastingBarBackground").background_color else {
        panic!("CastingBarBackground has no colour")
    };
    assert!(
        red.max(green).max(blue) < 0.3 && alpha > 0.5,
        "track is dark"
    );
}
