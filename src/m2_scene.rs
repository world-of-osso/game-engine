use std::path::Path;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::animation::{
    BonePivot, M2_BONE_SPHERICAL_BILLBOARD, M2AnimData, M2AnimPlayer, SphericalBillboard,
};
use crate::asset;
use crate::asset::m2_anim::{BoneAnimTracks, M2AnimSequence, M2Bone};
use crate::asset::m2_particle::M2ParticleEmitter;
use crate::camera::{CharacterFacing, MovementState, Player};
use crate::creature_display;
use crate::equipment;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;
use crate::particle;

/// Attach equipment (attachment points + default main-hand torch) to a model entity.
pub fn attach_equipment_to_model(
    commands: &mut Commands,
    model_entity: Entity,
    attachments: &[asset::m2_attach::M2Attachment],
    attachment_lookup: &[i16],
    default_main_hand_torch: bool,
) {
    if attachments.is_empty() {
        return;
    }
    let attach_pts = equipment::build_attachment_points(attachments, attachment_lookup);
    let mut equip = equipment::Equipment::default();
    if default_main_hand_torch {
        let torch = Path::new("data/models/club_1h_torch_a_01.m2");
        if torch.exists() {
            equip
                .slots
                .insert(equipment::EquipmentSlot::MainHand, torch.to_path_buf());
        }
    }
    commands.entity(model_entity).insert((attach_pts, equip));
}

#[allow(clippy::too_many_arguments)]
fn spawn_anim_and_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bones: &[M2Bone],
    sequences: Vec<M2AnimSequence>,
    bone_tracks: Vec<BoneAnimTracks>,
    particle_emitters: Vec<M2ParticleEmitter>,
    attachments: Vec<asset::m2_attach::M2Attachment>,
    attachment_lookup: Vec<i16>,
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
    visual_root: Entity,
    default_main_hand_torch: bool,
    model_scale: f32,
) {
    let joint_entities =
        attach_bone_pivots_and_player(commands, bones, &sequences, skinning, model_entity);
    spawn_particle_emitters(
        commands,
        meshes,
        materials,
        images,
        &particle_emitters,
        bones,
        skinning,
        visual_root,
        model_scale,
    );
    if let Some(joints) = joint_entities {
        commands.entity(model_entity).insert(M2AnimData {
            spherical_billboards: crate::animation::propagate_spherical_billboards(bones),
            bones: bones.to_vec(),
            sequences,
            bone_tracks,
            joint_entities: joints,
        });
    }
    attach_equipment_to_model(
        commands,
        model_entity,
        &attachments,
        &attachment_lookup,
        default_main_hand_torch,
    );
}

fn spawn_particle_emitters(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    particle_emitters: &[M2ParticleEmitter],
    bones: &[M2Bone],
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
    model_scale: f32,
) {
    if particle_emitters.is_empty() {
        return;
    }
    let bone_slice = skinning.as_ref().map(|(_, joints)| joints.as_slice());
    particle::spawn_emitters(
        commands,
        meshes,
        materials,
        images,
        particle_emitters,
        bones,
        bone_slice,
        model_entity,
        model_scale,
    );
}

fn load_m2_model(
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<asset::m2::M2Model> {
    let skin_fdids = creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    load_m2_model_with_skin_fdids(m2_path, &skin_fdids)
}

fn load_m2_model_with_skin_fdids(
    m2_path: &Path,
    skin_fdids: &[u32; 3],
) -> Option<asset::m2::M2Model> {
    asset::m2::load_m2_uncached(m2_path, &skin_fdids)
        .map_err(|e| {
            eprintln!("Failed to load M2 {}: {e}", m2_path.display());
        })
        .ok()
}

pub fn spawn_m2_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    let Some(model) = load_m2_model(m2_path, creature_display_map) else {
        return;
    };
    let model_entity = spawn_player_root(commands, m2_path);
    attach_m2_model_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inv_bp,
        model,
        model_entity,
        true,
        1.0,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_full_m2_on_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
    entity: Entity,
    model_scale: f32,
) -> bool {
    let Some(model) = load_m2_model(m2_path, creature_display_map) else {
        return false;
    };
    attach_m2_model_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inv_bp,
        model,
        entity,
        false,
        model_scale,
    );
    true
}

