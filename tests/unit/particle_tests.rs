use bevy::asset::Assets;
use bevy::ecs::system::RunSystemOnce;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::{
    App, Entity, GlobalTransform, Image, Mesh, Quat, StandardMaterial, Time, Transform, Update,
    Vec3,
};
use bevy_hanabi::{AlphaMode, Attribute, EffectProperties, ExprWriter, SimulationSpace, Value};
use std::path::Path;
use std::time::Instant;

use super::effect_builder::{
    FlipbookSpriteMode, PositionInitModifier, active_cell_track, build_effect_asset,
    build_effect_asset_with_mode, build_expr_modifiers, build_position_modifier,
    child_emitter_event_count, emitter_alpha_mode, emitter_spawn_radius, flipbook_sprite_mode,
    gravity_accel_bevy, has_authored_spin, has_authored_wind, lifetime_range, orient_mode,
    scaled_emission_rate, wind_accel_bevy, wind_strength_at_age,
};
use super::emitters::{
    ModelParticleEmitterComp, ModelParticleEmitterRuntime, ModelParticleInstance,
    ParticleEmitterComp, emitter_parent_entity, emitter_scale_source, emitter_simulation_space,
    emitter_spawn_offset, emitter_translation, emitter_uses_bone_scale, emitter_uses_dynamic_wind,
    emitter_uses_follow_position, emitter_uses_inherit_position, emitter_uses_inherit_velocity,
    emitter_uses_model_particles, emitter_uses_project_particle,
    emitter_uses_sphere_invert_velocity, inherit_position_back_delta_local,
    model_particle_spawn_count, projected_particle_spawn_y, spawn_emitters,
    spawn_loaded_child_emitters, sync_dynamic_wind_properties,
};
use super::visuals::{
    build_color_gradient, build_offset_by_spin_modifier, build_size_gradient,
    has_authored_size_variation, has_authored_twinkle,
};
use super::{
    DYNAMIC_WIND_ACCEL_PROPERTY, DynamicParticleWind, PARTICLE_FLAG_BONE_SCALE,
    PARTICLE_FLAG_CLAMP_TAIL_TO_AGE, PARTICLE_FLAG_INHERIT_POSITION,
    PARTICLE_FLAG_INHERIT_VELOCITY, PARTICLE_FLAG_NEGATE_SPIN, PARTICLE_FLAG_NO_GLOBAL_SCALE,
    PARTICLE_FLAG_OFFSET_BY_SPIN, PARTICLE_FLAG_RANDOM_TEXTURE, PARTICLE_FLAG_SIZE_VARIATION_2D,
    PARTICLE_FLAG_SPHERE_INVERT, PARTICLE_FLAG_TAIL_PARTICLES, PARTICLE_FLAG_VELOCITY_ORIENT,
    PARTICLE_FLAG_WIND_DYNAMIC, PARTICLE_FLAG_WIND_ENABLED, PARTICLE_FLAG_WORLD_SPACE,
    PARTICLE_FLAG_XY_QUAD, ParticleSpawnMode, ParticleSpawnSource,
};
use crate::asset::m2_anim::M2Bone;
use crate::asset::m2_particle::M2ParticleEmitter;
use crate::client_options::GraphicsOptions;
use crate::creature_display::CreatureDisplayMap;
use crate::m2_effect_material::M2EffectMaterial;
use crate::terrain_heightmap::TerrainHeightmap;
use bevy_hanabi::OrientMode;

#[path = "particle_tests/benchmark_tests.rs"]
mod benchmark_tests;
#[path = "particle_tests/builder_tests.rs"]
mod builder_tests;
#[path = "particle_tests/runtime_tests.rs"]
mod runtime_tests;
#[path = "particle_tests/visual_tests.rs"]
mod visual_tests;

struct SampleMotionDefaults {
    emission_speed: f32,
    speed_variation: f32,
    vertical_range: f32,
    horizontal_range: f32,
    gravity: f32,
    gravity_vector: [f32; 3],
    lifespan: f32,
    lifespan_variation: f32,
    emission_rate: f32,
    emission_rate_variation: f32,
    area_length: f32,
    area_width: f32,
    z_source: f32,
    tail_length: f32,
    drag: f32,
    base_spin: f32,
    base_spin_variation: f32,
    spin: f32,
    spin_variation: f32,
    wind_vector: [f32; 3],
    wind_time: f32,
    follow_speed1: f32,
    follow_scale1: f32,
    follow_speed2: f32,
    follow_scale2: f32,
}

struct SampleVisualDefaults {
    colors: [[f32; 3]; 3],
    color_keys: Vec<(f32, [f32; 3])>,
    opacity: [f32; 3],
    opacity_keys: Vec<(f32, f32)>,
    scales: [[f32; 2]; 3],
    scale_keys: Vec<(f32, [f32; 2])>,
    twinkle_speed: f32,
    twinkle_percent: f32,
    twinkle_scale_min: f32,
    twinkle_scale_max: f32,
    head_cell_track: [u16; 3],
    tail_cell_track: [u16; 3],
    burst_multiplier: f32,
    mid_point: f32,
}

