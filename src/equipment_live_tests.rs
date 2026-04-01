use std::path::Path;

use bevy::ecs::system::SystemState;
use bevy::mesh::VertexAttributeValues;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::mesh::{Mesh, Mesh3d};
use bevy::prelude::*;

use super::*;
use crate::animation::AnimationPlugin;
use crate::creature_display::CreatureDisplayMap;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;

#[test]
fn live_human_male_helm_wraps_head_and_binds_texture() {
    let Some((spawned, helm_path, mut app)) = setup_live_helm_test_app() else {
        return;
    };
    equip_live_helm(&mut app, spawned.model_root, &helm_path);
    app.update();
    app.update();

    let helm_entity = head_equipment_entity(app.world_mut()).expect("spawned head equipment item");
    let head_y = find_named_bone_pivot_y(app.world(), spawned.model_root, "Head")
        .expect("head joint world y");
    let (min_y, max_y) = mesh_world_y_bounds(app.world(), helm_entity).expect("helm mesh bounds");

    assert!(
        min_y <= head_y && max_y >= head_y,
        "expected live helm mesh to wrap head height; head_y={head_y:.3} min_y={min_y:.3} max_y={max_y:.3}"
    );
    assert!(
        count_textured_meshes(app.world(), helm_entity) > 0,
        "expected live helm to bind at least one textured mesh"
    );
}

#[test]
fn live_human_male_back_cloak_spawns_runtime_attachment() {
    let character_path = Path::new("data/models/humanmale_hd.m2");
    let cloak_path =
        Path::new("data/item-models/item/objectcomponents/cape/cape_special_keg_d_01.m2");
    if !character_path.exists() || !cloak_path.exists() {
        return;
    }

    let mut app = App::new();
    configure_live_test_app(&mut app);
    let spawned = spawn_live_character(&mut app, character_path);

    let mut equipment = app
        .world_mut()
        .get_mut::<Equipment>(spawned.model_root)
        .expect("equipment on model root");
    equipment
        .slots
        .insert(EquipmentSlot::Back, cloak_path.to_path_buf());
    equipment
        .slot_skin_fdids
        .insert(EquipmentSlot::Back, [5644278, 0, 0]);

    app.update();
    app.update();

    let mut query = app.world_mut().query::<(Entity, &EquipmentItem)>();
    let found = query
        .iter(app.world())
        .find(|(_, item)| item._slot == EquipmentSlot::Back)
        .map(|(entity, _)| entity);

    assert!(
        found.is_some(),
        "expected spawned back cloak equipment item"
    );
}

#[test]
fn live_human_male_chest_runtime_attachment_uses_character_joints_without_local_animation() {
    let Some((spawned, chest_path, mut app)) = setup_live_chest_test_app() else {
        return;
    };
    equip_live_chest(&mut app, spawned.model_root, &chest_path);
    app.update();
    app.update();

    let chest_entity =
        chest_equipment_entity(app.world_mut()).expect("spawned chest equipment item");
    let parent = app
        .world()
        .get::<ChildOf>(chest_entity)
        .expect("chest parent")
        .parent();
    assert_eq!(parent, spawned_visual_root(app.world(), &spawned));
    assert!(
        app.world()
            .get::<crate::animation::M2AnimPlayer>(chest_entity)
            .is_none()
    );
    assert!(
        app.world()
            .get::<crate::animation::M2AnimData>(chest_entity)
            .is_none()
    );
    assert!(
        find_named_bone_pivot_y(app.world(), chest_entity, "SpineLow").is_none(),
        "expected chest runtime attachment to avoid spawning its own named torso skeleton",
    );
}

#[test]
fn live_human_male_feet_runtime_attachment_uses_character_visual_root() {
    let Some((spawned, feet_path, mut app)) = setup_live_feet_test_app() else {
        return;
    };
    equip_live_feet(&mut app, spawned.model_root, &feet_path);
    app.update();
    app.update();

    let feet_entity = feet_equipment_entity(app.world_mut()).expect("spawned feet equipment item");
    let parent = app
        .world()
        .get::<ChildOf>(feet_entity)
        .expect("feet parent")
        .parent();
    assert_eq!(parent, spawned_visual_root(app.world(), &spawned));
    assert!(
        find_named_bone_pivot_y(app.world(), feet_entity, "FootL").is_none(),
        "expected feet runtime attachment to avoid spawning its own named foot skeleton",
    );
}

fn spawned_visual_root(world: &World, spawned: &m2_scene::SpawnedAnimatedStaticM2) -> Entity {
    let joints = &world
        .get::<crate::animation::M2AnimData>(spawned.model_root)
        .expect("character anim data")
        .joint_entities;
    world
        .get::<ChildOf>(joints[0])
        .expect("visual root parent")
        .parent()
}

fn setup_live_helm_test_app() -> Option<(m2_scene::SpawnedAnimatedStaticM2, &'static Path, App)> {
    let character_path = Path::new("data/models/humanmale_hd.m2");
    let helm_path = Path::new("data/item-models/item/objectcomponents/head/helm_plate_d_02_hum.m2");
    if !character_path.exists() || !helm_path.exists() {
        return None;
    }
    let mut app = App::new();
    configure_live_test_app(&mut app);
    let spawned = spawn_live_character(&mut app, character_path);
    Some((spawned, helm_path, app))
}

fn setup_live_chest_test_app() -> Option<(m2_scene::SpawnedAnimatedStaticM2, &'static Path, App)> {
    let character_path = Path::new("data/models/humanmale_hd.m2");
    let chest_path = Path::new(
        "data/item-models/item/objectcomponents/collections/collections_mail_warfrontsnightelfmythic_d_01_hu_m.m2",
    );
    if !character_path.exists() || !chest_path.exists() {
        return None;
    }
    let mut app = App::new();
    configure_live_test_app(&mut app);
    let spawned = spawn_live_character(&mut app, character_path);
    Some((spawned, chest_path, app))
}

