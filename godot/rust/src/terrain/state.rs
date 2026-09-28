//! Read-only diagnostics distinguish parsed terrain assets from rendered world readiness.

use godot::prelude::*;

use super::streaming::{ParsedTileState, StreamedTerrain};

pub(crate) fn terrain_state(terrain: &StreamedTerrain) -> VarDictionary {
    let snapshot = terrain.state();
    let mut state = VarDictionary::new();
    state.set("map", snapshot.map.as_deref().unwrap_or(""));
    state.set("wdt_path", path_text(snapshot.wdt_path.as_deref()));
    state.set("wdt_flags", &optional_id(snapshot.wdt_flags));
    state.set("global_wmo_fdid", &optional_id(snapshot.global_wmo_fdid));
    state.set(
        "pending_count",
        (snapshot.pending_tiles.len() + usize::from(snapshot.pending_map)) as i64,
    );
    let tiles: Array<VarDictionary> = snapshot.parsed_tiles.iter().map(tile_state).collect();
    state.set("parsed_tiles", &tiles);
    let mut failures = PackedStringArray::new();
    if let Some(error) = snapshot.map_error {
        failures.push(&error);
    }
    for failure in snapshot.failures {
        failures.push(&format!("Tile {:?}: {}", failure.tile, failure.error));
    }
    state.set("failures", &failures);
    state
}

fn tile_state(tile: &ParsedTileState) -> VarDictionary {
    let mut state = VarDictionary::new();
    state.set("tile_y", tile.tile.0 as i64);
    state.set("tile_x", tile.tile.1 as i64);
    state.set("root_path", path_text(Some(&tile.root_path)));
    state.set("tex_path", path_text(tile.tex_path.as_deref()));
    state.set("obj_path", path_text(tile.obj_path.as_deref()));
    state.set("chunk_count", tile.root_chunks as i64);
    state.set("height_grid_count", tile.root_height_grids as i64);
    state.set("texture_layer_count", tile.tex_chunk_layers as i64);
    state.set("doodad_count", tile.obj_doodads as i64);
    state.set("wmo_count", tile.obj_wmos as i64);
    state
}

fn optional_id(value: Option<u32>) -> Variant {
    value
        .map(|value| (value as i64).to_variant())
        .unwrap_or_default()
}

fn path_text(path: Option<&std::path::Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_default()
}
