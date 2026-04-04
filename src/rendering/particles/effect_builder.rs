use std::path::PathBuf;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::asset::blp;
use crate::asset::m2::wow_to_bevy;
use crate::asset::m2_particle::M2ParticleEmitter;

use super::emitters::{
    emitter_simulation_space, emitter_uses_dynamic_wind, emitter_uses_inherit_position,
    emitter_uses_inherit_velocity, emitter_uses_sphere_invert_velocity,
};
use super::visuals::{
    SizeVariationModifier, TwinkleSizeModifier, build_color_gradient,
    build_offset_by_spin_modifier, build_size_gradient, has_authored_size_variation,
    has_authored_twinkle,
};
use super::{
    BLEND_ADD, BLEND_ADD_ALPHA, BLEND_ALPHA, BLEND_ALPHA_3, BLEND_ALPHA_KEY, BLEND_MOD,
    BLEND_MOD2X, BLEND_OPAQUE, CHILD_EMITTER_FPS_APPROXIMATION, DYNAMIC_WIND_ACCEL_PROPERTY,
    INHERIT_POSITION_BACK_DELTA_PROPERTY, PARTICLE_FLAG_NEGATE_SPIN, PARTICLE_FLAG_NO_GLOBAL_SCALE,
    PARTICLE_FLAG_RANDOM_TEXTURE, PARTICLE_FLAG_SIZE_VARIATION_2D, PARTICLE_FLAG_VELOCITY_ORIENT,
    PARTICLE_FLAG_WIND_DYNAMIC, PARTICLE_FLAG_WIND_ENABLED, PARTICLE_FLAG_XY_QUAD,
    PARTICLE_TYPE_TRAIL, ParticleSpawnMode, ParticleSpawnSource,
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
    } = build_expr_modifiers(em, model_scale);
    let setup = build_effect_runtime_setup(
        em,
        particle_density_multiplier,
        spawn_mode,
        spawn_source,
        child_emitters,
    );
    let effect = assemble_effect(
        em,
        module,
        setup.spawner,
        setup.max_particles,
        alpha_mode,
        init,
        gravity,
        orient_rotation,
        model_scale,
        spawn_source,
        setup.child_event_counts,
    );
    apply_effect_runtime_modifiers(
        effect,
        em,
        drag,
        flipbook_sprite_index_init,
        flipbook_sprite_index_update,
        texture,
        twinkle,
        size_variation,
    )
}

struct EffectRuntimeSetup {
    spawner: SpawnerSettings,
    max_particles: u32,
    child_event_counts: Vec<u32>,
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
    let global_scale = if em.flags & PARTICLE_FLAG_NO_GLOBAL_SCALE != 0 {
        1.0
    } else {
        particle_density_multiplier.clamp(0.1, 1.0)
    };
    (mean_rate * global_scale).max(0.1)
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
    let per_frame =
        scaled_emission_rate(em, particle_density_multiplier) / CHILD_EMITTER_FPS_APPROXIMATION;
    per_frame.ceil().max(1.0) as u32
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

fn build_twinkle_seed_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    if !has_authored_twinkle(em) {
        return None;
    }
    Some(SetAttributeModifier::new(
        Attribute::F32_3,
        writer.rand(ScalarType::Float).expr(),
    ))
}

fn build_size_variation_modifier_attr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    if !has_authored_size_variation(em) {
        return None;
    }
    let authored_y = if em.flags & PARTICLE_FLAG_SIZE_VARIATION_2D != 0 {
        em.scale_variation_y
    } else {
        em.scale_variation
    };
    let x = size_variation_expr(em.scale_variation, writer)?;
    let y = size_variation_expr(authored_y, writer)?;
    Some(SetAttributeModifier::new(
        Attribute::F32X2_0,
        x.vec2(y).expr(),
    ))
}

fn authored_spin_expr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    base: f32,
    variation: f32,
) -> Option<ExprHandle> {
    if !has_authored_spin(em) {
        return None;
    }
    if variation > 0.0 {
        let offset =
            writer.rand(ScalarType::Float) * writer.lit(variation * 2.0) - writer.lit(variation);
        Some((writer.lit(base) + offset).expr())
    } else {
        Some(writer.lit(base).expr())
    }
}

