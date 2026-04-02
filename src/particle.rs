//! M2 particle emitter rendering — GPU particles via bevy_hanabi.
//!
//! Each M2 emitter is translated to a bevy_hanabi `EffectAsset` and spawned as
//! a `ParticleEffect` entity parented to the model (or its bone).

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::asset::blp;
use crate::asset::m2::wow_to_bevy;
use crate::asset::m2_anim::M2Bone;
use crate::asset::m2_particle::M2ParticleEmitter;

const PARTICLE_FLAG_ALONG_VELOCITY: u32 = 0x08;
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

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)
            .add_systems(Update, register_pending_particle_effects);
    }
}

/// Marker for a particle emitter entity.
///
/// The effect asset is built lazily in `register_pending_particle_effects`
/// once `GlobalTransform` is available, so the model/root scale is
/// automatically baked into particle size, spawn radius, and velocity.
#[derive(Component)]
pub struct ParticleEmitterComp {
    pub emitter: M2ParticleEmitter,
    pub bone_entity: Option<Entity>,
    pub scale_source: Entity,
    /// Optional texture handle to attach via `EffectMaterial`.
    pending_texture: Option<Handle<Image>>,
}

/// System: build `EffectAsset` from emitter data + model/root scale, then
/// register as `ParticleEffect`.
fn register_pending_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    query: Query<(Entity, &ParticleEmitterComp), Without<ParticleEffect>>,
    global_transforms: Query<&GlobalTransform>,
) {
    for (entity, comp) in &query {
        let model_scale = global_transforms
            .get(comp.scale_source)
            .map(|tf| tf.compute_transform().scale.x)
            .unwrap_or(1.0);
        let asset = build_effect_asset(&comp.emitter, model_scale);
        let handle = effects.add(asset);
        let mut ec = commands.entity(entity);
        ec.insert(ParticleEffect::new(handle));
        if let Some(tex) = comp.pending_texture.clone() {
            ec.insert(EffectMaterial { images: vec![tex] });
        }
    }
}

/// Spawn emitter entities for an M2 model's particle emitters.
pub fn spawn_emitters(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    emitters: &[M2ParticleEmitter],
    bones: &[M2Bone],
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) {
    for em in emitters {
        spawn_single_emitter(commands, images, em, bones, bone_entities, parent);
    }
}

fn spawn_single_emitter(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    em: &M2ParticleEmitter,
    bones: &[M2Bone],
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) {
    let bone_entity = bone_entities.and_then(|b| b.get(em.bone_index as usize).copied());
    let pending_texture = load_emitter_texture(em, images);
    let parent_entity = bone_entity.unwrap_or(parent);
    let local_offset = emitter_local_offset(em, bones);
    commands
        .spawn((
            Name::new("ParticleEmitter"),
            ParticleEmitterComp {
                emitter: em.clone(),
                bone_entity,
                scale_source: parent,
                pending_texture,
            },
            Transform::from_translation(local_offset),
            Visibility::default(),
        ))
        .set_parent_in_place(parent_entity);
}

/// Emitter position relative to its parent bone.
fn emitter_local_offset(em: &M2ParticleEmitter, bones: &[M2Bone]) -> Vec3 {
    let pos = emitter_translation(em);
    let bone_pivot = bones
        .get(em.bone_index as usize)
        .map(|b| Vec3::new(b.pivot[0], b.pivot[2], -b.pivot[1]))
        .unwrap_or(Vec3::ZERO);
    pos - bone_pivot
}

fn emitter_translation(em: &M2ParticleEmitter) -> Vec3 {
    let pos = em.position;
    Vec3::from(wow_to_bevy(pos[0], pos[1], pos[2]))
}

struct ExprModifiers {
    init: InitModifiers,
    gravity: AccelModifier,
    drag: Option<LinearDragModifier>,
    flipbook_sprite_index: Option<SetAttributeModifier>,
    texture: Option<ParticleTextureModifier>,
    alpha_mode: bevy_hanabi::AlphaMode,
    module: Module,
}

