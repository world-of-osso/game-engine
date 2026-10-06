//! Native projection of actual streamed and attached world state into shared loading rules.

use std::collections::{BTreeMap, BTreeSet};

use game_engine_core::loading_readiness::{
    GlobalWmoState, LoadingInput, TileState, evaluate_world_loading,
};

use crate::terrain::{object_progress::ENTRY_RADIUS, streaming::TerrainStreamState};

/// Terrain tiles covering the local entry bubble (a conservative X/Z square).
/// The stream already requests the center and its eight neighbours.
pub(crate) fn nearby_tiles(player: glam::Vec3) -> BTreeSet<(u32, u32)> {
    let tile_at = game_engine_core::terrain_height_data::bevy_to_tile_coords;
    let (min_row, max_col) = tile_at(player.x - ENTRY_RADIUS, player.z - ENTRY_RADIUS);
    let (max_row, min_col) = tile_at(player.x + ENTRY_RADIUS, player.z + ENTRY_RADIUS);
    (min_row..=max_row)
        .flat_map(|row| (min_col..=max_col).map(move |col| (row, col)))
        .collect()
}

/// Local entry bubble: nearby placements done (attached or failed and reported),
/// and nearby WMO collision groups not built yet. Distant work is not included.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TileObjects {
    pub done: usize,
    pub total: usize,
    pub collision_pending: usize,
}

