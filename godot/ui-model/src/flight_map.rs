//! Server-owned flight-master state, projected through the shared world-map catalog.
use crate::ui_map_data::{UiMapCatalog, map_type};
use shared::protocol::{ActivateTaxi, TaxiMap, TaxiNodeState};
use std::collections::BTreeSet;

pub const CLOSE_ACTION: &str = "flightmap_close";
pub const DESTINATION_ACTION: &str = "flightmap_destination:";

#[derive(Default)]
pub struct FlightMapSession {
    pub map: Option<TaxiMap>,
    pub hovered: Option<u32>,
}

impl FlightMapSession {
    pub fn open(&mut self, map: TaxiMap) {
        self.map = Some(map);
        self.hovered = None;
    }
    pub fn close(&mut self) {
        self.map = None;
        self.hovered = None;
    }
    pub fn close_for(&mut self, npc: u64) {
        if self.map.as_ref().is_some_and(|map| map.npc == npc) {
            self.close();
        }
    }
    pub fn activate(&mut self, destination: u32) -> Option<ActivateTaxi> {
        let map = self.map.as_ref()?;
        let reachable = map
            .nodes
            .iter()
            .any(|node| node.node == destination && node.state == TaxiNodeState::Reachable);
        if !reachable {
            return None;
        }
        let request = ActivateTaxi {
            npc: map.npc,
            destination,
        };
        self.close();
        Some(request)
    }
    pub fn click(&mut self, action: &str) -> Result<Option<ActivateTaxi>, String> {
        let Some(destination) = action.strip_prefix(DESTINATION_ACTION) else {
            return Ok(None);
        };
        let destination = destination
            .parse()
            .map_err(|_| format!("Invalid taxi destination: {action}"))?;
        Ok(self.activate(destination))
    }
}

