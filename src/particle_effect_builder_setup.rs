use bevy_hanabi::prelude::*;

use crate::asset::m2_particle::M2ParticleEmitter;
use crate::particle_effect_builder::{ExprModifiers, InitModifiers, lifetime_range};

pub(crate) struct EffectRuntimeModifiers {
    pub(crate) drag: Option<LinearDragModifier>,
    pub(crate) flipbook_sprite_index_init: Option<SetAttributeModifier>,
    pub(crate) flipbook_sprite_index_update: Option<SetAttributeModifier>,
    pub(crate) texture: Option<ParticleTextureModifier>,
    pub(crate) twinkle: Option<crate::particle_effect_builder::visuals::TwinkleSizeModifier>,
    pub(crate) size_variation:
        Option<crate::particle_effect_builder::visuals::SizeVariationModifier>,
}

pub(crate) struct EffectAssembleParts {
    pub(crate) module: Module,
    pub(crate) spawner: SpawnerSettings,
    pub(crate) max_particles: u32,
    pub(crate) alpha_mode: bevy_hanabi::AlphaMode,
    pub(crate) init: InitModifiers,
    pub(crate) gravity: AccelModifier,
    pub(crate) orient_rotation: Option<ExprHandle>,
    pub(crate) model_scale: f32,
}

struct EffectSetup {
    emission_rate: f32,
    max_particles: u32,
}

struct EffectAssembleSeed {
    module: Module,
    alpha_mode: bevy_hanabi::AlphaMode,
    init: InitModifiers,
    gravity: AccelModifier,
    orient_rotation: Option<ExprHandle>,
    model_scale: f32,
}

pub(crate) fn build_particle_effect_inputs(
    em: &M2ParticleEmitter,
    model_scale: f32,
    particle_density_multiplier: f32,
) -> (EffectAssembleParts, EffectRuntimeModifiers) {
    build_particle_effect_inputs_from_expr(
        em,
        particle_density_multiplier,
        model_scale,
        crate::particle_effect_builder::build_expr_modifiers(em, model_scale),
    )
}

fn build_particle_effect_inputs_from_expr(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
    model_scale: f32,
    expr_modifiers: ExprModifiers,
) -> (EffectAssembleParts, EffectRuntimeModifiers) {
    (
        build_effect_assemble_parts(
            em,
            particle_density_multiplier,
            EffectAssembleSeed {
                module: expr_modifiers.module,
                alpha_mode: expr_modifiers.alpha_mode,
                init: expr_modifiers.init,
                gravity: expr_modifiers.gravity,
                orient_rotation: expr_modifiers.orient_rotation,
                model_scale,
            },
        ),
        build_runtime_modifiers_from_expr(
            expr_modifiers.drag,
            expr_modifiers.flipbook_sprite_index_init,
            expr_modifiers.flipbook_sprite_index_update,
            expr_modifiers.texture,
            expr_modifiers.twinkle,
            expr_modifiers.size_variation,
        ),
    )
}

fn build_effect_assemble_parts(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
    seed: EffectAssembleSeed,
) -> EffectAssembleParts {
    let effect_setup = build_effect_setup(em, particle_density_multiplier);
    EffectAssembleParts {
        module: seed.module,
        spawner: SpawnerSettings::rate(effect_setup.emission_rate.into()),
        max_particles: effect_setup.max_particles,
        alpha_mode: seed.alpha_mode,
        init: seed.init,
        gravity: seed.gravity,
        orient_rotation: seed.orient_rotation,
        model_scale: seed.model_scale,
    }
}

fn build_effect_runtime_modifiers(
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<crate::particle_effect_builder::visuals::TwinkleSizeModifier>,
    size_variation: Option<crate::particle_effect_builder::visuals::SizeVariationModifier>,
) -> EffectRuntimeModifiers {
    EffectRuntimeModifiers {
        drag,
        flipbook_sprite_index_init,
        flipbook_sprite_index_update,
        texture,
        twinkle,
        size_variation,
    }
}

fn build_effect_setup(em: &M2ParticleEmitter, particle_density_multiplier: f32) -> EffectSetup {
    let emission_rate = scaled_emission_rate(em, particle_density_multiplier);
    let (_, max_lifetime) = lifetime_range(em);
    let max_particles =
        (((emission_rate * max_lifetime).max(emission_rate)).ceil() as u32).clamp(16, 4096);
    EffectSetup {
        emission_rate,
        max_particles,
    }
}

fn build_runtime_modifiers_from_expr(
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<crate::particle_effect_builder::visuals::TwinkleSizeModifier>,
    size_variation: Option<crate::particle_effect_builder::visuals::SizeVariationModifier>,
) -> EffectRuntimeModifiers {
    build_effect_runtime_modifiers(
        drag,
        flipbook_sprite_index_init,
        flipbook_sprite_index_update,
        texture,
        twinkle,
        size_variation,
    )
}

pub(crate) fn scaled_emission_rate(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
) -> f32 {
    let mean_rate = em.emission_rate + em.emission_rate_variation.max(0.0) * 0.5;
    let global_scale =
        if em.flags & crate::particle_effect_builder::PARTICLE_FLAG_NO_GLOBAL_SCALE != 0 {
            1.0
        } else {
            particle_density_multiplier.clamp(0.1, 1.0)
        };
    (mean_rate * global_scale).max(0.1)
}