fn build_spin_sign_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    if em.flags & PARTICLE_FLAG_NEGATE_SPIN == 0 || !has_authored_spin(em) {
        return None;
    }
    let negate = writer
        .rand(ScalarType::Float)
        .lt(writer.lit(0.5))
        .cast(ScalarType::Float);
    let sign = writer.lit(1.0) - negate * writer.lit(2.0);
    Some(SetAttributeModifier::new(
        Attribute::F32X2_1,
        sign.vec2(writer.lit(0.0)).expr(),
    ))
}

fn size_variation_expr(variation: f32, writer: &ExprWriter) -> Option<WriterExpr> {
    if variation == 0.0 {
        return None;
    }
    let random = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let scale = (writer.lit(1.0) + random * writer.lit(variation)).max(writer.lit(0.01));
    Some(scale)
}

fn build_orient_rotation_expr(em: &M2ParticleEmitter, writer: &ExprWriter) -> Option<ExprHandle> {
    if !has_authored_spin(em) {
        return None;
    }
    let angle = writer.attr(Attribute::F32_0);
    let angular_velocity = writer.attr(Attribute::F32_1);
    let age = writer.attr(Attribute::AGE);
    let rotation = angle + angular_velocity * age;
    if em.flags & PARTICLE_FLAG_NEGATE_SPIN != 0 {
        let sign = writer.attr(Attribute::F32X2_1).x();
        Some((rotation * sign).expr())
    } else {
        Some(rotation.expr())
    }
}

pub(crate) fn has_authored_spin(em: &M2ParticleEmitter) -> bool {
    em.base_spin != 0.0
        || em.base_spin_variation != 0.0
        || em.spin != 0.0
        || em.spin_variation != 0.0
}

pub(crate) fn has_authored_wind(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_WIND_ENABLED != 0
        && em.flags & PARTICLE_FLAG_WIND_DYNAMIC == 0
        && em.wind_time > 0.0
        && em.wind_vector.iter().any(|&value| value != 0.0)
}

fn build_twinkle_modifier(em: &M2ParticleEmitter) -> Option<TwinkleSizeModifier> {
    has_authored_twinkle(em).then(|| TwinkleSizeModifier {
        speed_steps: em.twinkle_speed.max(0.0),
        visible_ratio: em.twinkle_percent.clamp(0.0, 1.0),
        scale_min: em.twinkle_scale_min.max(0.0),
        scale_max: em.twinkle_scale_max.max(em.twinkle_scale_min.max(0.0)),
    })
}

fn build_size_variation_modifier(em: &M2ParticleEmitter) -> Option<SizeVariationModifier> {
    has_authored_size_variation(em).then_some(SizeVariationModifier)
}

pub(crate) fn wind_accel_bevy(em: &M2ParticleEmitter, model_scale: f32) -> Vec3 {
    let [x, y, z] = em.wind_vector;
    Vec3::from(wow_to_bevy(x, y, z)) * model_scale
}

pub(crate) fn gravity_accel_bevy(em: &M2ParticleEmitter) -> Vec3 {
    let [x, y, z] = em.gravity_vector;
    Vec3::from(wow_to_bevy(x, y, z))
}

pub(crate) fn wind_strength_at_age(age: f32, wind_time: f32) -> f32 {
    if wind_time > 0.0 && age <= wind_time {
        1.0
    } else {
        0.0
    }
}

pub(crate) fn build_position_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> PositionInitModifier {
    match em.emitter_type {
        1 => PositionInitModifier::Attribute(SetAttributeModifier::new(
            Attribute::POSITION,
            build_inherit_position_expr(
                em,
                writer,
                build_plane_position_expr(em, writer, model_scale),
            ),
        )),
        2 => PositionInitModifier::Sphere(SetPositionSphereModifier {
            center: writer.lit(Vec3::ZERO).expr(),
            radius: writer.lit(emitter_spawn_radius(em) * model_scale).expr(),
            dimension: ShapeDimension::Volume,
        }),
        _ => PositionInitModifier::Attribute(SetAttributeModifier::new(
            Attribute::POSITION,
            build_inherit_position_expr(em, writer, writer.lit(Vec3::ZERO)),
        )),
    }
}

fn build_inherit_position_expr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    base_position: WriterExpr,
) -> ExprHandle {
    if !emitter_uses_inherit_position(em) {
        return base_position.expr();
    }
    let back_delta = writer.add_property(INHERIT_POSITION_BACK_DELTA_PROPERTY, Vec3::ZERO.into());
    let offset = writer.rand(ScalarType::Float) * writer.prop(back_delta);
    (base_position + offset).expr()
}

