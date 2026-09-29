use std::collections::HashMap;

use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, UnitFrameMenuState, UnitFrameState, inworld_unit_frames_screen,
};
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::screen::{Screen, SharedContext};

use super::UiParent;
use crate::ui::layout::compute_layout_with_intrinsics;

/// Retail Modern preset TargetFrame: BOTTOMLEFT to UIParent BOTTOM at (300, 250)
/// (Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:245-257), 232×100
/// (Blizzard_UnitFrame/Mainline/TargetFrame.xml:144). `CheckClassification` puts its 126×20
/// health bar's BOTTOMRIGHT at the frame's LEFT + (149, -10) (TargetFrame.lua:417-419).
const RETAIL_TARGET: (f32, f32) = (300.0, 250.0);
const RETAIL_HEALTH_LEFT: f32 = 149.0 - 126.0;
const RETAIL_HEALTH_BOTTOM: f32 = 100.0 / 2.0 - 10.0;

fn pixels(parent: &UiParent, rect: &LayoutRect) -> LayoutRect {
    LayoutRect {
        x: rect.x * parent.scale,
        y: rect.y * parent.scale,
        width: rect.width * parent.scale,
        height: rect.height * parent.scale,
    }
}

/// Lay out the unit frames with a target on the UIParent canvas of `viewport`; pixel rects.
fn target_rects(viewport: (f32, f32)) -> (LayoutRect, LayoutRect) {
    let parent = UiParent::for_viewport(viewport.0, viewport.1);
    let mut registry = parent.registry();
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: false,
        show_target_frame: true,
        player: UnitFrameState::named(""),
        target: Some(UnitFrameState::named("Training Dummy")),
        target_of_target: None,
        focus: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
    });
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
    let rect = |name| pixels(&parent, &bounds[&registry.get_by_name(name).unwrap()]);
    (rect("TargetFrame"), rect("TargetHealthBar"))
}

/// Retail health bar left and bottom edges in pixels for `viewport`.
fn retail_health_corner((width, height): (f32, f32)) -> (f32, f32) {
    let scale = height / 768.0;
    let left = width / 2.0 + (RETAIL_TARGET.0 + RETAIL_HEALTH_LEFT) * scale;
    let bottom = height - (RETAIL_TARGET.1 + RETAIL_HEALTH_BOTTOM) * scale;
    (left, bottom)
}

fn assert_near(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() < 0.01,
        "{what}: {actual} != {expected}"
    );
}

fn assert_rect(actual: &LayoutRect, expected: [f32; 4], what: &str) {
    assert_near(actual.x, expected[0], &format!("{what} x"));
    assert_near(actual.y, expected[1], &format!("{what} y"));
    assert_near(actual.width, expected[2], &format!("{what} width"));
    assert_near(actual.height, expected[3], &format!("{what} height"));
}

#[test]
fn ui_parent_is_768_units_tall_at_any_resolution() {
    let hd = UiParent::for_viewport(1280.0, 720.0);
    let full = UiParent::for_viewport(1920.0, 1080.0);
    assert_eq!((hd.height, hd.scale), (768.0, 0.9375));
    assert_eq!((full.height, full.scale), (768.0, 1.40625));
    assert_near(hd.width, 4096.0 / 3.0, "16:9 width");
    assert_near(full.width, 4096.0 / 3.0, "16:9 width");
}

#[test]
fn target_frame_sits_at_the_modern_preset_at_720p() {
    let (frame, health) = target_rects((1280.0, 720.0));
    // UI units: x = 1365.33/2 + 320, y = 768 - 273 - 51; 133×51 portrait-off art.
    assert_rect(
        &frame,
        [940.0, 416.25, 124.6875, 47.8125],
        "720p TargetFrame",
    );
    let (left, bottom) = retail_health_corner((1280.0, 720.0));
    assert_near(health.x, left, "720p health left");
    assert_near(health.y + health.height, bottom, "720p health bottom");
}

#[test]
fn target_frame_sits_at_the_modern_preset_at_1080p() {
    let (frame, health) = target_rects((1920.0, 1080.0));
    assert_rect(
        &frame,
        [1410.0, 624.375, 187.03125, 71.71875],
        "1080p TargetFrame",
    );
    let (left, bottom) = retail_health_corner((1920.0, 1080.0));
    assert_near(health.x, left, "1080p health left");
    assert_near(health.y + health.height, bottom, "1080p health bottom");
}
