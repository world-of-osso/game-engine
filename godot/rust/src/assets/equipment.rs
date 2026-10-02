//! Native equipment resources and authored character attachments.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use game_engine_core::m2;
use godot::{
    classes::{MeshInstance3D, Node3D, Skeleton3D, Skin},
    prelude::*,
};
use osso_asset_resolver::CascListfileResolver;

use game_engine_core::customization_data::ChoiceSkinnedModel;

use super::{
    appearance::PreparedAppearance,
    build_model_filtered,
    creature::{cache_model_textures, load_model_files},
};
use crate::equipment_appearance_data::{
    EquipmentSlot, RuntimeModelAppearance, collection_mesh_part_in_slot, is_collection_model,
    model_attachment_id, runtime_mesh_part_allowed, slot_uses_bound_joints,
};

#[path = "../../../../src/asset/m2_format/m2_bone_names.rs"]
mod bone_names;
#[path = "../../../../src/game/equipment/equipment_transform_data.rs"]
mod transforms;

struct EquipmentContext<'a> {
    character: &'a mut Gd<Node3D>,
    character_model: &'a m2::Model,
    resolver: &'a CascListfileResolver,
    data_root: &'a Path,
    transforms: Arc<transforms::EquipmentTransformConfig>,
}

pub(super) fn attach_equipment(
    character: &mut Gd<Node3D>,
    character_model: &m2::Model,
    resolver: &CascListfileResolver,
    data_root: &Path,
    models: &[RuntimeModelAppearance],
) -> Result<(), String> {
    if models.is_empty() {
        return Ok(());
    }
    let mut context = EquipmentContext::new(character, character_model, resolver, data_root)?;
    for model in models {
        context.attach(model)?;
    }
    Ok(())
}

/// Attach each model at its slot's default attachment; a model that fails is reported
/// with its error and the others are still attached.
pub(super) fn attach_each_equipment(
    character: &mut Gd<Node3D>,
    character_model: &m2::Model,
    resolver: &CascListfileResolver,
    data_root: &Path,
    models: &[RuntimeModelAppearance],
    mut report: impl FnMut(&RuntimeModelAppearance, String),
) -> Result<(), String> {
    if models.is_empty() {
        return Ok(());
    }
    let mut context = EquipmentContext::new(character, character_model, resolver, data_root)?;
    for model in models {
        if let Err(error) = context.attach(model) {
            report(model, error);
        }
    }
    Ok(())
}

/// Attach each skinned-model collection M2 (ChrCustomizationSkinnedModel) with only
/// its selected submeshes, skinned to the character skeleton like bound equipment.
/// wow.export `update_skinned_models` loads one renderer per collection file, remaps
/// its bones and shows the selected `GeosetType * 100 + GeosetID` submeshes.
pub(super) fn attach_skinned_models(
    character: &mut Gd<Node3D>,
    character_model: &m2::Model,
    resolver: &CascListfileResolver,
    data_root: &Path,
    appearance: &PreparedAppearance,
    models: &[ChoiceSkinnedModel],
) -> Result<(), String> {
    let mut parts_by_file: Vec<(u32, Vec<u16>)> = Vec::new();
    for model in models {
        match parts_by_file
            .iter_mut()
            .find(|(fdid, _)| *fdid == model.collection_fdid)
        {
            Some((_, parts)) => parts.push(model.mesh_part_id()),
            None => parts_by_file.push((model.collection_fdid, vec![model.mesh_part_id()])),
        }
    }
    for (fdid, parts) in parts_by_file {
        let cached = load_model_files(resolver, data_root, fdid)?;
        let path = GString::from(cached.path.to_string_lossy().as_ref());
        let parsed = &cached.model;
        cache_model_textures(resolver, data_root, &[0; 3], parsed)?;
        let skin = bound_skin(character, character_model, parsed)?;
        let (mut collection, missing) =
            build_model_filtered(parsed, &path, &[0; 3], Some(appearance), |part| {
                parts.contains(&part)
            })?;
        if !missing.is_empty() {
            collection.free();
            return Err(format!(
                "Skinned model FDID {fdid} missing textures: {missing:?}"
            ));
        }
        collection.set_name(&format!("SkinnedModel{fdid}"));
        bind_character_skin(&mut collection, &skin);
        character.add_child(&collection);
    }
    Ok(())
}

