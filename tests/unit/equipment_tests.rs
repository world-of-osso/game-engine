use std::collections::HashMap;
use std::path::Path;

use bevy::mesh::Mesh3d;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy::transform::TransformPlugin;

use super::*;
use crate::animation::M2AnimData;
use crate::asset::m2_attach::M2Attachment;
use crate::m2_effect_material::M2EffectMaterial;

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
fn runtime_mesh_filter_excludes_dk_eye_glow_group() {
    assert!(!runtime_mesh_part_allowed(EquipmentSlot::Back, 1701));
    assert!(!runtime_mesh_part_allowed(EquipmentSlot::Chest, 1702));
    assert!(!runtime_mesh_part_allowed(EquipmentSlot::Legs, 1702));
    assert!(!runtime_mesh_part_allowed(EquipmentSlot::Feet, 1705));
    assert!(!runtime_mesh_part_allowed(EquipmentSlot::Chest, 401));
    assert!(runtime_mesh_part_allowed(EquipmentSlot::Feet, 501));
    assert!(runtime_mesh_part_allowed(EquipmentSlot::Feet, 2001));
    assert!(runtime_mesh_part_allowed(EquipmentSlot::Chest, 2202));
    assert!(runtime_mesh_part_allowed(EquipmentSlot::Waist, 0));
    assert!(runtime_mesh_part_allowed(EquipmentSlot::Waist, 1802));
    assert!(!runtime_mesh_part_allowed(EquipmentSlot::Waist, 2202));
    assert!(!runtime_mesh_part_allowed(EquipmentSlot::Chest, 2301));
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
    let model =
        crate::asset::m2::load_m2(character_path, &[0, 0, 0]).expect("failed to load humanmale_hd");
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
        attachment_joint_translation.y,
        head_offset.y,
    );
}

#[test]
fn spawned_helm_mesh_uses_textured_material_when_skin_fdid_present() {
    let Some((_, helm_path)) = test_model_paths() else {
        return;
    };

    let mut app = App::new();
    configure_equipment_test_app(&mut app);
    let owner =
        spawn_head_equipment_owner(&mut app, helm_path, Vec3::ZERO, Vec3::ZERO, [140455, 0, 0]);

    app.update();
    app.update();

    let descendants = descendant_entities(app.world(), owner);
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let textured_meshes = descendants
        .into_iter()
        .filter_map(|entity| {
            let material = app
                .world()
                .get::<MeshMaterial3d<StandardMaterial>>(entity)?;
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
    let helm_path = Path::new("data/item-models/item/objectcomponents/head/helm_plate_d_02_hum.m2");
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
    app.add_systems(
        Update,
        (attach_rendered_equipment_state, sync_equipment).chain(),
    );
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
                points: HashMap::from([(
                    slot_attachment_id(EquipmentSlot::Head),
                    (0, head_offset),
                )]),
            },
            M2AnimData {
                bones: vec![],
                spherical_billboards: vec![],
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
        let world_pos = global.transform_point(Vec3::new(position[0], position[1], position[2]));
        *min_y = min_y.min(world_pos.y);
        *max_y = max_y.max(world_pos.y);
    }
}
