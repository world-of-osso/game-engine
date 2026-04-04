use bevy::prelude::*;
use bevy_hanabi::prelude::*;

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
use super::{
    DYNAMIC_WIND_ACCEL_PROPERTY, PARTICLE_FLAG_RANDOM_TEXTURE, PARTICLE_FLAG_VELOCITY_ORIENT,
    PARTICLE_FLAG_XY_QUAD, ParticleSpawnMode, ParticleSpawnSource,
};

pub(crate) struct ExprModifiers {
    pub(crate) init: InitModifiers,
    pub(crate) gravity: AccelModifier,
    pub(crate) drag: Option<LinearDragModifier>,
    pub(crate) flipbook_sprite_index_init: Option<SetAttributeModifier>,
    pub(crate) flipbook_sprite_index_update: Option<SetAttributeModifier>,
    pub(crate) texture: Option<ParticleTextureModifier>,
    pub(crate) twinkle: Option<TwinkleSizeModifier>,
    pub(crate) size_variation: Option<SizeVariationModifier>,
    pub(crate) alpha_mode: bevy_hanabi::AlphaMode,
    pub(crate) orient_rotation: Option<ExprHandle>,
    pub(crate) module: Module,
}

pub(crate) fn build_expr_modifiers(em: &M2ParticleEmitter, model_scale: f32) -> ExprModifiers {
    let writer = ExprWriter::new();
    let init = build_init_modifiers(em, &writer, model_scale);
    let gravity = build_accel_modifier(em, &writer, model_scale);
    let drag = (em.drag > 0.0).then(|| LinearDragModifier::new(writer.lit(em.drag).expr()));
    let (flipbook_sprite_index_init, flipbook_sprite_index_update) =
        build_flipbook_sprite_index_modifiers(em, &writer);
    let mask_cutoff = writer.lit(0.5_f32).expr();
    let texture = em.texture_fdid.map(|_| ParticleTextureModifier {
        texture_slot: writer.lit(0u32).expr(),
        sample_mapping: ImageSampleMapping::Modulate,
    });
    let twinkle = build_twinkle_modifier(em);
    let size_variation = build_size_variation_modifier(em);
    let alpha_mode = emitter_alpha_mode(em.blend_type, mask_cutoff);
    let orient_rotation = build_orient_rotation_expr(em, &writer);
    let mut module = writer.finish();
    if texture.is_some() {
        module.add_texture_slot("color");
    }
    ExprModifiers {
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
    }
}

fn build_accel_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> AccelModifier {
    let gravity = writer.lit(gravity_accel_bevy(em));
    if emitter_uses_dynamic_wind(em) {
        let dynamic_wind = writer.add_property(DYNAMIC_WIND_ACCEL_PROPERTY, Vec3::ZERO.into());
        return AccelModifier::new((gravity + writer.prop(dynamic_wind)).expr());
    }
    if !has_authored_wind(em) {
        return AccelModifier::new(gravity.expr());
    }
    let age = writer.attr(Attribute::AGE);
    let wind_active = writer
        .lit(em.wind_time.max(0.0))
        .ge(age)
        .cast(ScalarType::Float);
    let wind = writer.lit(wind_accel_bevy(em, model_scale)) * wind_active;
    AccelModifier::new((gravity + wind).expr())
}

