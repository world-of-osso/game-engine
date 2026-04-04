use std::path::PathBuf;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::asset::m2::wow_to_bevy;
use crate::asset::{m2_anim::M2Bone, m2_particle::M2ParticleEmitter};
use crate::client_options::GraphicsOptions;

use super::effect_builder::{build_effect_asset_with_mode, load_emitter_texture};
use super::emitters_model_particles::spawn_model_particle_emitter;
use super::{
    DYNAMIC_WIND_ACCEL_PROPERTY, DynamicParticleWind, INHERIT_POSITION_BACK_DELTA_PROPERTY,
    PARTICLE_FLAG_BONE_SCALE, PARTICLE_FLAG_FOLLOW_POSITION, PARTICLE_FLAG_INHERIT_POSITION,
    PARTICLE_FLAG_INHERIT_VELOCITY, PARTICLE_FLAG_PROJECT_PARTICLE, PARTICLE_FLAG_SPHERE_INVERT,
    PARTICLE_FLAG_WIND_DYNAMIC, PARTICLE_FLAG_WIND_ENABLED, PARTICLE_FLAG_WORLD_SPACE,
    ParticleSpawnMode, ParticleSpawnSource, PendingParticleBurst,
};

#[derive(Component)]
pub(crate) struct InheritPositionMotionState {
    previous_world_position: Vec3,
}

#[derive(Component)]
pub struct ParticleEmitterComp {
    pub emitter: M2ParticleEmitter,
    pub bone_entity: Option<Entity>,
    pub scale_source: Entity,
    pub spawn_mode: ParticleSpawnMode,
    pub spawn_source: ParticleSpawnSource,
    pub child_emitters: Vec<M2ParticleEmitter>,
    pub effect_parent: Option<Entity>,
    pub(crate) pending_texture: Option<Handle<Image>>,
}

pub(crate) type ModelParticleEmitterComp =
    super::emitters_model_particles::ModelParticleEmitterComp;
pub(crate) type ModelParticleEmitterRuntime =
    super::emitters_model_particles::ModelParticleEmitterRuntime;
pub(crate) type ModelParticleInstance = super::emitters_model_particles::ModelParticleInstance;

pub(crate) fn tick_model_particle_emitters(
    commands: Commands,
    time: Res<Time>,
    graphics: Option<Res<GraphicsOptions>>,
    spawn_params: super::emitters_model_particles::ModelParticleSpawnParams<'_, '_>,
    query: Query<(
        &GlobalTransform,
        &mut ModelParticleEmitterRuntime,
        &ModelParticleEmitterComp,
    )>,
) {
    super::emitters_model_particles::tick_model_particle_emitters(
        commands,
        time,
        graphics,
        spawn_params,
        query,
    )
}

pub(crate) fn simulate_model_particle_instances(
    commands: Commands,
    time: Res<Time>,
    query: Query<(Entity, &mut Transform, &mut ModelParticleInstance)>,
) {
    super::emitters_model_particles::simulate_model_particle_instances(commands, time, query)
}

pub(crate) fn model_particle_spawn_count(
    em: &M2ParticleEmitter,
    spawn_mode: ParticleSpawnMode,
    particle_density_multiplier: f32,
    dt: f32,
    runtime: &mut ModelParticleEmitterRuntime,
) -> u32 {
    super::emitters_model_particles::model_particle_spawn_count(
        em,
        spawn_mode,
        particle_density_multiplier,
        dt,
        runtime,
    )
}

pub(crate) fn register_pending_particle_effects(
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
        apply_projected_particle_spawn(
            entity,
            comp,
            &global_transforms,
            terrain.as_deref(),
            &mut transform,
        );
        let mut ec = register_particle_effect_entity(
            entity,
            comp,
            &global_transforms,
            particle_density_multiplier,
            &mut effects,
            &mut commands,
        );
        insert_optional_particle_effect_properties(entity, comp, &global_transforms, &mut ec);
    }
}

