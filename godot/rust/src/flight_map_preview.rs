//! Read-only fixture observation of the real native taxi session and replicated flight.
use super::*;

#[godot_api(secondary)]
impl GameClient {
    #[func]
    fn flight_map_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("open", self.flight_map.session.map.is_some());
        state.set(
            "hovered",
            self.flight_map.session.hovered.map_or(-1, i64::from),
        );
        state.set("missing_tiles", self.flight_map.missing_tiles as i64);
        let controlled = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<shared::components::MovementControl>())
            .is_some_and(|control| control.controlled);
        state.set("controlled", controlled);
        if let Ok(view) = self.flight_map_view() {
            state.set("map_id", i64::from(view.map.map_id));
            state.set("route_count", view.projection.routes.len() as i64);
            let ids: PackedInt64Array = view
                .projection
                .pins
                .iter()
                .map(|pin| i64::from(pin.node))
                .collect();
            state.set("node_ids", &ids);
            let tooltip = view
                .projection
                .pins
                .iter()
                .find(|pin| Some(pin.node) == view.hovered)
                .map(|pin| pin.tooltip())
                .unwrap_or_default();
            state.set("tooltip", tooltip.as_str());
        }
        state
    }
}
