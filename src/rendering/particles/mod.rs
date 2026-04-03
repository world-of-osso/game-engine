//! M2 particle emitter rendering — GPU particles via bevy_hanabi.
//!
//! Each M2 emitter is translated to a bevy_hanabi `EffectAsset` and spawned as
//! a `ParticleEffect` entity parented to the model (or its bone).

mod visuals;

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::asset::blp;
use crate::asset::m2::wow_to_bevy;
use crate::asset::{m2_anim::M2Bone, m2_particle::M2ParticleEmitter};
use crate::client_options::GraphicsOptions;
use crate::creature_display;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::m2_spawn;
use visuals::{
    SizeVariationModifier, TwinkleSizeModifier, build_color_gradient, build_size_gradient,
    has_authored_size_variation, has_authored_twinkle,
};

// CParticleEmitter / retail runtime particle flag values.
pub(super) const PARTICLE_FLAG_TAIL_PARTICLES: u32 = 0x0000_0008;
pub(super) const PARTICLE_FLAG_WORLD_SPACE: u32 = 0x0000_0200;
pub(super) const PARTICLE_FLAG_BONE_SCALE: u32 = 0x0000_0400;
pub(super) const PARTICLE_FLAG_INHERIT_VELOCITY: u32 = 0x0000_0800;
pub(super) const PARTICLE_FLAG_INHERIT_POSITION: u32 = 0x0000_2000;
pub(super) const PARTICLE_FLAG_SPHERE_INVERT: u32 = 0x0000_1000;
pub(super) const PARTICLE_FLAG_XY_QUAD: u32 = 0x0000_4000;
pub(super) const PARTICLE_FLAG_NEGATE_SPIN: u32 = 0x0001_0000;
pub(super) const PARTICLE_FLAG_CLAMP_TAIL_TO_AGE: u32 = 0x0002_0000;
pub(super) const PARTICLE_FLAG_PROJECT_PARTICLE: u32 = 0x0004_0000;
pub(super) const PARTICLE_FLAG_FOLLOW_POSITION: u32 = 0x0008_0000;
pub(super) const PARTICLE_FLAG_RANDOM_TEXTURE: u32 = 0x0010_0000;
pub(super) const PARTICLE_FLAG_VELOCITY_ORIENT: u32 = 0x0020_0000;
pub(super) const PARTICLE_FLAG_SIZE_VARIATION_2D: u32 = 0x0080_0000;
pub(super) const PARTICLE_FLAG_WIND_DYNAMIC: u32 = 0x4000_0000;
pub(super) const PARTICLE_FLAG_WIND_ENABLED: u32 = 0x8000_0000;
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
const INHERIT_POSITION_BACK_DELTA_PROPERTY: &str = "inherit_position_back_delta";
const CHILD_EMITTER_FPS_APPROXIMATION: f32 = 60.0;
const MODEL_PARTICLE_MIN_SPEED: f32 = 0.0;

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin).add_systems(
            Update,
            (
                register_pending_particle_effects,
                sync_inherit_position_properties,
                trigger_pending_particle_bursts,
                tick_model_particle_emitters,
                simulate_model_particle_instances,
            ),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSpawnMode {
    Continuous,
    BurstOnce,
}

#[derive(Component, Default)]
pub struct PendingParticleBurst {
    pub armed: bool,
}

#[derive(Component)]
struct InheritPositionMotionState {
    previous_world_position: Vec3,
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
    pub spawn_mode: ParticleSpawnMode,
    pub spawn_source: ParticleSpawnSource,
    pub child_emitters: Vec<M2ParticleEmitter>,
    pub effect_parent: Option<Entity>,
    /// Optional texture handle to attach via `EffectMaterial`.
    pending_texture: Option<Handle<Image>>,
}

#[derive(Component)]
pub struct ModelParticleEmitterComp {
    pub emitter: M2ParticleEmitter,
    pub bone_entity: Option<Entity>,
    pub scale_source: Entity,
    pub spawn_mode: ParticleSpawnMode,
    pub spawn_source: ParticleSpawnSource,
    pub requested_model_path: String,
    pub resolved_model_path: Option<PathBuf>,
}

#[derive(Component, Default)]
struct ModelParticleEmitterRuntime {
    spawn_remainder: f32,
    burst_fired: bool,
    spawn_serial: u32,
}

#[derive(Component)]
struct ModelParticleInstance {
    velocity: Vec3,
    angular_velocity: Vec3,
    acceleration: Vec3,
    age: f32,
    lifetime: f32,
}

#[derive(SystemParam)]
struct ModelParticleSpawnParams<'w, 's> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inverse_bindposes: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    _marker: Local<'s, ()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSpawnSource {
    Standalone,
    ChildFromParentParticles,
}

