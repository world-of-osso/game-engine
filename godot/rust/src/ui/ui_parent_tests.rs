use std::collections::HashMap;

use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, SmallUnitFrameState, UnitFrameMenuState, UnitFrameState,
    inworld_unit_frames_screen,
};
use shared::components::CreatureClassification;
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

/// Lay out the player frame, `target` with a target of target, on the UIParent canvas of
/// `viewport`; pixel rects of frames `names`.
fn unit_frame_rects<const N: usize>(
    viewport: (f32, f32),
    target: UnitFrameState,
    names: [&str; N],
) -> [LayoutRect; N] {
    let parent = UiParent::for_viewport(viewport.0, viewport.1);
    let mut registry = parent.registry();
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Fbportrait"),
        target_of_target: Some(SmallUnitFrameState::from(&UnitFrameState::named(
            "Fbportrait",
        ))),
        target: Some(target),
        focus: None,
        pet: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
    names.map(|name| pixels(&parent, &bounds[&registry.get_by_name(name).expect(name)]))
}

fn target_rects(viewport: (f32, f32)) -> (LayoutRect, LayoutRect) {
    let [frame, health] = unit_frame_rects(
        viewport,
        UnitFrameState::named("Training Dummy"),
        ["TargetFrame", "TargetHealthBar"],
    );
    (frame, health)
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
    // UI units: x = 1365.33/2 + 300, y = 768 - 250 - 100; 232×100.
    assert_rect(&frame, [921.25, 391.875, 217.5, 93.75], "720p TargetFrame");
    let (left, bottom) = retail_health_corner((1280.0, 720.0));
    assert_near(health.x, left, "720p health left");
    assert_near(health.y + health.height, bottom, "720p health bottom");
}

#[test]
fn target_frame_sits_at_the_modern_preset_at_1080p() {
    let (frame, health) = target_rects((1920.0, 1080.0));
    assert_rect(
        &frame,
        [1381.875, 587.8125, 326.25, 140.625],
        "1080p TargetFrame",
    );
    let (left, bottom) = retail_health_corner((1920.0, 1080.0));
    assert_near(health.x, left, "1080p health left");
    assert_near(health.y + health.height, bottom, "1080p health bottom");
}

/// Timber (world.db creature_template 1132, rank 4 rare) at 1080p (scale 1.40625): the
/// TargetFrame `Portrait` 58×58 TOPRIGHT (-26, -19) (TargetFrame.xml:58-64) and the
/// `PlayerPortrait` 60×60 TOPLEFT (24, -19) (PlayerFrame.xml:27-31); the rare star centred on
/// the target portrait's BOTTOM (TargetFrame.xml:281-284); target of target clear of the
/// whole target frame.
#[test]
fn portraits_sit_at_their_retail_anchors_with_the_rare_star_on_the_target_portrait() {
    let scale = 1080.0 / 768.0;
    let timber = UnitFrameState {
        classification: CreatureClassification::Rare,
        ..UnitFrameState::named("Timber")
    };
    let [target, portrait, star, player, player_portrait, tot] = unit_frame_rects(
        (1920.0, 1080.0),
        timber,
        [
            "TargetFrame",
            "TargetFramePortrait",
            "TargetBossIcon",
            "PlayerFrame",
            "PlayerPortrait",
            "TargetOfTargetFrame",
        ],
    );
    assert_rect(
        &portrait,
        [
            target.x + 148.0 * scale,
            target.y + 19.0 * scale,
            58.0 * scale,
            58.0 * scale,
        ],
        "TargetFramePortrait",
    );
    assert_rect(
        &player_portrait,
        [
            player.x + 24.0 * scale,
            player.y + 19.0 * scale,
            60.0 * scale,
            60.0 * scale,
        ],
        "PlayerPortrait",
    );
    assert_near(
        star.x + star.width / 2.0,
        portrait.x + portrait.width / 2.0,
        "star centre x",
    );
    assert_near(
        star.y + star.height / 2.0,
        portrait.y + portrait.height,
        "star centre y",
    );
    assert!(
        tot.x >= target.x + target.width,
        "TargetOfTargetFrame at {} overlaps TargetFrame ending at {}",
        tot.x,
        target.x + target.width
    );
}
