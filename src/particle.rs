//! M2 particle emitter rendering — GPU particles via bevy_hanabi.
//!
//! Each M2 emitter is translated to a bevy_hanabi `EffectAsset` and spawned as
//! a `ParticleEffect` entity parented to the model (or its bone).

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::asset::blp;
use crate::asset::m2::wow_to_bevy;
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
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) {
    for em in emitters {
        spawn_single_emitter(commands, images, em, bone_entities, parent);
    }
}

fn spawn_single_emitter(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    em: &M2ParticleEmitter,
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) {
    let bone_entity = bone_entities.and_then(|b| b.get(em.bone_index as usize).copied());
    let pending_texture = load_emitter_texture(em, images);
    let pos = em.position;
    let offset = wow_to_bevy(pos[0], pos[1], pos[2]);
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
            Transform::from_translation(Vec3::from(offset)),
            Visibility::default(),
        ))
        .set_parent_in_place(parent_entity);
}

struct ExprModifiers {
    init: InitModifiers,
    gravity: AccelModifier,
    texture: Option<ParticleTextureModifier>,
    alpha_mode: bevy_hanabi::AlphaMode,
    module: Module,
}

fn build_expr_modifiers(em: &M2ParticleEmitter) -> ExprModifiers {
    let writer = ExprWriter::new();
    let init = build_init_modifiers(em, &writer);
    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, -em.gravity, 0.0)).expr());
    let mask_cutoff = writer.lit(0.5_f32).expr();
    let texture = em.texture_fdid.map(|_| ParticleTextureModifier {
        texture_slot: writer.lit(0u32).expr(),
        sample_mapping: ImageSampleMapping::Modulate,
    });
    let alpha_mode = emitter_alpha_mode(em.blend_type, mask_cutoff);
    ExprModifiers { init, gravity, texture, alpha_mode, module: writer.finish() }
}

fn build_effect_asset(em: &M2ParticleEmitter) -> EffectAsset {
    let m = build_expr_modifiers(em);
    let max_particles = ((em.emission_rate * em.lifespan) as u32).clamp(16, 4096);
    let spawner = SpawnerSettings::rate(em.emission_rate.max(0.1).into());

    let mut effect = assemble_effect(em, m.module, spawner, max_particles, m.alpha_mode, m.init, m.gravity);
    if let Some(tex) = m.texture {
        effect = effect.render(tex);
    }
    effect
}

fn assemble_effect(
    em: &M2ParticleEmitter, module: Module, spawner: SpawnerSettings,
    max_particles: u32, alpha_mode: bevy_hanabi::AlphaMode,
    init: InitModifiers, gravity: AccelModifier,
) -> EffectAsset {
    EffectAsset::new(max_particles, spawner, module)
        .with_name("m2_particle")
        .with_alpha_mode(alpha_mode)
        .with_simulation_space(SimulationSpace::Global)
        .init(init.age).init(init.lifetime).init(init.pos).init(init.vel)
        .update(gravity)
        .render(build_color_render_modifier(em))
        .render(SizeOverLifetimeModifier {
            gradient: build_size_gradient(em),
            screen_space_size: false,
        })
        .render(OrientModifier::new(OrientMode::FaceCameraPosition))
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
    vel: SetVelocitySphereModifier,
}

fn build_init_modifiers(em: &M2ParticleEmitter, writer: &ExprWriter) -> InitModifiers {
    let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let lifetime =
        SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(em.lifespan.max(0.1)).expr());
    let pos = build_position_modifier(em, writer);
    let vel = build_velocity_modifier(em, writer);
    InitModifiers { age, lifetime, pos, vel }
}

fn build_position_modifier(em: &M2ParticleEmitter, writer: &ExprWriter) -> SetPositionSphereModifier {
    let radius = if em.area_length > 0.0 || em.area_width > 0.0 {
        (em.area_length.max(em.area_width) * 0.5).max(0.01)
    } else {
        0.01_f32
    };
    SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(radius).expr(),
        dimension: ShapeDimension::Volume,
    }
}

fn build_velocity_modifier(em: &M2ParticleEmitter, writer: &ExprWriter) -> SetVelocitySphereModifier {
    let speed_expr = if em.speed_variation > 0.0 {
        let base = writer.lit(em.emission_speed);
        let variation = writer.rand(ScalarType::Float) * writer.lit(em.speed_variation * 2.0)
            - writer.lit(em.speed_variation);
        (base.clone() + base * variation).expr()
    } else {
        writer.lit(em.emission_speed.max(0.01)).expr()
    };
    SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: speed_expr,
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
    g.add_key(0.0, Vec4::new(c0[0] / 255.0, c0[1] / 255.0, c0[2] / 255.0, o0));
    g.add_key(mid, Vec4::new(c1[0] / 255.0, c1[1] / 255.0, c1[2] / 255.0, o1));
    g.add_key(1.0, Vec4::new(c2[0] / 255.0, c2[1] / 255.0, c2[2] / 255.0, o2));
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
