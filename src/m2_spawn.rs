use std::path::{Path, PathBuf};

use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;

use crate::asset;

/// Component tagging a mesh entity with its M2 geoset mesh_part_id.
#[derive(Component)]
pub struct GeosetMesh(pub u16);

/// Grouped asset params for M2 spawning.
pub struct SpawnAssets<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
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
    let model = match asset::m2::load_m2(m2_path, skin_fdids) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to load M2 {}: {e}", m2_path.display());
            return false;
        }
    };
    attach_m2_batches(commands, assets, model.batches, &model.bones, entity);
    true
}

/// Skinning data returned from mesh attachment, for animation setup.
pub type SkinningResult = Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>;

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
        let visible = asset::m2::default_geoset_visible(batch.mesh_part_id);
        let mat = load_batch_material(&batch, i, assets.images, assets.materials);
        spawn_skinned_mesh(
            commands,
            assets.meshes,
            mat,
            batch.mesh,
            root,
            &skinning,
            i,
            batch.mesh_part_id,
            visible,
        );
    }
    skinning
}

#[allow(clippy::too_many_arguments)]
fn spawn_skinned_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    mesh: Mesh,
    parent: Entity,
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    batch_index: usize,
    mesh_part_id: u16,
    visible: bool,
) {
    let vis = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let mut cmd = commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Name::new(format!("Mesh[{batch_index}]")),
        GeosetMesh(mesh_part_id),
        vis,
    ));
    cmd.set_parent_in_place(parent);
    if let Some((inv_bp, joints)) = skinning {
        cmd.insert(SkinnedMesh {
            inverse_bindposes: inv_bp.clone(),
            joints: joints.clone(),
        });
    }
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
) -> Handle<StandardMaterial> {
    let texture_dir = PathBuf::from("data/textures");
    if let Some(fdid) = batch.texture_fdid {
        let blp_path = asset::casc_resolver::ensure_texture(fdid)
            .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
        if let Some(mat) =
            try_load_textured_material(&blp_path, batch, &texture_dir, images, materials)
        {
            return mat;
        }
    }
    let color = PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()];
    materials.add(m2_material(None, Some(color), batch))
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
    let image = load_composited_texture(blp_path, &batch.overlays, texture_dir)?;
    Some(materials.add(m2_material(Some(images.add(image)), None, batch)))
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
        1 => AlphaMode::Mask(0.5),
        2 | 3 | 7 => AlphaMode::Blend,
        4..=6 => AlphaMode::Add,
        _ => AlphaMode::Opaque,
    };
    StandardMaterial {
        base_color_texture: texture,
        base_color: color.unwrap_or(Color::WHITE),
        unlit,
        cull_mode,
        double_sided: two_sided,
        alpha_mode,
        ..default()
    }
}

fn load_composited_texture(
    base_path: &Path,
    overlays: &[asset::m2::TextureOverlay],
    texture_dir: &Path,
) -> Option<Image> {
    let (mut pixels, w, h) = asset::blp::load_blp_rgba(base_path)
        .map_err(|e| eprintln!("Failed to load BLP {}: {e}", base_path.display()))
        .ok()?;
    for ov in overlays {
        composite_overlay(&mut pixels, w, ov, texture_dir);
    }
    Some(crate::rgba_image(pixels, w, h))
}

fn composite_overlay(
    pixels: &mut [u8],
    base_width: u32,
    ov: &asset::m2::TextureOverlay,
    texture_dir: &Path,
) {
    use asset::m2::OverlayScale;
    let ov_path = asset::casc_resolver::ensure_texture(ov.fdid)
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