/// Move the item attached for `slot` onto attachment `attachment` (keeping its item
/// transform), or hide it for `None`. A slot without an attached item is left alone.
pub(crate) fn place_equipment(
    character: &Gd<Node3D>,
    slot: EquipmentSlot,
    attachment: Option<u32>,
) -> Result<(), String> {
    let Some(mut item) = character
        .find_child_ex(&format!("Equipment{slot:?}"))
        .owned(false)
        .done()
        .and_then(|node| node.try_cast::<Node3D>().ok())
    else {
        return Ok(());
    };
    let Some(id) = attachment else {
        item.set_visible(false);
        return Ok(());
    };
    let parent = character
        .get_node_or_null(&format!("Skeleton3D/AttachmentBone{id}/Attachment{id}"))
        .ok_or_else(|| format!("Equipment {slot:?} requires missing attachment {id}"))?;
    if item.get_parent().as_ref() != Some(&parent) {
        item.reparent_ex(&parent)
            .keep_global_transform(false)
            .done();
    }
    item.set_visible(true);
    Ok(())
}

impl<'a> EquipmentContext<'a> {
    fn new(
        character: &'a mut Gd<Node3D>,
        character_model: &'a m2::Model,
        resolver: &'a CascListfileResolver,
        data_root: &'a Path,
    ) -> Result<Self, String> {
        Ok(Self {
            character,
            character_model,
            resolver,
            data_root,
            transforms: load_transforms(data_root)?,
        })
    }
}

/// `equipment_transforms.ron`, read once per process; an error is not kept.
fn load_transforms(data_root: &Path) -> Result<Arc<transforms::EquipmentTransformConfig>, String> {
    static TRANSFORMS: Mutex<Option<Arc<transforms::EquipmentTransformConfig>>> = Mutex::new(None);
    let mut loaded = TRANSFORMS.lock().expect("equipment transforms");
    if let Some(transforms) = loaded.as_ref() {
        return Ok(Arc::clone(transforms));
    }
    let path = data_root.join("equipment_transforms.ron");
    let content =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let transforms = Arc::new(
        transforms::EquipmentTransformConfig::parse(&content)
            .map_err(|error| format!("{}: {error}", path.display()))?,
    );
    *loaded = Some(Arc::clone(&transforms));
    Ok(transforms)
}

impl EquipmentContext<'_> {
    fn attach(&mut self, definition: &RuntimeModelAppearance) -> Result<(), String> {
        let authored = self.resolver.resolve_path(definition.fdid).ok_or_else(|| {
            format!(
                "Equipment {:?} FDID {} has no authored path",
                definition.slot, definition.fdid
            )
        })?;
        let authored_path = Path::new(&authored);
        let cached = load_model_files(self.resolver, self.data_root, definition.fdid)?;
        let parsed = &cached.model;
        let bound = slot_uses_bound_joints(
            definition.slot,
            authored_path,
            &parsed.bones,
            parsed.submeshes.iter().map(|mesh| mesh.mesh_part_id),
        );
        let mut parent = self.parent_for(definition.slot, authored_path, bound)?;
        let path = GString::from(cached.path.to_string_lossy().as_ref());
        cache_model_textures(
            self.resolver,
            self.data_root,
            &definition.skin_fdids,
            parsed,
        )?;
        let skin = if bound {
            Some(self.bound_skin(parsed)?)
        } else {
            None
        };
        let (mut item, missing) =
            build_model_filtered(parsed, &path, &definition.skin_fdids, None, |part| {
                equipment_mesh_part_allowed(definition.slot, authored_path, bound, part)
            })?;
        if !missing.is_empty() {
            item.free();
            return Err(format!(
                "Equipment {:?} FDID {} missing textures: {missing:?}",
                definition.slot, definition.fdid
            ));
        }
        item.set_name(&self.unused_item_name(definition.slot));
        item.set_transform(native_transform(
            &self.transforms.resolve(definition.slot, authored_path),
        ));
        if let Some(skin) = skin {
            bind_character_skin(&mut item, &skin);
        }
        parent.add_child(&item);
        Ok(())
    }

    fn parent_for(
        &self,
        slot: EquipmentSlot,
        authored: &Path,
        bound: bool,
    ) -> Result<Gd<Node3D>, String> {
        if bound {
            return Ok(self.character.clone());
        }
        let id = model_attachment_id(slot, authored);
        self.character
            .get_node_or_null(&format!("Skeleton3D/AttachmentBone{id}/Attachment{id}"))
            .and_then(|node| node.try_cast::<Node3D>().ok())
            .ok_or_else(|| format!("Equipment {slot:?} requires missing character attachment {id}"))
    }

    /// `Equipment{slot}`, numbered from 2 for a slot's further models (a belt's buckle
    /// and its collection).
    fn unused_item_name(&self, slot: EquipmentSlot) -> String {
        let base = format!("Equipment{slot:?}");
        let taken = |name: &str| {
            self.character
                .find_child_ex(name)
                .owned(false)
                .done()
                .is_some()
        };
        if !taken(&base) {
            return base;
        }
        (2..)
            .map(|index| format!("{base}{index}"))
            .find(|name| !taken(name))
            .expect("some numbered name is free")
    }

    fn bound_skin(&self, model: &m2::Model) -> Result<Gd<Skin>, String> {
        bound_skin(&*self.character, self.character_model, model)
    }
}