fn setup_live_feet_test_app() -> Option<(m2_scene::SpawnedAnimatedStaticM2, &'static Path, App)> {
    let character_path = Path::new("data/models/humanmale_hd.m2");
    let feet_path = Path::new(
        "data/item-models/item/objectcomponents/collections/collections_leather_raidroguemythic_q_01_hu_m.m2",
    );
    if !character_path.exists() || !feet_path.exists() {
        return None;
    }
    let mut app = App::new();
    configure_live_test_app(&mut app);
    let spawned = spawn_live_character(&mut app, character_path);
    Some((spawned, feet_path, app))
}

fn configure_live_test_app(app: &mut App) {
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        TransformPlugin,
    ));
    app.insert_state(GameState::CharSelect);
    app.add_plugins(AnimationPlugin);
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

fn spawn_live_character(app: &mut App, character_path: &Path) -> m2_scene::SpawnedAnimatedStaticM2 {
    let creature_display_map = CreatureDisplayMap::load_from_data_dir();
    let world = app.world_mut();
    let mut state: SystemState<(
        Commands,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
        ResMut<Assets<M2EffectMaterial>>,
        ResMut<Assets<Image>>,
        ResMut<Assets<SkinnedMeshInverseBindposes>>,
    )> = SystemState::new(world);
    let (mut commands, mut meshes, mut materials, mut effect_materials, mut images, mut inv_bp) =
        state.get_mut(world);
    let spawned = m2_scene::spawn_animated_static_m2_parts(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
        character_path,
        Transform::from_scale(Vec3::splat(1.1)),
        &creature_display_map,
        1.0,
    )
    .expect("spawned humanmale_hd");
    state.apply(world);
    app.update();
    spawned
}

fn equip_live_helm(app: &mut App, model_root: Entity, helm_path: &Path) {
    let mut equipment = app
        .world_mut()
        .get_mut::<Equipment>(model_root)
        .expect("equipment on model root");
    equipment
        .slots
        .insert(EquipmentSlot::Head, helm_path.to_path_buf());
    equipment
        .slot_skin_fdids
        .insert(EquipmentSlot::Head, [140455, 0, 0]);
}

fn equip_live_chest(app: &mut App, model_root: Entity, chest_path: &Path) {
    let mut equipment = app
        .world_mut()
        .get_mut::<Equipment>(model_root)
        .expect("equipment on model root");
    equipment
        .slots
        .insert(EquipmentSlot::Chest, chest_path.to_path_buf());
    equipment
        .slot_skin_fdids
        .insert(EquipmentSlot::Chest, [2373825, 0, 0]);
}

fn equip_live_feet(app: &mut App, model_root: Entity, feet_path: &Path) {
    let mut equipment = app
        .world_mut()
        .get_mut::<Equipment>(model_root)
        .expect("equipment on model root");
    equipment
        .slots
        .insert(EquipmentSlot::Feet, feet_path.to_path_buf());
    equipment
        .slot_skin_fdids
        .insert(EquipmentSlot::Feet, [1360784, 0, 0]);
}

fn head_equipment_entity(world: &mut World) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EquipmentItem)>();
    query
        .iter(world)
        .find(|(_, item)| item._slot == EquipmentSlot::Head)
        .map(|(entity, _)| entity)
}

fn chest_equipment_entity(world: &mut World) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EquipmentItem)>();
    query
        .iter(world)
        .find(|(_, item)| item._slot == EquipmentSlot::Chest)
        .map(|(entity, _)| entity)
}

fn feet_equipment_entity(world: &mut World) -> Option<Entity> {
    let mut query = world.query::<(Entity, &EquipmentItem)>();
    query
        .iter(world)
        .find(|(_, item)| item._slot == EquipmentSlot::Feet)
        .map(|(entity, _)| entity)
}

fn find_named_bone_pivot_y(world: &World, root: Entity, target: &str) -> Option<f32> {
    let mut entities = vec![root];
    collect_descendants(world, root, &mut entities);
    let root_scale = world
        .get::<GlobalTransform>(root)?
        .to_scale_rotation_translation()
        .0
        .y;
    entities.into_iter().find_map(|entity| {
        let name = world.get::<Name>(entity)?;
        let pivot = world.get::<crate::animation::BonePivot>(entity)?;
        (name.as_str() == target).then_some(pivot.0.y * root_scale)
    })
}

fn mesh_world_y_bounds(world: &World, root: Entity) -> Option<(f32, f32)> {
    let meshes = world.resource::<Assets<Mesh>>();
    let mut entities = vec![root];
    collect_descendants(world, root, &mut entities);
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for entity in entities {
        accumulate_mesh_bounds(world, &meshes, entity, &mut min_y, &mut max_y);
    }
    (min_y.is_finite() && max_y.is_finite()).then_some((min_y, max_y))
}

fn accumulate_mesh_bounds(
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
    let Some(VertexAttributeValues::Float32x3(positions)) =
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

fn count_textured_meshes(world: &World, root: Entity) -> usize {
    let materials = world.resource::<Assets<StandardMaterial>>();
    let mut entities = vec![root];
    collect_descendants(world, root, &mut entities);
    entities
        .into_iter()
        .filter_map(|entity| {
            let material = world.get::<MeshMaterial3d<StandardMaterial>>(entity)?;
            let material = materials.get(&material.0)?;
            material.base_color_texture.as_ref()
        })
        .count()
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
