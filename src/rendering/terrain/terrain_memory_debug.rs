use std::path::Path;
use std::sync::{Mutex, OnceLock};

use bevy::image::Image;
use bevy::mesh::Mesh;
use bevy::prelude::{Assets, StandardMaterial};

use crate::m2_effect_material::M2EffectMaterial;
pub use crate::process_memory_status::ProcessMemoryKb;
pub use crate::process_memory_status::current_process_memory_kb;
use crate::status_asset_stats::{self, AssetStoreStats};
use crate::terrain_material::TerrainMaterial;
use crate::water_material::WaterMaterial;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ProbeSnapshot {
    process: ProcessMemoryKb,
    assets: AssetStoreStats,
}

static LAST_SNAPSHOT: OnceLock<Mutex<Option<ProbeSnapshot>>> = OnceLock::new();

pub fn log_tile_spawn_stats(
    tile_y: u32,
    tile_x: u32,
    adt_path: &Path,
    images: &Assets<Image>,
    meshes: &Assets<Mesh>,
    standard_materials: &Assets<StandardMaterial>,
    terrain_materials: &Assets<TerrainMaterial>,
    water_materials: &Assets<WaterMaterial>,
    m2_effect_materials: &Assets<M2EffectMaterial>,
) {
    let current = ProbeSnapshot {
        process: current_process_memory_kb(),
        assets: status_asset_stats::collect_asset_store_stats(
            images,
            meshes,
            standard_materials,
            terrain_materials,
            water_materials,
            m2_effect_materials,
        ),
    };
    let mut last = LAST_SNAPSHOT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    eprintln!(
        "Tile mem ({tile_y},{tile_x}) {} rss={}MiB anon={}MiB img={} ({}) mesh={} ({}) mats std/terrain/water/effect={}/{}/{}/{} delta_rss={}MiB delta_img={} delta_mesh={}",
        adt_path.display(),
        mib(current.process.rss_kb),
        mib(current.process.anon_kb),
        current.assets.image_assets,
        bytes_mib(current.assets.image_asset_cpu_bytes),
        current.assets.mesh_assets,
        bytes_mib(current.assets.mesh_asset_est_cpu_bytes),
        current.assets.standard_material_assets,
        current.assets.terrain_material_assets,
        current.assets.water_material_assets,
        current.assets.m2_effect_material_assets,
        delta_mib(
            last.as_ref().map(|snapshot| snapshot.process.rss_kb),
            current.process.rss_kb
        ),
        delta_bytes_mib(
            last.as_ref()
                .map(|snapshot| snapshot.assets.image_asset_cpu_bytes),
            current.assets.image_asset_cpu_bytes,
        ),
        delta_bytes_mib(
            last.as_ref()
                .map(|snapshot| snapshot.assets.mesh_asset_est_cpu_bytes),
            current.assets.mesh_asset_est_cpu_bytes,
        ),
    );
    *last = Some(current);
}

fn mib(kb: u64) -> u64 {
    kb / 1024
}

fn bytes_mib(bytes: u64) -> String {
    format!("{}MiB", bytes / (1024 * 1024))
}

fn delta_mib(previous_kb: Option<u64>, current_kb: u64) -> i64 {
    let previous_kb = previous_kb.unwrap_or(0);
    (current_kb as i64 - previous_kb as i64) / 1024
}

fn delta_bytes_mib(previous_bytes: Option<u64>, current_bytes: u64) -> String {
    let previous = previous_bytes.unwrap_or(0) as i64;
    let current = current_bytes as i64;
    format!("{:+}MiB", (current - previous) / (1024 * 1024))
}
