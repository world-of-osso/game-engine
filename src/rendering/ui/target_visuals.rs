use std::path::Path;

use bevy::camera::primitives::Aabb;
use bevy::image::Image;
use bevy::pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt};
use bevy::prelude::*;
use game_engine::asset::blp::load_blp_to_image;

use crate::networking::ResolvedModelAssetInfo;
use crate::rendering::image_sampler::clamp_linear_sampler;

use super::{TargetCircleStyle, TargetMarker, TargetMarkerScaleFactor};

const TARGET_CIRCLE_SIZE_FACTOR: f32 = 0.7;

pub(super) fn target_circle_transform(target_translation: Vec3) -> Transform {
    target_circle_transform_scaled(target_translation, 1.0)
}

pub(super) fn target_circle_transform_scaled(target_translation: Vec3, scale: f32) -> Transform {
    Transform::from_translation(target_translation + Vec3::Y * 0.08)
        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat(scale.max(0.01)))
}

pub(super) fn target_circle_size(
    target: Entity,
    parent_query: &Query<&ChildOf>,
    target_global_q: &Query<&GlobalTransform, Without<TargetMarker>>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
    model_info_q: &Query<&ResolvedModelAssetInfo>,
) -> f32 {
    let mut size = target_footprint_size(target, parent_query, target_global_q, aabb_query);
    if size <= 0.0 {
        size = model_info_q
            .get(target)
            .ok()
            .and_then(|info| info.display_scale)
            .filter(|scale| *scale > 0.0)
            .unwrap_or(1.0);
    }
    (size * TARGET_CIRCLE_SIZE_FACTOR).max(0.35)
}

fn target_footprint_size(
    target: Entity,
    parent_query: &Query<&ChildOf>,
    target_global_q: &Query<&GlobalTransform, Without<TargetMarker>>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
) -> f32 {
    let Ok(target_global) = target_global_q.get(target) else {
        return 0.0;
    };
    let target_origin = target_global.translation();
    let mut max_extent = 0.0_f32;
    for (candidate, aabb, transform) in aabb_query.iter() {
        if !is_descendant_or_self(candidate, target, parent_query) {
            continue;
        }
        for point in world_aabb_corners(aabb, transform) {
            let delta = point - target_origin;
            max_extent = max_extent.max(delta.x.abs()).max(delta.z.abs());
        }
    }
    max_extent
}

fn world_aabb_corners(aabb: &Aabb, transform: &GlobalTransform) -> [Vec3; 8] {
    let center: Vec3 = aabb.center.into();
    let extents: Vec3 = aabb.half_extents.into();
    let affine = transform.affine();
    [
        point(center, extents, -1.0, -1.0, -1.0),
        point(center, extents, -1.0, -1.0, 1.0),
        point(center, extents, -1.0, 1.0, -1.0),
        point(center, extents, -1.0, 1.0, 1.0),
        point(center, extents, 1.0, -1.0, -1.0),
        point(center, extents, 1.0, -1.0, 1.0),
        point(center, extents, 1.0, 1.0, -1.0),
        point(center, extents, 1.0, 1.0, 1.0),
    ]
    .map(|v| affine.transform_point3(v))
}

fn point(center: Vec3, extents: Vec3, sx: f32, sy: f32, sz: f32) -> Vec3 {
    Vec3::new(
        center.x + extents.x * sx,
        center.y + extents.y * sy,
        center.z + extents.z * sz,
    )
}

fn is_descendant_or_self(
    candidate: Entity,
    ancestor: Entity,
    parent_query: &Query<&ChildOf>,
) -> bool {
    if candidate == ancestor {
        return true;
    }
    let mut current = candidate;
    while let Ok(parent) = parent_query.get(current) {
        current = parent.parent();
        if current == ancestor {
            return true;
        }
    }
    false
}

pub(super) fn spawn_target_circle(
    current: Res<game_engine::targeting::CurrentTarget>,
    style: Res<TargetCircleStyle>,
    mut commands: Commands,
    existing: Query<Entity, With<TargetMarker>>,
    parent_query: Query<&ChildOf>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut decal_materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
    mut images: ResMut<Assets<Image>>,
    target_tf: Query<&Transform>,
    target_global_q: Query<&GlobalTransform, Without<TargetMarker>>,
    aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
    model_info_q: Query<&ResolvedModelAssetInfo>,
) {
    if !current.is_changed() && !style.is_changed() {
        return;
    }
    for e in existing.iter() {
        commands.entity(e).despawn();
    }
    let Some(target) = current.0 else { return };
    let Ok(tf) = target_tf.get(target) else {
        return;
    };
    let circle_size = target_circle_size(
        target,
        &parent_query,
        &target_global_q,
        &aabb_query,
        &model_info_q,
    );
    match style.as_ref() {
        TargetCircleStyle::Procedural => {
            spawn_procedural_fill(
                &mut commands,
                &mut meshes,
                &mut materials,
                tf.translation,
                circle_size,
            );
            spawn_procedural_ring(
                &mut commands,
                &mut meshes,
                &mut materials,
                tf.translation,
                circle_size,
            );
        }
        TargetCircleStyle::Blp {
            base_fdid,
            glow_fdid,
            emissive,
            ..
        } => {
            let e = emissive_from_rgb(*emissive);
            let base = format!("data/textures/{base_fdid}.blp");
            spawn_target_textured(
                &mut commands,
                &mut decal_materials,
                &mut images,
                tf.translation,
                Path::new(&base),
                e,
                circle_size,
            );
            if let Some(glow) = glow_fdid {
                let glow_path = format!("data/textures/{glow}.blp");
                spawn_target_textured(
                    &mut commands,
                    &mut decal_materials,
                    &mut images,
                    tf.translation,
                    Path::new(&glow_path),
                    e,
                    circle_size,
                );
            }
        }
    }
}

