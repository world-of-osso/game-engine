//! Equipment rendering: attach item M2 models to character bone attachment points.
//!
//! WoW attachment lookup IDs (from wowdev.wiki/M2#Attachments):
//!   0  = HandRight (main hand weapon)
//!   1  = HandLeft (off-hand weapon/shield)
//!   26 = SheathedMainHand (back/hip sheathed)

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::animation::M2AnimData;
use crate::asset::m2_attach::M2Attachment;
use crate::m2_spawn;

/// Equipment slot for attaching items to a character model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub _slot: EquipmentSlot,
}

#[derive(Debug, Clone)]
struct RenderedItem {
    entity: Entity,
    path: PathBuf,
}

/// Tracks currently-rendered equipment for each character entity.
#[derive(Component, Default)]
pub struct RenderedEquipment {
    slots: HashMap<EquipmentSlot, RenderedItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct EquipmentTransformDef {
    #[serde(default)]
    translation: [f32; 3],
    #[serde(default)]
    rotation_deg: [f32; 3],
    #[serde(default = "default_scale")]
    scale: [f32; 3],
}

impl Default for EquipmentTransformDef {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: default_scale(),
        }
    }
}

impl EquipmentTransformDef {
    fn as_transform(&self) -> Transform {
        let [rx, ry, rz] = self.rotation_deg;
        Transform {
            translation: Vec3::new(
                self.translation[0],
                self.translation[1],
                self.translation[2],
            ),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                rx.to_radians(),
                ry.to_radians(),
                rz.to_radians(),
            ),
            scale: Vec3::new(self.scale[0], self.scale[1], self.scale[2]),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct EquipmentTransformConfig {
    #[serde(default)]
    slot_defaults: HashMap<EquipmentSlot, EquipmentTransformDef>,
    #[serde(default)]
    item_overrides: HashMap<String, EquipmentTransformDef>,
}

#[derive(Resource, Debug, Clone)]
pub struct EquipmentTransforms {
    slot_defaults: HashMap<EquipmentSlot, Transform>,
    item_overrides: HashMap<String, Transform>,
}

impl Default for EquipmentTransforms {
    fn default() -> Self {
        let mut slot_defaults = HashMap::new();
        slot_defaults.insert(EquipmentSlot::MainHand, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::OffHand, Transform::IDENTITY);
        Self {
            slot_defaults,
            item_overrides: HashMap::new(),
        }
    }
}

impl EquipmentTransforms {
    fn load_from_disk() -> Self {
        let path = Path::new("data/equipment_transforms.ron");
        let Ok(content) = std::fs::read_to_string(path) else {
            info!(
                "Equipment transform config not found at {}, using defaults",
                path.display()
            );
            return Self::default();
        };
        match ron::de::from_str::<EquipmentTransformConfig>(&content) {
            Ok(config) => Self::from_config(config),
            Err(e) => {
                warn!(
                    "Failed to parse {}: {e}. Using default equipment transforms",
                    path.display()
                );
                Self::default()
            }
        }
    }

    fn from_config(config: EquipmentTransformConfig) -> Self {
        let mut result = Self::default();
        for (slot, def) in config.slot_defaults {
            result.slot_defaults.insert(slot, def.as_transform());
        }
        result.item_overrides = config
            .item_overrides
            .into_iter()
            .map(|(key, def)| (key.to_ascii_lowercase(), def.as_transform()))
            .collect();
        result
    }

    fn resolve(&self, slot: EquipmentSlot, path: &Path) -> Transform {
        let key = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if let Some(transform) = self.item_overrides.get(&key) {
            return *transform;
        }
        self.slot_defaults
            .get(&slot)
            .copied()
            .unwrap_or(Transform::IDENTITY)
    }
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
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
        let pos = crate::asset::m2::wow_to_bevy(att.position[0], att.position[1], att.position[2]);
        points.insert(att.id, (att.bone, Vec3::from(pos)));
    }
    AttachmentPoints { points }
}

/// Ensure entities with equipment have tracking state.
fn attach_rendered_equipment_state(
    mut commands: Commands,
    query: Query<Entity, (With<Equipment>, Without<RenderedEquipment>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(RenderedEquipment::default());
    }
}

/// System: synchronize rendered equipment with desired slots.
#[allow(clippy::too_many_arguments)]
pub fn sync_equipment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    transforms: Res<EquipmentTransforms>,
    anim_data: Option<Res<M2AnimData>>,
    mut query: Query<(
        Entity,
        &Equipment,
        &AttachmentPoints,
        &mut RenderedEquipment,
    )>,
    existing_items: Query<(), With<EquipmentItem>>,
    mut warned: Local<HashSet<String>>,
) {
    for (owner, equipment, attach_points, mut rendered) in &mut query {
        sync_removed_slots(
            &mut commands,
            &equipment.slots,
            &mut rendered,
            &existing_items,
        );

        let Some(anim_data) = anim_data.as_ref() else {
            continue;
        };

        for (&slot, path) in &equipment.slots {
            if path.as_os_str().is_empty() {
                continue;
            }

            let should_respawn = match rendered.slots.get(&slot) {
                Some(item) => item.path != *path || existing_items.get(item.entity).is_err(),
                None => true,
            };
            if !should_respawn {
                continue;
            }

            if let Some(existing) = rendered.slots.remove(&slot) {
                commands.entity(existing.entity).despawn();
            }

            let Some(spawned) = spawn_equipment_slot(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut images,
                &mut inv_bp,
                &anim_data.joint_entities,
                attach_points,
                slot,
                path,
                &transforms,
                &mut warned,
                owner,
            ) else {
                continue;
            };

            rendered.slots.insert(
                slot,
                RenderedItem {
                    entity: spawned,
                    path: path.clone(),
                },
            );
        }
    }
}

