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
    unit_frames_with_target_of(skin, focus, None)
}

fn unit_frames_with_target_of(
    skin: ActiveSkin,
    focus: Option<SmallUnitFrameState>,
    target_of_target: Option<SmallUnitFrameState>,
) -> FrameRegistry {
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
        target: target_of_target
            .as_ref()
            .map(|_| UnitFrameState::named("Hogger")),
        target_of_target,
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

fn fixed_rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(registry, name);
    let (Dimension::Fixed(width), Dimension::Fixed(height)) = (f.width, f.height) else {
        panic!("{name} is not fixed-size");
    };
    let px = |value: ui_toolkit::layout_values::Val| match value {
        ui_toolkit::layout_values::Val::Px(px) => px,
        other => panic!("{name} is placed at {other:?}"),
    };
    (px(f.position.left), px(f.position.top), width, height)
}

fn parent_size(registry: &FrameRegistry, name: &str) -> (f32, f32) {
    let parent = registry
        .get(frame(registry, name).parent_id.expect(name))
        .unwrap();
    match (parent.width, parent.height) {
        (Dimension::Fixed(width), Dimension::Fixed(height)) => (width, height),
        other => panic!("{name}'s parent is not fixed-size: {other:?}"),
    }
}

/// Retail's small-frame `Name` is one 12-high line for a 10pt font (TargetFrame.xml:113-114,
/// 425-426) and FlareUI's is `SetWordWrap(false)` (UnitFrames.lua:2101): a long name stays
/// on one truncated line inside the frame. The client draws a fixed FontString on one line,
/// ellipsised, when its height holds no second line (projection.rs `update_label`).
#[test]
fn a_long_name_stays_on_one_line_inside_the_small_frames() {
    let long = || SmallUnitFrameState {
        name: "Stormwind Army Registrar".into(),
        ..hogger(1.0)
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = unit_frames_with_target_of(skin, Some(long()), Some(long()));
        for name in ["FocusName", "TargetOfTargetName"] {
            assert_eq!(
                text(&registry, name),
                "Stormwind Army Registrar",
                "{skin:?}"
            );
            let Some(WidgetData::FontString(data)) = frame(&registry, name).widget_data.as_ref()
            else {
                panic!("{name} is not a FontString");
            };
            let (x, y, width, height) = fixed_rect(&registry, name);
            let (parent_w, parent_h) = parent_size(&registry, name);
            let font_size = data.font_size;
            assert!(
                height < 2.0 * font_size,
                "{skin:?} {name}: {height} high holds two {font_size}pt lines"
            );
            assert!(
                x >= 0.0 && x + width <= parent_w,
                "{skin:?} {name}: {x}+{width} outside {parent_w}"
            );
            assert!(
                y >= 0.0 && y + height <= parent_h,
                "{skin:?} {name}: {y}+{height} outside {parent_h}"
            );
        }
    }
}