#[allow(clippy::too_many_arguments)]
fn attach_m2_model_parts(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    model: asset::m2::M2Model,
    model_entity: Entity,
    default_main_hand_torch: bool,
    model_scale: f32,
) {
    let asset::m2::M2Model {
        batches,
        bones,
        sequences,
        bone_tracks,
        particle_emitters,
        attachments,
        attachment_lookup,
        lights,
        ..
    } = model;
    let visual_root = m2_spawn::ensure_grounded_model_root(
        commands,
        model_entity,
        m2_spawn::ground_offset_y(&batches),
    );
    let skinning = m2_spawn::attach_m2_batches(
        commands,
        &mut m2_spawn::SpawnAssets {
            meshes,
            materials,
            effect_materials,
            images,
            inverse_bindposes: inv_bp,
        },
        batches,
        &bones,
        visual_root,
    );
    spawn_anim_and_particles(
        commands,
        meshes,
        materials,
        images,
        &bones,
        sequences,
        bone_tracks,
        particle_emitters,
        attachments,
        attachment_lookup,
        &skinning,
        model_entity,
        visual_root,
        default_main_hand_torch,
        model_scale,
    );
    m2_spawn::spawn_model_point_lights(commands, &lights, &skinning, visual_root, model_entity);
}

pub fn spawn_player_root(commands: &mut Commands, m2_path: &Path) -> Entity {
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("m2_model");
    commands
        .spawn((
            Name::new(name.to_owned()),
            Player,
            MovementState::default(),
            CharacterFacing::default(),
            crate::collision::CharacterPhysics::default(),
            Transform::from_xyz(0.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        ))
        .id()
}

/// Spawn a static (non-player) M2 model as a scene prop.
#[allow(clippy::too_many_arguments)]
pub fn spawn_static_m2(
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
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prop");
    let root = commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();
    // Keep the visual skeleton/meshes under an identity child so Bevy skinning
    // computes world_from_local from the actor root only once.
    let model_root = commands
        .spawn((
            Name::new(format!("{name}ModelRoot")),
            Transform::IDENTITY,
            Visibility::default(),
        ))
        .id();
    commands.entity(model_root).insert(ChildOf(root));
    let skin_fdids = creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    if m2_spawn::spawn_m2_on_entity(
        commands,
        &mut m2_spawn::SpawnAssets {
            meshes,
            materials,
            effect_materials,
            images,
            inverse_bindposes: skinned_mesh_inverse_bindposes,
        },
        m2_path,
        model_root,
        &skin_fdids,
    ) {
        Some(root)
    } else {
        commands.entity(root).despawn();
        None
    }
}

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
    model_scale: f32,
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = load_m2_model(m2_path, creature_display_map) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        skinned_mesh_inverse_bindposes,
        m2_path,
        transform,
        model,
        model_scale,
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
    model_scale: f32,
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = load_m2_model_with_skin_fdids(m2_path, skin_fdids) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        skinned_mesh_inverse_bindposes,
        m2_path,
        transform,
        model,
        model_scale,
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_animated_static_m2_parts_from_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    model: asset::m2::M2Model,
    model_scale: f32,
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
        images,
        skinned_mesh_inverse_bindposes,
        model,
        model_root,
        false,
        model_scale,
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
    model_scale: f32,
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
        model_scale,
    )
    .map(|spawned| spawned.root)
}

/// Attach BonePivot components to joint entities and insert M2AnimPlayer on the model.
/// Returns the joint entity list if animation is active, otherwise None.
pub fn attach_bone_pivots_and_player(
    commands: &mut Commands,
    bones: &[M2Bone],
    sequences: &[M2AnimSequence],
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
) -> Option<Vec<Entity>> {
    let (_, joints) = skinning.as_ref()?;
    for (i, bone) in bones.iter().enumerate() {
        let p = bone.pivot;
        let mut entity = commands.entity(joints[i]);
        entity.insert(BonePivot(Vec3::new(p[0], p[2], -p[1])));
        if bone.flags & M2_BONE_SPHERICAL_BILLBOARD != 0 {
            entity.insert(SphericalBillboard {
                pivot: Vec3::new(p[0], p[2], -p[1]),
            });
        }
    }
    if sequences.is_empty() {
        return None;
    }
    let stand_idx = sequences.iter().position(|s| s.id == 0).unwrap_or(0);
    commands.entity(model_entity).insert(M2AnimPlayer {
        current_seq_idx: stand_idx,
        time_ms: 0.0,
        looping: true,
        transition: None,
    });
    Some(joints.clone())
}
