#[path = "rendering/particles/effect_builder_motion_shared.rs"]
mod shared;

use super::*;

pub(super) use shared::{
    authored_spin_expr, build_orient_rotation_expr, build_position_modifier,
    build_size_variation_modifier, build_size_variation_modifier_attr, build_spin_sign_modifier,
    build_twinkle_modifier, build_twinkle_seed_modifier, build_velocity_modifier,
    emitter_alpha_mode, gravity_accel_bevy, has_authored_wind, is_trail_particle, wind_accel_bevy,
};

fn emitter_uses_inherit_position(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_INHERIT_POSITION != 0
}

pub(super) fn emitter_simulation_space(em: &M2ParticleEmitter) -> SimulationSpace {
    if em.flags & PARTICLE_FLAG_FOLLOW_POSITION != 0 {
        SimulationSpace::Local
    } else {
        SimulationSpace::Global
    }
}

pub(super) fn emitter_uses_dynamic_wind(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_WIND_DYNAMIC != 0
}