fn build_expr_modifiers(em: &M2ParticleEmitter, model_scale: f32) -> ExprModifiers {
    let writer = ExprWriter::new();
    let init = build_init_modifiers(em, &writer, model_scale);
    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, -em.gravity, 0.0)).expr());
    let drag = (em.drag > 0.0).then(|| LinearDragModifier::new(writer.lit(em.drag).expr()));
    let flipbook_sprite_index = build_flipbook_sprite_index(em, &writer);
    let mask_cutoff = writer.lit(0.5_f32).expr();
    let texture = em.texture_fdid.map(|_| ParticleTextureModifier {
        texture_slot: writer.lit(0u32).expr(),
        sample_mapping: ImageSampleMapping::Modulate,
    });
    let alpha_mode = emitter_alpha_mode(em.blend_type, mask_cutoff);
    let mut module = writer.finish();
    if texture.is_some() {
        module.add_texture_slot("color");
    }
    ExprModifiers {
        init,
        gravity,
        drag,
        flipbook_sprite_index,
        texture,
        alpha_mode,
        module,
    }
}

fn build_flipbook_sprite_index(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
) -> Option<SetAttributeModifier> {
    if em.tile_rows <= 1 && em.tile_cols <= 1 {
        return None;
    }
    let total = (em.tile_rows as i32) * (em.tile_cols as i32);
    let frame = if let Some(track) = active_cell_track(em) {
        build_cell_track_sprite_index(writer, track, em.mid_point, total)
    } else {
        let age = writer.attr(Attribute::AGE);
        let lifetime = writer.attr(Attribute::LIFETIME);
        (age / lifetime * writer.lit(total as f32))
            .cast(ScalarType::Int)
            .rem(writer.lit(total))
    };
    Some(SetAttributeModifier::new(
        Attribute::SPRITE_INDEX,
        frame.expr(),
    ))
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

fn build_effect_asset(em: &M2ParticleEmitter, model_scale: f32) -> EffectAsset {
    let m = build_expr_modifiers(em, model_scale);
    let emission_rate = (em.emission_rate * emitter_rate_scale(em)).max(0.1);
    let max_particles = ((emission_rate * em.lifespan) as u32).clamp(16, 4096);
    let spawner = SpawnerSettings::rate(emission_rate.into());

    let mut effect = assemble_effect(
        em,
        m.module,
        spawner,
        max_particles,
        m.alpha_mode,
        m.init,
        m.gravity,
        model_scale,
    );
    if let Some(drag) = m.drag {
        effect = effect.update(drag);
    }
    if let Some(sprite_idx) = m.flipbook_sprite_index {
        effect = effect.update(sprite_idx);
    }
    if let Some(tex) = m.texture {
        effect = effect.render(tex);
    }
    if em.tile_rows > 1 || em.tile_cols > 1 {
        effect = effect.render(FlipbookModifier {
            sprite_grid_size: UVec2::new(em.tile_cols as u32, em.tile_rows as u32),
        });
    }
    effect
}

fn emitter_rate_scale(em: &M2ParticleEmitter) -> f32 {
    if is_fire_effect(em) { 4.0 } else { 1.0 }
}

fn is_fire_effect(em: &M2ParticleEmitter) -> bool {
    em.texture_fdid.is_some() && em.blend_type >= 4 && (em.tile_rows > 1 || em.tile_cols > 1)
}

fn assemble_effect(
    em: &M2ParticleEmitter,
    module: Module,
    spawner: SpawnerSettings,
    max_particles: u32,
    alpha_mode: bevy_hanabi::AlphaMode,
    init: InitModifiers,
    gravity: AccelModifier,
    model_scale: f32,
) -> EffectAsset {
    EffectAsset::new(max_particles, spawner, module)
        .with_name("m2_particle")
        .with_alpha_mode(alpha_mode)
        .with_simulation_space(SimulationSpace::Global)
        .init(init.age)
        .init(init.lifetime)
        .init(init.pos)
        .init(init.vel)
        .update(gravity)
        .render(build_color_render_modifier(em))
        .render(SizeOverLifetimeModifier {
            gradient: build_size_gradient(em, model_scale),
            screen_space_size: false,
        })
        .render(OrientModifier::new(orient_mode(em)))
}

fn orient_mode(em: &M2ParticleEmitter) -> OrientMode {
    if is_trail_particle(em) || em.flags & PARTICLE_FLAG_ALONG_VELOCITY != 0 {
        OrientMode::AlongVelocity
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
    pos: SetPositionSphereModifier,
    vel: SetAttributeModifier,
}

fn build_init_modifiers(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> InitModifiers {
    let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let lifetime =
        SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(em.lifespan.max(0.1)).expr());
    let pos = build_position_modifier(em, writer, model_scale);
    let vel = build_velocity_modifier(em, writer, model_scale);
    InitModifiers {
        age,
        lifetime,
        pos,
        vel,
    }
}

fn build_position_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> SetPositionSphereModifier {
    let radius = emitter_spawn_radius(em) * model_scale;
    SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(radius).expr(),
        dimension: ShapeDimension::Volume,
    }
}

fn emitter_spawn_radius(em: &M2ParticleEmitter) -> f32 {
    if em.emitter_type == 1 && (em.area_length > 0.0 || em.area_width > 0.0) {
        (em.area_length.max(em.area_width) * 0.5).max(0.01)
    } else {
        0.0
    }
}

fn build_velocity_modifier(
    em: &M2ParticleEmitter,
    writer: &ExprWriter,
    model_scale: f32,
) -> SetAttributeModifier {
    let speed = build_speed_expr(em, writer) * writer.lit(model_scale);
    // Cone: yaw random over horizontal_range, pitch = random within vertical_range
    let yaw = writer.rand(ScalarType::Float) * writer.lit(em.horizontal_range);
    let pitch = writer.rand(ScalarType::Float) * writer.lit(em.vertical_range);
    let sin_p = pitch.clone().sin();
    let cos_p = pitch.cos();
    let vx = sin_p.clone() * yaw.clone().cos() * speed.clone();
    let vy = cos_p * speed.clone();
    let vz = sin_p * yaw.sin() * speed;
    SetAttributeModifier::new(Attribute::VELOCITY, vx.vec3(vy, vz).expr())
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

fn emitter_alpha_mode(blend_type: u8, mask_cutoff: ExprHandle) -> bevy_hanabi::AlphaMode {
    match blend_type {
        BLEND_OPAQUE => bevy_hanabi::AlphaMode::Opaque,
        BLEND_ALPHA_KEY => bevy_hanabi::AlphaMode::Mask(mask_cutoff),
        BLEND_ALPHA | BLEND_ALPHA_3 | BLEND_MOD2X => bevy_hanabi::AlphaMode::Blend,
        BLEND_ADD | BLEND_ADD_ALPHA | BLEND_MOD => bevy_hanabi::AlphaMode::Add,
        _ => bevy_hanabi::AlphaMode::Blend,
    }
}

fn build_color_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    if em.color_keys.len() >= 2 || em.opacity_keys.len() >= 2 {
        return build_fake_animblock_gradient(em);
    }
    build_simple_color_gradient(em)
}

fn build_simple_color_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    let [c0, c1, c2] = em.colors;
    let [o0, o1, o2] = em.opacity;
    let mid = em.mid_point.clamp(0.01, 0.99);
    let mut g = bevy_hanabi::Gradient::new();
    g.add_key(0.0, vec4_with_alpha(c0, o0));
    g.add_key(mid, vec4_with_alpha(c1, o1));
    g.add_key(1.0, vec4_with_alpha(c2, o2));
    g
}

