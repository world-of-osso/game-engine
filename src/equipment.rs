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
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;

/// Equipment slot for attaching items to a character model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    Back,
    MainHand,
    OffHand,
}

/// Maps equipment slots to item M2 file paths.
#[derive(Component, Default)]
pub struct Equipment {
    pub slots: HashMap<EquipmentSlot, PathBuf>,
    pub slot_skin_fdids: HashMap<EquipmentSlot, [u32; 3]>,
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
    skin_fdids: [u32; 3],
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
        slot_defaults.insert(EquipmentSlot::Head, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::Back, Transform::IDENTITY);
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
        EquipmentSlot::Head => 11,    // Helm
        EquipmentSlot::Back => 12,    // Back
        EquipmentSlot::MainHand => 0, // HandRight
        EquipmentSlot::OffHand => 1,  // HandLeft
    }
}

/// Build an `AttachmentPoints` component from parsed M2 attachment data.
pub fn build_attachment_points(
    attachments: &[M2Attachment],
    attachment_lookup: &[i16],
) -> AttachmentPoints {
    let mut points = HashMap::new();
    if attachment_lookup.is_empty() {
        for att in attachments {
            let pos =
                crate::asset::m2::wow_to_bevy(att.position[0], att.position[1], att.position[2]);
            points.insert(att.id, (att.bone, Vec3::from(pos)));
        }
        return AttachmentPoints { points };
    }
    for (slot_id, &attachment_index) in attachment_lookup.iter().enumerate() {
        let Ok(attachment_index) = usize::try_from(attachment_index) else {
            continue;
        };
        let Some(att) = attachments.get(attachment_index) else {
            continue;
        };
        let pos = crate::asset::m2::wow_to_bevy(att.position[0], att.position[1], att.position[2]);
        points.insert(slot_id as u32, (att.bone, Vec3::from(pos)));
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
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    transforms: Res<EquipmentTransforms>,
    mut query: Query<(
        Entity,
        &Equipment,
        &AttachmentPoints,
        &M2AnimData,
        &mut RenderedEquipment,
    )>,
    existing_items: Query<(), With<EquipmentItem>>,
    mut warned: Local<HashSet<String>>,
) {
    for (owner, equipment, attach_points, anim_data, mut rendered) in &mut query {
        sync_removed_slots(
            &mut commands,
            &equipment.slots,
            &mut rendered,
            &existing_items,
        );

        for (&slot, path) in &equipment.slots {
            if path.as_os_str().is_empty() {
                continue;
            }

            let skin_fdids = equipment.slot_skin_fdids.get(&slot).copied().unwrap_or([0, 0, 0]);
            let should_respawn = match rendered.slots.get(&slot) {
                Some(item) => {
                    item.path != *path
                        || item.skin_fdids != skin_fdids
                        || existing_items.get(item.entity).is_err()
                }
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
                &mut effect_materials,
                &mut images,
                &mut inv_bp,
                &anim_data.joint_entities,
                attach_points,
                slot,
                path,
                skin_fdids,
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
                    skin_fdids,
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
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    joint_entities: &[Entity],
    attach_points: &AttachmentPoints,
    slot: EquipmentSlot,
    m2_path: &Path,
    skin_fdids: [u32; 3],
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
            ChildOf(joint),
        ))
        .id();

    if !m2_spawn::spawn_m2_on_entity(
        commands,
        &mut m2_spawn::SpawnAssets {
            meshes,
            materials,
            effect_materials,
            images,
            inverse_bindposes: inv_bp,
        },
        m2_path,
        item_root,
        &skin_fdids,
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
    use crate::asset::m2_attach::M2Attachment;
    use bevy::mesh::Mesh3d;
    use bevy::transform::TransformPlugin;

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

    #[test]
    fn attachment_points_use_lookup_slots_when_present() {
        let attachments = vec![M2Attachment {
            id: 123,
            bone: 7,
            position: [1.0, 2.0, 3.0],
        }];
        let mut lookup = vec![-1; 12];
        lookup[11] = 0;

        let points = build_attachment_points(&attachments, &lookup);

        assert_eq!(points.points.get(&11).map(|(bone, _)| *bone), Some(7));
        assert!(!points.points.contains_key(&123));
    }

    #[test]
    fn spawned_helm_mesh_wraps_character_head_height() {
        let Some((character_path, helm_path)) = test_model_paths() else {
            return;
        };
        let model = crate::asset::m2::load_m2(character_path, &[0, 0, 0])
            .expect("failed to load humanmale_hd");
        let (attachment_joint_translation, head_offset, head_bone_height) =
            head_slot_reference_data(&model);

        let mut app = App::new();
        configure_equipment_test_app(&mut app);
        let owner = spawn_head_equipment_owner(
            &mut app,
            helm_path,
            attachment_joint_translation,
            head_offset,
            [140455, 0, 0],
        );

        app.update();
        app.update();

        let (mesh_min_y, mesh_max_y) =
            helmet_mesh_world_y_bounds(app.world(), owner).expect("helmet mesh bounds");

        assert!(
            mesh_min_y <= head_bone_height && mesh_max_y >= head_bone_height,
            "expected helmet mesh to wrap head height; head_y={head_bone_height:.3} min_y={mesh_min_y:.3} max_y={mesh_max_y:.3} joint_y={:.3} offset_y={:.3}",
            attachment_joint_translation.y, head_offset.y,
        );
    }

    #[test]
    fn spawned_helm_mesh_uses_textured_material_when_skin_fdid_present() {
        let Some((_, helm_path)) = test_model_paths() else {
            return;
        };

        let mut app = App::new();
        configure_equipment_test_app(&mut app);
        let owner = spawn_head_equipment_owner(
            &mut app,
            helm_path,
            Vec3::ZERO,
            Vec3::ZERO,
            [140455, 0, 0],
        );

        app.update();
        app.update();

        let descendants = descendant_entities(app.world(), owner);
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let textured_meshes = descendants
            .into_iter()
            .filter_map(|entity| {
                let material = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity)?;
                let material = materials.get(&material.0)?;
                material.base_color_texture.as_ref()
            })
            .count();

        assert!(
            textured_meshes > 0,
            "expected spawned helm to bind at least one textured material"
        );
    }

    fn test_model_paths() -> Option<(&'static Path, &'static Path)> {
        let character_path = Path::new("data/models/humanmale_hd.m2");
        let helm_path =
            Path::new("data/item-models/item/objectcomponents/head/helm_plate_d_02_hum.m2");
        (character_path.exists() && helm_path.exists()).then_some((character_path, helm_path))
    }

    fn head_slot_reference_data(model: &crate::asset::m2::M2Model) -> (Vec3, Vec3, f32) {
        let attachment_points = build_attachment_points(&model.attachments, &model.attachment_lookup);
        let &(attachment_bone, head_offset) = attachment_points
            .points
            .get(&slot_attachment_id(EquipmentSlot::Head))
            .expect("missing head attachment point");
        let attachment_bone = model
            .bones
            .get(attachment_bone as usize)
            .expect("head attachment bone index out of range");
        let head_bone = model
            .bones
            .iter()
            .find(|bone| bone.key_bone_id == 6)
            .expect("missing head key bone");
        (
            wow_vec3(attachment_bone.pivot),
            head_offset,
            wow_vec3(head_bone.pivot).y,
        )
    }

    fn wow_vec3(pivot: [f32; 3]) -> Vec3 {
        let [x, y, z] = crate::asset::m2::wow_to_bevy(pivot[0], pivot[1], pivot[2]);
        Vec3::new(x, y, z)
    }

    fn configure_equipment_test_app(app: &mut App) {
        app.add_plugins((MinimalPlugins, TransformPlugin));
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<StandardMaterial>::default());
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<M2EffectMaterial>::default());
        app.insert_resource(Assets::<SkinnedMeshInverseBindposes>::default());
        app.insert_resource(EquipmentTransforms::default());
        app.add_systems(Update, (attach_rendered_equipment_state, sync_equipment).chain());
    }

    fn spawn_head_equipment_owner(
        app: &mut App,
        helm_path: &Path,
        attachment_joint_translation: Vec3,
        head_offset: Vec3,
        skin_fdids: [u32; 3],
    ) -> Entity {
        let joint = app
            .world_mut()
            .spawn((
                Transform::from_translation(attachment_joint_translation),
                GlobalTransform::default(),
            ))
            .id();
        let owner = app
            .world_mut()
            .spawn((
                Equipment {
                    slots: HashMap::from([(EquipmentSlot::Head, helm_path.to_path_buf())]),
                    slot_skin_fdids: HashMap::from([(EquipmentSlot::Head, skin_fdids)]),
                },
                AttachmentPoints {
                    points: HashMap::from([(slot_attachment_id(EquipmentSlot::Head), (0, head_offset))]),
                },
                M2AnimData {
                    sequences: vec![],
                    bone_tracks: vec![],
                    joint_entities: vec![joint],
                },
                Transform::IDENTITY,
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut().entity_mut(joint).set_parent_in_place(owner);
        owner
    }

    fn helmet_mesh_world_y_bounds(world: &World, root: Entity) -> Option<(f32, f32)> {
        let meshes = world.resource::<Assets<Mesh>>();
        let entities = descendant_entities(world, root);
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for entity in entities {
            accumulate_mesh_world_y_bounds(world, &meshes, entity, &mut min_y, &mut max_y);
        }

        (min_y.is_finite() && max_y.is_finite()).then_some((min_y, max_y))
    }

    fn descendant_entities(world: &World, root: Entity) -> Vec<Entity> {
        let mut entities = vec![root];
        collect_descendants(world, root, &mut entities);
        entities
    }

    fn collect_descendants(world: &World, entity: Entity, out: &mut Vec<Entity>) {
        let Some(children) = world.get::<Children>(entity) else {
            return;
        };
        for child in children.iter() {
            out.push(child);
            collect_descendants(world, child, out);
        }
    }

    fn accumulate_mesh_world_y_bounds(
        world: &World,
        meshes: &Assets<Mesh>,
        entity: Entity,
        min_y: &mut f32,
        max_y: &mut f32,
    ) {
        let Some(mesh3d) = world.get::<Mesh3d>(entity) else {
            return;
        };
        let Some(global) = world.get::<GlobalTransform>(entity) else {
            return;
        };
        let Some(mesh) = meshes.get(&mesh3d.0) else {
            return;
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return;
        };
        for position in positions {
            let world_pos =
                global.transform_point(Vec3::new(position[0], position[1], position[2]));
            *min_y = min_y.min(world_pos.y);
            *max_y = max_y.max(world_pos.y);
        }
    }
}

#[cfg(test)]
#[path = "equipment_live_tests.rs"]
mod live_tests;