/// System: build `EffectAsset` from emitter data + model/root scale, then
/// register as `ParticleEffect`.
fn register_pending_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    mut query: Query<(Entity, &mut Transform, &ParticleEmitterComp), Without<ParticleEffect>>,
    global_transforms: Query<&GlobalTransform>,
    graphics: Option<Res<GraphicsOptions>>,
    terrain: Option<Res<crate::terrain_heightmap::TerrainHeightmap>>,
) {
    let particle_density_multiplier = graphics
        .as_deref()
        .map(GraphicsOptions::particle_density_multiplier)
        .unwrap_or(1.0);
    for (entity, mut transform, comp) in &mut query {
        if let Some(projected_y) =
            projected_particle_spawn_y(entity, comp, &global_transforms, terrain.as_deref())
        {
            transform.translation.y += projected_y;
        }
        let model_scale = global_transforms
            .get(comp.scale_source)
            .map(|tf| tf.compute_transform().scale.x)
            .unwrap_or(1.0);
        let asset = build_effect_asset_with_mode(
            &comp.emitter,
            model_scale,
            particle_density_multiplier,
            comp.spawn_mode,
            comp.spawn_source,
            &comp.child_emitters,
        );
        let handle = effects.add(asset);
        let mut ec = commands.entity(entity);
        ec.insert(ParticleEffect::new(handle));
        if let Some(parent_effect) = comp.effect_parent {
            ec.insert(EffectParent::new(parent_effect));
        }
        if let Some(tex) = comp.pending_texture.clone() {
            ec.insert(EffectMaterial { images: vec![tex] });
        }
        if emitter_uses_inherit_position(&comp.emitter) {
            let current_world_position = global_transforms
                .get(entity)
                .map(GlobalTransform::translation)
                .unwrap_or(Vec3::ZERO);
            ec.insert(EffectProperties::default().with_properties([(
                INHERIT_POSITION_BACK_DELTA_PROPERTY.to_string(),
                Vec3::ZERO.into(),
            )]));
            ec.insert(InheritPositionMotionState {
                previous_world_position: current_world_position,
            });
        }
        if comp.spawn_mode == ParticleSpawnMode::BurstOnce {
            ec.insert(PendingParticleBurst { armed: true });
        }
    }
}

fn sync_inherit_position_properties(
    mut query: Query<
        (
            &GlobalTransform,
            &mut EffectProperties,
            &mut InheritPositionMotionState,
        ),
        With<ParticleEmitterComp>,
    >,
) {
    for (global_transform, properties, mut motion_state) in &mut query {
        let current_world_position = global_transform.translation();
        let back_delta = inherit_position_back_delta_local(
            motion_state.previous_world_position,
            current_world_position,
            global_transform,
        );
        let _ = EffectProperties::set_if_changed(
            properties,
            INHERIT_POSITION_BACK_DELTA_PROPERTY,
            back_delta.into(),
        );
        motion_state.previous_world_position = current_world_position;
    }
}

fn projected_particle_spawn_y(
    entity: Entity,
    comp: &ParticleEmitterComp,
    global_transforms: &Query<&GlobalTransform>,
    terrain: Option<&crate::terrain_heightmap::TerrainHeightmap>,
) -> Option<f32> {
    if !emitter_uses_project_particle(&comp.emitter) {
        return None;
    }
    let current = global_transforms.get(entity).ok()?.translation();
    let terrain_y = terrain?.height_at(current.x, current.z)?;
    Some(terrain_y - current.y)
}

