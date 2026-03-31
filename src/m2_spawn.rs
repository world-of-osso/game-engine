use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::mesh::VertexAttributeValues;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;

use crate::asset;
use crate::m2_effect_material::{self, M2EffectMaterial, M2EffectSettings};

#[derive(Clone, PartialEq, Eq, Hash)]
struct TextureCacheKey {
    base_path: PathBuf,
    overlays: Vec<asset::m2::TextureOverlay>,
    texture_2_fdid: Option<u32>,
    shader_id: u16,
    blend_mode: u16,
}

static COMPOSITED_TEXTURE_CACHE: OnceLock<
    Mutex<std::collections::HashMap<TextureCacheKey, Result<Handle<Image>, String>>>,
> = OnceLock::new();

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
    attach_m2_batches(commands, assets, batches, &model.bones, grounded_root);
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
        spawn_batch_mesh(commands, assets, batch, entity, &skinning, i);
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

enum BatchMaterial {
    Standard(Handle<StandardMaterial>),
    Effect(Handle<M2EffectMaterial>),
}

/// Spawn M2 mesh batches as children of a root entity, with optional skinning.
/// Returns the skinning data for optional animation setup.
pub fn attach_m2_batches(
    commands: &mut Commands,
    assets: &mut SpawnAssets<'_>,
    batches: Vec<asset::m2::M2RenderBatch>,
    bones: &[asset::m2_anim::M2Bone],
    root: Entity,
) -> SkinningResult {
    let skinning = spawn_skeleton(commands, assets.inverse_bindposes, bones, root);
    for (i, batch) in batches.into_iter().enumerate() {
        spawn_batch_mesh(commands, assets, batch, root, &skinning, i);
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
) {
    let visible = asset::m2::default_geoset_visible(batch.mesh_part_id);
    let mat = load_batch_material(
        &batch,
        batch_index,
        assets.images,
        assets.materials,
        assets.effect_materials,
    );
    match mat {
        BatchMaterial::Standard(mat) => spawn_skinned_mesh_standard(
            commands,
            assets.meshes,
            mat,
            batch,
            root,
            skinning,
            batch_index,
            visible,
        ),
        BatchMaterial::Effect(mat) => spawn_skinned_mesh_effect(
            commands,
            assets.meshes,
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
    let inv_bp = inverse_bindposes.add(SkinnedMeshInverseBindposes::from(vec![
        Mat4::IDENTITY;
        bones.len()
    ]));
    Some((inv_bp, joint_entities))
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
) -> BatchMaterial {
    let texture_dir = PathBuf::from("data/textures");
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
    let image = load_composited_texture(blp_path, batch, texture_dir, images)
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
    let alpha_mode = match batch.blend_mode {
        1 => AlphaMode::Mask(224.0 / 255.0),
        2 | 3 | 7 => AlphaMode::Blend,
        4..=6 => AlphaMode::Add,
        _ => AlphaMode::Opaque,
    };
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

fn load_composited_texture(
    base_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let key = composited_texture_cache_key(base_path, batch);
    let cache =
        COMPOSITED_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&key).cloned() {
        return cached;
    }
    let handle = build_composited_texture_handle(base_path, batch, texture_dir, images)?;
    cache.lock().unwrap().insert(key, Ok(handle.clone()));
    Ok(handle)
}

fn composited_texture_cache_key(
    base_path: &Path,
    batch: &asset::m2::M2RenderBatch,
) -> TextureCacheKey {
    TextureCacheKey {
        base_path: base_path.to_path_buf(),
        overlays: batch.overlays.clone(),
        texture_2_fdid: batch.texture_2_fdid,
        shader_id: batch.shader_id,
        blend_mode: batch.blend_mode,
    }
}

fn build_composited_texture_handle(
    base_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let (mut pixels, w, h) = asset::blp::load_blp_rgba(base_path)
        .map_err(|e| format!("Failed to load BLP {}: {e}", base_path.display()))?;
    if let Some(texture_2_fdid) = batch.texture_2_fdid
        && !batch.use_env_map_2
    {
        composite_second_texture(
            &mut pixels,
            w,
            h,
            texture_2_fdid,
            batch.shader_id,
            texture_dir,
        );
    }
    for ov in &batch.overlays {
        composite_overlay(&mut pixels, w, ov, texture_dir);
    }
    let mut image = crate::rgba_image(pixels, w, h);
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    Ok(images.add(image))
}

fn composite_second_texture(
    base_pixels: &mut [u8],
    base_width: u32,
    base_height: u32,
    overlay_fdid: u32,
    shader_id: u16,
    texture_dir: &Path,
) {
    let overlay_path = asset::asset_cache::texture(overlay_fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{overlay_fdid}.blp")));
    let Ok((overlay_pixels, overlay_width, overlay_height)) =
        asset::blp::load_blp_rgba(&overlay_path)
    else {
        eprintln!(
            "Failed to load secondary texture {}",
            overlay_path.display()
        );
        return;
    };

    for y in 0..base_height {
        for x in 0..base_width {
            let base_idx = ((y * base_width + x) * 4) as usize;
            let ox = x.rem_euclid(overlay_width);
            let oy = y.rem_euclid(overlay_height);
            let overlay_idx = ((oy * overlay_width + ox) * 4) as usize;
            let base = &mut base_pixels[base_idx..base_idx + 4];
            let overlay = &overlay_pixels[overlay_idx..overlay_idx + 4];
            apply_m2_multitexture_shader(base, overlay, shader_id);
        }
    }
}

fn apply_m2_multitexture_shader(base: &mut [u8], overlay: &[u8], shader_id: u16) {
    let base_rgb = [
        base[0] as f32 / 255.0,
        base[1] as f32 / 255.0,
        base[2] as f32 / 255.0,
    ];
    let base_a = base[3] as f32 / 255.0;
    let overlay_rgb = [
        overlay[0] as f32 / 255.0,
        overlay[1] as f32 / 255.0,
        overlay[2] as f32 / 255.0,
    ];
    let overlay_a = overlay[3] as f32 / 255.0;

    let (rgb, a) = shader_blend(base_rgb, base_a, overlay_rgb, overlay_a, shader_id);

    base[0] = (rgb[0] * 255.0).round() as u8;
    base[1] = (rgb[1] * 255.0).round() as u8;
    base[2] = (rgb[2] * 255.0).round() as u8;
    base[3] = (a * 255.0).round() as u8;
}

fn shader_blend(
    base_rgb: [f32; 3],
    base_a: f32,
    overlay_rgb: [f32; 3],
    overlay_a: f32,
    shader_id: u16,
) -> ([f32; 3], f32) {
    match shader_id {
        0x8000 => (base_rgb, (base_a * overlay_a).clamp(0.0, 1.0)),
        0x4014 => (
            mul_2x_rgb(base_rgb, overlay_rgb),
            (base_a * overlay_a * 2.0).clamp(0.0, 1.0),
        ),
        0x0010 => (mul_rgb(base_rgb, overlay_rgb), base_a),
        0x0011 => (
            mul_rgb(base_rgb, overlay_rgb),
            (base_a * overlay_a).clamp(0.0, 1.0),
        ),
        0x4016 => (mul_2x_rgb(base_rgb, overlay_rgb), base_a),
        0x8015 => (add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, 1.0), 1.0),
        0x8001 => (shader_8001_rgb(base_rgb, base_a, overlay_rgb), 1.0),
        0x8002 => (add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, 1.0), 1.0),
        0x8003 => (
            add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, base_a),
            1.0,
        ),
        _ => (base_rgb, base_a),
    }
}

fn mul_rgb(base_rgb: [f32; 3], overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * overlay_rgb[0]).clamp(0.0, 1.0),
        (base_rgb[1] * overlay_rgb[1]).clamp(0.0, 1.0),
        (base_rgb[2] * overlay_rgb[2]).clamp(0.0, 1.0),
    ]
}

