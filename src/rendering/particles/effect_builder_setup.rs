use bevy_hanabi::prelude::*;

use crate::asset::m2_particle::M2ParticleEmitter;

use super::effect_builder::{ExprModifiers, InitModifiers, lifetime_range};
use super::{CHILD_EMITTER_FPS_APPROXIMATION, ParticleSpawnMode, ParticleSpawnSource};

pub(super) struct RuntimeEffectModifiers {
    pub(super) drag: Option<LinearDragModifier>,
    pub(super) flipbook_sprite_index_init: Option<SetAttributeModifier>,
    pub(super) flipbook_sprite_index_update: Option<SetAttributeModifier>,
    pub(super) texture: Option<ParticleTextureModifier>,
    pub(super) twinkle: Option<super::visuals::TwinkleSizeModifier>,
    pub(super) size_variation: Option<super::visuals::SizeVariationModifier>,
}

pub(super) struct EffectAssetParts {
    pub(super) module: Module,
    pub(super) spawner: SpawnerSettings,
    pub(super) max_particles: u32,
    pub(super) alpha_mode: bevy_hanabi::AlphaMode,
    pub(super) init: InitModifiers,
    pub(super) gravity: AccelModifier,
    pub(super) orient_rotation: Option<ExprHandle>,
    pub(super) model_scale: f32,
    pub(super) child_event_counts: Vec<u32>,
}

struct EffectExprParts {
    module: Module,
    alpha_mode: bevy_hanabi::AlphaMode,
    init: InitModifiers,
    gravity: AccelModifier,
    orient_rotation: Option<ExprHandle>,
}

struct EffectRuntimeSetup {
    spawner: SpawnerSettings,
    max_particles: u32,
    child_event_counts: Vec<u32>,
}

pub(super) fn build_effect_asset_inputs(
    em: &M2ParticleEmitter,
    model_scale: f32,
    particle_density_multiplier: f32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
    child_emitters: &[M2ParticleEmitter],
) -> (EffectAssetParts, RuntimeEffectModifiers) {
    build_effect_asset_inputs_from_expr(
        em,
        particle_density_multiplier,
        spawn_mode,
        spawn_source,
        child_emitters,
        model_scale,
        super::effect_builder::build_expr_modifiers(em, model_scale),
    )
}

fn build_effect_asset_inputs_from_expr(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
    child_emitters: &[M2ParticleEmitter],
    model_scale: f32,
    expr_modifiers: ExprModifiers,
) -> (EffectAssetParts, RuntimeEffectModifiers) {
    let (expr_parts, runtime_modifiers) = split_effect_expr_modifiers(expr_modifiers);
    (
        build_effect_asset_parts(
            em,
            particle_density_multiplier,
            spawn_mode,
            spawn_source,
            child_emitters,
            model_scale,
            expr_parts,
        ),
        runtime_modifiers,
    )
}

fn build_effect_asset_parts(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
    child_emitters: &[M2ParticleEmitter],
    model_scale: f32,
    expr_parts: EffectExprParts,
) -> EffectAssetParts {
    let setup = build_effect_runtime_setup(
        em,
        particle_density_multiplier,
        spawn_mode,
        spawn_source,
        child_emitters,
    );
    EffectAssetParts {
        module: expr_parts.module,
        spawner: setup.spawner,
        max_particles: setup.max_particles,
        alpha_mode: expr_parts.alpha_mode,
        init: expr_parts.init,
        gravity: expr_parts.gravity,
        orient_rotation: expr_parts.orient_rotation,
        model_scale,
        child_event_counts: setup.child_event_counts,
    }
}

fn split_effect_expr_modifiers(
    expr_modifiers: ExprModifiers,
) -> (EffectExprParts, RuntimeEffectModifiers) {
    let ExprModifiers {
        init,
        gravity,
        drag,
        flipbook_sprite_index_init,
        flipbook_sprite_index_update,
        texture,
        twinkle,
        size_variation,
        alpha_mode,
        orient_rotation,
        module,
    } = expr_modifiers;
    (
        EffectExprParts {
            module,
            alpha_mode,
            init,
            gravity,
            orient_rotation,
        },
        build_runtime_effect_modifiers(
            drag,
            flipbook_sprite_index_init,
            flipbook_sprite_index_update,
            texture,
            twinkle,
            size_variation,
        ),
    )
}

fn build_runtime_effect_modifiers(
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<super::visuals::TwinkleSizeModifier>,
    size_variation: Option<super::visuals::SizeVariationModifier>,
) -> RuntimeEffectModifiers {
    RuntimeEffectModifiers {
        drag,
        flipbook_sprite_index_init,
        flipbook_sprite_index_update,
        texture,
        twinkle,
        size_variation,
    }
}

fn build_effect_runtime_setup(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
    child_emitters: &[M2ParticleEmitter],
) -> EffectRuntimeSetup {
    let emission_rate = scaled_emission_rate(em, particle_density_multiplier);
    let (_, max_lifetime) = lifetime_range(em);
    let burst_count = emission_rate.max(0.0);
    EffectRuntimeSetup {
        spawner: build_spawner_settings(emission_rate, spawn_mode, spawn_source),
        max_particles: max_particles(emission_rate, max_lifetime, burst_count, spawn_source),
        child_event_counts: child_emitters
            .iter()
            .map(|child| child_emitter_event_count(child, particle_density_multiplier))
            .collect(),
    }
}

fn build_spawner_settings(
    emission_rate: f32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
) -> SpawnerSettings {
    if spawn_source == ParticleSpawnSource::ChildFromParentParticles {
        return SpawnerSettings::default();
    }
    match spawn_mode {
        ParticleSpawnMode::Continuous => SpawnerSettings::rate(emission_rate.into()),
        ParticleSpawnMode::BurstOnce => SpawnerSettings::once(emission_rate.max(0.0).into())
            .with_starts_active(true)
            .with_emit_on_start(false),
    }
}

fn max_particles(
    emission_rate: f32,
    max_lifetime: f32,
    burst_count: f32,
    spawn_source: ParticleSpawnSource,
) -> u32 {
    if spawn_source == ParticleSpawnSource::ChildFromParentParticles {
        return 4096;
    }
    (((emission_rate * max_lifetime).max(burst_count)).ceil() as u32).clamp(16, 4096)
}

pub(crate) fn scaled_emission_rate(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
) -> f32 {
    let mean_rate = em.emission_rate + em.emission_rate_variation.max(0.0) * 0.5;
    let global_scale = if em.flags & super::PARTICLE_FLAG_NO_GLOBAL_SCALE != 0 {
        1.0
    } else {
        particle_density_multiplier.clamp(0.1, 1.0)
    };
    (mean_rate * global_scale).max(0.1)
}

pub(crate) fn child_emitter_event_count(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
) -> u32 {
    let per_frame =
        scaled_emission_rate(em, particle_density_multiplier) / CHILD_EMITTER_FPS_APPROXIMATION;
    per_frame.ceil().max(1.0) as u32
}