fn build_flipbook_sprite_index_modifiers(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> (Option<SetAttributeModifier>, Option<SetAttributeModifier>) {
    let total = (em.tile_rows as i32) * (em.tile_cols as i32);
    let frame = match flipbook_sprite_mode(em) {
        Some(FlipbookSpriteMode::CellTrack(track)) => {
            let frame = build_cell_track_sprite_index(writer, track, em.mid_point, total);
            return (
                None,
                Some(SetAttributeModifier::new(
                    Attribute::SPRITE_INDEX,
                    frame.expr(),
                )),
            );
        }
        Some(FlipbookSpriteMode::FirstCell) => writer.lit(0),
        Some(FlipbookSpriteMode::RandomCell) => {
            (writer.rand(ScalarType::Float) * writer.lit(total as f32)).floor()
        }
        None => return (None, None),
    };
    (
        Some(SetAttributeModifier::new(
            Attribute::SPRITE_INDEX,
            frame.expr(),
        )),
        None,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlipbookSpriteMode {
    FirstCell,
    CellTrack([u16; 3]),
    RandomCell,
}

pub(crate) fn flipbook_sprite_mode(em: &M2ParticleEmitter) -> Option<FlipbookSpriteMode> {
    if em.tile_rows <= 1 && em.tile_cols <= 1 {
        return None;
    }
    if let Some(track) = active_cell_track(em) {
        return Some(FlipbookSpriteMode::CellTrack(track));
    }
    if em.flags & PARTICLE_FLAG_RANDOM_TEXTURE != 0 {
        return Some(FlipbookSpriteMode::RandomCell);
    }
    Some(FlipbookSpriteMode::FirstCell)
}

pub(crate) fn active_cell_track(em: &M2ParticleEmitter) -> Option<[u16; 3]> {
    if em.head_cell_track.iter().any(|&cell| cell != 0) {
        Some(em.head_cell_track)
    } else if em.tail_cell_track.iter().any(|&cell| cell != 0) {
        Some(em.tail_cell_track)
    } else {
        None
    }
}

pub(crate) fn build_cell_track_sprite_index(
    writer: &ExprWriter,
    track: [u16; 3],
    mid_point: f32,
    total_cells: i32,
) -> WriterExpr {
    let age = writer.attr(Attribute::AGE);
    let lifetime = writer.attr(Attribute::LIFETIME);
    let zero = writer.lit(0.0_f32);
    let one = writer.lit(1.0_f32);
    let age_ratio = (age / lifetime).clamp(zero.clone(), one.clone());
    let mid = writer.lit(mid_point.clamp(0.01, 0.99));
    let first_t = (age_ratio.clone() / mid.clone()).clamp(zero.clone(), one.clone());
    let second_t = ((age_ratio.clone() - mid.clone()) / (one.clone() - mid.clone()))
        .clamp(zero.clone(), one.clone());
    let first = writer
        .lit(track[0] as f32)
        .mix(writer.lit(track[1] as f32), first_t);
    let second = writer
        .lit(track[1] as f32)
        .mix(writer.lit(track[2] as f32), second_t);
    let cell = first
        .mix(second, age_ratio.ge(mid).cast(ScalarType::Float))
        .clamp(zero, writer.lit((total_cells - 1).max(0) as f32));
    cell.cast(ScalarType::Int)
}

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

fn add_optional_init_modifiers(
    mut effect: EffectAsset,
    rotation: Option<SetAttributeModifier>,
    angular_velocity: Option<SetAttributeModifier>,
    spin_sign: Option<SetAttributeModifier>,
    twinkle_phase: Option<SetAttributeModifier>,
    twinkle_enabled: Option<SetAttributeModifier>,
    size_variation: Option<SetAttributeModifier>,
) -> EffectAsset {
    if let Some(rotation) = rotation {
        effect = effect.init(rotation);
    }
    if let Some(angular_velocity) = angular_velocity {
        effect = effect.init(angular_velocity);
    }
    if let Some(spin_sign) = spin_sign {
        effect = effect.init(spin_sign);
    }
    if let Some(twinkle_phase) = twinkle_phase {
        effect = effect.init(twinkle_phase);
    }
    if let Some(twinkle_enabled) = twinkle_enabled {
        effect = effect.init(twinkle_enabled);
    }
    if let Some(size_variation) = size_variation {
        effect = effect.init(size_variation);
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

pub(crate) fn orient_mode(em: &M2ParticleEmitter) -> OrientMode {
    if is_trail_particle(em) || em.flags & PARTICLE_FLAG_VELOCITY_ORIENT != 0 {
        OrientMode::AlongVelocity
    } else if em.flags & PARTICLE_FLAG_XY_QUAD != 0 {
        OrientMode::ParallelCameraDepthPlane
    } else {
        OrientMode::FaceCameraPosition
    }
}

fn build_color_render_modifier(em: &M2ParticleEmitter) -> ColorOverLifetimeModifier {
    ColorOverLifetimeModifier {
        gradient: build_color_gradient(em),
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    }
}

pub(crate) struct InitModifiers {
    pub(crate) age: SetAttributeModifier,
    pub(crate) lifetime: SetAttributeModifier,
    pub(crate) pos: PositionInitModifier,
    pub(crate) vel: SetAttributeModifier,
    pub(crate) rotation: Option<SetAttributeModifier>,
    pub(crate) angular_velocity: Option<SetAttributeModifier>,
    pub(crate) spin_sign: Option<SetAttributeModifier>,
    pub(crate) twinkle_phase: Option<SetAttributeModifier>,
    pub(crate) twinkle_enabled: Option<SetAttributeModifier>,
    pub(crate) size_variation: Option<SetAttributeModifier>,
}

pub(crate) enum PositionInitModifier {
    Attribute(SetAttributeModifier),
    Sphere(SetPositionSphereModifier),
}

fn build_init_modifiers(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> InitModifiers {
    let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let lifetime = SetAttributeModifier::new(Attribute::LIFETIME, build_lifetime_expr(em, writer));
    let pos = build_position_modifier(em, writer, model_scale);
    let vel = build_velocity_modifier(em, writer, model_scale);
    InitModifiers {
        age,
        lifetime,
        pos,
        vel,
        rotation: build_initial_rotation_modifier(em, writer),
        angular_velocity: build_angular_velocity_modifier(em, writer),
        spin_sign: build_spin_sign_modifier(em, writer),
        twinkle_phase: build_twinkle_phase_modifier(em, writer),
        twinkle_enabled: build_twinkle_seed_modifier(em, writer),
        size_variation: build_size_variation_modifier_attr(em, writer),
    }
}

fn build_lifetime_expr(em: &M2ParticleEmitter, writer: &ExprWriter) -> ExprHandle {
    let (min_lifetime, max_lifetime) = lifetime_range(em);
    if (max_lifetime - min_lifetime).abs() < f32::EPSILON {
        return writer.lit(max_lifetime).expr();
    }
    let span = max_lifetime - min_lifetime;
    (writer.rand(ScalarType::Float) * writer.lit(span) + writer.lit(min_lifetime)).expr()
}

pub(crate) fn lifetime_range(em: &M2ParticleEmitter) -> (f32, f32) {
    let base = em.lifespan.max(0.1);
    let variation = em.lifespan_variation.max(0.0);
    ((base - variation).max(0.1), (base + variation).max(0.1))
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
