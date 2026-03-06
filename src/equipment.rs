//! Equipment rendering: attach item M2 models to character bone attachment points.
//!
//! WoW attachment lookup IDs (from wowdev.wiki/M2#Attachments):
//!   0  = HandRight (main hand weapon)
//!   1  = HandLeft (off-hand weapon/shield)
//!   26 = SheathedMainHand (back/hip sheathed)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;

use crate::animation::M2AnimData;
use crate::asset::m2_attach::M2Attachment;
use crate::m2_spawn;

/// Equipment slot for attaching items to a character model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
}

/// Maps equipment slots to item M2 file paths.
#[derive(Component, Default)]
pub struct Equipment {
    pub slots: HashMap<EquipmentSlot, PathBuf>,
}

/// Resolved attachment points from the character model.
#[derive(Component)]
pub struct AttachmentPoints {
    /// Attachment lookup ID → (bone_index, position offset in Bevy coords).
    pub points: HashMap<u32, (u16, Vec3)>,
}

/// Marker for spawned equipment entities so we can track/despawn them.
#[derive(Component)]
pub struct EquipmentItem {
    pub slot: EquipmentSlot,
}

/// Attachment lookup ID for each equipment slot.
fn slot_attachment_id(slot: EquipmentSlot) -> u32 {
    match slot {
        EquipmentSlot::MainHand => 0, // HandRight
        EquipmentSlot::OffHand => 1,  // HandLeft
    }
}

/// Build an `AttachmentPoints` component from parsed M2 attachment data.
pub fn build_attachment_points(attachments: &[M2Attachment]) -> AttachmentPoints {
    let mut points = HashMap::new();
    for att in attachments {
        let pos = crate::asset::m2::wow_to_bevy(
            att.position[0],
            att.position[1],
            att.position[2],
        );
        points.insert(att.id, (att.bone, Vec3::from(pos)));
    }
    AttachmentPoints { points }
}

/// System: spawn equipment M2 models on entities that have Equipment + AttachmentPoints.
/// Uses `Added<Equipment>` to run once per newly-equipped entity.
#[allow(clippy::too_many_arguments)]
pub fn spawn_equipment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    query: Query<(&Equipment, &AttachmentPoints), Added<Equipment>>,
    anim_data: Option<Res<M2AnimData>>,
) {
    let Some(data) = anim_data else { return };
    for (equipment, attach_points) in &query {
        for (&slot, path) in &equipment.slots {
            let att_id = slot_attachment_id(slot);
            let Some(&(bone_idx, offset)) = attach_points.points.get(&att_id) else {
                warn!("No attachment point {att_id} for slot {slot:?}");
                continue;
            };
            spawn_item_on_bone(
                &mut commands, &mut meshes, &mut materials, &mut images,
                &mut inv_bp, &data.joint_entities, bone_idx, offset, path, slot,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_item_on_bone(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    joint_entities: &[Entity],
    bone_idx: u16,
    offset: Vec3,
    m2_path: &Path,
    slot: EquipmentSlot,
) {
    let Some(&joint) = joint_entities.get(bone_idx as usize) else {
        warn!("Bone {bone_idx} not found for equipment slot {slot:?}");
        return;
    };
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("item");
    let item_root = commands
        .spawn((
            Name::new(format!("equip_{name}")),
            EquipmentItem { slot },
            Transform::from_translation(offset),
            Visibility::default(),
        ))
        .set_parent_in_place(joint)
        .id();
    if !m2_spawn::spawn_m2_on_entity(
        commands, meshes, materials, images, inv_bp, m2_path, item_root, &[0, 0, 0],
    ) {
        commands.entity(item_root).despawn();
    }
}

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_equipment);
    }
}
