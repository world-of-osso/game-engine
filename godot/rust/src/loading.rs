//! Native projection of actual streamed and attached world state into shared loading rules.

use std::collections::{BTreeMap, BTreeSet};

use game_engine_core::loading_readiness::{
    GlobalWmoState, LoadingInput, LoadingReadiness, TileState, evaluate_world_loading,
};

use crate::terrain::streaming::TerrainStreamState;

/// `local_visual_settled`: the local player's model is attached, or failed and was
/// reported; the loading screen does not show a world without it.
pub(crate) fn evaluate_native_loading(
    player_position: Option<(f32, f32)>,
    local_visual_settled: bool,
    stream: &TerrainStreamState,
    attached: &BTreeSet<(u32, u32)>,
    unbuildable: &BTreeMap<(u32, u32), String>,
    global_wmo: GlobalWmoState,
) -> LoadingReadiness {
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
    evaluate_world_loading(LoadingInput {
        local_player_ready: player_position.is_some() && local_visual_settled,
        map_ready: stream.map.as_ref().is_some_and(|map| !map.is_empty())
            && stream.wdt_path.is_some()
            && stream.map_error.is_none(),
        global_wmo,
        center_tile,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
                GlobalWmoState::None
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
            )
        };
        assert!(!loading(GlobalWmoState::Pending).complete);
        assert!(loading(GlobalWmoState::Spawned).complete);
        assert_eq!(
            loading(GlobalWmoState::Failed).status_text,
            "Terrain failed to load"
        );
    }
}