fn build_fake_animblock_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    let mut g = bevy_hanabi::Gradient::new();
    for time in fake_animblock_gradient_times(em) {
        let color = sample_fake_animblock_color(em, time);
        let opacity = sample_fake_animblock_opacity(em, time);
        g.add_key(time, Vec4::new(color.x, color.y, color.z, opacity));
    }
    g
}

fn fake_animblock_gradient_times(em: &M2ParticleEmitter) -> Vec<f32> {
    let mut times: Vec<f32> = em
        .color_keys
        .iter()
        .map(|&(time, _)| time)
        .chain(em.opacity_keys.iter().map(|&(time, _)| time))
        .collect();
    times.sort_by(|a, b| a.total_cmp(b));
    times.dedup_by(|a, b| (*a - *b).abs() < 0.0001);
    if times.is_empty() {
        vec![0.0, em.mid_point.clamp(0.01, 0.99), 1.0]
    } else {
        times
    }
}

fn sample_fake_animblock_color(em: &M2ParticleEmitter, time: f32) -> Vec3 {
    if em.color_keys.len() >= 2 {
        return sample_keyed_color(&em.color_keys, time);
    }
    let t = time.clamp(0.0, 1.0);
    let mid = em.mid_point.clamp(0.01, 0.99);
    let c0 = vec3_from_rgb255(em.colors[0]);
    let c1 = vec3_from_rgb255(em.colors[1]);
    let c2 = vec3_from_rgb255(em.colors[2]);
    if t < mid {
        c0.lerp(c1, (t / mid).clamp(0.0, 1.0))
    } else {
        c1.lerp(c2, ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0))
    }
}

