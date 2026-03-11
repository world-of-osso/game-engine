use std::path::Path;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::animation::{BonePivot, M2AnimData, M2AnimPlayer};
use crate::asset;
use crate::asset::m2_anim::{BoneAnimTracks, M2AnimSequence, M2Bone};
use crate::asset::m2_particle::M2ParticleEmitter;
use crate::camera::{CharacterFacing, MovementState, Player};
use crate::creature_display;
use crate::equipment;
use crate::m2_spawn;
use crate::particle;

/// Attach equipment (attachment points + default main-hand torch) to a model entity.
pub fn attach_equipment_to_model(
    commands: &mut Commands,
    model_entity: Entity,
    attachments: &[asset::m2_attach::M2Attachment],
) {
    if attachments.is_empty() {
        return;
    }
    let attach_pts = equipment::build_attachment_points(attachments);
    let mut equip = equipment::Equipment::default();
    let torch = Path::new("data/models/club_1h_torch_a_01.m2");
    if torch.exists() {
        equip
            .slots
            .insert(equipment::EquipmentSlot::MainHand, torch.to_path_buf());
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
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
) {
    let joint_entities =
        attach_bone_pivots_and_player(commands, bones, &sequences, skinning, model_entity);
    spawn_particle_emitters(commands, meshes, materials, images, &particle_emitters, skinning, model_entity);
    if let Some(joints) = joint_entities {
        commands.insert_resource(M2AnimData {
            sequences,
            bone_tracks,
            joint_entities: joints,
        });
    }
    attach_equipment_to_model(commands, model_entity, &attachments);
}

fn spawn_particle_emitters(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    particle_emitters: &[M2ParticleEmitter],
    skinning: &m2_spawn::SkinningResult,
    model_entity: Entity,
) {
    if particle_emitters.is_empty() {
        return;
    }
    let bone_slice = skinning.as_ref().map(|(_, joints)| joints.as_slice());
    particle::spawn_emitters(commands, meshes, materials, images, particle_emitters, bone_slice, model_entity);
}

fn load_m2_model(
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<asset::m2::M2Model> {
    let skin_fdids = creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    asset::m2::load_m2(m2_path, &skin_fdids).map_err(|e| {
        eprintln!("Failed to load M2 {}: {e}", m2_path.display());
    }).ok()
}

pub fn spawn_m2_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    let Some(model) = load_m2_model(m2_path, creature_display_map) else { return };
    let asset::m2::M2Model { batches, bones, sequences, bone_tracks, particle_emitters, attachments, .. } = model;
    let model_entity = spawn_player_root(commands, m2_path);
    let skinning = m2_spawn::attach_m2_batches(
        commands,
        &mut m2_spawn::SpawnAssets { meshes, materials, images, inverse_bindposes: inv_bp },
        batches,
        &bones,
        model_entity,
    );
    spawn_anim_and_particles(
        commands, meshes, materials, images,
        &bones, sequences, bone_tracks, particle_emitters, attachments, &skinning, model_entity,
    );
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
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<Entity> {
    let name = m2_path.file_stem().and_then(|s| s.to_str()).unwrap_or("prop");
    let root = commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();
    let skin_fdids = creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    if m2_spawn::spawn_m2_on_entity(
        commands,
        &mut m2_spawn::SpawnAssets { meshes, materials, images, inverse_bindposes: skinned_mesh_inverse_bindposes },
        m2_path,
        root,
        &skin_fdids,
    ) {
        Some(root)
    } else {
        commands.entity(root).despawn();
        None
    }
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
        commands
            .entity(joints[i])
            .insert(BonePivot(Vec3::new(p[0], p[2], -p[1])));
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