impl TileObjects {
    fn complete(self) -> bool {
        self.done >= self.total && self.collision_pending == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeLoading {
    pub complete: bool,
    pub progress_percent: u8,
    pub status_text: String,
}

/// `local_visual_settled`: the local player's model is attached, or failed and was
/// reported; the loading screen does not show a world without it. Once the shared rules
/// accept the center tile's terrain, nearby `objects` must be done too; `None`
/// until terrain covering the bubble is attached and its placements are queued. A WMO-only map has no tiles and finishes with its global WMO.
pub(crate) fn evaluate_native_loading(
    player_position: Option<(f32, f32)>,
    local_visual_settled: bool,
    stream: &TerrainStreamState,
    attached: &BTreeSet<(u32, u32)>,
    unbuildable: &BTreeMap<(u32, u32), String>,
    global_wmo: GlobalWmoState,
    objects: Option<TileObjects>,
) -> NativeLoading {
    let center = player_position
        .map(|(x, z)| game_engine_core::terrain_height_data::bevy_to_tile_coords(x, z));
    let center_tile = match center {
        Some(tile) if attached.contains(&tile) => TileState::Loaded,
        Some(tile)
            if unbuildable.contains_key(&tile)
                || stream.failures.iter().any(|failure| failure.tile == tile) =>
        {
            TileState::Failed
        }
        Some(tile)
            if stream.pending_tiles.contains(&tile)
                || stream.parsed_tiles.iter().any(|parsed| parsed.tile == tile) =>
        {
            TileState::Pending
        }
        _ => TileState::NotRequested,
    };
    let terrain = evaluate_world_loading(LoadingInput {
        local_player_ready: player_position.is_some() && local_visual_settled,
        map_ready: stream.map.as_ref().is_some_and(|map| !map.is_empty())
            && stream.wdt_path.is_some()
            && stream.map_error.is_none(),
        global_wmo,
        center_tile,
    });
    let terrain = NativeLoading {
        complete: terrain.complete,
        progress_percent: terrain.progress_percent,
        status_text: terrain.status_text.to_owned(),
    };
    if !terrain.complete || global_wmo != GlobalWmoState::None {
        return terrain;
    }
    match objects {
        Some(objects) if objects.complete() => terrain,
        Some(TileObjects { done, total, .. }) => NativeLoading {
            complete: false,
            progress_percent: (86 + 13 * done / total.max(1)) as u8,
            status_text: format!("Loading objects {done}/{total}..."),
        },
        None => NativeLoading {
            complete: false,
            progress_percent: 86,
            status_text: "Loading objects...".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_bubble_covers_neighbor_when_player_is_near_tile_edge() {
        assert_eq!(
            nearby_tiles(glam::Vec3::new(-8928.0, 99.295, -606.0)),
            BTreeSet::from([(30, 48), (31, 48)])
        );
        // Centered within the tile: no unrelated neighbour is a prerequisite.
        assert_eq!(
            nearby_tiles(glam::Vec3::new(-8800.0, 99.295, -800.0)),
            BTreeSet::from([(30, 48)])
        );
    }

    /// No object waits: the terrain rules alone decide.
    const DONE: Option<TileObjects> = Some(TileObjects {
        done: 0,
        total: 0,
        collision_pending: 0,
    });

    fn stream() -> TerrainStreamState {
        TerrainStreamState {
            map: Some("azeroth".into()),
            wdt_path: Some("azeroth.wdt".into()),
            wdt_flags: Some(0),
            global_wmo_fdid: None,
            global_wmo_present: false,
            pending_map: false,
            pending_tiles: vec![(32, 48)],
            parsed_tiles: vec![],
            failures: vec![],
            map_error: None,
        }
    }

    #[test]
    fn selected_player_and_successful_map_are_both_required() {
        let attached = BTreeSet::from([(32, 48)]);
        assert_eq!(
            evaluate_native_loading(
                None,
                true,
                &stream(),
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .progress_percent,
            35
        );
        let mut map = stream();
        map.map = None;
        assert!(
            !evaluate_native_loading(
                Some((-8949.0, 0.0)),
                true,
                &map,
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .complete
        );
        map.map = Some("azeroth".into());
        map.wdt_path = None;
        assert!(
            !evaluate_native_loading(
                Some((-8949.0, 0.0)),
                true,
                &map,
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .complete
        );
        map.wdt_path = Some("azeroth.wdt".into());
        map.map_error = Some("unreadable WDT".into());
        assert!(
            !evaluate_native_loading(
                Some((-8949.0, 0.0)),
                true,
                &map,
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .complete
        );
    }

    #[test]
    fn current_center_requires_attachment_not_parsing_or_neighbor() {
        let mut map = stream();
        // Position comes from the selected unit, not the initial LoadTerrain tile.
        let position = (-8949.0, 0.0);
        map.pending_tiles.clear();
        map.parsed_tiles
            .push(crate::terrain::streaming::ParsedTileState {
                tile: (32, 48),
                root_path: "azeroth_32_48.adt".into(),
                tex_path: None,
                obj_path: None,
                root_chunks: 256,
                root_height_grids: 256,
                tex_chunk_layers: 256,
                obj_doodads: 0,
                obj_wmos: 0,
            });
        let neighbor = BTreeSet::from([(32, 47)]);
        assert!(
            !evaluate_native_loading(
                Some(position),
                true,
                &map,
                &neighbor,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .complete
        );
        let attached = BTreeSet::from([(32, 48)]);
        // The loaded center tile does not finish while the player's model is loading.
        let character = evaluate_native_loading(
            Some(position),
            false,
            &map,
            &attached,
            &BTreeMap::new(),
            GlobalWmoState::None,
            DONE,
        );
        assert!(!character.complete);
        assert_eq!(character.status_text, "Initializing character...");
        assert!(
            evaluate_native_loading(
                Some(position),
                true,
                &map,
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .complete
        );
        // A moved player cannot finish based on their old center tile.
        assert!(
            !evaluate_native_loading(
                Some((0.0, 0.0)),
                true,
                &map,
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .complete
        );
        map.failures.push(crate::terrain::streaming::TileFailure {
            tile: (32, 32),
            error: "invalid ADT".into(),
        });
        assert_eq!(
            evaluate_native_loading(
                Some((0.0, 0.0)),
                true,
                &map,
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                DONE,
            )
            .status_text,
            "Terrain failed to load"
        );
    }

    #[test]
    fn center_tile_without_buildable_terrain_fails_instead_of_waiting() {
        let mut map = stream();
        map.pending_tiles.clear();
        let position = (-8949.0, 0.0);
        let unbuildable = BTreeMap::from([((32, 48), "Terrain (32, 48): bad chunk".to_string())]);
        let readiness = evaluate_native_loading(
            Some(position),
            true,
            &map,
            &BTreeSet::new(),
            &unbuildable,
            GlobalWmoState::None,
            DONE,
        );
        assert!(!readiness.complete);
        assert_eq!(readiness.status_text, "Terrain failed to load");
        // A neighbour that cannot be built does not block the player's own tile.
        let neighbour = BTreeMap::from([((32, 47), "Terrain (32, 47): bad chunk".to_string())]);
        let attached = BTreeSet::from([(32, 48)]);
        assert!(
            evaluate_native_loading(
                Some(position),
                true,
                &map,
                &attached,
                &neighbour,
                GlobalWmoState::None,
                DONE,
            )
            .complete
        );
    }

    #[test]
    fn global_wmo_map_completes_once_its_wmo_spawned_without_adt_tiles() {
        let mut map = stream();
        map.map = Some("stormwindjail".into());
        map.global_wmo_present = true;
        map.pending_tiles.clear();
        let loading = |global_wmo| {
            evaluate_native_loading(
                Some((103.0, -76.0)),
                true,
                &map,
                &BTreeSet::new(),
                &BTreeMap::new(),
                global_wmo,
                DONE,
            )
        };
        assert!(!loading(GlobalWmoState::Pending).complete);
        assert!(loading(GlobalWmoState::Spawned).complete);
        assert_eq!(
            loading(GlobalWmoState::Failed).status_text,
            "Terrain failed to load"
        );
    }

    /// Northshire: 7399 placements on azeroth_32_48 with the player on it.
    #[test]
    fn center_tile_objects_hold_loading_until_attached_or_failed() {
        let mut map = stream();
        map.pending_tiles.clear();
        let attached = BTreeSet::from([(32, 48)]);
        let loading = |objects| {
            evaluate_native_loading(
                Some((-8949.0, 0.0)),
                true,
                &map,
                &attached,
                &BTreeMap::new(),
                GlobalWmoState::None,
                objects,
            )
        };
        let queued = |done, collision_pending| {
            Some(TileObjects {
                done,
                total: 7399,
                collision_pending,
            })
        };
        let unqueued = loading(None);
        assert!(!unqueued.complete);
        assert_eq!(unqueued.status_text, "Loading objects...");
        let streaming = loading(queued(710, 0));
        assert!(!streaming.complete);
        assert_eq!(streaming.status_text, "Loading objects 710/7399...");
        assert_eq!(streaming.progress_percent, 87);
        // Every placement attached or failed, but a WMO group's collision is not built.
        assert!(!loading(queued(7399, 3)).complete);
        let ready = loading(queued(7399, 0));
        assert!(ready.complete);
        assert_eq!(ready.status_text, "Entering world...");
    }
}
