use std::path::{Path, PathBuf};

use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::m2_particle::M2ParticleEmitter;
use crate::client_options::GraphicsOptions;
use crate::creature_display;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::m2_spawn;

use super::effect_builder::{
    emitter_spawn_radius, gravity_accel_bevy, lifetime_range, scaled_emission_rate,
};
use super::emitters::emitter_uses_sphere_invert_velocity;
use super::{MODEL_PARTICLE_MIN_SPEED, ParticleSpawnMode, ParticleSpawnSource};

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
pub(crate) struct ModelParticleEmitterRuntime {
    pub(crate) spawn_remainder: f32,
    pub(crate) burst_fired: bool,
    pub(crate) spawn_serial: u32,
}

#[derive(Component)]
pub(crate) struct ModelParticleInstance {
    pub(crate) velocity: Vec3,
    pub(crate) angular_velocity: Vec3,
    pub(crate) acceleration: Vec3,
    pub(crate) age: f32,
    pub(crate) lifetime: f32,
}

struct ModelParticleInstanceState {
    transform: Transform,
    velocity: Vec3,
    angular_velocity: Vec3,
    acceleration: Vec3,
    lifetime: f32,
}

#[derive(SystemParam)]
pub(crate) struct ModelParticleSpawnParams<'w, 's> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inverse_bindposes: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    _marker: Local<'s, ()>,
}

pub(crate) fn tick_model_particle_emitters(
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

pub(crate) fn simulate_model_particle_instances(
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

pub(crate) fn spawn_model_particle_emitter(
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

pub(crate) fn model_particle_spawn_count(
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
    model_path: &Path,
    runtime: &mut ModelParticleEmitterRuntime,
) {
    let seed = runtime.spawn_serial;
    runtime.spawn_serial = runtime.spawn_serial.wrapping_add(1);
    let instance = build_model_particle_instance_state(&emitter.emitter, emitter_transform, seed);
    let Some(spawned_root) =
        spawn_model_particle_root(commands, spawn_params, model_path, instance.transform)
    else {
        return;
    };
    commands.entity(spawned_root).insert(ModelParticleInstance {
        velocity: instance.velocity,
        angular_velocity: instance.angular_velocity,
        acceleration: instance.acceleration,
        age: 0.0,
        lifetime: instance.lifetime,
    });
}

fn build_model_particle_instance_state(
    emitter: &M2ParticleEmitter,
    emitter_transform: &GlobalTransform,
    seed: u32,
) -> ModelParticleInstanceState {
    ModelParticleInstanceState {
        transform: model_particle_spawn_transform(emitter, emitter_transform, seed),
        velocity: sample_model_particle_velocity(
            emitter,
            emitter_transform.compute_transform().scale.x,
            seed,
        ),
        angular_velocity: model_particle_angular_velocity(emitter, seed),
        acceleration: gravity_accel_bevy(emitter),
        lifetime: sample_model_particle_lifetime(emitter, seed),
    }
}

fn spawn_model_particle_root(
    commands: &mut Commands,
    spawn_params: &mut ModelParticleSpawnParams<'_, '_>,
    model_path: &Path,
    transform: Transform,
) -> Option<Entity> {
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
    m2_scene::spawn_animated_static_m2_parts(&mut ctx, model_path, transform)
        .map(|spawned| spawned.root)
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

fn resolve_particle_model_path(path: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(path);
    if direct.exists() {
        return Some(direct);
    }
    let fdid = game_engine::listfile::lookup_path(path)?;
    crate::asset::asset_cache::model(fdid)
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
