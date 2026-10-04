//! A dead unit's frame: Retail's target, target-of-target and focus frames show `DEAD` on
//! an empty health bar (TargetFrame.lua:467-480, 915-924); under Forever, FlareUI's
//! health text reads `DEAD` and the power text goes (UnitFrames.lua:386-393, 411).

use std::path::PathBuf;

use game_engine_ui_model::faction_reaction::Reaction;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PowerBarState, SmallUnitFrameState, UnitFrameMenuState, UnitFrameState,
    inworld_unit_frames_screen,
};
use game_engine_ui_model::status_text_data::{StatusTextDisplay, TextStatusBar};
use shared::components::PowerType;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

/// A 120-health Stonetusk Boar with `current` health left, as the target frame builds it
/// (`zeroText = ""`, TargetFrame.lua:760-769).
fn boar(current: i64) -> UnitFrameState {
    UnitFrameState {
        health_text: TextStatusBar::HEALTH.with_zero_text("").text(
            current,
            120,
            StatusTextDisplay::Numeric,
            false,
        ),
        health_fraction: current as f32 / 120.0,
        dead: current == 0,
        reaction: Some(Reaction::Hostile),
        ..UnitFrameState::named("Stonetusk Boar")
    }
}

/// The player at 0 of 400 health with 80 of 100 mana.
fn dead_player() -> UnitFrameState {
    UnitFrameState {
        dead: true,
        power: Some(PowerBarState {
            power: PowerType::Mana,
            current: 80,
            max: 100,
        }),
        ..UnitFrameState::named("Fbunitstates")
    }
}

fn frames(skin: ActiveSkin, player: UnitFrameState, target: UnitFrameState) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    let tot = SmallUnitFrameState::from(&target);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        player,
        target: Some(target),
        target_of_target: Some(tot.clone()),
        focus: Some(tot),
        pet: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

/// The text a font string shows, `None` while it is hidden or absent.
fn shown_text(registry: &FrameRegistry, name: &str) -> Option<String> {
    let frame = registry.get(registry.get_by_name(name)?)?;
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(font)) => (!frame.hidden).then(|| font.text.clone()),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

fn shown(registry: &FrameRegistry, name: &str) -> bool {
    registry
        .get_by_name(name)
        .and_then(|id| registry.get(id))
        .is_some_and(|frame| !frame.hidden)
}

#[test]
fn modern_dead_target_shows_dead_on_an_empty_bar() {
    let registry = frames(ActiveSkin::Modern, dead_player(), boar(0));
    for prefix in ["Target", "TargetOfTarget", "Focus"] {
        assert_eq!(
            shown_text(&registry, &format!("{prefix}DeadText")).as_deref(),
            Some("Dead"),
            "{prefix}"
        );
        assert!(
            !shown(&registry, &format!("{prefix}HealthBarFill")),
            "{prefix} bar empty"
        );
    }
    assert_eq!(shown_text(&registry, "TargetHealthBarText"), None);
    assert_eq!(
        shown_text(&registry, "PlayerDeadText"),
        None,
        "PlayerFrame has no DeadText"
    );
}

#[test]
fn modern_living_target_shows_no_dead_text() {
    let registry = frames(ActiveSkin::Modern, dead_player(), boar(1));
    for prefix in ["Target", "TargetOfTarget", "Focus"] {
        assert_eq!(shown_text(&registry, &format!("{prefix}DeadText")), None);
    }
    assert_eq!(
        shown_text(&registry, "TargetHealthBarText").as_deref(),
        Some("1 / 120")
    );
}

#[test]
fn forever_dead_units_read_dead_instead_of_percent_and_hide_power() {
    let registry = frames(ActiveSkin::Forever, dead_player(), boar(0));
    for prefix in ["Player", "Target", "TargetOfTarget", "Focus"] {
        assert_eq!(
            shown_text(&registry, &format!("{prefix}HealthBarText")).as_deref(),
            Some("Dead"),
            "{prefix}"
        );
    }
    assert_eq!(shown_text(&registry, "PlayerManaBarText"), None);

    let living = frames(
        ActiveSkin::Forever,
        UnitFrameState {
            dead: false,
            health_fraction: 0.5,
            ..dead_player()
        },
        boar(60),
    );
    assert_eq!(
        shown_text(&living, "TargetHealthBarText").as_deref(),
        Some("50%")
    );
    assert_eq!(
        shown_text(&living, "PlayerManaBarText").as_deref(),
        Some("80")
    );
}
