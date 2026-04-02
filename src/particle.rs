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
    _meshes: &mut Assets<Mesh>,
    _materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    emitters: &[M2ParticleEmitter],
    _bones: &[M2Bone],
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) {
    for em in emitters {
        spawn_single_emitter(commands, images, em, _bones, bone_entities, parent);
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
    let pos = emitter_translation(em, bones);
    let bone_pivot = bones
        .get(em.bone_index as usize)
        .map(|b| Vec3::new(b.pivot[0], b.pivot[2], -b.pivot[1]))
        .unwrap_or(Vec3::ZERO);
    pos - bone_pivot
}

fn emitter_translation(em: &M2ParticleEmitter, _bones: &[M2Bone]) -> Vec3 {
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

fn sample_cell_track_frame(
    track: [u16; 3],
    mid_point: f32,
    age_ratio: f32,
    total_cells: u32,
) -> u32 {
    let mid = mid_point.clamp(0.01, 0.99);
    let t = age_ratio.clamp(0.0, 1.0);
    let frame = if t < mid {
        let segment_t = (t / mid).clamp(0.0, 1.0);
        (track[0] as f32) + ((track[1] as f32) - (track[0] as f32)) * segment_t
    } else {
        let segment_t = ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0);
        (track[1] as f32) + ((track[2] as f32) - (track[1] as f32)) * segment_t
    };
    frame
        .floor()
        .clamp(0.0, total_cells.saturating_sub(1) as f32) as u32
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
    if em.flags & 0x08 != 0 {
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
        0 => bevy_hanabi::AlphaMode::Opaque,
        1 => bevy_hanabi::AlphaMode::Mask(mask_cutoff),
        2 | 3 | 7 => bevy_hanabi::AlphaMode::Blend,
        4..=6 => bevy_hanabi::AlphaMode::Add,
        _ => bevy_hanabi::AlphaMode::Blend,
    }
}

fn build_color_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    if em.opacity_keys.len() >= 2 {
        return build_fake_animblock_opacity_gradient(em);
    }
    build_simple_color_gradient(em)
}

fn build_simple_color_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    let [c0, c1, c2] = em.colors;
    let [o0, o1, o2] = em.opacity;
    let mid = em.mid_point.clamp(0.01, 0.99);
    let mut g = bevy_hanabi::Gradient::new();
    g.add_key(
        0.0,
        Vec4::new(c0[0] / 255.0, c0[1] / 255.0, c0[2] / 255.0, o0),
    );
    g.add_key(
        mid,
        Vec4::new(c1[0] / 255.0, c1[1] / 255.0, c1[2] / 255.0, o1),
    );
    g.add_key(
        1.0,
        Vec4::new(c2[0] / 255.0, c2[1] / 255.0, c2[2] / 255.0, o2),
    );
    g
}

fn build_fake_animblock_opacity_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
    let mut g = bevy_hanabi::Gradient::new();
    for &(time, opacity) in &em.opacity_keys {
        let color = sample_fake_animblock_color(em, time);
        g.add_key(time, Vec4::new(color.x, color.y, color.z, opacity));
    }
    g
}

fn sample_fake_animblock_color(em: &M2ParticleEmitter, time: f32) -> Vec3 {
    let t = time.clamp(0.0, 1.0);
    let mid = em.mid_point.clamp(0.01, 0.99);
    let c0 = Vec3::new(
        em.colors[0][0] / 255.0,
        em.colors[0][1] / 255.0,
        em.colors[0][2] / 255.0,
    );
    let c1 = Vec3::new(
        em.colors[1][0] / 255.0,
        em.colors[1][1] / 255.0,
        em.colors[1][2] / 255.0,
    );
    let c2 = Vec3::new(
        em.colors[2][0] / 255.0,
        em.colors[2][1] / 255.0,
        em.colors[2][2] / 255.0,
    );
    if t < mid {
        c0.lerp(c1, (t / mid).clamp(0.0, 1.0))
    } else {
        c1.lerp(c2, ((t - mid) / (1.0 - mid)).clamp(0.0, 1.0))
    }
}