fn apply_projected_particle_spawn(
    entity: Entity,
    comp: &ParticleEmitterComp,
    global_transforms: &Query<&GlobalTransform>,
    terrain: Option<&crate::terrain_heightmap::TerrainHeightmap>,
    transform: &mut Transform,
) {
    if let Some(projected_y) = projected_particle_spawn_y(entity, comp, global_transforms, terrain)
    {
        transform.translation.y += projected_y;
    }
}

fn register_particle_effect_entity<'a>(
    entity: Entity,
    comp: &ParticleEmitterComp,
    global_transforms: &Query<&GlobalTransform>,
    particle_density_multiplier: f32,
    effects: &mut Assets<EffectAsset>,
    commands: &'a mut Commands,
) -> EntityCommands<'a> {
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
    ec
}

fn insert_optional_particle_effect_properties(
    entity: Entity,
    comp: &ParticleEmitterComp,
    global_transforms: &Query<&GlobalTransform>,
    ec: &mut EntityCommands,
) {
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
    if emitter_uses_dynamic_wind(&comp.emitter) {
        let properties = EffectProperties::default()
            .with_properties([(DYNAMIC_WIND_ACCEL_PROPERTY.to_string(), Vec3::ZERO.into())]);
        ec.insert(properties);
    }
    if comp.spawn_mode == ParticleSpawnMode::BurstOnce {
        ec.insert(PendingParticleBurst { armed: true });
    }
}

