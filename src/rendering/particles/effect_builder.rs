use bevy::prelude::*;
use bevy_hanabi::prelude::*;

#[path = "effect_builder_shared.rs"]
mod shared;

use crate::asset::m2_particle::M2ParticleEmitter;

use super::effect_builder_motion::{
    authored_spin_expr, build_orient_rotation_expr, build_size_variation_modifier,
    build_size_variation_modifier_attr, build_spin_sign_modifier, build_twinkle_modifier,
    build_twinkle_seed_modifier, build_velocity_modifier, is_trail_particle,
};
use super::effect_builder_setup::{
    EffectAssetParts, RuntimeEffectModifiers, build_effect_asset_inputs,
};
use super::emitters::{
    emitter_simulation_space, emitter_uses_dynamic_wind, emitter_uses_inherit_velocity,
};
use super::visuals::{
    SizeVariationModifier, TwinkleSizeModifier, build_color_gradient,
    build_offset_by_spin_modifier, build_size_gradient, has_authored_twinkle,
};
use shared::{
    add_optional_init_modifiers, build_color_render_modifier, orient_mode as shared_orient_mode,
};

use super::{
    DYNAMIC_WIND_ACCEL_PROPERTY, PARTICLE_FLAG_RANDOM_TEXTURE, PARTICLE_FLAG_VELOCITY_ORIENT,
    PARTICLE_FLAG_XY_QUAD, ParticleSpawnMode, ParticleSpawnSource,
};
pub(crate) use shared::{
    ExprModifiers, InitModifiers, PositionInitModifier, build_expr_modifiers, lifetime_range,
};

pub(crate) fn build_effect_asset(
    em: &M2ParticleEmitter,
    model_scale: f32,
    particle_density_multiplier: f32,
) -> EffectAsset {
    build_effect_asset_with_mode(
        em,
        model_scale,
        particle_density_multiplier,
        ParticleSpawnMode::Continuous,
        ParticleSpawnSource::Standalone,
        &[],
    )
}

pub(crate) type FlipbookSpriteMode = shared::FlipbookSpriteMode;

pub(crate) fn active_cell_track(em: &M2ParticleEmitter) -> Option<[u16; 3]> {
    shared::active_cell_track(em)
}

pub(crate) fn flipbook_sprite_mode(em: &M2ParticleEmitter) -> Option<FlipbookSpriteMode> {
    shared::flipbook_sprite_mode(em)
}

pub(crate) fn orient_mode(em: &M2ParticleEmitter) -> OrientMode {
    shared_orient_mode(em)
}

pub(crate) fn build_effect_asset_with_mode(
    em: &M2ParticleEmitter,
    model_scale: f32,
    particle_density_multiplier: f32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
    child_emitters: &[M2ParticleEmitter],
) -> EffectAsset {
    let (parts, runtime_modifiers) = build_effect_asset_inputs(
        em,
        model_scale,
        particle_density_multiplier,
        spawn_mode,
        spawn_source,
        child_emitters,
    );
    let effect = assemble_effect_from_parts(em, spawn_source, parts);
    apply_runtime_effect_modifiers(effect, em, runtime_modifiers)
}

fn assemble_effect_from_parts(
    em: &M2ParticleEmitter,
    spawn_source: ParticleSpawnSource,
    parts: EffectAssetParts,
) -> EffectAsset {
    assemble_effect(
        em,
        parts.module,
        parts.spawner,
        parts.max_particles,
        parts.alpha_mode,
        parts.init,
        parts.gravity,
        parts.orient_rotation,
        parts.model_scale,
        spawn_source,
        parts.child_event_counts,
    )
}

fn apply_runtime_effect_modifiers(
    effect: EffectAsset,
    em: &M2ParticleEmitter,
    runtime_modifiers: RuntimeEffectModifiers,
) -> EffectAsset {
    apply_effect_runtime_modifiers(
        effect,
        em,
        runtime_modifiers.drag,
        runtime_modifiers.flipbook_sprite_index_init,
        runtime_modifiers.flipbook_sprite_index_update,
        runtime_modifiers.texture,
        runtime_modifiers.twinkle,
        runtime_modifiers.size_variation,
    )
}

