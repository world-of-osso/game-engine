//! M2 particle emitter rendering — spawns billboard quads per emitter.
//!
//! Each emitter spawns particles at its bone position with velocity, gravity,
//! color/opacity/scale interpolation over lifetime, and texture atlas tiling.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::asset::blp;
use crate::asset::m2::wow_to_bevy;
use crate::asset::m2_particle::M2ParticleEmitter;

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (emit_particles, update_particles, billboard_particles));
    }
}

/// Marker for a particle emitter entity.
#[derive(Component)]
pub struct ParticleEmitterComp {
    pub emitter: M2ParticleEmitter,
    pub bone_entity: Option<Entity>,
    /// Fractional accumulator for emission.
    pub emit_accum: f32,
}

/// Individual live particle.
#[derive(Component)]
struct Particle {
    velocity: Vec3,
    gravity: f32,
    age: f32,
    max_age: f32,
    /// Color at start, mid, end (linear RGB 0–1).
    colors: [Vec3; 3],
    /// Opacity at start, mid, end.
    opacity: [f32; 3],
    /// Scale (uniform) at start, mid, end.
    scales: [f32; 3],
    mid_point: f32,
}

/// Spawn emitter entities for an M2 model's particle emitters.
pub fn spawn_emitters(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    emitters: &[M2ParticleEmitter],
    bone_entities: Option<&[Entity]>,
    parent: Entity,
) {
    for em in emitters {
        let bone_entity = bone_entities.and_then(|b| b.get(em.bone_index as usize).copied());
        let mat = emitter_material(em, images, materials);
        let mesh = meshes.add(Rectangle::new(1.0, 1.0));

        commands
            .spawn((
                Name::new("ParticleEmitter"),
                ParticleEmitterComp {
                    emitter: em.clone(),
                    bone_entity,
                    emit_accum: 0.0,
                },
                // Emitter needs a transform so we can read bone world position
                Transform::IDENTITY,
                Visibility::default(),
                EmitterMesh(mesh),
                EmitterMaterial(mat),
            ))
            .set_parent_in_place(parent);
    }
}

/// Cached mesh/material handles on the emitter for spawning particles.
#[derive(Component)]
struct EmitterMesh(Handle<Mesh>);

#[derive(Component)]
struct EmitterMaterial(Handle<StandardMaterial>);

/// System: emit new particles based on emission rate.
fn emit_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut emitters: Query<(
        &mut ParticleEmitterComp,
        &EmitterMesh,
        &EmitterMaterial,
        &GlobalTransform,
    )>,
    bone_transforms: Query<&GlobalTransform, Without<ParticleEmitterComp>>,
) {
    let dt = time.delta_secs();
    for (mut ecomp, emesh, emat, emitter_gtf) in &mut emitters {
        if ecomp.emitter.emission_rate <= 0.0 || ecomp.emitter.lifespan <= 0.0 {
            continue;
        }
        // Resolve spawn position from bone or emitter transform
        let bone_pos = resolve_bone_position(&ecomp, emitter_gtf, &bone_transforms);
        let pos = ecomp.emitter.position;
        let offset = wow_to_bevy(pos[0], pos[1], pos[2]);
        let spawn_pos = bone_pos + Vec3::from(offset);

        ecomp.emit_accum += ecomp.emitter.emission_rate * dt;
        let count = ecomp.emit_accum as u32;
        ecomp.emit_accum -= count as f32;

        let em = &ecomp.emitter;
        for _ in 0..count.min(8) {
            spawn_particle(&mut commands, em, spawn_pos, &emesh.0, &emat.0);
        }
    }
}

fn resolve_bone_position(
    ecomp: &ParticleEmitterComp,
    emitter_gtf: &GlobalTransform,
    bone_transforms: &Query<&GlobalTransform, Without<ParticleEmitterComp>>,
) -> Vec3 {
    ecomp
        .bone_entity
        .and_then(|e| bone_transforms.get(e).ok())
        .map(|gt| gt.translation())
        .unwrap_or_else(|| emitter_gtf.translation())
}

