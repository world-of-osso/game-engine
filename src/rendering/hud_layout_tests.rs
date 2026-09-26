//! Lays out the whole in-world HUD with the native layout engine at 16:9 and at a taller
//! aspect ratio, where auto-fit keeps the 1920 reference width and grows the canvas height.

use game_engine::experience_data::ExperienceState;
use game_engine::ui::layout::LayoutRect;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::buff_frame_component::{BuffFrameState, buff_frame_screen};
use game_engine::ui::screens::casting_bar_frame_component::{
    CastingBarState, casting_bar_frame_screen,
};
use game_engine::ui::screens::chat_frame_component::{ChatFrameView, chat_frame_screen};
use game_engine::ui::screens::inworld_hud_component::minimap_screen;
use game_engine::ui::screens::inworld_unit_frames_component::{
    InWorldUnitFramesState, SmallUnitFrameState, UnitFrameMenuState, UnitFrameState,
    inworld_unit_frames_screen,
};
use game_engine::ui::screens::status_tracking_bar_component::{
    StatusTrackingBarState, status_tracking_bar_screen,
};
use game_engine::ui::screens::ui_errors_frame_component::ui_errors_frame_screen;
use game_engine::ui::ui_errors::UiErrors;
use shared::protocol::PlayerXpUpdate;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::edit_mode::elements::EDIT_MODE_ELEMENTS;

#[path = "../ui/screens/menu_character_layout_test_support.rs"]
mod layout_test_support;

/// 1920×1080 window, and the 1472×1137 window from the live report (canvas 1920×1483).
const CANVASES: [(f32, f32); 2] = [(1920.0, 1080.0), (1920.0, 1137.0 * 1920.0 / 1472.0)];

const CORE_ELEMENTS: &[&str] = &[
    "MainActionBar",
    "MainStatusTrackingBarContainer",
    "PlayerFrame",
    "TargetFrame",
    "TargetOfTargetFrame",
    "FocusFrame",
    "PlayerCastingBarFrame",
    "MinimapCluster",
    "BuffFrame",
    "ChatFrame1",
    "MicroMenuContainer",
    "BagsBar",
];

fn unit(name: &str) -> UnitFrameState {
    UnitFrameState::named(name)
}

fn small(name: &str) -> SmallUnitFrameState {
    SmallUnitFrameState::from(&unit(name))
}

/// Registry size when the HUD is built, before the in-world auto-fit scale applies
/// (1472×1137 at the pre-world UI scale 1.15, as in the live report).
const BUILD_SIZE: (f32, f32) = (1280.0, 989.0);

