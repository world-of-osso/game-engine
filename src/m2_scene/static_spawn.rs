use std::path::Path;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::creature_display;
use crate::m2_effect_material::M2EffectMaterial;
use crate::skybox_m2_material::SkyboxM2Material;

use super::{attach_m2_model_parts, load_m2_model, load_m2_model_with_skin_fdids};

/// Spawn a static M2 model that still carries animation data.
pub struct SpawnedAnimatedStaticM2 {
    pub root: Entity,
    pub model_root: Entity,
}

/// Spawn a static M2 model that still carries animation data.
#[allow(clippy::too_many_arguments)]
pub fn spawn_animated_static_m2_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = load_m2_model(m2_path, creature_display_map) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(
        commands,
        meshes,
        materials,
        effect_materials,
        None,
        images,
        skinned_mesh_inverse_bindposes,
        m2_path,
        transform,
        model,
        false,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_animated_static_skybox_m2_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    skybox_materials: &mut Assets<SkyboxM2Material>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    creature_display_map: &creature_display::CreatureDisplayMap,
    skybox_color: Option<Color>,
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = load_m2_model(m2_path, creature_display_map) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(
        commands,
        meshes,
        materials,
        effect_materials,
        Some(skybox_materials),
        images,
        skinned_mesh_inverse_bindposes,
        m2_path,
        transform,
        model,
        true,
        skybox_color,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_animated_static_m2_parts_with_skin_fdids(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    skin_fdids: &[u32; 3],
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = load_m2_model_with_skin_fdids(m2_path, skin_fdids) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(
        commands,
        meshes,
        materials,
        effect_materials,
        None,
        images,
        skinned_mesh_inverse_bindposes,
        m2_path,
        transform,
        model,
        false,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_animated_static_m2_parts_from_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    skybox_materials: Option<&mut Assets<SkyboxM2Material>>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    model: crate::asset::m2::M2Model,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
) -> Option<SpawnedAnimatedStaticM2> {
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prop");
    let root = commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();
    let model_root = commands
        .spawn((
            Name::new(format!("{name}ModelRoot")),
            Transform::IDENTITY,
            Visibility::default(),
        ))
        .id();
    commands.entity(model_root).insert(ChildOf(root));
    attach_m2_model_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        skybox_materials,
        images,
        skinned_mesh_inverse_bindposes,
        model,
        model_root,
        false,
        force_skybox_material,
        skybox_color,
    );
    Some(SpawnedAnimatedStaticM2 { root, model_root })
}

/// Spawn a static M2 model that still carries animation data.
#[allow(clippy::too_many_arguments)]
pub fn spawn_animated_static_m2(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<Entity> {
    spawn_animated_static_m2_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        skinned_mesh_inverse_bindposes,
        m2_path,
        transform,
        creature_display_map,
    )
    .map(|spawned| spawned.root)
}