fn sample_keyed_color(keys: &[(f32, [f32; 3])], time: f32) -> Vec3 {
    let t = time.clamp(0.0, 1.0);
    if t <= keys[0].0 {
        return vec3_from_rgb255(keys[0].1);
    }
    for window in keys.windows(2) {
        let [(start_t, start_c), (end_t, end_c)] = [window[0], window[1]];
        if t <= end_t {
            let span = (end_t - start_t).max(0.0001);
            let factor = ((t - start_t) / span).clamp(0.0, 1.0);
            return vec3_from_rgb255(start_c).lerp(vec3_from_rgb255(end_c), factor);
        }
    }
    vec3_from_rgb255(keys[keys.len() - 1].1)
}

fn sample_fake_animblock_opacity(em: &M2ParticleEmitter, time: f32) -> f32 {
    if em.opacity_keys.len() >= 2 {
        return sample_keyed_opacity(&em.opacity_keys, time);
    }
    let t = time.clamp(0.0, 1.0);
    let mid = em.mid_point.clamp(0.01, 0.99);
    if t < mid {
        em.opacity[0] + (em.opacity[1] - em.opacity[0]) * (t / mid).clamp(0.0, 1.0)
    } else {
        em.opacity[1] + (em.opacity[2] - em.opacity[1]) * ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0)
    }
}

fn sample_keyed_opacity(keys: &[(f32, f32)], time: f32) -> f32 {
    let t = time.clamp(0.0, 1.0);
    if t <= keys[0].0 {
        return keys[0].1;
    }
    for window in keys.windows(2) {
        let [(start_t, start_o), (end_t, end_o)] = [window[0], window[1]];
        if t <= end_t {
            let span = (end_t - start_t).max(0.0001);
            let factor = ((t - start_t) / span).clamp(0.0, 1.0);
            return start_o + (end_o - start_o) * factor;
        }
    }
    keys[keys.len() - 1].1
}

fn vec3_from_rgb255(color: [f32; 3]) -> Vec3 {
    Vec3::new(color[0] / 255.0, color[1] / 255.0, color[2] / 255.0)
}

fn vec4_with_alpha(color: [f32; 3], alpha: f32) -> Vec4 {
    let color = vec3_from_rgb255(color);
    Vec4::new(color.x, color.y, color.z, alpha)
}

fn build_size_gradient(em: &M2ParticleEmitter, model_scale: f32) -> bevy_hanabi::Gradient<Vec3> {
    let burst = em.burst_multiplier.max(0.0);
    let mut g = bevy_hanabi::Gradient::new();
    if em.scale_keys.len() >= 2 {
        for &(time, scale) in &em.scale_keys {
            g.add_key(time, size_key_value(em, scale, burst, model_scale, time));
        }
        return g;
    }
    let mid = em.mid_point.clamp(0.01, 0.99);
    g.add_key(
        0.0,
        size_key_value(em, em.scales[0], burst, model_scale, 0.0),
    );
    g.add_key(
        mid,
        size_key_value(em, em.scales[1], burst, model_scale, mid),
    );
    g.add_key(
        1.0,
        size_key_value(em, em.scales[2], burst, model_scale, 1.0),
    );
    g
}

fn size_key_value(
    em: &M2ParticleEmitter,
    scale: [f32; 2],
    burst: f32,
    model_scale: f32,
    time: f32,
) -> Vec3 {
    let width = scale[0].max(0.01) * burst * model_scale;
    let height = scale[1].max(0.01) * burst * model_scale;
    if is_trail_particle(em) {
        let trail_length =
            em.emission_speed.max(0.0) * em.lifespan.max(0.0) * TRAIL_LENGTH_FACTOR * time;
        Vec3::new(width + trail_length * model_scale, height, 1.0)
    } else {
        Vec3::new(width, height, 1.0)
    }
}

fn is_trail_particle(em: &M2ParticleEmitter) -> bool {
    em.particle_type == PARTICLE_TYPE_TRAIL
}

fn load_emitter_texture(
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

#[cfg(test)]
#[path = "../tests/unit/particle_tests.rs"]
mod tests;
