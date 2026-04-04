//! Minimal public particle effect builder for library/benchmark use.
//!
//! This intentionally exposes only the pure EffectAsset construction path for
//! a single emitter. The ECS/runtime particle systems stay in
//! `rendering/particles/mod.rs`.

#[path = "particle_effect_builder_motion.rs"]
mod motion;
#[path = "particle_effect_builder_setup.rs"]
mod setup;
#[path = "rendering/particles/effect_builder_shared.rs"]
mod shared;
#[path = "rendering/particles/visuals.rs"]
mod visuals;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::asset::m2::wow_to_bevy;
use crate::asset::m2_particle::M2ParticleEmitter;
use motion::{
    authored_spin_expr, build_orient_rotation_expr, build_position_modifier,
    build_size_variation_modifier, build_size_variation_modifier_attr, build_spin_sign_modifier,
    build_twinkle_modifier, build_twinkle_seed_modifier, build_velocity_modifier,
    emitter_alpha_mode, emitter_simulation_space, emitter_uses_dynamic_wind, gravity_accel_bevy,
    has_authored_wind, is_trail_particle, wind_accel_bevy,
};
use setup::{EffectAssembleParts, EffectRuntimeModifiers, build_particle_effect_inputs};
pub(crate) use shared::{
    ExprModifiers, InitModifiers, PositionInitModifier, build_expr_modifiers, lifetime_range,
};
use shared::{add_optional_init_modifiers, build_color_render_modifier, orient_mode};
use visuals::{
    SizeVariationModifier, TwinkleSizeModifier, build_color_gradient,
    build_offset_by_spin_modifier, build_size_gradient, has_authored_size_variation,
    has_authored_twinkle,
};

const PARTICLE_FLAG_TAIL_PARTICLES: u32 = 0x0000_0008;
const PARTICLE_FLAG_SPHERE_INVERT: u32 = 0x0000_1000;
const PARTICLE_FLAG_INHERIT_POSITION: u32 = 0x0000_2000;
const PARTICLE_FLAG_FOLLOW_POSITION: u32 = 0x0008_0000;
const PARTICLE_FLAG_XY_QUAD: u32 = 0x0000_4000;
const PARTICLE_FLAG_NEGATE_SPIN: u32 = 0x0001_0000;
const PARTICLE_FLAG_CLAMP_TAIL_TO_AGE: u32 = 0x0002_0000;
const PARTICLE_FLAG_RANDOM_TEXTURE: u32 = 0x0010_0000;
const PARTICLE_FLAG_VELOCITY_ORIENT: u32 = 0x0020_0000;
const PARTICLE_FLAG_SIZE_VARIATION_2D: u32 = 0x0080_0000;
const PARTICLE_FLAG_NO_GLOBAL_SCALE: u32 = 0x1000_0000;
const PARTICLE_FLAG_OFFSET_BY_SPIN: u32 = 0x2000_0000;
const PARTICLE_FLAG_WIND_DYNAMIC: u32 = 0x4000_0000;
const PARTICLE_FLAG_WIND_ENABLED: u32 = 0x8000_0000;
const BLEND_OPAQUE: u8 = 0;
const BLEND_ALPHA_KEY: u8 = 1;
const BLEND_ALPHA: u8 = 2;
const BLEND_ALPHA_3: u8 = 3;
const BLEND_ADD: u8 = 4;
const BLEND_ADD_ALPHA: u8 = 5;
const BLEND_MOD: u8 = 6;
const BLEND_MOD2X: u8 = 7;
const PARTICLE_TYPE_TRAIL: u8 = 1;
const INHERIT_POSITION_BACK_DELTA_PROPERTY: &str = "inherit_position_back_delta";
const DYNAMIC_WIND_ACCEL_PROPERTY: &str = "dynamic_wind_accel";

pub fn build_particle_effect_asset(
    em: &M2ParticleEmitter,
    model_scale: f32,
    particle_density_multiplier: f32,
) -> EffectAsset {
    let (parts, runtime_modifiers) =
        build_particle_effect_inputs(em, model_scale, particle_density_multiplier);
    build_runtime_particle_effect(em, runtime_modifiers, parts)
}

fn build_runtime_particle_effect(
    em: &M2ParticleEmitter,
    runtime_modifiers: EffectRuntimeModifiers,
    parts: EffectAssembleParts,
) -> EffectAsset {
    let effect = assemble_effect(em, parts);
    apply_effect_runtime_modifiers(effect, em, runtime_modifiers)
}

struct BaseEffectParts {
    module: Module,
    spawner: SpawnerSettings,
    max_particles: u32,
    alpha_mode: bevy_hanabi::AlphaMode,
    age: SetAttributeModifier,
    lifetime: SetAttributeModifier,
    vel: SetAttributeModifier,
    gravity: AccelModifier,
    orient: OrientModifier,
    model_scale: f32,
}

struct AssembleEffectParts {
    age: SetAttributeModifier,
    lifetime: SetAttributeModifier,
    pos: PositionInitModifier,
    vel: SetAttributeModifier,
    gravity: AccelModifier,
    rotation: Option<SetAttributeModifier>,
    angular_velocity: Option<SetAttributeModifier>,
    spin_sign: Option<SetAttributeModifier>,
    twinkle_phase: Option<SetAttributeModifier>,
    twinkle_enabled: Option<SetAttributeModifier>,
    size_variation: Option<SetAttributeModifier>,
    orient: OrientModifier,
}