fn sample_motion_defaults() -> SampleMotionDefaults {
    SampleMotionDefaults {
        emission_speed: 1.0,
        speed_variation: 0.0,
        vertical_range: 0.1,
        horizontal_range: std::f32::consts::TAU,
        gravity: 0.0,
        gravity_vector: [0.0, 0.0, 0.0],
        lifespan: 1.0,
        lifespan_variation: 0.0,
        emission_rate: 20.0,
        emission_rate_variation: 0.0,
        area_length: 0.1,
        area_width: 0.1,
        z_source: 0.0,
        tail_length: 1.0,
        drag: 0.0,
        base_spin: 0.0,
        base_spin_variation: 0.0,
        spin: 0.0,
        spin_variation: 0.0,
        wind_vector: [0.0, 0.0, 0.0],
        wind_time: 0.0,
        follow_speed1: 0.0,
        follow_scale1: 0.0,
        follow_speed2: 0.0,
        follow_scale2: 0.0,
    }
}

fn sample_visual_defaults() -> SampleVisualDefaults {
    SampleVisualDefaults {
        colors: [[255.0, 128.0, 64.0]; 3],
        color_keys: Vec::new(),
        opacity: [1.0, 1.0, 0.0],
        opacity_keys: Vec::new(),
        scales: [[0.1, 0.1], [0.2, 0.2], [0.05, 0.05]],
        scale_keys: Vec::new(),
        twinkle_speed: 0.0,
        twinkle_percent: 0.0,
        twinkle_scale_min: 1.0,
        twinkle_scale_max: 1.0,
        head_cell_track: [0, 0, 0],
        tail_cell_track: [0, 0, 0],
        burst_multiplier: 1.0,
        mid_point: 0.5,
    }
}

fn sample_emitter() -> M2ParticleEmitter {
    let motion = sample_motion_defaults();
    let visuals = sample_visual_defaults();
    let mut emitter = M2ParticleEmitter {
        blend_type: 4,
        emitter_type: 1,
        tile_rows: 4,
        tile_cols: 4,
        ..M2ParticleEmitter::default()
    };
    apply_sample_motion_defaults(&mut emitter, motion);
    apply_sample_visual_defaults(&mut emitter, visuals);
    emitter
}

fn apply_sample_motion_defaults(emitter: &mut M2ParticleEmitter, motion: SampleMotionDefaults) {
    emitter.emission_speed = motion.emission_speed;
    emitter.speed_variation = motion.speed_variation;
    emitter.vertical_range = motion.vertical_range;
    emitter.horizontal_range = motion.horizontal_range;
    emitter.gravity = motion.gravity;
    emitter.gravity_vector = motion.gravity_vector;
    emitter.lifespan = motion.lifespan;
    emitter.lifespan_variation = motion.lifespan_variation;
    emitter.emission_rate = motion.emission_rate;
    emitter.emission_rate_variation = motion.emission_rate_variation;
    emitter.area_length = motion.area_length;
    emitter.area_width = motion.area_width;
    emitter.z_source = motion.z_source;
    emitter.tail_length = motion.tail_length;
    emitter.drag = motion.drag;
    emitter.base_spin = motion.base_spin;
    emitter.base_spin_variation = motion.base_spin_variation;
    emitter.spin = motion.spin;
    emitter.spin_variation = motion.spin_variation;
    emitter.wind_vector = motion.wind_vector;
    emitter.wind_time = motion.wind_time;
    emitter.follow_speed1 = motion.follow_speed1;
    emitter.follow_scale1 = motion.follow_scale1;
    emitter.follow_speed2 = motion.follow_speed2;
    emitter.follow_scale2 = motion.follow_scale2;
}

fn apply_sample_visual_defaults(emitter: &mut M2ParticleEmitter, visuals: SampleVisualDefaults) {
    emitter.colors = visuals.colors;
    emitter.color_keys = visuals.color_keys;
    emitter.opacity = visuals.opacity;
    emitter.opacity_keys = visuals.opacity_keys;
    emitter.scales = visuals.scales;
    emitter.scale_keys = visuals.scale_keys;
    emitter.twinkle_speed = visuals.twinkle_speed;
    emitter.twinkle_percent = visuals.twinkle_percent;
    emitter.twinkle_scale_min = visuals.twinkle_scale_min;
    emitter.twinkle_scale_max = visuals.twinkle_scale_max;
    emitter.head_cell_track = visuals.head_cell_track;
    emitter.tail_cell_track = visuals.tail_cell_track;
    emitter.burst_multiplier = visuals.burst_multiplier;
    emitter.mid_point = visuals.mid_point;
}

fn sample_cell_track_frame(
    track: [u16; 3],
    mid_point: f32,
    age_ratio: f32,
    total_cells: u32,
) -> u32 {
    let mid = mid_point.clamp(0.01, 0.99);
    let t = age_ratio.clamp(0.0, 1.0);
    let frame = if t < mid {
        let segment_t = (t / mid).clamp(0.0, 1.0);
        (track[0] as f32) + ((track[1] as f32) - (track[0] as f32)) * segment_t
    } else {
        let segment_t = ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0);
        (track[1] as f32) + ((track[2] as f32) - (track[1] as f32)) * segment_t
    };
    frame
        .floor()
        .clamp(0.0, total_cells.saturating_sub(1) as f32) as u32
}

fn benchmark_particle_model() -> Option<crate::asset::m2::M2Model> {
    let paths = [
        Path::new("data/models/5152423.m2"),
        Path::new("data/models/390126.m2"),
    ];
    for path in paths {
        if path.exists() {
            return crate::asset::m2::load_m2_uncached(path, &[0, 0, 0]).ok();
        }
    }
    None
}
