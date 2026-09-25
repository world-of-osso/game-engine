use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: FlightMapFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(flight_map_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rect"))
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
}

/// Stormwind (current) and Sentinel Hill (reachable, hovered, 5 copper).
fn stormwind_map() -> FlightMapFrameState {
    FlightMapFrameState {
        visible: true,
        left: 458.0,
        top: 195.0,
        tiles: vec![FlightMapTile {
            fdid: 2_353_944,
            rect: (1.0, 20.0, 66.8, 66.8),
        }],
        lines: vec![FlightMapLine {
            from: (445.0, 531.0),
            to: (432.0, 572.0),
            highlight: true,
        }],
        pins: vec![
            FlightMapPin {
                node: 2,
                x: 445.0,
                y: 531.0,
                look: PinLook::Current,
            },
            FlightMapPin {
                node: 4,
                x: 432.0,
                y: 572.0,
                look: PinLook::Hovered,
            },
        ],
        tooltip: Some(FlightMapTooltip {
            x: 442.0,
            y: 562.0,
            name: "Sentinel Hill, Westfall".into(),
            detail: Some(TooltipDetail::Cost(5)),
        }),
    }
}

#[test]
fn flight_map_draws_the_titled_frame_over_its_map_tiles() {
    let reg = build(stormwind_map());
    let root = rect(&reg, FRAME_NAME);
    assert_eq!(
        (root.x, root.y, root.width, root.height),
        (458.0, 195.0, 1004.0, 689.0)
    );
    assert_eq!(
        fontstring_text(&reg, "FlightMapFrameTitleText"),
        "Flight Map"
    );
    let tile = rect(&reg, "FlightMapFrameTile0");
    assert_eq!((tile.x - root.x, tile.y - root.y), (1.0, 20.0));
    assert_eq!(
        onclick(&reg, "FlightMapFrameCloseButton").as_deref(),
        Some(ACTION_CLOSE)
    );
}

#[test]
fn pins_are_sized_by_state_centred_on_their_node_and_clickable() {
    let reg = build(stormwind_map());
    let root = rect(&reg, FRAME_NAME);
    let current = rect(&reg, "FlightMapFramePin2");
    assert_eq!((current.width, current.height), (28.0, 28.0));
    assert_eq!((current.x - root.x, current.y - root.y), (431.0, 517.0));
    let hovered = rect(&reg, "FlightMapFramePin4");
    assert_eq!(hovered.width, 20.0);
    assert_eq!(
        onclick(&reg, "FlightMapFramePin4").as_deref(),
        Some("taxi_node:4")
    );
    assert!(
        reg.get_by_name("FlightMapFrameLine0Dot0").is_some(),
        "route drawn"
    );
}

#[test]
fn the_tooltip_names_the_node_and_its_cost() {
    let reg = build(stormwind_map());
    assert_eq!(
        fontstring_text(&reg, "FlightMapFrameTooltipName"),
        "Sentinel Hill, Westfall"
    );
    assert_eq!(
        fontstring_text(&reg, "FlightMapFrameTooltipMoneyAmount0"),
        "5"
    );
    let mut here = stormwind_map();
    here.tooltip.as_mut().unwrap().detail = Some(TooltipDetail::YouAreHere);
    let reg = build(here);
    assert_eq!(
        fontstring_text(&reg, "FlightMapFrameTooltipDetail"),
        "You are here"
    );
}