fn trigger_pending_particle_bursts(
    mut query: Query<(&mut EffectSpawner, &mut PendingParticleBurst), With<ParticleEmitterComp>>,
) {
    for (mut spawner, mut pending) in &mut query {
        if pending.armed {
            spawner.reset();
            pending.armed = false;
        }
    }
}

fn tick_model_particle_emitters(
    mut commands: Commands,
    time: Res<Time>,
    graphics: Option<Res<GraphicsOptions>>,
    mut spawn_params: ModelParticleSpawnParams,
    mut query: Query<(
        &GlobalTransform,
        &mut ModelParticleEmitterRuntime,
        &ModelParticleEmitterComp,
    )>,
) {
    let particle_density_multiplier = graphics
        .as_deref()
        .map(GraphicsOptions::particle_density_multiplier)
        .unwrap_or(1.0);
    for (global_transform, mut runtime, emitter) in &mut query {
        let Some(model_path) = emitter.resolved_model_path.as_deref() else {
            continue;
        };
        // Child model-particle emitters are currently attached to the parent
        // emitter origin, not each live parent particle. Hanabi's GPU child
        // spawn events are not readable back on CPU, so this remains an
        // approximation until we have a full CPU-side child-particle runtime.
        let spawn_count = model_particle_spawn_count(
            &emitter.emitter,
            emitter.spawn_mode,
            particle_density_multiplier,
            time.delta_secs(),
            &mut runtime,
        );
        for _ in 0..spawn_count {
            spawn_model_particle_instance(
                &mut commands,
                &mut spawn_params,
                emitter,
                global_transform,
                model_path,
                &mut runtime,
            );
        }
    }
}

fn simulate_model_particle_instances(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ModelParticleInstance)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut instance) in &mut query {
        instance.age += dt;
        if instance.age >= instance.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let acceleration = instance.acceleration;
        instance.velocity += acceleration * dt;
        transform.translation += instance.velocity * dt;
        let angular = instance.angular_velocity * dt;
        if angular.length_squared() > 0.0 {
            transform.rotate(Quat::from_euler(
                EulerRot::XYZ,
                angular.x,
                angular.y,
                angular.z,
            ));
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
        spawn_single_emitter(
            commands,
            images,
            em,
            bones,
            bone_entities,
            parent,
            ParticleSpawnMode::Continuous,
        );
    }
}

pub fn spawn_burst_emitters(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    emitters: &[M2ParticleEmitter],
    bones: &[M2Bone],
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) {
    for em in emitters {
        spawn_single_emitter(
            commands,
            images,
            em,
            bones,
            bone_entities,
            parent,
            ParticleSpawnMode::BurstOnce,
        );
    }
}

fn spawn_single_emitter(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    em: &M2ParticleEmitter,
    bones: &[M2Bone],
    bone_entities: Option<&[Entity]>,
    parent: Entity,
    spawn_mode: ParticleSpawnMode,
) {
    let bone_entity = bone_entities.and_then(|b| b.get(em.bone_index as usize).copied());
    let parent_entity = emitter_parent_entity(em, bone_entity, parent);
    let local_offset = emitter_spawn_offset(em, bones);
    if emitter_uses_model_particles(em) {
        spawn_model_particle_emitter(
            commands,
            em,
            bone_entity,
            parent_entity,
            local_offset,
            emitter_scale_source(em, bone_entity, parent),
            spawn_mode,
            ParticleSpawnSource::Standalone,
        );
        return;
    }
    let pending_texture = load_emitter_texture(em, images);
    let emitter_entity = commands
        .spawn((
            Name::new("ParticleEmitter"),
            ParticleEmitterComp {
                emitter: em.clone(),
                bone_entity,
                scale_source: emitter_scale_source(em, bone_entity, parent),
                spawn_mode,
                spawn_source: ParticleSpawnSource::Standalone,
                child_emitters: load_child_particle_emitters(em),
                effect_parent: None,
                pending_texture,
            },
            Transform::from_translation(local_offset),
            Visibility::default(),
        ))
        .set_parent_in_place(parent_entity)
        .id();
    spawn_child_emitter_effects(commands, images, em, emitter_entity, parent);
}

