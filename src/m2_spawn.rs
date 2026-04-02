use std::path::Path;

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::VertexAttributeValues;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;

use crate::asset;
use crate::m2_effect_material::M2EffectMaterial;
use crate::skybox_m2_material::SkyboxM2Material;

#[path = "m2_spawn_cache_stats.rs"]
mod cache_stats;
pub use cache_stats::{CompositedTextureCacheStats, composited_texture_cache_stats};

#[path = "m2_spawn_material.rs"]
mod material;
use material::load_batch_material;
#[cfg(test)]
pub(crate) use material::skybox_m2_material;

/// Component tagging a mesh entity with its M2 geoset mesh_part_id.
#[derive(Component)]
pub struct GeosetMesh(pub u16);

/// Component tagging a mesh entity with the resolved M2 texture type.
#[derive(Component)]
pub struct BatchTextureType(pub u32);

/// Grouped asset params for M2 spawning.
pub struct SpawnAssets<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub effect_materials: &'a mut Assets<M2EffectMaterial>,
    pub skybox_materials: Option<&'a mut Assets<SkyboxM2Material>>,
    pub images: &'a mut Assets<Image>,
    pub inverse_bindposes: &'a mut Assets<SkinnedMeshInverseBindposes>,
}

/// Attach M2 model meshes as children of an existing entity.
/// Returns true if the model was loaded and attached successfully.
pub fn spawn_m2_on_entity(
    commands: &mut Commands,
    assets: &mut SpawnAssets<'_>,
    m2_path: &Path,
    entity: Entity,
    skin_fdids: &[u32; 3],
) -> bool {
    spawn_m2_on_entity_filtered(commands, assets, m2_path, entity, skin_fdids, |_| true)
}

pub fn spawn_m2_on_entity_filtered(
    commands: &mut Commands,
    assets: &mut SpawnAssets<'_>,
    m2_path: &Path,
    entity: Entity,
    skin_fdids: &[u32; 3],
    filter: impl Fn(u16) -> bool,
) -> bool {
    let model = match asset::m2::load_m2_uncached(m2_path, skin_fdids) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to load M2 {}: {e}", m2_path.display());
            return false;
        }
    };
    let batches = model
        .batches
        .into_iter()
        .filter(|batch| filter(batch.mesh_part_id))
        .collect::<Vec<_>>();
    let grounded_root = ensure_grounded_model_root(commands, entity, ground_offset_y(&batches));
    attach_m2_batches(
        commands,
        assets,
        batches,
        &model.bones,
        grounded_root,
        false,
        None,
    );
    true
}

pub fn spawn_m2_on_entity_filtered_bound_to_existing_joints(
    commands: &mut Commands,
    assets: &mut SpawnAssets<'_>,
    m2_path: &Path,
    entity: Entity,
    skin_fdids: &[u32; 3],
    filter: impl Fn(u16) -> bool,
    target_joints: &[Entity],
    names: &Query<&Name>,
) -> bool {
    let model = match asset::m2::load_m2_uncached(m2_path, skin_fdids) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to load M2 {}: {e}", m2_path.display());
            return false;
        }
    };
    let asset::m2::M2Model { batches, bones, .. } = model;
    let batches = batches
        .into_iter()
        .filter(|batch| filter(batch.mesh_part_id))
        .collect::<Vec<_>>();
    let skinning = bind_existing_skeleton(
        commands,
        assets.inverse_bindposes,
        &bones,
        entity,
        target_joints,
        names,
    );
    for (i, batch) in batches.into_iter().enumerate() {
        spawn_batch_mesh(commands, assets, batch, entity, &skinning, i, false, None);
    }
    true
}

pub fn ground_offset_y(batches: &[asset::m2::M2RenderBatch]) -> f32 {
    let mut min_y = f32::INFINITY;
    for batch in batches {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            batch.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        for position in positions {
            min_y = min_y.min(position[1]);
        }
    }
    if min_y.is_finite() && min_y.abs() > 0.001 {
        min_y
    } else {
        0.0
    }
}

