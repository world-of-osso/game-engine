//! Minimal public particle effect builder for library/benchmark use.
//!
//! This intentionally exposes only the pure EffectAsset construction path for
//! a single emitter. The ECS/runtime particle systems stay in
//! `rendering/particles/mod.rs`.

#[path = "particle_effect_builder_motion.rs"]
mod motion;
#[path = "rendering/particles/visuals.rs"]
mod visuals;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::asset::m2::wow_to_bevy;
use crate::asset::m2_particle::M2ParticleEmitter;
use motion::{
    build_position_modifier, build_size_variation_modifier, build_twinkle_modifier,
    build_velocity_modifier, emitter_alpha_mode, emitter_simulation_space,
    emitter_uses_dynamic_wind, gravity_accel_bevy, has_authored_spin, has_authored_wind,
    is_trail_particle, wind_accel_bevy,
};
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

struct ExprModifiers {
    init: InitModifiers,
    gravity: AccelModifier,
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<TwinkleSizeModifier>,
    size_variation: Option<SizeVariationModifier>,
    alpha_mode: bevy_hanabi::AlphaMode,
    orient_rotation: Option<ExprHandle>,
    module: Module,
}

struct EffectRuntimeModifiers {
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<TwinkleSizeModifier>,
    size_variation: Option<SizeVariationModifier>,
}

fn build_effect_runtime_modifiers(
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<TwinkleSizeModifier>,
    size_variation: Option<SizeVariationModifier>,
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

fn build_particle_effect_inputs(
    em: &M2ParticleEmitter,
    model_scale: f32,
    particle_density_multiplier: f32,
) -> (EffectAssembleParts, EffectRuntimeModifiers) {
    build_particle_effect_inputs_from_expr(
        em,
        particle_density_multiplier,
        model_scale,
        build_expr_modifiers(em, model_scale),
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

fn build_runtime_modifiers_from_expr(
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index_init: Option<SetAttributeModifier>,
    flipbook_sprite_index_update: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    twinkle: Option<TwinkleSizeModifier>,
    size_variation: Option<SizeVariationModifier>,
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

fn build_runtime_particle_effect(
    em: &M2ParticleEmitter,
    runtime_modifiers: EffectRuntimeModifiers,
    parts: EffectAssembleParts,
) -> EffectAsset {
    let effect = assemble_effect(em, parts);
    apply_effect_runtime_modifiers(effect, em, runtime_modifiers)
}

struct EffectAssembleParts {
    module: Module,
    spawner: SpawnerSettings,
    max_particles: u32,
    alpha_mode: bevy_hanabi::AlphaMode,
    init: InitModifiers,
    gravity: AccelModifier,
    orient_rotation: Option<ExprHandle>,
    model_scale: f32,
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

fn build_expr_modifiers(em: &M2ParticleEmitter, model_scale: f32) -> ExprModifiers {
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
enum FlipbookSpriteMode {
    FirstCell,
    CellTrack([u16; 3]),
    RandomCell,
}

fn flipbook_sprite_mode(em: &M2ParticleEmitter) -> Option<FlipbookSpriteMode> {
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

fn active_cell_track(em: &M2ParticleEmitter) -> Option<[u16; 3]> {
    if em.head_cell_track.iter().any(|&cell| cell != 0) {
        Some(em.head_cell_track)
    } else if em.tail_cell_track.iter().any(|&cell| cell != 0) {
        Some(em.tail_cell_track)
    } else {
        None
    }
}

fn build_cell_track_sprite_index(
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

fn scaled_emission_rate(em: &M2ParticleEmitter, particle_density_multiplier: f32) -> f32 {
    let mean_rate = em.emission_rate + em.emission_rate_variation.max(0.0) * 0.5;
    let global_scale = if em.flags & PARTICLE_FLAG_NO_GLOBAL_SCALE != 0 {
        1.0
    } else {
        particle_density_multiplier.clamp(0.1, 1.0)
    };
    (mean_rate * global_scale).max(0.1)
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
    let effect = build_base_effect(
        em,
        BaseEffectParts {
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
        },
    );
    let effect = add_position_init(effect, pos);
    add_optional_init_modifiers(
        effect,
        rotation,
        angular_velocity,
        spin_sign,
        twinkle_phase,
        twinkle_enabled,
        size_variation,
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

fn orient_mode(em: &M2ParticleEmitter) -> OrientMode {
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

struct InitModifiers {
    age: SetAttributeModifier,
    lifetime: SetAttributeModifier,
    pos: PositionInitModifier,
    vel: SetAttributeModifier,
    rotation: Option<SetAttributeModifier>,
    angular_velocity: Option<SetAttributeModifier>,
    spin_sign: Option<SetAttributeModifier>,
    twinkle_phase: Option<SetAttributeModifier>,
    twinkle_enabled: Option<SetAttributeModifier>,
    size_variation: Option<SetAttributeModifier>,
}

enum PositionInitModifier {
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

fn lifetime_range(em: &M2ParticleEmitter) -> (f32, f32) {
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
