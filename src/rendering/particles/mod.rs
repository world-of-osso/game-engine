//! M2 particle emitter rendering — GPU particles via bevy_hanabi.
//!
//! Each M2 emitter is translated to a bevy_hanabi `EffectAsset` and spawned as
//! a `ParticleEffect` entity parented to the model (or its bone).

pub(crate) mod effect_builder;
mod effect_builder_motion;
mod emitters;
mod emitters_model_particles;
mod visuals;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

pub use emitters::spawn_emitters;

// CParticleEmitter / retail runtime particle flag values.
pub(super) const PARTICLE_FLAG_TAIL_PARTICLES: u32 = 0x0000_0008;
pub(super) const PARTICLE_FLAG_WORLD_SPACE: u32 = 0x0000_0200;
pub(super) const PARTICLE_FLAG_BONE_SCALE: u32 = 0x0000_0400;
pub(super) const PARTICLE_FLAG_INHERIT_VELOCITY: u32 = 0x0000_0800;
pub(super) const PARTICLE_FLAG_INHERIT_POSITION: u32 = 0x0000_2000;
pub(super) const PARTICLE_FLAG_SPHERE_INVERT: u32 = 0x0000_1000;
pub(super) const PARTICLE_FLAG_XY_QUAD: u32 = 0x0000_4000;
pub(super) const PARTICLE_FLAG_NEGATE_SPIN: u32 = 0x0001_0000;
pub(super) const PARTICLE_FLAG_CLAMP_TAIL_TO_AGE: u32 = 0x0002_0000;
pub(super) const PARTICLE_FLAG_PROJECT_PARTICLE: u32 = 0x0004_0000;
pub(super) const PARTICLE_FLAG_FOLLOW_POSITION: u32 = 0x0008_0000;
pub(super) const PARTICLE_FLAG_RANDOM_TEXTURE: u32 = 0x0010_0000;
pub(super) const PARTICLE_FLAG_VELOCITY_ORIENT: u32 = 0x0020_0000;
pub(super) const PARTICLE_FLAG_SIZE_VARIATION_2D: u32 = 0x0080_0000;
pub(super) const PARTICLE_FLAG_NO_GLOBAL_SCALE: u32 = 0x1000_0000;
pub(super) const PARTICLE_FLAG_OFFSET_BY_SPIN: u32 = 0x2000_0000;
pub(super) const PARTICLE_FLAG_WIND_DYNAMIC: u32 = 0x4000_0000;
pub(super) const PARTICLE_FLAG_WIND_ENABLED: u32 = 0x8000_0000;
const BLEND_OPAQUE: u8 = 0;
const BLEND_ALPHA_KEY: u8 = 1;
const BLEND_ALPHA: u8 = 2;
const BLEND_ALPHA_3: u8 = 3;
const BLEND_ADD: u8 = 4;
const BLEND_ADD_ALPHA: u8 = 5;
const BLEND_MOD: u8 = 6;
const BLEND_MOD2X: u8 = 7;
const PARTICLE_TYPE_TRAIL: u8 = 1;
const TRAIL_LENGTH_FACTOR: f32 = 0.6;
const INHERIT_POSITION_BACK_DELTA_PROPERTY: &str = "inherit_position_back_delta";
pub(crate) const DYNAMIC_WIND_ACCEL_PROPERTY: &str = "dynamic_wind_accel";
const CHILD_EMITTER_FPS_APPROXIMATION: f32 = 60.0;
const MODEL_PARTICLE_MIN_SPEED: f32 = 0.0;

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DynamicParticleWind>()
            .add_plugins(HanabiPlugin)
            .add_systems(
                Update,
                (
                    emitters::register_pending_particle_effects,
                    emitters::sync_inherit_position_properties,
                    emitters::sync_dynamic_wind_properties,
                    emitters::trigger_pending_particle_bursts,
                    emitters::tick_model_particle_emitters,
                    emitters::simulate_model_particle_instances,
                ),
            );
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct DynamicParticleWind {
    pub effect_space_accel: Vec3,
}

impl Default for DynamicParticleWind {
    fn default() -> Self {
        Self {
            effect_space_accel: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSpawnMode {
    Continuous,
    BurstOnce,
}

#[derive(Component, Default)]
pub struct PendingParticleBurst {
    pub armed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSpawnSource {
    Standalone,
    ChildFromParentParticles,
}

#[cfg(test)]
#[path = "../../../tests/unit/particle_tests.rs"]
mod tests;
