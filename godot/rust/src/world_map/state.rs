//! Fixture-facing world map state serialization.

use super::*;

#[godot_api(secondary)]
impl GameClient {
    /// Displayed map, the player's map and marker, for fixtures.
    #[func]
    fn world_map_state(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        let Some(data) = self.world_map.data() else {
            return result;
        };
        let Some(state) = self.world_map_view() else {
            return result;
        };
        let map = data.catalog.map(state.map_id);
        result.set("open", self.world_map.is_open());
        result.set("map_id", state.map_id as i64);
        result.set("map_name", state.map_name.as_str());
        result.set("map_kind", map.map_or(-1, |map| i64::from(map.kind)));
        let player_map_id = self
            .world_map_player()
            .and_then(|player| player_map(data, &player));
        result.set("player_map_id", player_map_id.map_or(-1, i64::from));
        write_player_and_breadcrumbs(&state, &mut result);
        write_map_contents(&state, &mut result);
        self.write_world_map_display(&mut result);
        result
    }
}

fn write_player_and_breadcrumbs(state: &WorldMapFrameState, result: &mut VarDictionary) {
    result.set(
        "player_uv",
        &state
            .player
            .as_ref()
            .map(|player| Vector2::new(player.x, player.y).to_variant())
            .unwrap_or_default(),
    );
    let names: PackedStringArray = state
        .breadcrumbs
        .iter()
        .map(|crumb| GString::from(crumb.name.as_str()))
        .collect();
    let ids: PackedInt64Array = state
        .breadcrumbs
        .iter()
        .map(|crumb| i64::from(crumb.map_id))
        .collect();
    result.set("breadcrumbs", &names);
    result.set("breadcrumb_ids", &ids);
}

fn write_map_contents(state: &WorldMapFrameState, result: &mut VarDictionary) {
    result.set(
        "highlight",
        state
            .highlight
            .as_ref()
            .map_or("", |highlight| highlight.name.as_str()),
    );
    result.set(
        "highlighted_quest",
        state.highlighted_quest.map_or(0, i64::from),
    );
    result.set("pin_count", state.pins.len() as i64);
    result.set("quest_area_count", state.quest_areas.len() as i64);
    let mut pins = VarArray::new();
    for pin in &state.pins {
        let mut entry = VarDictionary::new();
        entry.set("quest_id", pin.quest_id.map_or(0, i64::from));
        entry.set("type", format!("{:?}", pin.pin_type).as_str());
        entry.set("label", pin.label.as_str());
        entry.set("badge", pin.badge.as_str());
        entry.set("uv", Vector2::new(pin.x, pin.y));
        pins.push(&entry.to_variant());
    }
    result.set("pins", &pins);
    result.set("tile_count", state.tiles.len() as i64);
}

impl GameClient {
    fn write_world_map_display(&self, result: &mut VarDictionary) {
        result.set("missing_tiles", self.world_map.missing_tiles as i64);
        result.set("maximized", self.world_map.display.maximized);
        let layout = placed_map_layout(
            self.world_map_viewport(),
            self.world_map.position,
            self.world_map.display.maximized,
        );
        let [_, _, width, height] = layout.frame_rect();
        result.set("frame_size", Vector2::new(width, height));
        result.set("quest_details", self.world_map.display.quest_details);
    }
}