pub fn ensure_grounded_model_root(
    commands: &mut Commands,
    parent: Entity,
    ground_offset_y: f32,
) -> Entity {
    if ground_offset_y.abs() <= 0.001 {
        return parent;
    }
    let root = commands
        .spawn((
            Name::new("GroundedModelRoot"),
            Transform::from_xyz(0.0, -ground_offset_y, 0.0),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).set_parent_in_place(parent);
    root
}

/// Skinning data returned from mesh attachment, for animation setup.
pub type SkinningResult = Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>;

/// Runtime data linking a spawned point light to its M2 light definition and animation owner.
#[derive(Component)]
pub struct RuntimeM2PointLight {
    pub light: asset::m2_light::M2Light,
    pub anim_owner: Entity,
}

fn point_light_parent(
    light: &asset::m2_light::M2Light,
    skinning: &SkinningResult,
    root: Entity,
) -> Entity {
    let Some(bone_index) = usize::try_from(light.bone_index).ok() else {
        return root;
    };
    skinning
        .as_ref()
        .and_then(|(_, joints)| joints.get(bone_index).copied())
        .unwrap_or(root)
}

fn authored_point_light(authored: &asset::m2_light::EvaluatedLight) -> PointLight {
    PointLight {
        color: Color::linear_rgb(authored.color[0], authored.color[1], authored.color[2]),
        intensity: authored.intensity,
        range: authored.attenuation_end,
        radius: authored.attenuation_start.min(authored.attenuation_end),
        shadows_enabled: false,
        ..default()
    }
}

pub fn spawn_model_point_lights(
    commands: &mut Commands,
    lights: &[asset::m2_light::M2Light],
    skinning: &SkinningResult,
    root: Entity,
    anim_owner: Entity,
) {
    for (index, light) in lights
        .iter()
        .filter(|l| l.light_type == asset::m2_light::M2_LIGHT_TYPE_POINT)
        .enumerate()
    {
        let parent = point_light_parent(light, skinning, root);
        let pos = asset::m2::wow_to_bevy(light.position[0], light.position[1], light.position[2]);
        let authored = asset::m2_light::evaluate_light(light, 0, 0);
        let vis = if authored.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        commands.entity(parent).with_children(|children| {
            children.spawn((
                Name::new(format!("M2PointLight{index}")),
                Transform::from_translation(pos.into()),
                authored_point_light(&authored),
                vis,
                RuntimeM2PointLight {
                    light: light.clone(),
                    anim_owner,
                },
            ));
        });
    }
}

enum BatchMaterial {
    Standard(Handle<StandardMaterial>),
    Effect(Handle<M2EffectMaterial>),
    Skybox(Handle<SkyboxM2Material>),
}

/// Spawn M2 mesh batches as children of a root entity, with optional skinning.
/// Returns the skinning data for optional animation setup.
pub fn attach_m2_batches(
    commands: &mut Commands,
    assets: &mut SpawnAssets<'_>,
    batches: Vec<asset::m2::M2RenderBatch>,
    bones: &[asset::m2_anim::M2Bone],
    root: Entity,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
) -> SkinningResult {
    let skinning = spawn_skeleton(commands, assets.inverse_bindposes, bones, root);
    for (i, batch) in batches.into_iter().enumerate() {
        spawn_batch_mesh(
            commands,
            assets,
            batch,
            root,
            &skinning,
            i,
            force_skybox_material,
            skybox_color,
        );
    }
    skinning
}

fn bind_existing_skeleton(
    commands: &mut Commands,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    bones: &[asset::m2_anim::M2Bone],
    model_entity: Entity,
    target_joints: &[Entity],
    names: &Query<&Name>,
) -> SkinningResult {
    if bones.is_empty() {
        return None;
    }
    let named_targets = target_joints
        .iter()
        .filter_map(|entity| {
            names
                .get(*entity)
                .ok()
                .map(|name| (name.as_str().to_owned(), *entity))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut mapped_joints = Vec::with_capacity(bones.len());
    for (i, bone) in bones.iter().enumerate() {
        let bone_name = asset::m2_bone_names::bone_display_name(bone.key_bone_id, i);
        if let Some(entity) = named_targets.get(&bone_name) {
            mapped_joints.push(*entity);
            continue;
        }
        let fallback = commands
            .spawn((
                Transform::IDENTITY,
                Visibility::default(),
                Name::new(format!("Unmapped{bone_name}")),
            ))
            .id();
        commands.entity(fallback).set_parent_in_place(model_entity);
        mapped_joints.push(fallback);
    }
    let inv_bp = inverse_bindposes.add(SkinnedMeshInverseBindposes::from(vec![
        Mat4::IDENTITY;
        bones.len()
    ]));
    Some((inv_bp, mapped_joints))
}

fn spawn_batch_mesh(
    commands: &mut Commands,
    assets: &mut SpawnAssets<'_>,
    batch: asset::m2::M2RenderBatch,
    root: Entity,
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    batch_index: usize,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
) {
    let visible = asset::m2::default_geoset_visible(batch.mesh_part_id);
    let mat = load_batch_material(
        &batch,
        batch_index,
        assets.images,
        assets.materials,
        assets.effect_materials,
        assets.skybox_materials.as_deref_mut(),
        force_skybox_material,
        skybox_color,
    );
    let mut context = MeshSpawnContext {
        parent: root,
        skinning,
        batch_index,
        visible,
    };
    spawn_mesh_with_material(commands, assets.meshes, &mut context, mat, batch);
}

struct MeshSpawnContext<'a> {
    parent: Entity,
    skinning: &'a Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    batch_index: usize,
    visible: bool,
}

fn spawn_mesh_with_material(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    context: &mut MeshSpawnContext<'_>,
    mat: BatchMaterial,
    batch: asset::m2::M2RenderBatch,
) {
    match mat {
        BatchMaterial::Standard(mat) => spawn_skinned_mesh_standard(
            commands,
            meshes,
            mat,
            batch,
            context.parent,
            context.skinning,
            context.batch_index,
            context.visible,
        ),
        BatchMaterial::Effect(mat) => spawn_skinned_mesh_effect(
            commands,
            meshes,
            mat,
            batch,
            context.parent,
            context.skinning,
            context.batch_index,
            context.visible,
        ),
        BatchMaterial::Skybox(mat) => spawn_skinned_mesh_skybox(
            commands,
            meshes,
            mat,
            batch,
            context.parent,
            context.skinning,
            context.batch_index,
            context.visible,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_common_mesh_components(
    cmd: &mut EntityCommands,
    texture_type: Option<u32>,
    mesh_part_id: u16,
    parent: Entity,
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
) {
    cmd.insert(GeosetMesh(mesh_part_id));
    if let Some(texture_type) = texture_type {
        cmd.insert(BatchTextureType(texture_type));
    }
    cmd.set_parent_in_place(parent);
    if let Some((inv_bp, joints)) = skinning {
        cmd.insert(SkinnedMesh {
            inverse_bindposes: inv_bp.clone(),
            joints: joints.clone(),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_skinned_mesh_standard(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    batch: asset::m2::M2RenderBatch,
    parent: Entity,
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    batch_index: usize,
    visible: bool,
) {
    let asset::m2::M2RenderBatch {
        mesh,
        texture_type,
        mesh_part_id,
        ..
    } = batch;
    let vis = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let mut cmd = commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Name::new(format!("Mesh[{batch_index}]")),
        vis,
    ));
    spawn_common_mesh_components(&mut cmd, texture_type, mesh_part_id, parent, skinning);
}

#[allow(clippy::too_many_arguments)]
fn spawn_skinned_mesh_effect(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<M2EffectMaterial>,
    batch: asset::m2::M2RenderBatch,
    parent: Entity,
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    batch_index: usize,
    visible: bool,
) {
    let asset::m2::M2RenderBatch {
        mesh,
        texture_type,
        mesh_part_id,
        ..
    } = batch;
    let vis = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let mut cmd = commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Name::new(format!("Mesh[{batch_index}]")),
        vis,
    ));
    spawn_common_mesh_components(&mut cmd, texture_type, mesh_part_id, parent, skinning);
}

#[allow(clippy::too_many_arguments)]
fn spawn_skinned_mesh_skybox(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<SkyboxM2Material>,
    batch: asset::m2::M2RenderBatch,
    parent: Entity,
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    batch_index: usize,
    visible: bool,
) {
    let asset::m2::M2RenderBatch {
        mesh,
        texture_type,
        mesh_part_id,
        ..
    } = batch;
    let vis = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let mut cmd = commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Name::new(format!("Mesh[{batch_index}]")),
        NoFrustumCulling,
        NotShadowCaster,
        NotShadowReceiver,
        vis,
    ));
    spawn_common_mesh_components(&mut cmd, texture_type, mesh_part_id, parent, skinning);
}

/// Spawn bone entities in parent-child hierarchy and create inverse bind poses.
fn spawn_skeleton(
    commands: &mut Commands,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    bones: &[asset::m2_anim::M2Bone],
    model_entity: Entity,
) -> Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)> {
    if bones.is_empty() {
        return None;
    }
    let joint_entities: Vec<Entity> = bones
        .iter()
        .enumerate()
        .map(|(i, bone)| {
            commands
                .spawn((
                    Transform::IDENTITY,
                    Visibility::default(),
                    Name::new(asset::m2_bone_names::bone_display_name(bone.key_bone_id, i)),
                ))
                .id()
        })
        .collect();
    for (i, bone) in bones.iter().enumerate() {
        let parent = if bone.parent_bone_id >= 0 {
            joint_entities[bone.parent_bone_id as usize]
        } else {
            model_entity
        };
        commands
            .entity(joint_entities[i])
            .set_parent_in_place(parent);
    }
    let inv_bp = inverse_bindposes.add(SkinnedMeshInverseBindposes::from(
        build_inverse_bind_poses(bones),
    ));
    Some((inv_bp, joint_entities))
}

fn build_inverse_bind_poses(bones: &[asset::m2_anim::M2Bone]) -> Vec<Mat4> {
    bones
        .iter()
        .map(|bone| {
            if bone.flags & crate::animation::M2_BONE_SPHERICAL_BILLBOARD != 0 {
                let pivot = Vec3::new(bone.pivot[0], bone.pivot[2], -bone.pivot[1]);
                Mat4::from_translation(-pivot)
            } else {
                Mat4::IDENTITY
            }
        })
        .collect()
}

const PLACEHOLDER_COLORS: &[Color] = &[
    Color::srgb(0.8, 0.5, 0.3),
    Color::srgb(0.3, 0.5, 0.8),
    Color::srgb(0.7, 0.7, 0.3),
    Color::srgb(0.6, 0.3, 0.7),
];