fn apply_effect_runtime_modifiers(
    mut effect: EffectAsset,
    em: &M2ParticleEmitter,
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<TwinkleSizeModifier>,
    size_variation: Option<SizeVariationModifier>,
) -> EffectAsset {
    if let Some(sprite_idx) = flipbook_sprite_index_init {
        effect = effect.init(sprite_idx);
    }
    if let Some(drag) = drag {
        effect = effect.update(drag);
    }
    if let Some(sprite_idx) = flipbook_sprite_index_update {
        effect = effect.update(sprite_idx);
    }
    if let Some(tex) = texture {
        effect = effect.render(tex);
    }
    if let Some(twinkle) = twinkle {
        effect = effect.render(twinkle);
    }
    if let Some(size_variation) = size_variation {
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

fn assemble_effect(
    em: &M2ParticleEmitter,
    mut module: Module,
    spawner: SpawnerSettings,
    max_particles: u32,
    alpha_mode: bevy_hanabi::AlphaMode,
    init: InitModifiers,
    gravity: AccelModifier,
    orient_rotation: Option<ExprHandle>,
    model_scale: f32,
    spawn_source: ParticleSpawnSource,
    child_event_counts: Vec<u32>,
) -> EffectAsset {
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
    let child_event_count_exprs: Vec<ExprHandle> = child_event_counts
        .into_iter()
        .map(|count| module.lit(count))
        .collect();
    let effect = build_base_effect(
        em,
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
    );
    let effect = add_position_and_inherit_init(effect, em, pos, spawn_source);
    let effect = add_optional_init_modifiers(
        effect,
        rotation,
        angular_velocity,
        spin_sign,
        twinkle_phase,
        twinkle_enabled,
        size_variation,
    );
    add_child_spawn_events(effect, child_event_count_exprs)
}

fn build_base_effect(
    em: &M2ParticleEmitter,
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
) -> EffectAsset {
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

fn add_position_and_inherit_init(
    mut effect: EffectAsset,
    em: &M2ParticleEmitter,
    pos: PositionInitModifier,
    spawn_source: ParticleSpawnSource,
) -> EffectAsset {
    effect = match pos {
        PositionInitModifier::Attribute(pos) => effect.init(pos),
        PositionInitModifier::Sphere(pos) => effect.init(pos),
    };
    if spawn_source == ParticleSpawnSource::ChildFromParentParticles {
        effect = effect.init(InheritAttributeModifier::new(Attribute::POSITION));
        if emitter_uses_inherit_velocity(em) {
            effect = effect.init(InheritAttributeModifier::new(Attribute::VELOCITY));
        }
    }
    effect
}

fn add_child_spawn_events(
    mut effect: EffectAsset,
    child_event_count_exprs: Vec<ExprHandle>,
) -> EffectAsset {
    for (child_index, count_expr) in child_event_count_exprs.into_iter().enumerate() {
        effect = effect.update(EmitSpawnEventModifier {
            condition: EventEmitCondition::Always,
            count: count_expr,
            child_index: child_index as u32,
        });
    }
    effect
}

pub(crate) fn child_emitter_event_count(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
) -> u32 {
    super::effect_builder_setup::child_emitter_event_count(em, particle_density_multiplier)
}

pub(crate) fn scaled_emission_rate(
    em: &M2ParticleEmitter,
    particle_density_multiplier: f32,
) -> f32 {
    super::effect_builder_setup::scaled_emission_rate(em, particle_density_multiplier)
}

pub(crate) fn build_position_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> PositionInitModifier {
    super::effect_builder_motion::build_position_modifier(em, writer, model_scale)
}

pub(crate) fn emitter_spawn_radius(em: &M2ParticleEmitter) -> f32 {
    super::effect_builder_motion::emitter_spawn_radius(em)
}

pub(crate) fn gravity_accel_bevy(em: &M2ParticleEmitter) -> Vec3 {
    super::effect_builder_motion::gravity_accel_bevy(em)
}

pub(crate) fn has_authored_spin(em: &M2ParticleEmitter) -> bool {
    super::effect_builder_motion::has_authored_spin(em)
}

pub(crate) fn has_authored_wind(em: &M2ParticleEmitter) -> bool {
    super::effect_builder_motion::has_authored_wind(em)
}

pub(crate) fn wind_accel_bevy(em: &M2ParticleEmitter, model_scale: f32) -> Vec3 {
    super::effect_builder_motion::wind_accel_bevy(em, model_scale)
}

pub(crate) fn wind_strength_at_age(age: f32, wind_time: f32) -> f32 {
    super::effect_builder_motion::wind_strength_at_age(age, wind_time)
}

pub(crate) fn emitter_alpha_mode(
    blend_type: u8,
    mask_cutoff: ExprHandle,
) -> bevy_hanabi::AlphaMode {
    super::effect_builder_motion::emitter_alpha_mode(blend_type, mask_cutoff)
}

pub(crate) fn load_emitter_texture(
    em: &M2ParticleEmitter,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    super::effect_builder_motion::load_emitter_texture(em, images)
}

fn build_initial_rotation_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    authored_spin_expr(em, writer, em.base_spin, em.base_spin_variation)
        .map(|expr| SetAttributeModifier::new(Attribute::F32_0, expr))
}

fn build_angular_velocity_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    authored_spin_expr(em, writer, em.spin, em.spin_variation)
        .map(|expr| SetAttributeModifier::new(Attribute::F32_1, expr))
}

fn build_twinkle_phase_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    has_authored_twinkle(em).then(|| {
        SetAttributeModifier::new(
            Attribute::F32_2,
            (writer.rand(ScalarType::Float) * writer.lit(std::f32::consts::TAU)).expr(),
        )
    })
}