fn spawn_model_particle_emitter(
    commands: &mut Commands,
    em: &M2ParticleEmitter,
    bone_entity: Option<Entity>,
    parent_entity: Entity,
    local_offset: Vec3,
    scale_source: Entity,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
) {
    let requested_model_path = em.particle_model_filename.clone().unwrap_or_default();
    commands
        .spawn((
            Name::new("ModelParticleEmitter"),
            ModelParticleEmitterComp {
                emitter: em.clone(),
                bone_entity,
                scale_source,
                spawn_mode,
                spawn_source,
                resolved_model_path: resolve_particle_model_path(&requested_model_path),
                requested_model_path,
            },
            ModelParticleEmitterRuntime::default(),
            Transform::from_translation(local_offset),
            Visibility::default(),
        ))
        .set_parent_in_place(parent_entity);
}

fn spawn_child_emitter_effects(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    emitter: &M2ParticleEmitter,
    parent_effect_entity: Entity,
    scale_source: Entity,
) {
    let Some((child_emitters, child_bones)) = load_child_particle_emitters_and_bones(emitter)
    else {
        return;
    };
    spawn_loaded_child_emitters(
        commands,
        images,
        &child_emitters,
        &child_bones,
        parent_effect_entity,
        scale_source,
    );
}

fn spawn_child_emitter_effect(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    em: &M2ParticleEmitter,
    bones: &[M2Bone],
    parent_effect_entity: Entity,
    scale_source: Entity,
) {
    let pending_texture = load_emitter_texture(em, images);
    let local_offset = emitter_spawn_offset(em, bones);
    commands.spawn((
        Name::new("ChildParticleEmitter"),
        ParticleEmitterComp {
            emitter: em.clone(),
            bone_entity: None,
            scale_source,
            spawn_mode: ParticleSpawnMode::Continuous,
            spawn_source: ParticleSpawnSource::ChildFromParentParticles,
            child_emitters: Vec::new(),
            effect_parent: Some(parent_effect_entity),
            pending_texture,
        },
        Transform::from_translation(local_offset),
        Visibility::default(),
    ));
}

fn spawn_loaded_child_emitters(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    child_emitters: &[M2ParticleEmitter],
    child_bones: &[M2Bone],
    parent_effect_entity: Entity,
    scale_source: Entity,
) {
    for child_emitter in child_emitters {
        if emitter_uses_model_particles(child_emitter) {
            spawn_model_particle_emitter(
                commands,
                child_emitter,
                None,
                parent_effect_entity,
                emitter_spawn_offset(child_emitter, child_bones),
                scale_source,
                ParticleSpawnMode::Continuous,
                ParticleSpawnSource::ChildFromParentParticles,
            );
        } else {
            spawn_child_emitter_effect(
                commands,
                images,
                child_emitter,
                child_bones,
                parent_effect_entity,
                scale_source,
            );
        }
    }
}

fn load_child_particle_emitters(em: &M2ParticleEmitter) -> Vec<M2ParticleEmitter> {
    load_child_particle_emitters_and_bones(em)
        .map(|(emitters, _)| emitters)
        .unwrap_or_default()
}

fn load_child_particle_emitters_and_bones(
    em: &M2ParticleEmitter,
) -> Option<(Vec<M2ParticleEmitter>, Vec<M2Bone>)> {
    let child_path =
        resolve_child_emitter_model_path(em.child_emitters_model_filename.as_deref()?)?;
    let child_model = crate::asset::m2::load_m2(&child_path, &[0, 0, 0]).ok()?;
    if child_model.particle_emitters.is_empty() {
        return None;
    }
    Some((child_model.particle_emitters, child_model.bones))
}

fn resolve_child_emitter_model_path(path: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(path);
    if direct.exists() {
        return Some(direct);
    }
    let fdid = game_engine::listfile::lookup_path(path)?;
    crate::asset::asset_cache::model(fdid)
}

fn resolve_particle_model_path(path: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(path);
    if direct.exists() {
        return Some(direct);
    }
    let fdid = game_engine::listfile::lookup_path(path)?;
    crate::asset::asset_cache::model(fdid)
}