fn equipment_mesh_part_allowed(
    slot: EquipmentSlot,
    authored: &Path,
    bound: bool,
    mesh_part: u16,
) -> bool {
    if bound && is_collection_model(authored) {
        collection_mesh_part_in_slot(slot, mesh_part)
    } else {
        runtime_mesh_part_allowed(slot, mesh_part)
    }
}

/// A skin binding `model`'s bones to the matching character skeleton joints.
fn bound_skin(
    character: &Gd<Node3D>,
    character_model: &m2::Model,
    model: &m2::Model,
) -> Result<Gd<Skin>, String> {
    let mapping = map_equipment_bones(&character_model.bones, &model.bones)?;
    let skeleton = character
        .get_node_or_null("Skeleton3D")
        .and_then(|node| node.try_cast::<Skeleton3D>().ok())
        .ok_or("Character has no skeleton for bound equipment")?;
    let mut skin = Skin::new_gd();
    for index in mapping {
        skin.add_bind(
            index as i32,
            skeleton.get_bone_global_rest(index as i32).affine_inverse(),
        );
    }
    Ok(skin)
}

/// Character joint of each collection bone: the joint with the same nonzero
/// `boneNameCRC` (wow.export `M2RendererGL.buildBoneRemapTable`), else the one
/// with the same key-bone name.
pub(super) fn map_equipment_bones(
    character: &[m2::Bone],
    equipment: &[m2::Bone],
) -> Result<Vec<usize>, String> {
    let by_crc: HashMap<u32, usize> = character
        .iter()
        .enumerate()
        .filter(|(_, bone)| bone.name_crc != 0)
        .map(|(index, bone)| (bone.name_crc, index))
        .collect();
    let by_name: HashMap<_, _> = character
        .iter()
        .enumerate()
        .map(|(index, bone)| {
            (
                bone_names::bone_display_name(bone.key_bone_id, index),
                index,
            )
        })
        .collect();
    equipment
        .iter()
        .enumerate()
        .map(|(index, bone)| {
            if let Some(&joint) = (bone.name_crc != 0)
                .then(|| by_crc.get(&bone.name_crc))
                .flatten()
            {
                return Ok(joint);
            }
            let name = bone_names::bone_display_name(bone.key_bone_id, index);
            by_name
                .get(&name)
                .copied()
                .ok_or_else(|| format!("Bound equipment bone {name} has no character joint"))
        })
        .collect()
}

fn bind_character_skin(item: &mut Gd<Node3D>, skin: &Gd<Skin>) {
    for child in item.get_children().iter_shared() {
        if let Ok(mut mesh) = child.try_cast::<MeshInstance3D>() {
            mesh.set_skin(skin);
            mesh.set_skeleton_path("../../Skeleton3D");
        }
    }
    if let Some(animation) = item.get_node_or_null("M2Animation") {
        animation.free();
    }
    if let Some(skeleton) = item.get_node_or_null("Skeleton3D") {
        skeleton.free();
    }
}