fn emissive_from_rgb(rgb: [u8; 3]) -> LinearRgba {
    LinearRgba::rgb(
        rgb[0] as f32 / 255.0 * 1.5,
        rgb[1] as f32 / 255.0 * 1.5,
        rgb[2] as f32 / 255.0 * 1.5,
    )
}

fn spawn_procedural_fill(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    translation: Vec3,
    scale: f32,
) {
    let fill = meshes.add(Circle::new(0.68).mesh().resolution(64).build());
    let fill_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.92, 0.18, 0.10),
        emissive: LinearRgba::rgb(0.8, 0.68, 0.1),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(fill),
        MeshMaterial3d(fill_mat),
        target_circle_transform_scaled(translation, scale),
        TargetMarkerScaleFactor(1.0),
        TargetMarker,
    ));
}

fn spawn_procedural_ring(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    translation: Vec3,
    scale: f32,
) {
    let ring = meshes.add(Annulus::new(0.7, 0.95).mesh().resolution(64));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.95, 0.2, 0.45),
        emissive: LinearRgba::rgb(3.0, 2.6, 0.55),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(ring),
        MeshMaterial3d(mat),
        target_circle_transform_scaled(translation, scale),
        TargetMarkerScaleFactor(1.0),
        TargetMarker,
    ));
}

fn spawn_target_textured(
    commands: &mut Commands,
    materials: &mut Assets<ForwardDecalMaterial<StandardMaterial>>,
    images: &mut Assets<Image>,
    translation: Vec3,
    blp_path: &Path,
    emissive: LinearRgba,
    scale: f32,
) {
    let Ok(mut image) = load_blp_to_image(blp_path) else {
        warn!("Failed to load target texture: {}", blp_path.display());
        return;
    };
    if is_fully_opaque(&image) {
        convert_opaque_image_to_alpha_mask(&mut image);
    }
    image.sampler = clamp_linear_sampler();
    let texture = images.add(image);
    let tint = Color::linear_rgba(emissive.red, emissive.green, emissive.blue, 1.0);
    let mat = materials.add(ForwardDecalMaterial {
        base: StandardMaterial {
            base_color: tint,
            base_color_texture: Some(texture.clone()),
            emissive,
            emissive_texture: Some(texture),
            unlit: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            reflectance: 0.0,
            perceptual_roughness: 1.0,
            ..default()
        },
        extension: ForwardDecalMaterialExt {
            depth_fade_factor: 0.35,
        },
    });
    commands.spawn((
        MeshMaterial3d(mat),
        ForwardDecal,
        target_circle_decal_transform(translation, scale),
        TargetMarkerScaleFactor(2.0),
        TargetMarker,
    ));
}

fn is_fully_opaque(image: &Image) -> bool {
    let Some(data) = image.data.as_ref() else {
        return false;
    };
    data.iter().skip(3).step_by(4).all(|&a| a == 255)
}

pub(super) fn convert_opaque_image_to_alpha_mask(image: &mut Image) {
    let Some(data) = image.data.as_mut() else {
        return;
    };
    for rgba in data.chunks_exact_mut(4) {
        let intensity = rgba[0].max(rgba[1]).max(rgba[2]);
        rgba[0] = intensity;
        rgba[1] = intensity;
        rgba[2] = intensity;
        rgba[3] = intensity;
    }
}

fn target_circle_decal_transform(target_translation: Vec3, scale: f32) -> Transform {
    Transform::from_translation(target_translation + Vec3::Y * 0.08)
        .with_scale(Vec3::splat((scale * 2.0).max(0.01)))
}

pub(super) fn update_target_circle(
    current: Res<game_engine::targeting::CurrentTarget>,
    target_tf: Query<&Transform, Without<TargetMarker>>,
    target_global_q: Query<&GlobalTransform, Without<TargetMarker>>,
    parent_query: Query<&ChildOf>,
    aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
    model_info_q: Query<&ResolvedModelAssetInfo>,
    mut circle_q: Query<(&mut Transform, &TargetMarkerScaleFactor), With<TargetMarker>>,
) {
    let Some(target) = current.0 else { return };
    let Ok(tf) = target_tf.get(target) else {
        return;
    };
    let circle_size = target_circle_size(
        target,
        &parent_query,
        &target_global_q,
        &aabb_query,
        &model_info_q,
    );
    for (mut circle_tf, scale_factor) in circle_q.iter_mut() {
        circle_tf.translation = tf.translation + Vec3::Y * 0.05;
        circle_tf.scale = Vec3::splat((circle_size * scale_factor.0).max(0.01));
    }
}
