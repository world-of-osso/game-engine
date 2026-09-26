use shared::protocol::PlayerXpUpdate;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::screen_test_helpers::fontstring_text;

#[path = "menu_character_layout_test_support.rs"]
mod layout_test_support;

fn experience(xp: u32, next_level_xp: u32, rested_xp: u32) -> ExperienceState {
    ExperienceState(Some(PlayerXpUpdate {
        xp,
        next_level_xp,
        rested_xp,
    }))
}

fn laid_out(state: StatusTrackingBarState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(status_tracking_bar_screen).sync(&shared, &mut reg);
    layout_test_support::compute_layout(&mut reg);
    reg
}

fn frame<'a>(reg: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    reg.get(
        reg.get_by_name(name)
            .unwrap_or_else(|| panic!("{name} missing")),
    )
    .expect(name)
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    frame(reg, name)
        .layout_rect
        .clone()
        .unwrap_or_else(|| panic!("{name} has no layout"))
}

/// Layout snaps to whole pixels.
fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.5,
        "{actual} is not within half a pixel of {expected}"
    );
}

fn shown(reg: &FrameRegistry, name: &str) -> bool {
    !frame(reg, name).hidden
}

#[test]
fn container_is_the_retail_size_at_the_bottom_centre() {
    let reg = laid_out(StatusTrackingBarState::new(
        &experience(0, 400, 0),
        false,
        false,
    ));
    let container = rect(&reg, "MainStatusTrackingBarContainer");
    assert_eq!((container.width, container.height), (571.0, 17.0));
    assert_eq!(container.x, (1920.0 - 571.0) / 2.0);
    assert_eq!(container.y + container.height, 1080.0);
    let bar = rect(&reg, "MainStatusTrackingBarContainerExpBar");
    assert_eq!((bar.width, bar.height), (565.0, 11.0));
    assert_eq!(bar.x - container.x, 1.0);
    assert_eq!(container.y + container.height - (bar.y + bar.height), 5.0);
}

#[test]
fn fill_width_follows_the_player_xp_update() {
    let reg = laid_out(StatusTrackingBarState::new(
        &experience(1_000, 4_000, 0),
        false,
        false,
    ));
    let fill = rect(&reg, "MainStatusTrackingBarContainerExpBarFill");
    assert_near(fill.width, 565.0 * 0.25);
    let bar = rect(&reg, "MainStatusTrackingBarContainerExpBar");
    assert_eq!(fill.x, bar.x);
    let fill_frame = frame(&reg, "MainStatusTrackingBarContainerExpBarFill");
    match &fill_frame.widget_data {
        Some(ui_toolkit::frame::WidgetData::Texture(texture)) => assert_eq!(
            texture.source,
            crate::ui::widgets::texture::TextureSource::FileDataId(4_615_784)
        ),
        other => panic!("fill is not a texture: {other:?}"),
    }
    assert!(!shown(
        &reg,
        "MainStatusTrackingBarContainerExhaustionLevelFillBar"
    ));
    assert!(!shown(&reg, "MainStatusTrackingBarContainerExhaustionTick"));
}

#[test]
fn rested_overlay_and_tick_mark_where_rested_xp_ends() {
    let state = StatusTrackingBarState::new(&experience(1_000, 4_000, 1_000), false, false);
    assert!(state.xp.as_ref().unwrap().rested);
    let reg = laid_out(state);
    let bar = rect(&reg, "MainStatusTrackingBarContainerExpBar");
    let overlay = rect(&reg, "MainStatusTrackingBarContainerExhaustionLevelFillBar");
    assert!(shown(
        &reg,
        "MainStatusTrackingBarContainerExhaustionLevelFillBar"
    ));
    assert_eq!(overlay.x, bar.x);
    assert_near(overlay.width, 565.0 * 0.5);
    let tick = rect(&reg, "MainStatusTrackingBarContainerExhaustionTick");
    assert!(shown(&reg, "MainStatusTrackingBarContainerExhaustionTick"));
    assert_eq!((tick.width, tick.height), (10.0, 14.0));
    assert_near(tick.x + tick.width / 2.0, bar.x + 565.0 * 0.5);
    // yOffset 2: the pip centre sits 2 above the bar centre.
    assert_near(tick.y + tick.height / 2.0, bar.y + bar.height / 2.0 - 2.0);
}

#[test]
fn rested_pool_past_the_level_hides_overlay_and_tick() {
    let state = StatusTrackingBarState::new(&experience(3_000, 4_000, 2_000), false, false);
    let xp = state.xp.clone().unwrap();
    assert!(xp.rested);
    assert_eq!((xp.prediction, xp.tick), (None, None));
    let reg = laid_out(state);
    assert!(!shown(
        &reg,
        "MainStatusTrackingBarContainerExhaustionLevelFillBar"
    ));
    assert!(!shown(&reg, "MainStatusTrackingBarContainerExhaustionTick"));
}

#[test]
fn tick_hides_at_the_bar_edge_but_overlay_stays() {
    let xp = StatusTrackingBarState::new(&experience(3_970, 4_000, 5), false, false)
        .xp
        .unwrap();
    assert_eq!(xp.tick, None);
    assert!(xp.prediction.is_some());
}

#[test]
fn hidden_at_the_level_cap_and_before_any_update() {
    for experience in [experience(0, 0, 0), ExperienceState(None)] {
        let state = StatusTrackingBarState::new(&experience, false, true);
        assert_eq!(state.xp, None);
        let reg = laid_out(state);
        assert!(!shown(&reg, "MainStatusTrackingBarContainer"));
    }
}

#[test]
fn edit_mode_shows_the_empty_container_at_the_level_cap() {
    let reg = laid_out(StatusTrackingBarState::new(
        &experience(0, 0, 0),
        true,
        false,
    ));
    assert!(shown(&reg, "MainStatusTrackingBarContainer"));
    assert!(
        reg.get_by_name("MainStatusTrackingBarContainerExpBar")
            .is_none()
    );
}

#[test]
fn bar_text_shows_only_on_hover() {
    let idle = laid_out(StatusTrackingBarState::new(
        &experience(1_234, 4_000, 0),
        false,
        false,
    ));
    assert!(!shown(&idle, "MainStatusTrackingBarContainerExpBarText"));
    let hovered = laid_out(StatusTrackingBarState::new(
        &experience(1_234, 4_000, 0),
        false,
        true,
    ));
    assert!(shown(&hovered, "MainStatusTrackingBarContainerExpBarText"));
    assert_eq!(
        fontstring_text(&hovered, "MainStatusTrackingBarContainerExpBarText"),
        "XP: 1234/4000"
    );
}