pub fn continent_map(catalog: &UiMapCatalog, map: &TaxiMap) -> Option<u32> {
    let current = map
        .nodes
        .iter()
        .find(|node| node.state == TaxiNodeState::Current)?;
    let best =
        catalog.best_map_for_position(map.continent, [current.world_x, current.world_y, 0.0])?;
    catalog.lineage(best).into_iter().find(|id| {
        catalog
            .map(*id)
            .is_some_and(|info| info.kind == map_type::CONTINENT)
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlightPin {
    pub node: u32,
    pub name: String,
    pub state: TaxiNodeState,
    pub cost: u64,
    pub uv: [f32; 2],
}
impl FlightPin {
    pub fn tooltip(&self) -> String {
        let detail = match self.state {
            TaxiNodeState::Current => "You are here".into(),
            // Reachable costs are rendered by the tooltip's SmallMoneyFrame coins.
            TaxiNodeState::Reachable => return self.name.clone(),
            TaxiNodeState::Unreachable => "Not Discovered".into(),
        };
        format!("{}\n{detail}", self.name)
    }
    pub fn size(&self) -> f32 {
        if self.state == TaxiNodeState::Current {
            28.0
        } else {
            20.0
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FlightProjection {
    pub pins: Vec<FlightPin>,
    pub routes: Vec<[[f32; 2]; 2]>,
}

pub fn project(catalog: &UiMapCatalog, map: &TaxiMap, hovered: Option<u32>) -> FlightProjection {
    let map_id = continent_map(catalog, map);
    let pins: Vec<_> = map
        .nodes
        .iter()
        .filter(|node| node.state != TaxiNodeState::Unreachable)
        .filter_map(|node| {
            let uv =
                catalog.map_position(map_id?, map.continent, [node.world_x, node.world_y, 0.0])?;
            Some(FlightPin {
                node: node.node,
                name: node.name.clone(),
                state: node.state,
                cost: node.cost,
                uv,
            })
        })
        .collect();
    let hops = route_hops(map, hovered);
    let routes = hops
        .into_iter()
        .filter_map(|(from, to)| {
            let from = pins.iter().find(|pin| pin.node == from)?.uv;
            let to = pins.iter().find(|pin| pin.node == to)?.uv;
            Some([from, to])
        })
        .collect();
    FlightProjection { pins, routes }
}

fn route_hops(map: &TaxiMap, hovered: Option<u32>) -> BTreeSet<(u32, u32)> {
    let highlighted = hovered.and_then(|id| {
        map.nodes
            .iter()
            .find(|node| node.node == id && node.state == TaxiNodeState::Reachable)
    });
    if let Some(node) = highlighted {
        return node
            .route
            .windows(2)
            .map(|pair| (pair[0], pair[1]))
            .collect();
    }
    map.nodes
        .iter()
        .filter(|node| node.state == TaxiNodeState::Reachable)
        .filter_map(|node| {
            let pair = node.route.get(..2)?;
            Some((pair[0], pair[1]))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{TaxiNodeInfo, TaxiNodeState};
    fn taxi() -> TaxiMap {
        TaxiMap {
            npc: 701,
            continent: 0,
            nodes: vec![
                TaxiNodeInfo {
                    node: 2,
                    name: "Stormwind, Elwynn".into(),
                    world_x: -8841.06,
                    world_y: 489.656,
                    state: TaxiNodeState::Current,
                    cost: 0,
                    route: vec![],
                },
                TaxiNodeInfo {
                    node: 4,
                    name: "Sentinel Hill, Westfall".into(),
                    world_x: -10551.9,
                    world_y: 1034.39,
                    state: TaxiNodeState::Reachable,
                    cost: 12345,
                    route: vec![2, 4],
                },
                TaxiNodeInfo {
                    node: 5,
                    name: "Lakeshire, Redridge".into(),
                    world_x: -9429.1,
                    world_y: -2231.4,
                    state: TaxiNodeState::Unreachable,
                    cost: 0,
                    route: vec![],
                },
            ],
        }
    }
    #[test]
    fn flightmap_click_emits_exact_request_and_closes() {
        let mut session = FlightMapSession::default();
        session.open(taxi());
        assert_eq!(
            session.click("flightmap_destination:4").unwrap(),
            Some(ActivateTaxi {
                npc: 701,
                destination: 4
            })
        );
        assert!(session.map.is_none());
        assert_eq!(session.activate(4), None);
    }
    #[test]
    fn flightmap_current_unknown_and_unreachable_do_not_activate() {
        let mut session = FlightMapSession::default();
        session.open(taxi());
        for node in [2, 5, 999] {
            assert_eq!(session.activate(node), None);
            assert!(session.map.is_some());
        }
    }
    #[test]
    fn flightmap_resolves_current_nodes_continent_not_its_zone() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir = [
            root.join("data/db2/12.1.0.69933"),
            root.join("../../data/db2/12.1.0.69933"),
        ]
        .into_iter()
        .find(|p| p.join("UiMapArtTile.csv").exists())
        .unwrap();
        let catalog = UiMapCatalog::load(&dir).unwrap();
        assert_eq!(continent_map(&catalog, &taxi()), Some(13));
        let projection = project(&catalog, &taxi(), None);
        assert_eq!(
            projection.pins.iter().map(|p| p.node).collect::<Vec<_>>(),
            vec![2, 4]
        );
        let stormwind = projection.pins[0].uv;
        assert!((stormwind[0] - 0.443).abs() < 0.003, "{stormwind:?}");
        assert!((stormwind[1] - 0.764).abs() < 0.003, "{stormwind:?}");
        assert_eq!(
            projection.routes,
            vec![[projection.pins[0].uv, projection.pins[1].uv]]
        );
        assert_eq!(
            project(&catalog, &taxi(), Some(4)).routes,
            projection.routes
        );
        assert_eq!(projection.pins[0].state, TaxiNodeState::Current);
        assert_eq!(projection.pins[0].size(), 28.0);
        assert_eq!(
            projection.pins[0].tooltip(),
            "Stormwind, Elwynn\nYou are here"
        );
        assert_eq!(projection.pins[1].tooltip(), "Sentinel Hill, Westfall");
        let mut multi_hop = taxi();
        multi_hop.nodes[2].state = TaxiNodeState::Reachable;
        multi_hop.nodes[2].route = vec![2, 4, 5];
        assert_eq!(project(&catalog, &multi_hop, None).routes.len(), 1);
        assert_eq!(project(&catalog, &multi_hop, Some(5)).routes.len(), 2);
    }
}
