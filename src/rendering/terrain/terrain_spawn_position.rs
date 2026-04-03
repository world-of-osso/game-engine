use bevy::prelude::*;

use crate::asset::adt::{self};
use crate::asset::adt_format::adt_obj;
use crate::terrain_heightmap::sample_chunk_height;

pub(super) fn choose_safe_spawn_position(
    adt_data: &adt::AdtData,
    obj_data: Option<&adt_obj::AdtObjData>,
) -> Option<Vec3> {
    let tile_center = Vec2::new(adt_data.center_surface[0], adt_data.center_surface[2]);
    adt_data
        .height_grids
        .iter()
        .enumerate()
        .filter(|(i, _)| !chunk_has_water(adt_data, *i))
        .filter_map(|(_, grid)| {
            let center = chunk_center_position(grid)?;
            let relief = spawn_patch_relief(grid, center)?;
            let dist = Vec2::new(center.x, center.z).distance(tile_center) / adt::CHUNK_SIZE;
            let occupancy_penalty = spawn_occupancy_penalty(center, obj_data);
            Some((spawn_score(relief, dist, occupancy_penalty), center))
        })
        .min_by(|(score_a, _), (score_b, _)| score_a.total_cmp(score_b))
        .map(|(_, center)| center)
}

fn chunk_has_water(adt_data: &adt::AdtData, index: usize) -> bool {
    adt_data
        .water
        .as_ref()
        .and_then(|water| water.chunks.get(index))
        .is_some_and(|chunk| {
            chunk
                .layers
                .iter()
                .any(|layer| layer.width > 0 && layer.height > 0)
        })
}

fn chunk_center_position(grid: &adt::ChunkHeightGrid) -> Option<Vec3> {
    let x = grid.origin_x - adt::CHUNK_SIZE / 2.0;
    let z = grid.origin_z + adt::CHUNK_SIZE / 2.0;
    let y = sample_chunk_height(grid, x, z)?;
    Some(Vec3::new(x, y, z))
}

fn spawn_patch_relief(grid: &adt::ChunkHeightGrid, center: Vec3) -> Option<f32> {
    let sample_radius = adt::UNIT_SIZE;
    let mut min_height = f32::INFINITY;
    let mut max_height = f32::NEG_INFINITY;
    let mut sampled = 0usize;
    for (dx, dz) in [
        (0.0, 0.0),
        (-sample_radius, 0.0),
        (sample_radius, 0.0),
        (0.0, -sample_radius),
        (0.0, sample_radius),
    ] {
        let height = sample_chunk_height(grid, center.x + dx, center.z + dz)?;
        min_height = min_height.min(height);
        max_height = max_height.max(height);
        sampled += 1;
    }
    (sampled > 0).then_some(max_height - min_height)
}

fn spawn_occupancy_penalty(center: Vec3, obj_data: Option<&adt_obj::AdtObjData>) -> f32 {
    let Some(obj_data) = obj_data else {
        return 0.0;
    };
    let candidate = Vec2::new(center.x, center.z);
    let mut penalty = 0.0;

    for wmo in &obj_data.wmos {
        let distance = candidate.distance(world_position_2d(wmo.position));
        if distance < adt::CHUNK_SIZE * 0.75 {
            penalty += 1_000.0;
        }
    }

    let doodads_nearby = obj_data
        .doodads
        .iter()
        .filter(|doodad| {
            candidate.distance(world_position_2d(doodad.position)) < adt::UNIT_SIZE * 3.0
        })
        .count() as f32;
    penalty + doodads_nearby.min(12.0)
}

fn world_position_2d(wow_position: [f32; 3]) -> Vec2 {
    let [x, _, z] =
        crate::asset::m2::wow_to_bevy(wow_position[0], wow_position[1], wow_position[2]);
    Vec2::new(x, z)
}

fn spawn_score(relief: f32, dist_from_center_chunks: f32, occupancy_penalty: f32) -> f32 {
    relief * 10.0 + dist_from_center_chunks + occupancy_penalty
}