fn build_size_gradient(em: &M2ParticleEmitter, model_scale: f32) -> bevy_hanabi::Gradient<Vec3> {
    let mid = em.mid_point.clamp(0.01, 0.99);
    let burst = em.burst_multiplier.max(0.0);
    let mut g = bevy_hanabi::Gradient::new();
    g.add_key(
        0.0,
        Vec3::splat(em.scales[0][0].max(0.01) * burst * model_scale),
    );
    g.add_key(
        mid,
        Vec3::splat(em.scales[1][0].max(0.01) * burst * model_scale),
    );
    g.add_key(
        1.0,
        Vec3::splat(em.scales[2][0].max(0.01) * burst * model_scale),
    );
    g
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
mod tests {
    use bevy::prelude::Vec3;
    use bevy_hanabi::{AlphaMode, ExprWriter};

    use super::{
        active_cell_track, build_color_gradient, build_effect_asset, build_expr_modifiers,
        build_size_gradient, emitter_alpha_mode, emitter_rate_scale, emitter_spawn_radius,
        emitter_translation, is_fire_effect, sample_cell_track_frame,
    };
    use crate::asset::m2_anim::M2Bone;
    use crate::asset::m2_particle::M2ParticleEmitter;

    fn sample_emitter() -> M2ParticleEmitter {
        M2ParticleEmitter {
            flags: 0,
            position: [0.0, 0.0, 0.0],
            bone_index: 0,
            texture_index: 0,
            texture_fdid: None,
            blend_type: 4,
            emitter_type: 1,
            tile_rows: 4,
            tile_cols: 4,
            emission_speed: 1.0,
            speed_variation: 0.0,
            vertical_range: 0.1,
            horizontal_range: std::f32::consts::TAU,
            gravity: 0.0,
            lifespan: 1.0,
            emission_rate: 20.0,
            area_length: 0.1,
            area_width: 0.1,
            drag: 0.0,
            colors: [[255.0, 128.0, 64.0]; 3],
            opacity: [1.0, 1.0, 0.0],
            opacity_keys: Vec::new(),
            scales: [[0.1, 0.1], [0.2, 0.2], [0.05, 0.05]],
            head_cell_track: [0, 0, 0],
            tail_cell_track: [0, 0, 0],
            burst_multiplier: 1.0,
            mid_point: 0.5,
        }
    }

    fn sample_bone(pivot: [f32; 3]) -> M2Bone {
        M2Bone {
            key_bone_id: 0,
            flags: 0,
            parent_bone_id: -1,
            submesh_id: 0,
            pivot,
        }
    }

    #[test]
    fn textured_emitters_declare_hanabi_texture_slot() {
        let mut emitter = sample_emitter();
        emitter.texture_fdid = Some(145513);

        let asset = build_effect_asset(&emitter, 1.0);

        assert_eq!(asset.texture_layout().layout.len(), 1);
        assert_eq!(asset.texture_layout().layout[0].name, "color");
    }

    #[test]
    fn untextured_emitters_do_not_declare_hanabi_texture_slot() {
        let emitter = sample_emitter();
        let modifiers = build_expr_modifiers(&emitter, 1.0);

        assert!(modifiers.module.texture_layout().layout.is_empty());
    }

    #[test]
    fn emitter_translation_uses_raw_model_position() {
        let mut emitter = sample_emitter();
        emitter.position = [1.0, 2.0, 3.0];
        let bones = vec![sample_bone([0.25, 0.5, 0.75])];

        let translation = emitter_translation(&emitter, &bones);

        assert_eq!(translation, Vec3::new(1.0, 3.0, -2.0));
    }

    #[test]
    fn sphere_emitters_use_area_as_spawn_radius() {
        let mut emitter = sample_emitter();
        emitter.emitter_type = 1;
        emitter.area_length = 0.4;
        emitter.area_width = 0.2;

        assert_eq!(emitter_spawn_radius(&emitter), 0.2);
    }

    #[test]
    fn non_sphere_emitters_do_not_expand_spawn_radius() {
        let mut emitter = sample_emitter();
        emitter.emitter_type = 0;
        emitter.area_length = 0.4;
        emitter.area_width = 0.2;

        assert_eq!(emitter_spawn_radius(&emitter), 0.0);
    }

    #[test]
    fn fire_emitters_use_four_x_rate_scale() {
        let mut emitter = sample_emitter();
        emitter.texture_fdid = Some(145513);

        assert!(is_fire_effect(&emitter));
        assert_eq!(emitter_rate_scale(&emitter), 4.0);
    }

    #[test]
    fn non_fire_emitters_keep_default_rate_scale() {
        let mut emitter = sample_emitter();
        emitter.texture_fdid = None;

        assert!(!is_fire_effect(&emitter));
        assert_eq!(emitter_rate_scale(&emitter), 1.0);
    }

    #[test]
    fn burst_multiplier_scales_particle_size_gradient() {
        let mut emitter = sample_emitter();
        emitter.burst_multiplier = 2.5;

        let gradient = build_size_gradient(&emitter, 1.0);
        let keys = gradient.keys();

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].value, Vec3::splat(0.25));
        assert_eq!(keys[1].value, Vec3::splat(0.5));
        assert_eq!(keys[2].value, Vec3::splat(0.125));
    }

    #[test]
    fn color_gradient_uses_full_opacity_key_timeline_when_present() {
        let mut emitter = sample_emitter();
        emitter.colors = [[255.0, 0.0, 0.0], [0.0, 255.0, 0.0], [0.0, 0.0, 255.0]];
        emitter.mid_point = 0.5;
        emitter.opacity_keys = vec![(0.0, 0.1), (0.25, 0.4), (0.75, 0.8), (1.0, 0.2)];

        let gradient = build_color_gradient(&emitter);
        let keys = gradient.keys();

        assert_eq!(keys.len(), 4);
        assert_eq!(keys[0].ratio(), 0.0);
        assert_eq!(keys[0].value.w, 0.1);
        assert!((keys[1].ratio() - 0.25).abs() < 0.0001);
        assert!((keys[1].value.x - 0.5).abs() < 0.0001);
        assert!((keys[1].value.y - 0.5).abs() < 0.0001);
        assert_eq!(keys[1].value.w, 0.4);
        assert!((keys[2].ratio() - 0.75).abs() < 0.0001);
        assert!((keys[2].value.y - 0.5).abs() < 0.0001);
        assert!((keys[2].value.z - 0.5).abs() < 0.0001);
        assert_eq!(keys[2].value.w, 0.8);
        assert_eq!(keys[3].ratio(), 1.0);
        assert_eq!(keys[3].value.w, 0.2);
    }

    #[test]
    fn active_cell_track_prefers_head_track() {
        let mut emitter = sample_emitter();
        emitter.head_cell_track = [2, 4, 6];
        emitter.tail_cell_track = [7, 8, 9];

        assert_eq!(active_cell_track(&emitter), Some([2, 4, 6]));
    }

    #[test]
    fn active_cell_track_falls_back_to_tail_track() {
        let mut emitter = sample_emitter();
        emitter.tail_cell_track = [3, 5, 7];

        assert_eq!(active_cell_track(&emitter), Some([3, 5, 7]));
    }

    #[test]
    fn sample_cell_track_frame_uses_midpoint_segments() {
        let track = [2, 6, 10];

        assert_eq!(sample_cell_track_frame(track, 0.25, 0.0, 16), 2);
        assert_eq!(sample_cell_track_frame(track, 0.25, 0.25, 16), 6);
        assert_eq!(sample_cell_track_frame(track, 0.25, 0.625, 16), 8);
        assert_eq!(sample_cell_track_frame(track, 0.25, 1.0, 16), 10);
    }

    #[test]
    fn particle_blend_type_zero_is_opaque() {
        let writer = ExprWriter::new();
        let alpha_mode = emitter_alpha_mode(0, writer.lit(0.5_f32).expr());

        assert!(matches!(alpha_mode, AlphaMode::Opaque));
    }

    #[test]
    fn particle_blend_type_one_uses_alpha_key() {
        let writer = ExprWriter::new();
        let alpha_mode = emitter_alpha_mode(1, writer.lit(0.5_f32).expr());

        assert!(matches!(alpha_mode, AlphaMode::Mask(_)));
    }

    #[test]
    fn particle_blend_type_three_uses_alpha_blend() {
        let writer = ExprWriter::new();
        let alpha_mode = emitter_alpha_mode(3, writer.lit(0.5_f32).expr());

        assert!(matches!(alpha_mode, AlphaMode::Blend));
    }

    #[test]
    fn particle_blend_type_four_is_additive() {
        let writer = ExprWriter::new();
        let alpha_mode = emitter_alpha_mode(4, writer.lit(0.5_f32).expr());

        assert!(matches!(alpha_mode, AlphaMode::Add));
    }

    #[test]
    fn torch_emitter_translation_matches_particle_position() {
        let path = std::path::Path::new("data/models/club_1h_torch_a_01.m2");
        if !path.exists() {
            return;
        }

        let skin_fdids = [0_u32; 3];
        let model = crate::asset::m2::load_m2_uncached(path, &skin_fdids).unwrap();
        let emitter = model.particle_emitters.into_iter().next().unwrap();

        let translation = emitter_translation(&emitter, &model.bones);

        let expected = Vec3::new(0.63709766, -0.07413276, 0.0009614461);
        assert!(
            translation.distance(expected) < 0.00001,
            "translation={translation:?}"
        );
    }
}