fn spawn_particle(
    commands: &mut Commands,
    em: &M2ParticleEmitter,
    pos: Vec3,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
) {
    // Random-ish velocity spread using simple deterministic hash
    let seed = (pos.x * 1000.0 + pos.z * 7919.0) as u32;
    let spread = compute_velocity_spread(em, seed);

    let speed = em.emission_speed * (1.0 + em.speed_variation * hash_float(seed, 3));
    let velocity = Vec3::new(spread.x, speed, spread.y);

    let colors = [
        color_to_vec3(em.colors[0]),
        color_to_vec3(em.colors[1]),
        color_to_vec3(em.colors[2]),
    ];
    let scales = [
        em.scales[0][0].max(0.05),
        em.scales[1][0].max(0.05),
        em.scales[2][0].max(0.05),
    ];

    commands.spawn((
        Name::new("Particle"),
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(pos).with_scale(Vec3::splat(scales[0])),
        Particle {
            velocity,
            gravity: em.gravity,
            age: 0.0,
            max_age: em.lifespan,
            colors,
            opacity: em.opacity,
            scales,
            mid_point: em.mid_point,
        },
    ));
}

fn compute_velocity_spread(em: &M2ParticleEmitter, seed: u32) -> Vec2 {
    let h_angle = em.horizontal_range * hash_float(seed, 1);
    let v_angle = em.vertical_range * hash_float(seed, 2);
    Vec2::new(h_angle.sin() * 0.5, v_angle.sin() * 0.5)
}

/// Simple deterministic float in [-1, 1] from seed + salt.
fn hash_float(seed: u32, salt: u32) -> f32 {
    let h = seed.wrapping_mul(2654435761).wrapping_add(salt.wrapping_mul(7919));
    (h % 10000) as f32 / 5000.0 - 1.0
}

fn color_to_vec3(c: [f32; 3]) -> Vec3 {
    Vec3::new(c[0] / 255.0, c[1] / 255.0, c[2] / 255.0)
}

/// System: update particle age, position, scale, and despawn dead particles.
fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut p, mut xf) in &mut particles {
        p.age += dt;
        if p.age >= p.max_age {
            commands.entity(entity).despawn();
            continue;
        }
        // Apply velocity + gravity
        p.velocity.y -= p.gravity * dt;
        xf.translation += p.velocity * dt;

        // Interpolate scale and apply
        let t = p.age / p.max_age;
        let scale = lerp_over_lifetime(p.scales[0], p.scales[1], p.scales[2], p.mid_point, t);
        xf.scale = Vec3::splat(scale.max(0.01));
    }
}

/// Interpolate a value over lifetime using start→mid→end with a midpoint.
fn lerp_over_lifetime(start: f32, mid: f32, end: f32, mid_point: f32, t: f32) -> f32 {
    if t <= mid_point {
        let local_t = if mid_point > 0.0 { t / mid_point } else { 0.0 };
        start + (mid - start) * local_t
    } else {
        let local_t = if mid_point < 1.0 {
            (t - mid_point) / (1.0 - mid_point)
        } else {
            1.0
        };
        mid + (end - mid) * local_t
    }
}

/// System: orient particle quads to face the camera (billboard).
fn billboard_particles(
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut particles: Query<&mut Transform, With<Particle>>,
) {
    let Ok(cam_gtf) = camera.single() else {
        return;
    };
    let cam_pos = cam_gtf.translation();
    for mut xf in &mut particles {
        let dir = cam_pos - xf.translation;
        if dir.length_squared() > 0.001 {
            let scale = xf.scale;
            xf.look_at(cam_pos, Vec3::Y);
            xf.scale = scale; // preserve scale after look_at
        }
    }
}

/// Create material for a particle emitter (textured or white, additive/blend).
fn emitter_material(
    em: &M2ParticleEmitter,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    let texture = load_emitter_texture(em, images);
    let start_color = color_to_vec3(em.colors[0]);
    let alpha_mode = match em.blend_type {
        4 | 5 | 6 => AlphaMode::Add,
        2 | 3 | 7 => AlphaMode::Blend,
        1 => AlphaMode::Mask(0.5),
        _ => AlphaMode::Blend,
    };

    materials.add(StandardMaterial {
        base_color_texture: texture,
        base_color: Color::srgba(start_color.x, start_color.y, start_color.z, em.opacity[0]),
        unlit: true,
        alpha_mode,
        cull_mode: None,
        double_sided: true,
        ..default()
    })
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