fn emitter_parent_entity(
    em: &M2ParticleEmitter,
    bone_entity: Option<Entity>,
    parent: Entity,
) -> Entity {
    if emitter_uses_world_space(em) {
        parent
    } else {
        bone_entity.unwrap_or(parent)
    }
}

fn emitter_scale_source(
    em: &M2ParticleEmitter,
    bone_entity: Option<Entity>,
    parent: Entity,
) -> Entity {
    if emitter_uses_bone_scale(em) {
        bone_entity.unwrap_or(parent)
    } else {
        parent
    }
}

fn emitter_spawn_offset(em: &M2ParticleEmitter, bones: &[M2Bone]) -> Vec3 {
    if emitter_uses_world_space(em) {
        emitter_translation(em)
    } else {
        emitter_local_offset(em, bones)
    }
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

fn emitter_uses_world_space(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_WORLD_SPACE != 0
}

fn emitter_uses_follow_position(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_FOLLOW_POSITION != 0
}

fn emitter_uses_inherit_position(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_INHERIT_POSITION != 0
}

fn emitter_uses_inherit_velocity(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_INHERIT_VELOCITY != 0
}

fn emitter_uses_model_particles(em: &M2ParticleEmitter) -> bool {
    em.particle_model_filename
        .as_deref()
        .is_some_and(|path| !path.trim().is_empty())
}

fn model_particle_spawn_count(
    em: &M2ParticleEmitter,
    spawn_mode: ParticleSpawnMode,
    particle_density_multiplier: f32,
    dt: f32,
    runtime: &mut ModelParticleEmitterRuntime,
) -> u32 {
    match spawn_mode {
        ParticleSpawnMode::BurstOnce => {
            if runtime.burst_fired {
                0
            } else {
                runtime.burst_fired = true;
                scaled_emission_rate(em, particle_density_multiplier).ceil() as u32
            }
        }
        ParticleSpawnMode::Continuous => {
            let total = scaled_emission_rate(em, particle_density_multiplier) * dt
                + runtime.spawn_remainder;
            let whole = total.floor();
            runtime.spawn_remainder = total - whole;
            whole as u32
        }
    }
}

fn spawn_model_particle_instance(
    commands: &mut Commands,
    spawn_params: &mut ModelParticleSpawnParams<'_, '_>,
    emitter: &ModelParticleEmitterComp,
    emitter_transform: &GlobalTransform,
    model_path: &std::path::Path,
    runtime: &mut ModelParticleEmitterRuntime,
) {
    let seed = runtime.spawn_serial;
    runtime.spawn_serial = runtime.spawn_serial.wrapping_add(1);
    let transform = model_particle_spawn_transform(&emitter.emitter, emitter_transform, seed);
    let lifetime = sample_model_particle_lifetime(&emitter.emitter, seed);
    let velocity = sample_model_particle_velocity(
        &emitter.emitter,
        emitter_transform.compute_transform().scale.x,
        seed,
    );
    let acceleration = gravity_accel_bevy(&emitter.emitter);
    let angular_velocity = model_particle_angular_velocity(&emitter.emitter, seed);
    let mut ctx = m2_scene::M2SceneSpawnContext {
        commands,
        assets: m2_spawn::SpawnAssets {
            meshes: &mut spawn_params.meshes,
            materials: &mut spawn_params.materials,
            effect_materials: &mut spawn_params.effect_materials,
            skybox_materials: None,
            images: &mut spawn_params.images,
            inverse_bindposes: &mut spawn_params.inverse_bindposes,
        },
        creature_display_map: &spawn_params.creature_display_map,
    };
    let Some(spawned) = m2_scene::spawn_animated_static_m2_parts(&mut ctx, model_path, transform)
    else {
        return;
    };
    ctx.commands
        .entity(spawned.root)
        .insert(ModelParticleInstance {
            velocity,
            angular_velocity,
            acceleration,
            age: 0.0,
            lifetime,
        });
}

fn model_particle_spawn_transform(
    em: &M2ParticleEmitter,
    emitter_transform: &GlobalTransform,
    seed: u32,
) -> Transform {
    let local_offset =
        sample_model_particle_local_offset(em, emitter_transform.compute_transform().scale.x, seed);
    let world_translation = emitter_transform.affine().transform_point3(local_offset);
    let base_rotation = emitter_transform.compute_transform().rotation;
    let initial_rotation = Quat::from_rotation_y(sample_model_particle_base_spin(em, seed));
    Transform::from_translation(world_translation)
        .with_rotation(base_rotation * initial_rotation)
        .with_scale(emitter_transform.compute_transform().scale)
}

fn sample_model_particle_local_offset(em: &M2ParticleEmitter, model_scale: f32, seed: u32) -> Vec3 {
    match em.emitter_type {
        1 => {
            let half_length = em.area_length.max(0.0) * model_scale;
            let half_width = em.area_width.max(0.0) * model_scale;
            let x = lerp(-half_length, half_length, pseudo_random01(seed, 0));
            let z = lerp(-half_width, half_width, pseudo_random01(seed, 1));
            Vec3::new(x, 0.0, z)
        }
        2 => {
            let radius = emitter_spawn_radius(em) * model_scale;
            let dir = pseudo_random_unit_vector(seed);
            dir * radius * pseudo_random01(seed, 2).cbrt()
        }
        _ => Vec3::ZERO,
    }
}

fn sample_model_particle_velocity(em: &M2ParticleEmitter, model_scale: f32, seed: u32) -> Vec3 {
    let speed = sample_model_particle_speed(em, seed) * model_scale;
    if em.z_source > 0.0 {
        let position = sample_model_particle_local_offset(em, model_scale, seed);
        let source = Vec3::new(0.0, 0.0, em.z_source);
        let direction = (position - source).normalize_or_zero();
        return direction * speed;
    }
    if emitter_uses_sphere_invert_velocity(em) {
        let position = sample_model_particle_local_offset(em, model_scale, seed);
        return (-position).normalize_or_zero() * speed;
    }
    let yaw = pseudo_random01(seed, 3) * em.horizontal_range;
    let pitch = pseudo_random01(seed, 4) * em.vertical_range;
    let sin_pitch = pitch.sin();
    let cos_pitch = pitch.cos();
    Vec3::new(
        sin_pitch * yaw.cos() * speed,
        cos_pitch * speed,
        sin_pitch * yaw.sin() * speed,
    )
}

fn sample_model_particle_speed(em: &M2ParticleEmitter, seed: u32) -> f32 {
    if em.speed_variation > 0.0 {
        let variation = lerp(
            -em.speed_variation,
            em.speed_variation,
            pseudo_random01(seed, 5),
        );
        (em.emission_speed * (1.0 + variation)).max(MODEL_PARTICLE_MIN_SPEED)
    } else {
        em.emission_speed.max(MODEL_PARTICLE_MIN_SPEED)
    }
}

fn sample_model_particle_lifetime(em: &M2ParticleEmitter, seed: u32) -> f32 {
    let (min_lifetime, max_lifetime) = lifetime_range(em);
    lerp(min_lifetime, max_lifetime, pseudo_random01(seed, 6))
}

fn sample_model_particle_base_spin(em: &M2ParticleEmitter, seed: u32) -> f32 {
    if em.base_spin_variation > 0.0 {
        em.base_spin
            + lerp(
                -em.base_spin_variation,
                em.base_spin_variation,
                pseudo_random01(seed, 7),
            )
    } else {
        em.base_spin
    }
}

fn model_particle_angular_velocity(em: &M2ParticleEmitter, seed: u32) -> Vec3 {
    let y = if em.spin_variation > 0.0 {
        em.spin
            + lerp(
                -em.spin_variation,
                em.spin_variation,
                pseudo_random01(seed, 8),
            )
    } else {
        em.spin
    };
    Vec3::new(0.0, y, 0.0)
}

fn pseudo_random01(seed: u32, lane: u32) -> f32 {
    let mut x = seed
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(lane.wrapping_mul(0x85EB_CA6B));
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x as f32 / u32::MAX as f32
}

fn pseudo_random_unit_vector(seed: u32) -> Vec3 {
    let z = lerp(-1.0, 1.0, pseudo_random01(seed, 9));
    let theta = pseudo_random01(seed, 10) * std::f32::consts::TAU;
    let r = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(r * theta.cos(), r * theta.sin(), z)
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
    min + (max - min) * t
}

fn emitter_uses_project_particle(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_PROJECT_PARTICLE != 0 && !emitter_uses_world_space(em)
}

fn emitter_uses_bone_scale(em: &M2ParticleEmitter) -> bool {
    em.flags & (PARTICLE_FLAG_WORLD_SPACE | PARTICLE_FLAG_BONE_SCALE) == PARTICLE_FLAG_BONE_SCALE
}

fn emitter_simulation_space(em: &M2ParticleEmitter) -> SimulationSpace {
    if emitter_uses_follow_position(em) {
        // WoW applies the emitter motion delta to live particles every update.
        // Hanabi doesn't expose that delta directly, so local simulation is the
        // closest match for follow-position emitters.
        SimulationSpace::Local
    } else {
        SimulationSpace::Global
    }
}

fn inherit_position_back_delta_local(
    previous_world_position: Vec3,
    current_world_position: Vec3,
    current_global_transform: &GlobalTransform,
) -> Vec3 {
    current_global_transform
        .affine()
        .inverse()
        .transform_vector3(previous_world_position - current_world_position)
}

fn emitter_uses_sphere_invert_velocity(em: &M2ParticleEmitter) -> bool {
    em.emitter_type == 2 && em.flags & PARTICLE_FLAG_SPHERE_INVERT != 0
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
    // WoW keeps atlas emitters on the first cell unless an authored cell track
    // or RANDOM_TEXTURE path overrides it. We should not invent a lifetime-wide
    // flipbook animation for emitters with no cell track.
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

fn build_effect_asset(
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

fn build_effect_asset_with_mode(
    em: &M2ParticleEmitter,
    model_scale: f32,
    particle_density_multiplier: f32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
    child_emitters: &[M2ParticleEmitter],
) -> EffectAsset {
    let m = build_expr_modifiers(em, model_scale);
    let emission_rate = scaled_emission_rate(em, particle_density_multiplier);
    let (_, max_lifetime) = lifetime_range(em);
    let burst_count = emission_rate.max(0.0);
    let child_event_counts: Vec<u32> = child_emitters
        .iter()
        .map(|child| child_emitter_event_count(child, particle_density_multiplier))
        .collect();
    let max_particles = max_particles(emission_rate, max_lifetime, burst_count, spawn_source);
    let spawner = build_spawner_settings(emission_rate, spawn_mode, spawn_source);

    let mut effect = assemble_effect(
        em,
        m.module,
        spawner,
        max_particles,
        m.alpha_mode,
        m.init,
        m.gravity,
        m.orient_rotation,
        model_scale,
        spawn_source,
        child_event_counts,
    );
    if let Some(sprite_idx) = m.flipbook_sprite_index_init {
        effect = effect.init(sprite_idx);
    }
    if let Some(drag) = m.drag {
        effect = effect.update(drag);
    }
    if let Some(sprite_idx) = m.flipbook_sprite_index_update {
        effect = effect.update(sprite_idx);
    }
    if let Some(tex) = m.texture {
        effect = effect.render(tex);
    }
    if let Some(twinkle) = m.twinkle {
        effect = effect.render(twinkle);
    }
    if let Some(size_variation) = m.size_variation {
        effect = effect.render(size_variation);
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
        ParticleSpawnMode::Continuous => {
            // WoW emits particles via an accumulator (`rate * dt + carry`) and
            // can vary the instantaneous spawn rate per tick. Hanabi only gives
            // us a steady rate spawner here, so this remains an approximation.
            SpawnerSettings::rate(emission_rate.into())
        }
        ParticleSpawnMode::BurstOnce => {
            // WoW's PROP_BURST_EMIT is a one-shot runtime event, not a steady
            // parsed emitter flag. Keep burst emitters dormant until the engine
            // explicitly arms them, then fire a single-frame burst.
            SpawnerSettings::once(emission_rate.max(0.0).into())
                .with_starts_active(true)
                .with_emit_on_start(false)
        }
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

fn scaled_emission_rate(em: &M2ParticleEmitter, particle_density_multiplier: f32) -> f32 {
    // WoW samples `base + rand * variation` per emission step. Hanabi exposes
    // only a constant rate, so use the expected mean rate here.
    let mean_rate = em.emission_rate + em.emission_rate_variation.max(0.0) * 0.5;
    (mean_rate * particle_density_multiplier.clamp(0.1, 1.0)).max(0.1)
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
    let orient = if let Some(rotation) = orient_rotation {
        OrientModifier::new(orient_mode(em)).with_rotation(rotation)
    } else {
        OrientModifier::new(orient_mode(em))
    };
    let child_event_count_exprs: Vec<ExprHandle> = child_event_counts
        .into_iter()
        .map(|count| module.lit(count))
        .collect();
    let mut effect = EffectAsset::new(max_particles, spawner, module)
        .with_name("m2_particle")
        .with_alpha_mode(alpha_mode)
        .with_simulation_space(emitter_simulation_space(em))
        .init(init.age)
        .init(init.lifetime)
        .init(init.vel)
        .update(gravity)
        .render(build_color_render_modifier(em))
        .render(SizeOverLifetimeModifier {
            gradient: build_size_gradient(em, model_scale),
            screen_space_size: false,
        })
        .render(orient);
    effect = match init.pos {
        PositionInitModifier::Attribute(pos) => effect.init(pos),
        PositionInitModifier::Sphere(pos) => effect.init(pos),
    };
    if spawn_source == ParticleSpawnSource::ChildFromParentParticles {
        effect = effect.init(InheritAttributeModifier::new(Attribute::POSITION));
        if emitter_uses_inherit_velocity(em) {
            effect = effect.init(InheritAttributeModifier::new(Attribute::VELOCITY));
        }
    }
    if let Some(rotation) = init.rotation {
        effect = effect.init(rotation);
    }
    if let Some(angular_velocity) = init.angular_velocity {
        effect = effect.init(angular_velocity);
    }
    if let Some(spin_sign) = init.spin_sign {
        effect = effect.init(spin_sign);
    }
    if let Some(twinkle_phase) = init.twinkle_phase {
        effect = effect.init(twinkle_phase);
    }
    if let Some(twinkle_enabled) = init.twinkle_enabled {
        effect = effect.init(twinkle_enabled);
    }
    if let Some(size_variation) = init.size_variation {
        effect = effect.init(size_variation);
    }
    for (child_index, count_expr) in child_event_count_exprs.into_iter().enumerate() {
        effect = effect.update(EmitSpawnEventModifier {
            condition: EventEmitCondition::Always,
            count: count_expr,
            child_index: child_index as u32,
        });
    }
    effect
}

fn child_emitter_event_count(em: &M2ParticleEmitter, particle_density_multiplier: f32) -> u32 {
    let per_frame =
        scaled_emission_rate(em, particle_density_multiplier) / CHILD_EMITTER_FPS_APPROXIMATION;
    per_frame.ceil().max(1.0) as u32
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

fn has_authored_spin(em: &M2ParticleEmitter) -> bool {
    em.base_spin != 0.0
        || em.base_spin_variation != 0.0
        || em.spin != 0.0
        || em.spin_variation != 0.0
}

fn has_authored_wind(em: &M2ParticleEmitter) -> bool {
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

fn wind_accel_bevy(em: &M2ParticleEmitter, model_scale: f32) -> Vec3 {
    let [x, y, z] = em.wind_vector;
    Vec3::from(wow_to_bevy(x, y, z)) * model_scale
}

fn gravity_accel_bevy(em: &M2ParticleEmitter) -> Vec3 {
    let [x, y, z] = em.gravity_vector;
    Vec3::from(wow_to_bevy(x, y, z))
}

fn wind_strength_at_age(age: f32, wind_time: f32) -> f32 {
    if wind_time > 0.0 && age <= wind_time {
        1.0
    } else {
        0.0
    }
}

fn build_position_modifier(
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

fn emitter_spawn_radius(em: &M2ParticleEmitter) -> f32 {
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

fn emitter_alpha_mode(blend_type: u8, mask_cutoff: ExprHandle) -> bevy_hanabi::AlphaMode {
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
#[path = "../../../tests/unit/particle_tests.rs"]
mod tests;
