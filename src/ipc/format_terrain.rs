use crate::status::TerrainStatusSnapshot;

pub(super) fn format_terrain_status(snapshot: &TerrainStatusSnapshot) -> String {
    format!(
        "map_name: {}
initial_tile: {},{}
load_radius: {}
loaded_tiles: {}
pending_tiles: {}
failed_tiles: {}
server_requested_tiles: {}
heightmap_tiles: {}
process_rss_kb: {}
process_anon_kb: {}
process_data_kb: {}
m2_model_cache_entries: {}
m2_model_cache_est_cpu_bytes: {}
composited_texture_cache_entries: {}
composited_texture_cache_est_cpu_bytes: {}
image_assets: {}
image_asset_cpu_bytes: {}
mesh_assets: {}
mesh_asset_est_cpu_bytes: {}
standard_material_assets: {}
terrain_material_assets: {}
water_material_assets: {}
m2_effect_material_assets: {}",
        map_name(snapshot),
        snapshot.initial_tile.0,
        snapshot.initial_tile.1,
        snapshot.load_radius,
        snapshot.loaded_tiles,
        snapshot.pending_tiles,
        snapshot.failed_tiles,
        snapshot.server_requested_tiles,
        snapshot.heightmap_tiles,
        snapshot.process_rss_kb,
        snapshot.process_anon_kb,
        snapshot.process_data_kb,
        snapshot.m2_model_cache_entries,
        snapshot.m2_model_cache_est_cpu_bytes,
        snapshot.composited_texture_cache_entries,
        snapshot.composited_texture_cache_est_cpu_bytes,
        snapshot.image_assets,
        snapshot.image_asset_cpu_bytes,
        snapshot.mesh_assets,
        snapshot.mesh_asset_est_cpu_bytes,
        snapshot.standard_material_assets,
        snapshot.terrain_material_assets,
        snapshot.water_material_assets,
        snapshot.m2_effect_material_assets,
    )
}

fn map_name(snapshot: &TerrainStatusSnapshot) -> &str {
    if snapshot.map_name.is_empty() {
        "-"
    } else {
        &snapshot.map_name
    }
}
