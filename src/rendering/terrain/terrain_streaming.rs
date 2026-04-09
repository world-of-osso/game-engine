use bevy::prelude::*;

use super::{
    AdtManager, ParsedTile, SpawnRefs, TerrainHeightmap, TileLoadResult, log_adt_spawn,
    parse_tile_background, resolve_tile_path, spawn_parsed_tile, tile_lod_for_distance,
};

pub(super) fn report_initial_world_load_complete(mut adt_manager: ResMut<AdtManager>) {
    if !crate::terrain_load_progress::should_report_initial_world_load(&adt_manager) {
        return;
    }
    let desired_tiles = crate::terrain_load_progress::initial_desired_tiles(&adt_manager);
    let (loaded, failed, pending) =
        crate::terrain_load_progress::count_initial_tile_progress(&adt_manager, &desired_tiles);
    if pending != 0 || loaded + failed != desired_tiles.len() {
        return;
    }
    info!(
        "Initial world load complete: map={} initial_tile=({}, {}) desired_tiles={} loaded={} failed={}",
        adt_manager.map_name,
        adt_manager.initial_tile.0,
        adt_manager.initial_tile.1,
        desired_tiles.len(),
        loaded,
        failed,
    );
    adt_manager.initial_load_reported = true;
}

pub(super) fn handle_tile_result(
    refs: &mut SpawnRefs,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    result: TileLoadResult,
) {
    match result {
        TileLoadResult::Success(parsed) => {
            handle_tile_success(refs, adt_manager, heightmap, parsed);
        }
        TileLoadResult::Failed {
            map_name,
            tile_y,
            tile_x,
            error,
        } => {
            if map_name != adt_manager.map_name {
                debug!(
                    "Ignoring stale terrain load failure for map {} tile ({}, {}) while {} is active",
                    map_name, tile_y, tile_x, adt_manager.map_name
                );
                return;
            }
            adt_manager.pending.remove(&(tile_y, tile_x));
            adt_manager.failed.insert((tile_y, tile_x));
            eprintln!("Cannot load ADT tile ({tile_y}, {tile_x}): {error}");
        }
    }
}

fn handle_tile_success(
    refs: &mut SpawnRefs,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    parsed: Box<ParsedTile>,
) {
    if parsed.map_name != adt_manager.map_name {
        debug!(
            "Ignoring stale terrain tile for map {} tile ({}, {}) while {} is active",
            parsed.map_name, parsed.tile_y, parsed.tile_x, adt_manager.map_name
        );
        return;
    }
    let key = (parsed.tile_y, parsed.tile_x);
    adt_manager.pending.remove(&key);
    register_heightmap_tile(heightmap, &parsed);
    record_loaded_tile_entities(refs, adt_manager, heightmap, key, &parsed);
}

fn register_heightmap_tile(heightmap: &mut TerrainHeightmap, parsed: &ParsedTile) {
    eprintln!(
        "handle_tile_success before register_tile ({}, {}) {}",
        parsed.tile_y,
        parsed.tile_x,
        parsed.adt_path.display()
    );
    heightmap.register_tile(
        parsed.tile_y,
        parsed.tile_x,
        &parsed.adt_data,
        parsed.tex_data.as_ref(),
    );
    eprintln!(
        "handle_tile_success after register_tile ({}, {}) {}",
        parsed.tile_y,
        parsed.tile_x,
        parsed.adt_path.display()
    );
}

fn record_loaded_tile_entities(
    refs: &mut SpawnRefs,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    key: (u32, u32),
    parsed: &ParsedTile,
) {
    let (root, doodad_entities) = spawn_parsed_tile(refs, heightmap, parsed);
    adt_manager.loaded.insert(key, root);
    adt_manager.tile_lod.insert(key, parsed.lod);
    adt_manager
        .tile_doodad_entities
        .insert(key, doodad_entities);
    log_adt_spawn(&parsed.adt_data, &parsed.adt_path);
    log_tile_memory_stats(refs, parsed);
}

fn log_tile_memory_stats(refs: &SpawnRefs, parsed: &ParsedTile) {
    crate::terrain_memory_debug::log_tile_spawn_stats(
        parsed.tile_y,
        parsed.tile_x,
        &parsed.adt_path,
        refs.images,
        refs.meshes,
        refs.materials,
        refs.terrain_materials,
        refs.water_materials,
        refs.effect_materials,
    );
}

pub(super) fn compute_desired_tiles(center_y: u32, center_x: u32, radius: u32) -> Vec<(u32, u32)> {
    let r = radius as i32;
    (-r..=r)
        .flat_map(|dy| {
            (-r..=r).filter_map(move |dx| {
                let ty = center_y as i32 + dy;
                let tx = center_x as i32 + dx;
                ((0..64).contains(&ty) && (0..64).contains(&tx)).then_some((ty as u32, tx as u32))
            })
        })
        .collect()
}

pub(super) fn unload_distant_tiles(
    commands: &mut Commands,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    desired: &[(u32, u32)],
) {
    let to_remove: Vec<(u32, u32)> = adt_manager
        .loaded
        .keys()
        .filter(|k| !desired.contains(k))
        .copied()
        .collect();

    for key in to_remove {
        if let Some(root) = adt_manager.loaded.remove(&key) {
            commands.entity(root).despawn();
        }
        adt_manager.tile_lod.remove(&key);
        crate::terrain_lod::despawn_tile_doodad_entities(commands, adt_manager, key);
        heightmap.remove_tile(key.0, key.1);
        eprintln!("Unloaded ADT tile ({}, {})", key.0, key.1);
    }
}

pub(super) fn dispatch_tile_loads(
    adt_manager: &mut AdtManager,
    desired: &[(u32, u32)],
    center_y: u32,
    center_x: u32,
) {
    for &(ty, tx) in desired {
        dispatch_single_tile(adt_manager, ty, tx, center_y, center_x);
    }
    let requested: Vec<_> = adt_manager.server_requested.drain().collect();
    for (ty, tx) in requested {
        dispatch_single_tile(adt_manager, ty, tx, center_y, center_x);
    }
}

fn dispatch_single_tile(
    adt_manager: &mut AdtManager,
    ty: u32,
    tx: u32,
    center_y: u32,
    center_x: u32,
) {
    if adt_manager.loaded.contains_key(&(ty, tx)) {
        return;
    }
    if adt_manager.failed.contains(&(ty, tx)) {
        return;
    }
    if adt_manager.pending.contains(&(ty, tx)) {
        return;
    }
    if adt_manager.pending.len() >= crate::terrain_load_limits::max_pending_tile_loads() {
        return;
    }

    let path = match resolve_tile_path(&adt_manager.map_name, ty, tx) {
        Ok(p) => p,
        Err(e) => {
            adt_manager.failed.insert((ty, tx));
            eprintln!("Cannot load ADT tile ({ty}, {tx}): {e}");
            return;
        }
    };

    let lod = tile_lod_for_distance(ty, tx, center_y, center_x);
    adt_manager.pending.insert((ty, tx));
    let tx_chan = adt_manager.tile_tx.clone();
    let map_name = adt_manager.map_name.clone();
    let thread_name = format!("adt-load-{ty}-{tx}");
    let spawn_result = std::thread::Builder::new()
        .name(thread_name)
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            tx_chan
                .send(parse_tile_background(map_name, ty, tx, path, lod))
                .ok();
        });
    if let Err(err) = spawn_result {
        adt_manager.pending.remove(&(ty, tx));
        eprintln!("Cannot spawn ADT loader thread ({ty}, {tx}): {err}");
    }
}
