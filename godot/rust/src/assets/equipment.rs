//! Native equipment resources and authored character attachments.

use std::{collections::HashMap, path::Path};

use game_engine_core::m2;
use godot::{
    classes::{MeshInstance3D, Node3D, Skeleton3D, Skin},
    prelude::*,
};
use osso_asset_resolver::CascListfileResolver;

use super::{
    build_model_filtered,
    creature::{cache_model_files, cache_model_textures},
    read_model,
};
use crate::equipment_appearance_data::{
    EquipmentSlot, RuntimeModelAppearance, model_attachment_id, runtime_mesh_part_allowed,
    slot_uses_bound_joints,
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
    transforms: transforms::EquipmentTransformConfig,
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
    let path = data_root.join("equipment_transforms.ron");
    let content =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let transforms = transforms::EquipmentTransformConfig::parse(&content)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let mut context = EquipmentContext {
        character,
        character_model,
        resolver,
        data_root,
        transforms,
    };
    for model in models {
        context.attach(model)?;
    }
    Ok(())
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
        let bound = slot_uses_bound_joints(definition.slot, authored_path);
        let mut parent = self.parent_for(definition.slot, authored_path, bound)?;
        let path = cache_model_files(self.resolver, self.data_root, definition.fdid)?;
        cache_model_textures(self.resolver, self.data_root, &definition.skin_fdids, &path)?;
        let path = GString::from(path.to_string_lossy().as_ref());
        let parsed = read_model(&path)?;
        let skin = if bound {
            Some(self.bound_skin(&parsed)?)
        } else {
            None
        };
        let (mut item, missing) =
            build_model_filtered(&parsed, &path, &definition.skin_fdids, None, |part| {
                runtime_mesh_part_allowed(definition.slot, part)
            })?;
        if !missing.is_empty() {
            item.free();
            return Err(format!(
                "Equipment {:?} FDID {} missing textures: {missing:?}",
                definition.slot, definition.fdid
            ));
        }
        item.set_name(&format!("Equipment{:?}", definition.slot));
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
            .get_node_or_null(&format!("Skeleton3D/Attachment{id}"))
            .and_then(|node| node.try_cast::<Node3D>().ok())
            .ok_or_else(|| format!("Equipment {slot:?} requires missing character attachment {id}"))
    }

    fn bound_skin(&self, model: &m2::Model) -> Result<Gd<Skin>, String> {
        let mapping = map_equipment_bones(&self.character_model.bones, &model.bones)?;
        let skeleton = self
            .character
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
}

fn map_equipment_bones(
    character: &[m2::Bone],
    equipment: &[m2::Bone],
) -> Result<Vec<usize>, String> {
    let targets: HashMap<_, _> = character
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
            let name = bone_names::bone_display_name(bone.key_bone_id, index);
            targets
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