/// Every HUD screen the in-world stage mounts, with all optional frames shown, built at
/// [`BUILD_SIZE`] and laid out after the canvas becomes `width`×`height`.
fn mount_hud(width: f32, height: f32) -> FrameRegistry {
    let mut reg = FrameRegistry::new(BUILD_SIZE.0, BUILD_SIZE.1);
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: unit("Theron"),
        target: Some(unit("Defias Thug")),
        target_of_target: Some(small("Theron")),
        focus: Some(small("Hogger")),
        menu: UnitFrameMenuState::default(),
    });
    shared.insert(CastingBarState {
        visible: true,
        ..CastingBarState::default()
    });
    shared.insert(BuffFrameState::default());
    shared.insert(ChatFrameView::default());
    shared.insert(UiErrors::default());
    let experience = ExperienceState(Some(PlayerXpUpdate {
        xp: 100,
        next_level_xp: 400,
        rested_xp: 50,
    }));
    shared.insert(StatusTrackingBarState::new(&experience, false, false));
    for build in [
        inworld_unit_frames_screen,
        casting_bar_frame_screen,
        buff_frame_screen,
        chat_frame_screen,
        ui_errors_frame_screen,
        status_tracking_bar_screen,
        minimap_screen,
    ] {
        Screen::new(build).sync(&shared, &mut reg);
    }
    let minimap = reg.get_by_name("MinimapCluster").expect("minimap cluster");
    reg.set_hidden(minimap, false);
    crate::rendering::action_bar::create_action_bars(&mut reg);
    reg.screen_width = width;
    reg.screen_height = height;
    layout_test_support::compute_layout(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> Option<LayoutRect> {
    let frame = reg.get(reg.get_by_name(name)?)?;
    let rect = frame.layout_rect.clone()?;
    (frame.visible && rect.width > 0.0 && rect.height > 0.0).then_some(rect)
}

fn visible_elements(reg: &FrameRegistry) -> Vec<(&'static str, LayoutRect)> {
    EDIT_MODE_ELEMENTS
        .iter()
        .filter_map(|element| Some((element.frame_name, rect(reg, element.frame_name)?)))
        .collect()
}

fn overlaps(a: &LayoutRect, b: &LayoutRect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn bottom(rect: &LayoutRect) -> f32 {
    rect.y + rect.height
}

#[test]
fn every_hud_element_is_on_screen_and_clear_of_its_neighbours() {
    for (width, height) in CANVASES {
        let reg = mount_hud(width, height);
        let elements = visible_elements(&reg);
        for name in CORE_ELEMENTS {
            assert!(
                elements.iter().any(|(shown, _)| shown == name),
                "{name} not laid out at {width}x{height}"
            );
        }
        for (name, r) in &elements {
            let on_screen =
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= width && bottom(r) <= height;
            assert!(on_screen, "{name} {r:?} off the {width}x{height} canvas");
        }
        for (i, (a_name, a)) in elements.iter().enumerate() {
            for (b_name, b) in &elements[i + 1..] {
                assert!(
                    !overlaps(a, b),
                    "{a_name} {a:?} overlaps {b_name} {b:?} at {width}x{height}"
                );
            }
        }
    }
}

#[test]
fn hud_regions_follow_the_accepted_composition() {
    for (width, height) in CANVASES {
        let reg = mount_hud(width, height);
        let get = |name| rect(&reg, name).unwrap_or_else(|| panic!("{name} not laid out"));
        let bar = get("MainActionBar");
        let player = get("PlayerFrame");
        let minimap = get("MinimapCluster");
        let chat = get("ChatFrame1");
        let micro = get("MicroMenuContainer");
        let bags = get("BagsBar");
        let size = format!("{width}x{height}");

        assert!(
            (bar.x + bar.width / 2.0 - width / 2.0).abs() < 1.0,
            "bar centred {size}"
        );
        assert!(height - bottom(&bar) < 60.0, "bar at the bottom {size}");
        // Retail Modern: status bar 1 at the screen bottom, the main bar 45 above it.
        let status = get("MainStatusTrackingBarContainer");
        assert!(
            (bottom(&status) - height).abs() < 0.5,
            "status bar at the bottom edge {size}"
        );
        assert!(
            (status.x + status.width / 2.0 - width / 2.0).abs() < 1.0,
            "status bar centred {size}"
        );
        assert_eq!(
            bottom(&status) - bottom(&bar),
            45.0,
            "main bar above it {size}"
        );
        assert!(
            status.y >= bottom(&bar),
            "status bar below the main bar {size}"
        );
        assert!(bottom(&player) <= bar.y, "cluster above the bar {size}");
        assert!(
            bar.y - bottom(&player) < 80.0,
            "cluster next to the bar {size}"
        );
        assert!(
            minimap.y < 20.0 && width - (minimap.x + minimap.width) < 20.0,
            "minimap top-right {size}"
        );
        // Chattynator's default window sits at BOTTOMLEFT (0, 40) (Core/Config.lua:28).
        assert!(
            chat.x < 40.0 && (height - bottom(&chat) - 40.0).abs() < 0.5,
            "chat bottom-left {size}"
        );
        for (name, r) in [("micro menu", &micro), ("bags", &bags)] {
            assert!(
                width - (r.x + r.width) < 20.0 && height - bottom(r) < 100.0,
                "{name} bottom-right {size}"
            );
        }
    }
}

#[test]
fn action_buttons_sit_in_one_row_on_the_retail_grid() {
    let reg = mount_hud(1920.0, 1080.0);
    let bar = rect(&reg, "MainActionBar").expect("main bar");
    let buttons: Vec<LayoutRect> = (1..=12)
        .map(|index| rect(&reg, &format!("ActionButton1_{index}")).expect("button"))
        .collect();
    for (index, button) in buttons.iter().enumerate() {
        assert_eq!((button.width, button.height), (45.0, 45.0));
        assert!(
            (button.x - (bar.x + index as f32 * 47.0)).abs() < 1.0,
            "button {} at {} (bar {})",
            index + 1,
            button.x,
            bar.x
        );
        assert!((button.y - buttons[0].y).abs() < 0.5);
    }
    assert!(bottom(&buttons[11]) <= bottom(&bar) + 0.5);
    assert!(buttons[11].x + buttons[11].width <= bar.x + bar.width);
}

#[test]
fn chat_tabs_are_laid_out_ten_apart() {
    let reg = mount_hud(1920.0, 1080.0);
    let tabs: Vec<LayoutRect> = (0..3)
        .map(|index| rect(&reg, &format!("ChatFrame1TabsTab{index}")).expect("tab laid out"))
        .collect();
    for pair in tabs.windows(2) {
        assert_eq!(pair[0].height, 22.0);
        assert!(pair[0].width >= 50.0, "label width plus 30 padding");
        assert!((pair[1].x - (pair[0].x + pair[0].width) - 10.0).abs() < 0.5);
    }
}

#[test]
fn chat_tab_caps_meet_the_middle_without_a_seam() {
    for (width, height) in CANVASES {
        let reg = mount_hud(width, height);
        for index in 0..3 {
            let part = |name: &str| {
                rect(&reg, &format!("ChatFrame1TabsTab{index}{name}")).expect("tab part laid out")
            };
            let (left, middle, right) = (part("Left"), part("Middle"), part("Right"));
            assert_eq!(left.x + left.width, middle.x, "tab {index} left cap");
            assert_eq!(middle.x + middle.width, right.x, "tab {index} right cap");
        }
    }
}
