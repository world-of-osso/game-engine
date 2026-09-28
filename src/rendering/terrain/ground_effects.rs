use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::rendering::db2_path::ensure_db2_path;
use game_engine::footstep_data::FootstepSurface;
use game_engine::ground_effect_data::{
    classify_surface_from_terrain_sound_name, parse_ground_effect_entries,
    parse_terrain_type_sounds,
};

const GROUND_EFFECT_TEXTURE_DB2_FDID: u32 = 1_308_499;
const TERRAIN_TYPE_SOUNDS_DB2_FDID: u32 = 1_284_822;
const DETERMINISTIC_SEED_MIX_HIGH: u32 = 0x045D;
const DETERMINISTIC_SEED_MIX_LOW: u32 = 0x9F3B;
const DETERMINISTIC_SEED_MIX_CONSTANT: u32 =
    (DETERMINISTIC_SEED_MIX_HIGH << 16) | DETERMINISTIC_SEED_MIX_LOW;
const RANDOM_FRACTION_MASK: u32 = 0xFFFF;

static GROUND_EFFECTS: OnceLock<HashMap<u32, GroundEffectEntry>> = OnceLock::new();
static TERRAIN_SOUND_SURFACES: OnceLock<HashMap<u8, FootstepSurface>> = OnceLock::new();

pub use game_engine::ground_effect_data::GroundEffectEntry;

/// A ground clutter placement for a single chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundClutterPlacement {
    /// Position in world space.
    pub position: [f32; 3],
    /// Scale randomization (0.8–1.2 typical).
    pub scale: f32,
    /// Y-axis rotation in radians.
    pub rotation: f32,
    /// M2 model FDID for this clutter instance.
    pub model_fdid: u32,
}

/// Generate ground clutter placements for a chunk based on its effect density.
///
/// Uses a deterministic pseudo-random scatter based on chunk coordinates,
/// so the same chunk always produces the same placements.
pub fn generate_chunk_clutter(
    chunk_x: u32,
    chunk_y: u32,
    effect: &GroundEffectEntry,
    model_fdids: &[u32],
    chunk_origin: [f32; 3],
    chunk_size: f32,
) -> Vec<GroundClutterPlacement> {
    if model_fdids.is_empty() || effect.density == 0 {
        return Vec::new();
    }
    let count = effect.density.min(64) as usize;
    let mut placements = Vec::with_capacity(count);
    for i in 0..count {
        let seed = deterministic_seed(chunk_x, chunk_y, i as u32);
        let fx = pseudo_random_f32(seed, 0);
        let fz = pseudo_random_f32(seed, 1);
        let model_index = (seed as usize) % model_fdids.len();
        placements.push(GroundClutterPlacement {
            position: [
                chunk_origin[0] + fx * chunk_size,
                chunk_origin[1],
                chunk_origin[2] + fz * chunk_size,
            ],
            scale: 0.8 + pseudo_random_f32(seed, 2) * 0.4,
            rotation: pseudo_random_f32(seed, 3) * std::f32::consts::TAU,
            model_fdid: model_fdids[model_index],
        });
    }
    placements
}

fn deterministic_seed(chunk_x: u32, chunk_y: u32, index: u32) -> u32 {
    let mut h = chunk_x.wrapping_mul(73856093)
        ^ chunk_y.wrapping_mul(19349663)
        ^ index.wrapping_mul(83492791);
    h ^= h >> 16;
    h = h.wrapping_mul(DETERMINISTIC_SEED_MIX_CONSTANT);
    h ^= h >> 16;
    h
}

fn pseudo_random_f32(seed: u32, channel: u32) -> f32 {
    let mixed = seed.wrapping_add(channel.wrapping_mul(2654435761));
    let mixed = mixed ^ (mixed >> 13);
    (mixed & RANDOM_FRACTION_MASK) as f32 / 65535.0
}

pub fn resolve_ground_effect(effect_id: u32) -> Option<GroundEffectEntry> {
    cached_ground_effects().get(&effect_id).copied()
}

pub fn resolve_ground_effect_surface(effect_id: u32) -> Option<FootstepSurface> {
    let effect = resolve_ground_effect(effect_id)?;
    cached_terrain_sound_surfaces()
        .get(&effect.terrain_sound_id)
        .copied()
}

fn cached_ground_effects() -> &'static HashMap<u32, GroundEffectEntry> {
    GROUND_EFFECTS.get_or_init(load_ground_effects)
}

fn cached_terrain_sound_surfaces() -> &'static HashMap<u8, FootstepSurface> {
    TERRAIN_SOUND_SURFACES.get_or_init(load_terrain_sound_surfaces)
}

