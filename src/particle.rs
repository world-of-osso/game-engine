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
/// Carries the pending effect asset until `register_pending_particle_effects`
/// picks it up and inserts the actual `ParticleEffect` + handle.
#[derive(Component)]
pub struct ParticleEmitterComp {
    pub emitter: M2ParticleEmitter,
    pub bone_entity: Option<Entity>,
    /// Unregistered effect asset, consumed by `register_pending_particle_effects`.
    pending_effect: Option<EffectAsset>,
    /// Optional texture handle to attach via `EffectMaterial`.
    pending_texture: Option<Handle<Image>>,
}

/// System: convert pending `EffectAsset`s into registered `ParticleEffect` handles.
fn register_pending_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut query: Query<(Entity, &mut ParticleEmitterComp), Without<ParticleEffect>>,
) {
    for (entity, mut comp) in &mut query {
        let Some(asset) = comp.pending_effect.take() else {
            continue;
        };
        let handle = effects.add(asset);
        let mut ec = commands.entity(entity);
        ec.insert(ParticleEffect::new(handle));
        if let Some(tex) = comp.pending_texture.take() {
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
    commands
        .spawn((
            Name::new("ParticleEmitter"),
            ParticleEmitterComp {
                emitter: em.clone(),
                bone_entity,
                pending_effect: Some(build_effect_asset(em)),
                pending_texture,
            },
            Transform::IDENTITY,
            Visibility::default(),
        ))
        .set_parent_in_place(parent_entity);
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

fn build_expr_modifiers(em: &M2ParticleEmitter) -> ExprModifiers {
    let writer = ExprWriter::new();
    let init = build_init_modifiers(em, &writer);
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
    let age = writer.attr(Attribute::AGE);
    let lifetime = writer.attr(Attribute::LIFETIME);
    let frame = (age / lifetime * writer.lit(total as f32))
        .cast(ScalarType::Int)
        .rem(writer.lit(total));
    Some(SetAttributeModifier::new(
        Attribute::SPRITE_INDEX,
        frame.expr(),
    ))
}

fn build_effect_asset(em: &M2ParticleEmitter) -> EffectAsset {
    let m = build_expr_modifiers(em);
    let max_particles = ((em.emission_rate * em.lifespan) as u32).clamp(16, 4096);
    let spawner = SpawnerSettings::rate(em.emission_rate.max(0.1).into());

    let mut effect = assemble_effect(
        em,
        m.module,
        spawner,
        max_particles,
        m.alpha_mode,
        m.init,
        m.gravity,
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

fn assemble_effect(
    em: &M2ParticleEmitter,
    module: Module,
    spawner: SpawnerSettings,
    max_particles: u32,
    alpha_mode: bevy_hanabi::AlphaMode,
    init: InitModifiers,
    gravity: AccelModifier,
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
            gradient: build_size_gradient(em),
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

fn build_init_modifiers(em: &M2ParticleEmitter, writer: &ExprWriter) -> InitModifiers {
    let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let lifetime =
        SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(em.lifespan.max(0.1)).expr());
    let pos = build_position_modifier(em, writer);
    let vel = build_velocity_modifier(em, writer);
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
) -> SetPositionSphereModifier {
    let center = emitter_translation(em, &[]);
    let radius = if em.area_length > 0.0 || em.area_width > 0.0 {
        (em.area_length.max(em.area_width) * 0.5).max(0.01)
    } else {
        0.01_f32
    };
    SetPositionSphereModifier {
        center: writer.lit(center).expr(),
        radius: writer.lit(radius).expr(),
        dimension: ShapeDimension::Volume,
    }
}

fn build_velocity_modifier(em: &M2ParticleEmitter, writer: &ExprWriter) -> SetAttributeModifier {
    let speed = build_speed_expr(em, writer);
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
        4..=6 => bevy_hanabi::AlphaMode::Add,
        1 => bevy_hanabi::AlphaMode::Mask(mask_cutoff),
        _ => bevy_hanabi::AlphaMode::Blend,
    }
}

fn build_color_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec4> {
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

fn build_size_gradient(em: &M2ParticleEmitter) -> bevy_hanabi::Gradient<Vec3> {
    let mid = em.mid_point.clamp(0.01, 0.99);
    let mut g = bevy_hanabi::Gradient::new();
    g.add_key(0.0, Vec3::splat(em.scales[0][0].max(0.01)));
    g.add_key(mid, Vec3::splat(em.scales[1][0].max(0.01)));
    g.add_key(1.0, Vec3::splat(em.scales[2][0].max(0.01)));
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

    use super::{build_effect_asset, build_expr_modifiers, emitter_translation};
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
            scales: [[0.1, 0.1], [0.2, 0.2], [0.05, 0.05]],
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

        let asset = build_effect_asset(&emitter);

        assert_eq!(asset.texture_layout().layout.len(), 1);
        assert_eq!(asset.texture_layout().layout[0].name, "color");
    }

    #[test]
    fn untextured_emitters_do_not_declare_hanabi_texture_slot() {
        let emitter = sample_emitter();
        let modifiers = build_expr_modifiers(&emitter);

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