fn mul_2x_rgb(base_rgb: [f32; 3], overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * overlay_rgb[0] * 2.0).clamp(0.0, 1.0),
        (base_rgb[1] * overlay_rgb[1] * 2.0).clamp(0.0, 1.0),
        (base_rgb[2] * overlay_rgb[2] * 2.0).clamp(0.0, 1.0),
    ]
}

fn add_overlay_rgb(
    base_rgb: [f32; 3],
    overlay_rgb: [f32; 3],
    overlay_a: f32,
    weight: f32,
) -> [f32; 3] {
    [
        (base_rgb[0] + overlay_rgb[0] * overlay_a * weight).clamp(0.0, 1.0),
        (base_rgb[1] + overlay_rgb[1] * overlay_a * weight).clamp(0.0, 1.0),
        (base_rgb[2] + overlay_rgb[2] * overlay_a * weight).clamp(0.0, 1.0),
    ]
}

fn shader_8001_rgb(base_rgb: [f32; 3], base_a: f32, overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * ((overlay_rgb[0] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
        (base_rgb[1] * ((overlay_rgb[1] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
        (base_rgb[2] * ((overlay_rgb[2] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::{apply_m2_multitexture_shader, ground_offset_y};
    use crate::asset;
    use bevy::mesh::{Mesh, PrimitiveTopology};

    #[test]
    fn shader_8015_uses_secondary_alpha_as_additive_mask() {
        let mut base = [128, 64, 32, 51];
        let overlay = [255, 128, 0, 128];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x8015);

        assert_eq!(base, [255, 128, 32, 255]);
    }

    #[test]
    fn shader_0011_modulates_rgb_and_alpha() {
        let mut base = [255, 255, 255, 255];
        let overlay = [128, 64, 32, 64];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x0011);

        assert_eq!(base, [128, 64, 32, 64]);
    }

    #[test]
    fn shader_4016_modulates_rgb_2x_and_keeps_base_alpha() {
        let mut base = [128, 128, 128, 51];
        let overlay = [128, 255, 64, 13];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x4016);

        assert_eq!(base, [129, 255, 64, 51]);
    }

    #[test]
    fn shader_8000_uses_secondary_alpha_as_mask_only() {
        let mut base = [128, 64, 32, 128];
        let overlay = [0, 255, 255, 64];
        apply_m2_multitexture_shader(&mut base, &overlay, 0x8000);

        assert_eq!(base, [128, 64, 32, 32]);
    }

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
}

fn composite_overlay(
    pixels: &mut [u8],
    base_width: u32,
    ov: &asset::m2::TextureOverlay,
    texture_dir: &Path,
) {
    use asset::m2::OverlayScale;
    let ov_path = asset::asset_cache::texture(ov.fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{}.blp", ov.fdid)));
    match asset::blp::load_blp_rgba(&ov_path) {
        Ok((ov_pixels, ov_w, ov_h)) => match ov.scale {
            OverlayScale::None => {
                asset::blp::blit_region(pixels, base_width, &ov_pixels, ov_w, ov_h, ov.x, ov.y);
            }
            OverlayScale::Uniform2x => {
                let (scaled, sw, sh) = asset::blp::scale_2x(&ov_pixels, ov_w, ov_h);
                asset::blp::blit_region(pixels, base_width, &scaled, sw, sh, ov.x, ov.y);
            }
        },
        Err(e) => eprintln!("Failed to load overlay {}: {e}", ov_path.display()),
    }
}