fn native_transform(definition: &transforms::EquipmentTransformDef) -> Transform3D {
    let radians = Vector3::from_array(definition.rotation_deg.map(f32::to_radians));
    Transform3D::new(
        Basis::from_euler(EulerOrder::XYZ, radians),
        Vector3::from_array(definition.translation),
    )
    .scaled_local(Vector3::from_array(definition.scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(key_bone_id: i32) -> m2::Bone {
        m2::Bone {
            key_bone_id,
            flags: 0,
            parent_bone_id: -1,
            submesh_id: 0,
            name_crc: 0,
            pivot: [0.0; 3],
        }
    }

    #[test]
    fn bound_item_uses_authored_named_joints_not_same_numeric_index() {
        let character = [bone(6), bone(22), bone(-1)];
        let item = [bone(22), bone(6), bone(-1)];
        assert_eq!(
            map_equipment_bones(&character, &item).unwrap(),
            vec![1, 0, 2]
        );
        assert!(
            map_equipment_bones(&character, &[bone(999)])
                .unwrap_err()
                .contains("KeyBone999")
        );
    }

    #[test]
    fn bound_item_bones_match_by_bone_name_crc_before_key_bone() {
        let crc = |key_bone_id, name_crc| m2::Bone {
            name_crc,
            ..bone(key_bone_id)
        };
        // Non-key bones (-1) with the character's CRCs, in another order.
        let character = [crc(-1, 11), crc(-1, 22), crc(6, 33)];
        let item = [crc(-1, 22), crc(-1, 11), crc(6, 0)];
        assert_eq!(
            map_equipment_bones(&character, &item).unwrap(),
            vec![1, 0, 2]
        );
        // An unknown CRC on a non-key bone past the character's bones has no joint.
        let unknown = [crc(-1, 22), crc(-1, 11), crc(6, 0), crc(-1, 99)];
        assert!(map_equipment_bones(&character, &unknown).is_err());
    }

    #[test]
    fn native_waist_base_mesh_and_skeletal_collection_keep_their_parts() {
        let path =
            Path::new("item/objectcomponents/collections/belt_leather_questbloodelf_b_01.m2");
        let rigid = [m2::Bone {
            name_crc: 3_962_896_125,
            pivot: [0.014494737, 0.0, -0.20134047],
            ..bone(-1)
        }];
        let bound = slot_uses_bound_joints(EquipmentSlot::Waist, path, &rigid, [0]);
        assert!(!bound);
        assert_eq!(model_attachment_id(EquipmentSlot::Waist, path), 53);
        assert!(equipment_mesh_part_allowed(
            EquipmentSlot::Waist,
            path,
            bound,
            0
        ));

        // Actual body belt collections retain their 18xx parts, not base mesh0.
        let skeletal = [m2::Bone {
            flags: 0x200,
            ..bone(-1)
        }];
        let bound =
            slot_uses_bound_joints(EquipmentSlot::Waist, path, &skeletal, [1801, 1802, 2201]);
        assert!(bound);
        for part in [1801, 1802] {
            assert!(equipment_mesh_part_allowed(
                EquipmentSlot::Waist,
                path,
                bound,
                part
            ));
        }
        for part in [0, 2201] {
            assert!(!equipment_mesh_part_allowed(
                EquipmentSlot::Waist,
                path,
                bound,
                part
            ));
        }
    }

    #[test]
    fn skeletal_waist_missing_root_joint_remains_an_error() {
        let path = Path::new("item/objectcomponents/collections/belt.m2");
        let character = [bone(0), bone(6)];
        let item = [m2::Bone {
            flags: 0x200,
            name_crc: 3_962_896_125,
            ..bone(-1)
        }];
        assert!(slot_uses_bound_joints(
            EquipmentSlot::Waist,
            path,
            &item,
            [1801]
        ));
        assert!(
            map_equipment_bones(&character, &item)
                .unwrap_err()
                .contains("no character joint")
        );
    }

    #[test]
    fn item_transform_preserves_translation_xyz_rotation_and_local_scale() {
        let definition = transforms::EquipmentTransformDef {
            translation: [1.0, 2.0, 3.0],
            rotation_deg: [90.0, 0.0, 0.0],
            scale: [2.0, 3.0, 4.0],
        };
        let transform = native_transform(&definition);
        assert!(transform.origin.distance_to(Vector3::new(1.0, 2.0, 3.0)) < 0.00001);
        assert!((transform * Vector3::UP).distance_to(Vector3::new(1.0, 2.0, 6.0)) < 0.00001);
    }
}
