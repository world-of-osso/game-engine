//! Read-only ordinary-client Rain of Fire objects. Replication owns lifetime;
//! the harness receives actual geometry, never reconstructed cast destinations.
use std::collections::HashMap;

use game_engine_network::replica::{Replica, Unit};
use godot::prelude::*;
use serde::Serialize;
use shared::components::{DynamicObjectInfo, Position, RAIN_OF_FIRE_SPELL_ID, UnitFactionTemplate};

use crate::faction_reaction::{FactionTemplateEntry, Reaction, reaction};

#[derive(Debug, Serialize)]
pub(crate) struct GroundAreaSnapshot {
    areas: Vec<GroundArea>,
}

#[derive(Debug, Serialize)]
struct GroundArea {
    id: u64,
    caster: u64,
    spell: u32,
    center: [f32; 3],
    radius: f32,
    hostile: bool,
}

fn unit_faction<'a>(
    replica: &Replica,
    unit: u64,
    templates: &'a HashMap<u32, FactionTemplateEntry>,
) -> Result<&'a FactionTemplateEntry, String> {
    let faction = replica
        .unit(unit)
        .and_then(|unit| unit.get::<UnitFactionTemplate>())
        .ok_or_else(|| format!("Ground area faction unavailable for unit {unit}"))?;
    templates
        .get(&faction.0)
        .ok_or_else(|| format!("Ground area faction template {} unavailable", faction.0))
}

fn project_area(
    replica: &Replica,
    unit: Unit,
    info: DynamicObjectInfo,
    viewer: &FactionTemplateEntry,
    templates: &HashMap<u32, FactionTemplateEntry>,
) -> Result<GroundArea, String> {
    let position = unit
        .get::<Position>()
        .ok_or_else(|| format!("Ground area {} position unavailable", unit.server_id))?;
    let center = [position.x, position.y, position.z];
    if center.iter().any(|value| !value.is_finite()) {
        return Err(format!(
            "Ground area {} position is non-finite",
            unit.server_id
        ));
    }
    if !info.radius.is_finite() || info.radius <= 0.0 {
        return Err(format!("Ground area {} radius unavailable", unit.server_id));
    }
    let caster = unit_faction(replica, info.caster, templates)?;
    Ok(GroundArea {
        id: unit.server_id,
        caster: info.caster,
        spell: info.spell_id,
        center,
        radius: info.radius,
        hostile: reaction(Some(caster), Some(viewer)) == Reaction::Hostile,
    })
}

fn collect_rainfire_areas(
    replica: &Replica,
    player: u64,
    templates: &HashMap<u32, FactionTemplateEntry>,
) -> Result<GroundAreaSnapshot, String> {
    let viewer = unit_faction(replica, player, templates)?;
    let mut areas = replica
        .units()
        .filter_map(|unit| unit.get::<DynamicObjectInfo>().map(|info| (unit, *info)))
        .filter(|(_, info)| info.spell_id == RAIN_OF_FIRE_SPELL_ID)
        .map(|(unit, info)| project_area(replica, unit, info, viewer, templates))
        .collect::<Result<Vec<_>, _>>()?;
    areas.sort_by_key(|area| area.id);
    Ok(GroundAreaSnapshot { areas })
}

impl crate::GameClient {
    pub(super) fn ground_areas_snapshot(&mut self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let result = self.read_ground_areas().and_then(|snapshot| {
            serde_json::to_string(&snapshot).map_err(|error| error.to_string())
        });
        match result {
            Ok(json) => state.set("state_json", json),
            Err(error) => state.set("error", error),
        }
        state
    }

    fn read_ground_areas(&mut self) -> Result<GroundAreaSnapshot, String> {
        let player = self
            .world
            .local_player_id()
            .ok_or("Ground areas require the received local player")?;
        let templates = self.nameplates.templates(&self.data_root)?;
        collect_rainfire_areas(&self.replica, player, templates)
    }
}

#[cfg(test)]
#[path = "rainfire_tests.rs"]
mod tests;