pub(crate) fn sync_inherit_position_properties(
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

pub(crate) fn sync_dynamic_wind_properties(
    dynamic_wind: Res<DynamicParticleWind>,
    mut query: Query<(&ParticleEmitterComp, &mut EffectProperties)>,
) {
    if !dynamic_wind.is_changed() {
        return;
    }
    for (comp, properties) in &mut query {
        if !emitter_uses_dynamic_wind(&comp.emitter) {
            continue;
        }
        let _ = EffectProperties::set_if_changed(
            properties,
            DYNAMIC_WIND_ACCEL_PROPERTY,
            dynamic_wind.effect_space_accel.into(),
        );
    }
}

pub(crate) fn projected_particle_spawn_y(
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

pub(crate) fn trigger_pending_particle_bursts(
    mut query: Query<(&mut EffectSpawner, &mut PendingParticleBurst), With<ParticleEmitterComp>>,
) {
    for (mut spawner, mut pending) in &mut query {
        if pending.armed {
            spawner.reset();
            pending.armed = false;
        }
    }
}

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

struct EmitterSpawnContext {
    bone_entity: Option<Entity>,
    parent_entity: Entity,
    local_offset: Vec3,
    scale_source: Entity,
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
    let spawn = resolve_emitter_spawn_context(em, bones, bone_entities, parent);
    if emitter_uses_model_particles(em) {
        spawn_model_particle_emitter(
            commands,
            em,
            spawn.bone_entity,
            spawn.parent_entity,
            spawn.local_offset,
            spawn.scale_source,
            spawn_mode,
            ParticleSpawnSource::Standalone,
        );
        return;
    }
    let emitter_entity = spawn_gpu_particle_emitter(
        commands,
        images,
        em,
        &spawn,
        spawn_mode,
        ParticleSpawnSource::Standalone,
    );
    spawn_child_emitter_effects(commands, images, em, emitter_entity, parent);
}

fn resolve_emitter_spawn_context(
    em: &M2ParticleEmitter,
    bones: &[M2Bone],
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) -> EmitterSpawnContext {
    let bone_entity = bone_entities.and_then(|b| b.get(em.bone_index as usize).copied());
    EmitterSpawnContext {
        bone_entity,
        parent_entity: emitter_parent_entity(em, bone_entity, parent),
        local_offset: emitter_spawn_offset(em, bones),
        scale_source: emitter_scale_source(em, bone_entity, parent),
    }
}

fn spawn_gpu_particle_emitter(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    em: &M2ParticleEmitter,
    spawn: &EmitterSpawnContext,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
) -> Entity {
    let pending_texture = load_emitter_texture(em, images);
    commands
        .spawn((
            Name::new("ParticleEmitter"),
            ParticleEmitterComp {
                emitter: em.clone(),
                bone_entity: spawn.bone_entity,
                scale_source: spawn.scale_source,
                spawn_mode,
                spawn_source,
                child_emitters: load_child_particle_emitters(em),
                effect_parent: None,
                pending_texture,
            },
            Transform::from_translation(spawn.local_offset),
            Visibility::default(),
        ))
        .set_parent_in_place(spawn.parent_entity)
        .id()
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

pub(crate) fn spawn_loaded_child_emitters(
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

pub(crate) fn emitter_parent_entity(
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

pub(crate) fn emitter_scale_source(
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

pub(crate) fn emitter_spawn_offset(em: &M2ParticleEmitter, bones: &[M2Bone]) -> Vec3 {
    if emitter_uses_world_space(em) {
        emitter_translation(em)
    } else {
        emitter_local_offset(em, bones)
    }
}

fn emitter_local_offset(em: &M2ParticleEmitter, bones: &[M2Bone]) -> Vec3 {
    let pos = emitter_translation(em);
    let bone_pivot = bones
        .get(em.bone_index as usize)
        .map(|b| Vec3::new(b.pivot[0], b.pivot[2], -b.pivot[1]))
        .unwrap_or(Vec3::ZERO);
    pos - bone_pivot
}

pub(crate) fn emitter_translation(em: &M2ParticleEmitter) -> Vec3 {
    let pos = em.position;
    Vec3::from(wow_to_bevy(pos[0], pos[1], pos[2]))
}

fn emitter_uses_world_space(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_WORLD_SPACE != 0
}

pub(crate) fn emitter_uses_follow_position(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_FOLLOW_POSITION != 0
}

pub(crate) fn emitter_uses_dynamic_wind(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_WIND_ENABLED != 0 && em.flags & PARTICLE_FLAG_WIND_DYNAMIC != 0
}

pub(crate) fn emitter_uses_inherit_position(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_INHERIT_POSITION != 0
}

pub(crate) fn emitter_uses_inherit_velocity(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_INHERIT_VELOCITY != 0
}

pub(crate) fn emitter_uses_model_particles(em: &M2ParticleEmitter) -> bool {
    em.particle_model_filename
        .as_deref()
        .is_some_and(|path| !path.trim().is_empty())
}

pub(crate) fn emitter_uses_project_particle(em: &M2ParticleEmitter) -> bool {
    em.flags & PARTICLE_FLAG_PROJECT_PARTICLE != 0 && !emitter_uses_world_space(em)
}

pub(crate) fn emitter_uses_bone_scale(em: &M2ParticleEmitter) -> bool {
    em.flags & (PARTICLE_FLAG_WORLD_SPACE | PARTICLE_FLAG_BONE_SCALE) == PARTICLE_FLAG_BONE_SCALE
}

pub(crate) fn emitter_simulation_space(em: &M2ParticleEmitter) -> SimulationSpace {
    if emitter_uses_follow_position(em) {
        SimulationSpace::Local
    } else {
        SimulationSpace::Global
    }
}

pub(crate) fn inherit_position_back_delta_local(
    previous_world_position: Vec3,
    current_world_position: Vec3,
    current_global_transform: &GlobalTransform,
) -> Vec3 {
    current_global_transform
        .affine()
        .inverse()
        .transform_vector3(previous_world_position - current_world_position)
}

pub(crate) fn emitter_uses_sphere_invert_velocity(em: &M2ParticleEmitter) -> bool {
    em.emitter_type == 2 && em.flags & PARTICLE_FLAG_SPHERE_INVERT != 0
}
