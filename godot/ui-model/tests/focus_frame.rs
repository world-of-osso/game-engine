//! FocusFrame under both presets: shown with the focus unit's name, level and health while
//! there is a focus (`PLAYER_FOCUS_CHANGED`, TargetFrame.lua:239-247), hidden without one.

use std::path::PathBuf;

use game_engine_ui_model::faction_reaction::Reaction;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PowerBarState, SmallUnitFrameState, UnitFrameMenuState, UnitFrameState,
    inworld_unit_frames_screen,
};
use shared::components::PowerType;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn hogger(health_fraction: f32) -> SmallUnitFrameState {
    SmallUnitFrameState {
        name: "Hogger".into(),
        level: Some(("11".into(), "1.0,0.82,0.0,1.0".into())),
        health_fraction,
        dead: false,
        reaction: Some(Reaction::Hostile),
        class_id: None,
        power: Some(PowerBarState {
            power: PowerType::Rage,
            current: 20,
            max: 100,
        }),
    }
}

fn unit_frames(skin: ActiveSkin, focus: Option<SmallUnitFrameState>) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        player: UnitFrameState::named("Fbfocus"),
        target: None,
        target_of_target: None,
        focus,
        pet: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => text.text.clone(),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

fn width(frame: &Frame) -> f32 {
    match frame.width {
        Dimension::Fixed(width) => width,
        other => panic!("{:?} has no fixed width: {other:?}", frame.name),
    }
}

/// The share of the health bar its fill covers.
fn health_fill(registry: &FrameRegistry) -> f32 {
    width(frame(registry, "FocusHealthBarFill")) / width(frame(registry, "FocusHealthBar"))
}

#[test]
fn focus_frame_shows_the_focus_units_name_level_and_health_and_hides_without_one() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let shown = unit_frames(skin, Some(hogger(1.0)));
        assert!(!frame(&shown, "FocusFrame").hidden, "{skin:?}");
        assert_eq!(text(&shown, "FocusName"), "Hogger", "{skin:?}");
        assert_eq!(text(&shown, "FocusLevelText"), "11", "{skin:?}");
        assert!((health_fill(&shown) - 1.0).abs() < 0.01, "{skin:?}");

        let hurt = unit_frames(skin, Some(hogger(0.5)));
        assert!((health_fill(&hurt) - 0.5).abs() < 0.01, "{skin:?}");

        let cleared = unit_frames(skin, None);
        assert!(frame(&cleared, "FocusFrame").hidden, "{skin:?}");
    }
}

/// Retail FocusFrame (`TargetFrameTemplate`) keeps its `ManaBar`; FlareUI's focus frame has
/// none (`powerHeight = 0`, Core.lua:295).
#[test]
fn modern_focus_frame_shows_the_focus_units_power() {
    let shown = unit_frames(ActiveSkin::Modern, Some(hogger(1.0)));
    let mana = frame(&shown, "FocusManaBar");
    assert!(!mana.hidden);
    let fill = width(frame(&shown, "FocusManaBarFill")) / width(mana);
    assert!((fill - 0.2).abs() < 0.01, "20 of 100 rage: {fill}");
}