fn apply_effect_runtime_modifiers(
    mut effect: EffectAsset,
    em: &M2ParticleEmitter,
    modifiers: EffectRuntimeModifiers,
) -> EffectAsset {
    if let Some(sprite_idx) = modifiers.flipbook_sprite_index_init {
        effect = effect.init(sprite_idx);
    }
    if let Some(drag) = modifiers.drag {
        effect = effect.update(drag);
    }
    if let Some(sprite_idx) = modifiers.flipbook_sprite_index_update {
        effect = effect.update(sprite_idx);
    }
    if let Some(tex) = modifiers.texture {
        effect = effect.render(tex);
    }
    if let Some(twinkle) = modifiers.twinkle {
        effect = effect.render(twinkle);
    }
    if let Some(size_variation) = modifiers.size_variation {
        effect = effect.render(size_variation);
    }
    if let Some(offset_by_spin) = build_offset_by_spin_modifier(em) {
        effect = effect.render(offset_by_spin);
    }
    if em.tile_rows > 1 || em.tile_cols > 1 {
        effect = effect.render(FlipbookModifier {
            sprite_grid_size: UVec2::new(em.tile_cols as u32, em.tile_rows as u32),
        });
    }
    effect
}

pub fn scaled_emission_rate(em: &M2ParticleEmitter, particle_density_multiplier: f32) -> f32 {
    setup::scaled_emission_rate(em, particle_density_multiplier)
}

fn assemble_effect(em: &M2ParticleEmitter, parts: EffectAssembleParts) -> EffectAsset {
    let EffectAssembleParts {
        module,
        spawner,
        max_particles,
        alpha_mode,
        init,
        gravity,
        orient_rotation,
        model_scale,
    } = parts;
    let assemble_parts = build_assemble_effect_parts(em, init, gravity, orient_rotation);
    let effect = build_base_effect_with_parts(
        em,
        &assemble_parts,
        module,
        spawner,
        max_particles,
        alpha_mode,
        model_scale,
    );
    add_effect_init_modifiers(effect, assemble_parts)
}

fn build_assemble_effect_parts(
    em: &M2ParticleEmitter,
    init: InitModifiers,
    gravity: AccelModifier,
    orient_rotation: Option<ExprHandle>,
) -> AssembleEffectParts {
    let InitModifiers {
        age,
        lifetime,
        pos,
        vel,
        rotation,
        angular_velocity,
        spin_sign,
        twinkle_phase,
        twinkle_enabled,
        size_variation,
    } = init;
    let orient = if let Some(rotation) = orient_rotation {
        OrientModifier::new(orient_mode(em)).with_rotation(rotation)
    } else {
        OrientModifier::new(orient_mode(em))
    };
    AssembleEffectParts {
        age,
        lifetime,
        pos,
        vel,
        gravity,
        rotation,
        angular_velocity,
        spin_sign,
        twinkle_phase,
        twinkle_enabled,
        size_variation,
        orient,
    }
}

fn build_base_effect_with_parts(
    em: &M2ParticleEmitter,
    parts: &AssembleEffectParts,
    module: Module,
    spawner: SpawnerSettings,
    max_particles: u32,
    alpha_mode: bevy_hanabi::AlphaMode,
    model_scale: f32,
) -> EffectAsset {
    build_base_effect(
        em,
        BaseEffectParts {
            module,
            spawner,
            max_particles,
            alpha_mode,
            age: parts.age,
            lifetime: parts.lifetime,
            vel: parts.vel,
            gravity: parts.gravity,
            orient: parts.orient,
            model_scale,
        },
    )
}

fn add_effect_init_modifiers(effect: EffectAsset, parts: AssembleEffectParts) -> EffectAsset {
    let effect = add_position_init(effect, parts.pos);
    add_optional_init_modifiers(
        effect,
        parts.rotation,
        parts.angular_velocity,
        parts.spin_sign,
        parts.twinkle_phase,
        parts.twinkle_enabled,
        parts.size_variation,
    )
}

fn build_base_effect(em: &M2ParticleEmitter, parts: BaseEffectParts) -> EffectAsset {
    let BaseEffectParts {
        module,
        spawner,
        max_particles,
        alpha_mode,
        age,
        lifetime,
        vel,
        gravity,
        orient,
        model_scale,
    } = parts;
    EffectAsset::new(max_particles, spawner, module)
        .with_name("m2_particle")
        .with_alpha_mode(alpha_mode)
        .with_simulation_space(emitter_simulation_space(em))
        .init(age)
        .init(lifetime)
        .init(vel)
        .update(gravity)
        .render(build_color_render_modifier(em))
        .render(SizeOverLifetimeModifier {
            gradient: build_size_gradient(em, model_scale),
            screen_space_size: false,
        })
        .render(orient)
}

fn add_position_init(mut effect: EffectAsset, pos: PositionInitModifier) -> EffectAsset {
    effect = match pos {
        PositionInitModifier::Attribute(pos) => effect.init(pos),
        PositionInitModifier::Sphere(pos) => effect.init(pos),
    };
    effect
}