fn sync_removed_slots(
    commands: &mut Commands,
    desired: &HashMap<EquipmentSlot, PathBuf>,
    rendered: &mut RenderedEquipment,
    existing_items: &Query<(), With<EquipmentItem>>,
) {
    let mut to_remove = Vec::new();
    for (&slot, item) in &rendered.slots {
        let removed = !desired.contains_key(&slot);
        let missing_entity = existing_items.get(item.entity).is_err();
        if removed || missing_entity {
            to_remove.push(slot);
        }
    }
    for slot in to_remove {
        if let Some(item) = rendered.slots.remove(&slot) {
            commands.entity(item.entity).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_equipment_slot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    joint_entities: &[Entity],
    attach_points: &AttachmentPoints,
    slot: EquipmentSlot,
    m2_path: &Path,
    transforms: &EquipmentTransforms,
    warned: &mut HashSet<String>,
    owner: Entity,
) -> Option<Entity> {
    let att_id = slot_attachment_id(slot);
    let Some(&(bone_idx, base_offset)) = attach_points.points.get(&att_id) else {
        warn_once(
            warned,
            format!("missing attachment {att_id} for slot {slot:?} on {owner:?}"),
        );
        return None;
    };

    let Some(&joint) = joint_entities.get(bone_idx as usize) else {
        warn_once(
            warned,
            format!("missing bone {bone_idx} for slot {slot:?} on {owner:?}"),
        );
        return None;
    };

    if !m2_path.exists() {
        warn_once(
            warned,
            format!(
                "equipment model missing for slot {slot:?}: {}",
                m2_path.display()
            ),
        );
        return None;
    }

    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("item");
    let mut transform = transforms.resolve(slot, m2_path);
    transform.translation += base_offset;

    let item_root = commands
        .spawn((
            Name::new(format!("equip_{name}")),
            EquipmentItem { _slot: slot },
            transform,
            Visibility::default(),
        ))
        .set_parent_in_place(joint)
        .id();

    if !m2_spawn::spawn_m2_on_entity(
        commands,
        &mut m2_spawn::SpawnAssets { meshes, materials, images, inverse_bindposes: inv_bp },
        m2_path,
        item_root,
        &[0, 0, 0],
    ) {
        commands.entity(item_root).despawn();
        warn_once(
            warned,
            format!(
                "failed loading equipment model for slot {slot:?}: {}",
                m2_path.display()
            ),
        );
        return None;
    }

    Some(item_root)
}

fn warn_once(warned: &mut HashSet<String>, message: String) {
    if warned.insert(message.clone()) {
        warn!("{message}");
    }
}

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EquipmentTransforms::load_from_disk())
            .add_systems(
                Update,
                (attach_rendered_equipment_state, sync_equipment).chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_def_defaults_to_identity() {
        let def = EquipmentTransformDef::default();
        let t = def.as_transform();
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn config_from_ron_parses_overrides() {
        let ron = r#"(
            slot_defaults: {
                MainHand: (translation: (1.0, 2.0, 3.0), rotation_deg: (0.0, 90.0, 0.0), scale: (1.0, 1.0, 1.0)),
            },
            item_overrides: {
                "club_1h_torch_a_01": (translation: (0.5, 0.0, 0.0), rotation_deg: (0.0, 0.0, 180.0), scale: (1.0, 1.0, 1.0)),
            },
        )"#;
        let parsed: EquipmentTransformConfig = ron::de::from_str(ron).unwrap();
        let transforms = EquipmentTransforms::from_config(parsed);
        let t = transforms.resolve(
            EquipmentSlot::MainHand,
            Path::new("data/models/club_1h_torch_a_01.m2"),
        );
        assert!((t.translation.x - 0.5).abs() < f32::EPSILON);
    }
}
