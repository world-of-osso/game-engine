use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::VertexAttributeValues;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;

use crate::asset;
use crate::m2_effect_material::{self, M2EffectMaterial, M2EffectSettings};
use crate::skybox_m2_material::{SkyboxM2Material, SkyboxM2Settings};

static REPEAT_TEXTURE_CACHE: OnceLock<Mutex<std::collections::HashMap<u32, Handle<Image>>>> =
    OnceLock::new();

#[path = "m2_spawn_cache_stats.rs"]
mod cache_stats;
pub use cache_stats::{CompositedTextureCacheStats, composited_texture_cache_stats};

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
    spawn_mesh_with_material(
        commands,
        assets.meshes,
        mat,
        batch,
        root,
        skinning,
        batch_index,
        visible,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_mesh_with_material(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: BatchMaterial,
    batch: asset::m2::M2RenderBatch,
    root: Entity,
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    batch_index: usize,
    visible: bool,
) {
    match mat {
        BatchMaterial::Standard(mat) => spawn_skinned_mesh_standard(
            commands,
            meshes,
            mat,
            batch,
            root,
            skinning,
            batch_index,
            visible,
        ),
        BatchMaterial::Effect(mat) => spawn_skinned_mesh_effect(
            commands,
            meshes,
            mat,
            batch,
            root,
            skinning,
            batch_index,
            visible,
        ),
        BatchMaterial::Skybox(mat) => spawn_skinned_mesh_skybox(
            commands,
            meshes,
            mat,
            batch,
            root,
            skinning,
            batch_index,
            visible,
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

fn load_batch_material(
    batch: &asset::m2::M2RenderBatch,
    index: usize,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    skybox_materials: Option<&mut Assets<SkyboxM2Material>>,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
) -> BatchMaterial {
    let texture_dir = PathBuf::from("data/textures");
    if force_skybox_material {
        if let Some(materials) = skybox_materials {
            if let Some(mat) =
                try_load_skybox_material(batch, &texture_dir, images, materials, skybox_color)
            {
                return BatchMaterial::Skybox(mat);
            }
            return BatchMaterial::Skybox(materials.add(skybox_m2_material(
                None,
                Some(PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()]),
                batch,
            )));
        }
    }
    if should_use_effect_material(batch)
        && let Some(mat) = try_load_effect_material(batch, &texture_dir, images, effect_materials)
    {
        return BatchMaterial::Effect(mat);
    }
    if let Some(fdid) = batch.texture_fdid {
        let blp_path = asset::asset_cache::texture(fdid)
            .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
        if let Some(mat) =
            try_load_textured_material(&blp_path, batch, &texture_dir, images, materials)
        {
            return BatchMaterial::Standard(mat);
        }
    }
    let color = PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()];
    BatchMaterial::Standard(materials.add(m2_material(None, Some(color), batch)))
}

fn should_use_effect_material(batch: &asset::m2::M2RenderBatch) -> bool {
    batch.texture_2_fdid.is_some() && batch.blend_mode >= 2 && batch.overlays.is_empty()
}

fn try_load_textured_material(
    blp_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Option<Handle<StandardMaterial>> {
    if !blp_path.exists() {
        return None;
    }
    let image =
        crate::m2_texture_composite::load_composited_texture(blp_path, batch, texture_dir, images)
            .map_err(|e| eprintln!("{e}"))
            .ok()?;
    Some(materials.add(m2_material(Some(image), None, batch)))
}

fn try_load_effect_material(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<M2EffectMaterial>,
) -> Option<Handle<M2EffectMaterial>> {
    let base_fdid = batch.texture_fdid?;
    let second_fdid = batch.texture_2_fdid?;
    let base_texture = load_repeat_texture(base_fdid, texture_dir, images)?;
    let second_texture = load_repeat_texture(second_fdid, texture_dir, images)?;
    let alpha_test = match batch.blend_mode {
        1 => 224.0 / 255.0 * batch.transparency,
        2..=7 => (1.0 / 255.0) * batch.transparency,
        _ => 0.0,
    };
    Some(materials.add(M2EffectMaterial {
        settings: M2EffectSettings {
            transparency: batch.transparency,
            alpha_test,
            shader_id: batch.shader_id as u32,
            blend_mode: batch.blend_mode as u32,
            uv_mode_1: u32::from(batch.use_uv_2_1),
            uv_mode_2: u32::from(batch.use_uv_2_2),
            render_flags: batch.render_flags as u32,
            uv_offset_1: Vec2::ZERO,
            uv_offset_2: Vec2::ZERO,
        },
        base_texture,
        second_texture,
        blend_mode: batch.blend_mode,
        two_sided: batch.render_flags & 0x04 != 0,
        texture_anim_1: batch.texture_anim.clone(),
        texture_anim_2: batch.texture_anim_2.clone(),
    }))
}

fn try_load_skybox_material(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<SkyboxM2Material>,
    color: Option<Color>,
) -> Option<Handle<SkyboxM2Material>> {
    let fdid = batch.texture_fdid?;
    let blp_path = asset::asset_cache::texture(fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
    if !blp_path.exists() {
        return None;
    }
    let image =
        crate::m2_texture_composite::load_composited_texture(&blp_path, batch, texture_dir, images)
            .map_err(|e| eprintln!("{e}"))
            .ok()?;
    Some(materials.add(skybox_m2_material(Some(image), color, batch)))
}

fn load_repeat_texture(
    fdid: u32,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let cache = REPEAT_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(handle) = cache.lock().unwrap().get(&fdid).cloned() {
        return Some(handle);
    }
    let blp_path = asset::asset_cache::texture(fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
    if !blp_path.exists() {
        return None;
    }
    let (pixels, width, height) = asset::blp::load_blp_rgba(&blp_path).ok()?;
    let mut image = crate::rgba_image(pixels, width, height);
    image.sampler = m2_effect_material::repeat_sampler();
    let handle = images.add(image);
    cache.lock().unwrap().insert(fdid, handle.clone());
    Some(handle)
}

/// Build a StandardMaterial from M2 render flags (two-sided, unlit, blend mode).
pub fn m2_material(
    texture: Option<Handle<Image>>,
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
) -> StandardMaterial {
    let two_sided = batch.render_flags & 0x04 != 0;
    let unlit = batch.render_flags & 0x01 != 0;
    let cull_mode = if two_sided {
        None
    } else {
        Some(bevy::render::render_resource::Face::Back)
    };
    let alpha_mode = m2_effect_material::alpha_mode_for_blend(batch.blend_mode);
    StandardMaterial {
        base_color_texture: texture,
        base_color: color.unwrap_or(Color::srgba(1.0, 1.0, 1.0, batch.transparency)),
        unlit,
        cull_mode,
        double_sided: two_sided,
        alpha_mode,
        ..default()
    }
}

pub fn skybox_m2_material(
    texture: Option<Handle<Image>>,
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
) -> SkyboxM2Material {
    SkyboxM2Material {
        settings: SkyboxM2Settings {
            color: color
                .unwrap_or(Color::srgba(1.0, 1.0, 1.0, batch.transparency))
                .to_linear()
                .to_f32_array()
                .into(),
        },
        base_texture: texture.unwrap_or_default(),
        blend_mode: batch.blend_mode,
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchMaterial, ground_offset_y, load_batch_material};
    use crate::asset;
    use crate::m2_effect_material;
    use crate::skybox_m2_material::SkyboxM2Material;
    use bevy::mesh::{Mesh, PrimitiveTopology};
    use bevy::prelude::{AlphaMode, Assets, Image, StandardMaterial};

    #[test]
    fn ground_offset_uses_lowest_vertex_y() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0, 0.35, 0.0], [0.0, 1.0, 0.0], [0.0, 0.6, 0.0]],
        );
        let batch = asset::m2::M2RenderBatch {
            mesh,
            texture_fdid: None,
            texture_2_fdid: None,
            texture_type: None,
            overlays: Vec::new(),
            render_flags: 0,
            blend_mode: 0,
            transparency: 1.0,
            texture_anim: None,
            texture_anim_2: None,
            use_uv_2_1: false,
            use_uv_2_2: false,
            use_env_map_2: false,
            shader_id: 0,
            texture_count: 0,
            mesh_part_id: 0,
        };
        assert!((ground_offset_y(&[batch]) - 0.35).abs() < 0.001);
    }

    #[test]
    fn invalid_blend_modes_use_additive_alpha_mode() {
        assert!(matches!(
            m2_effect_material::alpha_mode_for_blend(u16::MAX),
            AlphaMode::Add
        ));
    }

    #[test]
    fn forced_skybox_batches_keep_dedicated_material_without_texture() {
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut effect_materials = Assets::<crate::m2_effect_material::M2EffectMaterial>::default();
        let mut skybox_materials = Assets::<SkyboxM2Material>::default();
        let batch = asset::m2::M2RenderBatch {
            mesh: Mesh::new(
                PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            ),
            texture_fdid: None,
            texture_2_fdid: None,
            texture_type: None,
            overlays: Vec::new(),
            render_flags: 0,
            blend_mode: 1,
            transparency: 1.0,
            texture_anim: None,
            texture_anim_2: None,
            use_uv_2_1: false,
            use_uv_2_2: false,
            use_env_map_2: false,
            shader_id: 0,
            texture_count: 0,
            mesh_part_id: 0,
        };

        let material = load_batch_material(
            &batch,
            0,
            &mut images,
            &mut materials,
            &mut effect_materials,
            Some(&mut skybox_materials),
            true,
            None,
        );

        assert!(matches!(material, BatchMaterial::Skybox(_)));
    }
}
