use std::path::Path;

use shared::protocol::TaxiMap;

use super::*;

fn eastern_kingdoms() -> Option<FlightMapArt> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(DB2_DIR);
    if !dir.join("UiMapArtTile.csv").exists() {
        eprintln!("skipping: no UiMap CSVs");
        return None;
    }
    Some(FlightMapArt::load(&dir, 0).unwrap())
}

fn node(
    id: u32,
    name: &str,
    at: (f32, f32),
    state: TaxiNodeState,
    cost: u64,
    route: &[u32],
) -> TaxiNodeInfo {
    TaxiNodeInfo {
        node: id,
        name: name.into(),
        world_x: at.0,
        world_y: at.1,
        state,
        cost,
        route: route.to_vec(),
    }
}

/// Stormwind current, Sentinel Hill reachable, Moonbrook known but only via Sentinel
/// Hill, Lakeshire unreachable.
fn stormwind() -> TaxiMapState {
    let mut state = TaxiMapState::default();
    state.open(TaxiMap {
        npc: 9,
        continent: 0,
        nodes: vec![
            node(
                2,
                "Stormwind, Elwynn",
                (-8841.06, 489.66),
                TaxiNodeState::Current,
                0,
                &[],
            ),
            node(
                4,
                "Sentinel Hill, Westfall",
                (-10551.9, 1034.39),
                TaxiNodeState::Reachable,
                5,
                &[2, 4],
            ),
            node(
                583,
                "Moonbrook, Westfall",
                (-11009.0, 1488.0),
                TaxiNodeState::Reachable,
                10,
                &[2, 4, 583],
            ),
            node(
                5,
                "Lakeshire, Redridge",
                (-9429.1, -2231.4),
                TaxiNodeState::Unreachable,
                0,
                &[],
            ),
        ],
    });
    state
}

const SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

#[test]
fn the_map_fills_the_canvas_and_pins_sit_on_their_nodes() {
    let Some(art) = eastern_kingdoms() else {
        return;
    };
    let state = build_state(&stormwind(), Some(&art), SCREEN);
    assert!(state.visible);
    assert_eq!((state.left, state.top), (458.0, 195.5));
    assert_eq!(state.tiles.len(), 150);
    let first = &state.tiles[0].rect;
    assert!((first.0 - 1.0).abs() < 0.01 && (first.1 - 20.0).abs() < 0.01);
    let stormwind = state.pins.iter().find(|pin| pin.node == 2).unwrap();
    assert!(
        (stormwind.x - (1.0 + 0.4435 * 1002.0)).abs() < 1.0,
        "{stormwind:?}"
    );
    assert!(
        (stormwind.y - (20.0 + 0.7647 * 668.0)).abs() < 1.0,
        "{stormwind:?}"
    );
    assert_eq!(stormwind.look, PinLook::Current);
}

#[test]
fn unreachable_nodes_stay_hidden_and_background_lines_run_from_the_current_node() {
    let Some(art) = eastern_kingdoms() else {
        return;
    };
    let state = build_state(&stormwind(), Some(&art), SCREEN);
    let shown: Vec<u32> = state.pins.iter().map(|pin| pin.node).collect();
    assert_eq!(shown, [2, 4, 583]);
    // Both reachable nodes start with the 2 -> 4 hop: one background line.
    assert_eq!(state.lines.len(), 1);
    assert!(!state.lines[0].highlight);
    assert!(state.tooltip.is_none());
}

#[test]
fn hovering_a_reachable_node_highlights_its_route_and_shows_its_cost() {
    let Some(art) = eastern_kingdoms() else {
        return;
    };
    let mut taxi = stormwind();
    taxi.hovered = Some(583);
    let state = build_state(&taxi, Some(&art), SCREEN);
    assert_eq!(
        state.lines.len(),
        2,
        "Stormwind -> Sentinel Hill -> Moonbrook"
    );
    assert!(state.lines.iter().all(|line| line.highlight));
    let moonbrook = state.pins.iter().find(|pin| pin.node == 583).unwrap();
    assert_eq!(moonbrook.look, PinLook::Hovered);
    let tooltip = state.tooltip.unwrap();
    assert_eq!(tooltip.name, "Moonbrook, Westfall");
    assert_eq!(tooltip.detail, Some(TooltipDetail::Cost(10)));
}

#[test]
fn clicks_take_the_flight_or_close_the_map() {
    assert_eq!(
        request_for_action("taxi_node:4"),
        Some(TaxiRequest::Fly { destination: 4 })
    );
    assert_eq!(
        request_for_action("flight_map_close"),
        Some(TaxiRequest::Close)
    );
    assert_eq!(request_for_action("loot_close"), None);
    assert!(!build_state(&TaxiMapState::default(), None, SCREEN).visible);
}
