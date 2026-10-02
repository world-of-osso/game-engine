//! Creature vignettes the server shows near the player (`C_VignetteInfo.GetVignettes`):
//! every replicated unit carrying `UnitVignette`, with its `Vignette` DB2 row. The
//! minimap draws each `onMinimap` one; the world map's `VignetteDataProvider` each
//! `onWorldMap` one.

use std::collections::HashMap;

use game_engine_core::minimap_data::{MinimapView, VignetteRow};
use game_engine_network::replica::Replica;
use game_engine_ui_model::minimap::{BlipKind, MinimapBlip};
use game_engine_ui_model::world_map_view_data::{MapVignette, engine_to_world};
use shared::components::{CreatureClassification, Position, UnitVignette};

pub(crate) struct Sighting<'a> {
    pub(crate) unit: u64,
    pub(crate) row: &'a VignetteRow,
    /// `VignetteKillElite` (`MAP_LEGEND_RAREELITE`, MapLegendFrame.lua) for elite
    /// classifications, else `VignetteKill` (`MAP_LEGEND_RARE`).
    pub(crate) elite: bool,
    /// Engine position.
    pub(crate) position: Position,
}

/// Units with a vignette and a position, by unit; a vignette ID missing from
/// `Vignette.csv` is an error (the server only sends `Vignette` rows).
pub(crate) fn sightings<'a>(
    replica: &Replica,
    rows: &'a HashMap<u32, VignetteRow>,
) -> Result<Vec<Sighting<'a>>, String> {
    let mut sightings = Vec::new();
    for unit in replica.units() {
        let (Some(UnitVignette(id)), Some(position)) =
            (unit.get::<UnitVignette>(), unit.get::<Position>())
        else {
            continue;
        };
        let row = rows.get(id).ok_or_else(|| {
            format!(
                "Vignette {id} of unit {} is not in Vignette.csv",
                unit.server_id
            )
        })?;
        let elite = matches!(
            unit.get::<CreatureClassification>(),
            Some(
                CreatureClassification::Elite
                    | CreatureClassification::RareElite
                    | CreatureClassification::WorldBoss
            )
        );
        sightings.push(Sighting {
            unit: unit.server_id,
            row,
            elite,
            position: *position,
        });
    }
    sightings.sort_by_key(|sighting| sighting.unit);
    Ok(sightings)
}

/// Minimap blips of the `onMinimap` vignettes inside `view`.
pub(crate) fn minimap_blips(view: &MinimapView, sightings: &[Sighting]) -> Vec<MinimapBlip> {
    sightings
        .iter()
        .filter(|sighting| sighting.row.on_minimap())
        .filter_map(|sighting| {
            let offset = view.blip_offset([sighting.position.x, sighting.position.z])?;
            Some(MinimapBlip {
                unit: sighting.unit,
                kind: BlipKind::Vignette {
                    elite: sighting.elite,
                },
                offset,
            })
        })
        .collect()
}

/// The `onWorldMap` vignettes on `map_id` (the player's map) for the world map.
pub(crate) fn map_vignettes(map_id: u32, sightings: &[Sighting]) -> Vec<MapVignette> {
    sightings
        .iter()
        .filter(|sighting| sighting.row.on_world_map())
        .map(|sighting| MapVignette {
            name: sighting.row.name.clone(),
            elite: sighting.elite,
            hide_on_continent_maps: sighting.row.hide_on_continent_maps(),
            map_id,
            position: engine_to_world([
                sighting.position.x,
                sighting.position.y,
                sighting.position.z,
            ]),
        })
        .collect()
}

#[cfg(test)]
#[path = "vignettes_tests.rs"]
mod tests;