pub(crate) fn emitter_spawn_radius(em: &M2ParticleEmitter) -> f32 {
    if em.emitter_type == 2 && (em.area_length > 0.0 || em.area_width > 0.0) {
        em.area_length.max(em.area_width).max(0.01)
    } else {
        0.0
    }
}

fn build_plane_position_expr(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> WriterExpr {
    let half_length = writer.lit(em.area_length.max(0.0) * model_scale);
    let half_width = writer.lit(em.area_width.max(0.0) * model_scale);
    let x = writer.rand(ScalarType::Float) * half_length.clone() * writer.lit(2.0) - half_length;
    let z = writer.rand(ScalarType::Float) * half_width.clone() * writer.lit(2.0) - half_width;
    x.vec3(writer.lit(0.0), z)
}

fn build_velocity_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> SetAttributeModifier {
    let speed = build_speed_expr(em, writer) * writer.lit(model_scale);
    if em.z_source > 0.0 {
        return build_z_source_velocity_modifier(em, writer, speed);
    }
    if emitter_uses_sphere_invert_velocity(em) {
        return build_sphere_invert_velocity_modifier(writer, speed);
    }
    let yaw = writer.rand(ScalarType::Float) * writer.lit(em.horizontal_range);
    let pitch = writer.rand(ScalarType::Float) * writer.lit(em.vertical_range);
    let sin_p = pitch.clone().sin();
    let cos_p = pitch.cos();
    let vx = sin_p.clone() * yaw.clone().cos() * speed.clone();
    let vy = cos_p * speed.clone();
    let vz = sin_p * yaw.sin() * speed;
    SetAttributeModifier::new(Attribute::VELOCITY, vx.vec3(vy, vz).expr())
}

fn build_sphere_invert_velocity_modifier(
    writer: &ExprWriter,
    speed: WriterExpr,
) -> SetAttributeModifier {
    let pos = writer.attr(Attribute::POSITION);
    let inward = (writer.lit(Vec3::ZERO) - pos).normalized();
    SetAttributeModifier::new(Attribute::VELOCITY, (inward * speed).expr())
}

fn build_z_source_velocity_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    speed: WriterExpr,
) -> SetAttributeModifier {
    let pos = writer.attr(Attribute::POSITION);
    let source = writer.lit(Vec3::new(0.0, 0.0, em.z_source));
    let direction = (pos - source).normalized();
    SetAttributeModifier::new(Attribute::VELOCITY, (direction * speed).expr())
}

fn build_speed_expr(em: &M2ParticleEmitter, writer: &ExprWriter) -> WriterExpr {
    if em.speed_variation > 0.0 {
        let var = writer.rand(ScalarType::Float) * writer.lit(em.speed_variation * 2.0)
            - writer.lit(em.speed_variation);
        writer.lit(em.emission_speed) * (writer.lit(1.0) + var)
    } else {
        writer.lit(em.emission_speed)
    }
}

pub(crate) fn emitter_alpha_mode(
    blend_type: u8,
    mask_cutoff: ExprHandle,
) -> bevy_hanabi::AlphaMode {
    match blend_type {
        BLEND_OPAQUE => bevy_hanabi::AlphaMode::Opaque,
        BLEND_ALPHA_KEY => bevy_hanabi::AlphaMode::Mask(mask_cutoff),
        BLEND_ALPHA | BLEND_ALPHA_3 | BLEND_MOD2X => bevy_hanabi::AlphaMode::Blend,
        BLEND_ADD | BLEND_ADD_ALPHA | BLEND_MOD => bevy_hanabi::AlphaMode::Add,
        _ => bevy_hanabi::AlphaMode::Blend,
    }
}

fn is_trail_particle(em: &M2ParticleEmitter) -> bool {
    em.particle_type == PARTICLE_TYPE_TRAIL
}

pub(crate) fn load_emitter_texture(
    em: &M2ParticleEmitter,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let fdid = em.texture_fdid?;
    let path = PathBuf::from(format!("data/textures/{fdid}.blp"));
    if !path.exists() {
        return None;
    }
    let image = blp::load_blp_gpu_image(&path).ok()?;
    Some(images.add(image))
}