fn load_ground_effects() -> HashMap<u32, GroundEffectEntry> {
    let Some(path) = ensure_db2_path(
        GROUND_EFFECT_TEXTURE_DB2_FDID,
        Path::new("data/dbfilesclient/1308499.db2"),
    ) else {
        return HashMap::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return HashMap::new();
    };
    parse_ground_effect_entries(&bytes).unwrap_or_default()
}

fn load_terrain_sound_surfaces() -> HashMap<u8, FootstepSurface> {
    let Some(path) = ensure_db2_path(
        TERRAIN_TYPE_SOUNDS_DB2_FDID,
        Path::new("data/dbfilesclient/1284822.db2"),
    ) else {
        return HashMap::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return HashMap::new();
    };
    parse_terrain_type_sounds(&bytes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(terrain_sound_id, name)| {
            classify_surface_from_terrain_sound_name(&name)
                .map(|surface| (terrain_sound_id, surface))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Ground clutter placement ---

    fn sample_effect(density: u32) -> GroundEffectEntry {
        GroundEffectEntry {
            effect_id: 1,
            density,
            terrain_sound_id: 0,
        }
    }

    #[test]
    fn generate_clutter_respects_density() {
        let effect = sample_effect(10);
        let models = vec![100, 200];
        let placements = generate_chunk_clutter(0, 0, &effect, &models, [0.0; 3], 33.0);
        assert_eq!(placements.len(), 10);
    }

    #[test]
    fn generate_clutter_zero_density_empty() {
        let effect = sample_effect(0);
        let models = vec![100];
        let placements = generate_chunk_clutter(0, 0, &effect, &models, [0.0; 3], 33.0);
        assert!(placements.is_empty());
    }

    #[test]
    fn generate_clutter_no_models_empty() {
        let effect = sample_effect(10);
        let placements = generate_chunk_clutter(0, 0, &effect, &[], [0.0; 3], 33.0);
        assert!(placements.is_empty());
    }

    #[test]
    fn generate_clutter_capped_at_64() {
        let effect = sample_effect(200);
        let models = vec![100];
        let placements = generate_chunk_clutter(0, 0, &effect, &models, [0.0; 3], 33.0);
        assert_eq!(placements.len(), 64);
    }

    #[test]
    fn generate_clutter_positions_within_chunk() {
        let effect = sample_effect(20);
        let models = vec![100];
        let origin = [100.0, 50.0, 200.0];
        let size = 33.0;
        let placements = generate_chunk_clutter(5, 3, &effect, &models, origin, size);
        for p in &placements {
            assert!(p.position[0] >= origin[0] && p.position[0] <= origin[0] + size);
            assert_eq!(p.position[1], origin[1]); // Y is from origin
            assert!(p.position[2] >= origin[2] && p.position[2] <= origin[2] + size);
        }
    }

    #[test]
    fn generate_clutter_scale_in_range() {
        let effect = sample_effect(20);
        let models = vec![100];
        let placements = generate_chunk_clutter(0, 0, &effect, &models, [0.0; 3], 33.0);
        for p in &placements {
            assert!(
                p.scale >= 0.8 && p.scale <= 1.2,
                "scale {} out of range",
                p.scale
            );
        }
    }

    #[test]
    fn generate_clutter_deterministic() {
        let effect = sample_effect(10);
        let models = vec![100, 200, 300];
        let a = generate_chunk_clutter(5, 3, &effect, &models, [0.0; 3], 33.0);
        let b = generate_chunk_clutter(5, 3, &effect, &models, [0.0; 3], 33.0);
        assert_eq!(a, b);
    }

    #[test]
    fn generate_clutter_different_chunks_differ() {
        let effect = sample_effect(10);
        let models = vec![100];
        let a = generate_chunk_clutter(0, 0, &effect, &models, [0.0; 3], 33.0);
        let b = generate_chunk_clutter(1, 0, &effect, &models, [0.0; 3], 33.0);
        assert_ne!(a[0].position, b[0].position);
    }

    #[test]
    fn generate_clutter_uses_all_models() {
        let effect = sample_effect(30);
        let models = vec![100, 200, 300];
        let placements = generate_chunk_clutter(7, 7, &effect, &models, [0.0; 3], 33.0);
        let used: std::collections::HashSet<u32> =
            placements.iter().map(|p| p.model_fdid).collect();
        // With 30 placements and 3 models, all should be used
        assert!(used.len() >= 2, "should use multiple models, got {used:?}");
    }
}
