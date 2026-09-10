use std::path::Path;
use std::time::{Duration, Instant};

use bevy::ecs::system::SystemState;
use bevy::mesh::VertexAttributeValues;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Mesh, Mesh3d};
use bevy::prelude::*;

use super::*;
use crate::animation::AnimationPlugin;
use crate::creature_display::CreatureDisplayMap;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;

#[path = "equipment_live_tests/skin_palette_census.rs"]
mod skin_palette_census;

#[path = "equipment_live_tests/hand_attachments.rs"]
mod hand_attachments;

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

    let mut app = game_engine::test_harness::headless_app_with(configure_live_test_app);
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
    app.world_mut().trigger(EquipmentChanged {
        entity: spawned.model_root,
    });

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

#[test]
fn live_bevy_animation_helm_follows_character_bone() {
    let (spawned, path, mut app) =
        setup_live_helm_test_app().expect("required human and helm assets");
    equip_live_helm(&mut app, spawned.model_root, path);
    app.update();
    app.update();
    let helm = head_equipment_entity(app.world_mut()).expect("helm");
    let bone = app.world().get::<ChildOf>(helm).unwrap().parent();
    let local = app.world().get::<Transform>(helm).unwrap().to_matrix();
    let mut positions = Vec::new();
    for fraction in [0.0, 0.5] {
        sample_live_stand(&mut app, spawned.model_root, fraction);
        let expected = sampled_joint_world(app.world(), spawned.model_root, bone) * local;
        let actual = app
            .world()
            .get::<GlobalTransform>(helm)
            .unwrap()
            .to_matrix();
        assert_matrix_near(actual, expected);
        positions.push(actual.transform_point3(Vec3::ZERO));
    }
    assert!(
        positions[0].distance(positions[1]) > 0.0001,
        "helm must move with the sampled head"
    );
}

#[test]
fn offline_charselect_animated_helm_survives_despawn_and_respawn() {
    let (spawned, helm_path, mut app) =
        setup_live_helm_test_app().expect("required human and helm assets for offline lifecycle");
    let root = spawned.model_root;
    let helm = equip_and_assert_offline_helm_motion(&mut app, root, helm_path);
    let joints = app
        .world()
        .get::<crate::animation::M2AnimData>(root)
        .expect("animated character joints")
        .joint_entities
        .clone();
    assert!(!joints.is_empty());

    assert!(app.world_mut().despawn(root));
    app.update();
    for entity in std::iter::once(root)
        .chain(joints)
        .chain(std::iter::once(helm))
    {
        assert!(
            app.world().get_entity(entity).is_err(),
            "retained entity {entity:?}"
        );
    }
    assert!(head_equipment_entity(app.world_mut()).is_none());

    let respawned = spawn_live_character(&mut app, Path::new("data/models/humanmale_hd.m2"));
    assert_ne!(respawned.model_root, root);
    let new_helm = equip_and_assert_offline_helm_motion(&mut app, respawned.model_root, helm_path);
    assert_ne!(new_helm, helm);
}

fn equip_and_assert_offline_helm_motion(app: &mut App, root: Entity, path: &Path) -> Entity {
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::CharSelect
    );
    assert!(
        !app.world()
            .contains_resource::<game_engine::network_runtime::worker::NetworkRuntime>()
    );
    equip_live_helm(app, root, path);
    app.update();
    app.update();
    let helm = head_equipment_entity(app.world_mut()).expect("equipped offline helm");
    let bone = app
        .world()
        .get::<ChildOf>(helm)
        .expect("helm bone parent")
        .parent();
    let local = app.world().get::<Transform>(helm).unwrap().to_matrix();
    let mut positions = Vec::new();
    for fraction in [0.0, 0.5] {
        sample_live_stand(app, root, fraction);
        let actual = app
            .world()
            .get::<GlobalTransform>(helm)
            .unwrap()
            .to_matrix();
        assert_matrix_near(actual, sampled_joint_world(app.world(), root, bone) * local);
        positions.push(actual.transform_point3(Vec3::ZERO));
    }
    assert!(
        positions[0].distance(positions[1]) > 0.0001,
        "offline helm must animate"
    );
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::CharSelect
    );
    assert!(
        !app.world()
            .contains_resource::<game_engine::network_runtime::worker::NetworkRuntime>()
    );
    helm
}

#[test]
fn live_bevy_animation_chest_vertex_follows_character_skin() {
    let (spawned, path, mut app) =
        setup_live_chest_test_app().expect("required human and chest assets");
    equip_live_chest(&mut app, spawned.model_root, path);
    app.update();
    app.update();
    let chest = chest_equipment_entity(app.world_mut()).expect("chest");
    let mut descendants = Vec::new();
    collect_descendants(app.world(), chest, &mut descendants);
    let mesh_entity = descendants
        .into_iter()
        .find(|&entity| {
            app.world().get::<SkinnedMesh>(entity).is_some()
                && app.world().get::<Mesh3d>(entity).is_some()
        })
        .expect("shared-skin chest mesh");
    let skin = app.world().get::<SkinnedMesh>(mesh_entity).unwrap().clone();
    let mesh = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&app.world().get::<Mesh3d>(mesh_entity).unwrap().0)
        .unwrap();
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions")
    };
    let Some(VertexAttributeValues::Uint16x4(indices)) =
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
    else {
        panic!("joint indices")
    };
    let Some(VertexAttributeValues::Float32x4(weights)) =
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
    else {
        panic!("joint weights")
    };
    let vertex = Vec3::from(positions[0]);
    let indices = indices[0];
    let weights = weights[0];
    assert!((weights.iter().sum::<f32>() - 1.0).abs() < 0.0001);
    let inverse_bindposes = app
        .world()
        .resource::<Assets<SkinnedMeshInverseBindposes>>()
        .get(&skin.inverse_bindposes)
        .unwrap()
        .to_vec();
    let character_joints = app
        .world()
        .get::<crate::animation::M2AnimData>(spawned.model_root)
        .unwrap()
        .joint_entities
        .clone();
    let mut samples = Vec::new();
    for fraction in [0.0, 0.5] {
        sample_live_stand(&mut app, spawned.model_root, fraction);
        let retained = app.world().get::<SkinnedMesh>(mesh_entity).unwrap();
        assert_eq!(retained.joints, skin.joints);
        assert_eq!(retained.inverse_bindposes, skin.inverse_bindposes);
        let mut actual = Vec3::ZERO;
        let mut expected = Vec3::ZERO;
        for (&index, &weight) in indices.iter().zip(&weights) {
            if weight == 0.0 {
                continue;
            }
            let index = usize::from(index);
            let joint = skin.joints[index];
            assert!(
                character_joints.contains(&joint),
                "weighted chest vertex must use character joint"
            );
            assert_matrix_near(inverse_bindposes[index], Mat4::IDENTITY);
            let global = app
                .world()
                .get::<GlobalTransform>(joint)
                .unwrap()
                .to_matrix();
            actual += (global * inverse_bindposes[index]).transform_point3(vertex) * weight;
            expected += sampled_joint_world(app.world(), spawned.model_root, joint)
                .transform_point3(vertex)
                * weight;
        }
        assert!(
            actual.distance(expected) < 0.0001,
            "skinned vertex {actual:?}, expected {expected:?}"
        );
        samples.push(actual);
    }
    assert!(
        samples[0].distance(samples[1]) > 0.0001,
        "concrete chest vertex must deform across stand samples"
    );
}

fn sample_live_stand(app: &mut App, owner: Entity, fraction: f32) {
    let data = app
        .world()
        .get::<crate::animation::M2AnimData>(owner)
        .unwrap();
    let sequence = data
        .sequences
        .iter()
        .position(|sequence| sequence.id == 0)
        .expect("human stand clip");
    let time_ms = data.sequences[sequence].duration as f32 * fraction;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::ZERO,
    ));
    let mut player = app
        .world_mut()
        .get_mut::<crate::animation::M2AnimPlayer>(owner)
        .unwrap();
    player.current_seq_idx = sequence;
    player.time_ms = time_ms;
    player.transition = None;
    app.update();
    let player = app
        .world()
        .get::<crate::animation::M2AnimPlayer>(owner)
        .unwrap();
    assert_eq!(player.current_seq_idx, sequence);
    assert!(
        player.transition.is_none(),
        "sample must not introduce a policy crossfade"
    );
}

fn sampled_joint_world(world: &World, owner: Entity, joint: Entity) -> Mat4 {
    let data = world.get::<crate::animation::M2AnimData>(owner).unwrap();
    let Some(index) = data
        .joint_entities
        .iter()
        .position(|&candidate| candidate == joint)
    else {
        return world.get::<GlobalTransform>(joint).unwrap().to_matrix();
    };
    let player = world.get::<crate::animation::M2AnimPlayer>(owner).unwrap();
    let (translation, rotation, scale) = crate::animation::evaluate_bone_components(
        &data.bone_tracks[index],
        player.current_seq_idx,
        player.time_ms as u32,
    );
    let pivot = world.get::<crate::animation::BonePivot>(joint).unwrap().0;
    let local = Mat4::from_scale_rotation_translation(
        scale,
        rotation,
        translation + pivot - rotation * (scale * pivot),
    );
    let parent = world.get::<ChildOf>(joint).unwrap().parent();
    sampled_joint_world(world, owner, parent) * local
}

fn assert_matrix_near(actual: Mat4, expected: Mat4) {
    assert!(
        actual.abs_diff_eq(expected, 0.0001),
        "transform {actual:?}, expected {expected:?}"
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
    let mut app = game_engine::test_harness::headless_app_with(configure_live_test_app);
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
    let mut app = game_engine::test_harness::headless_app_with(configure_live_test_app);
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
    let mut app = game_engine::test_harness::headless_app_with(configure_live_test_app);
    let spawned = spawn_live_character(&mut app, character_path);
    Some((spawned, feet_path, app))
}

fn configure_live_test_app(app: &mut App) {
    app.add_plugins((bevy::state::app::StatesPlugin, TransformPlugin));
    app.insert_state(GameState::CharSelect);
    app.add_plugins(AnimationPlugin);
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<StandardMaterial>::default());
    app.insert_resource(Assets::<Image>::default());
    app.insert_resource(Assets::<M2EffectMaterial>::default());
    app.insert_resource(Assets::<SkinnedMeshInverseBindposes>::default());
    app.insert_resource(EquipmentTransforms::default());
    register_equipment_observers(app);
}

#[test]
#[ignore = "benchmark-style integration test; run explicitly"]
fn bench_m2_spawn_pipeline_headless() {
    const TORCH_P99_BUDGET_MS: f64 = 500.0;
    const HUMAN_HD_P99_BUDGET_MS: f64 = 1_500.0;
    let cases = [
        (
            "torch",
            Path::new("data/models/145513.m2"),
            10_usize,
            TORCH_P99_BUDGET_MS,
        ),
        (
            "humanmale_hd",
            Path::new("data/models/humanmale_hd.m2"),
            5_usize,
            HUMAN_HD_P99_BUDGET_MS,
        ),
    ];
    for (label, model_path, iterations, p99_budget_ms) in cases {
        if !model_path.exists() {
            println!("Skipping {label}: missing {}", model_path.display());
            continue;
        }
        let (samples, entities) = measure_headless_m2_spawn_pipeline(model_path, iterations);
        let elapsed: Duration = samples.iter().copied().sum();
        let average = elapsed.div_f64(iterations as f64);
        let p99 = game_engine::test_harness::p99_duration(&samples).expect("benchmark samples");
        println!(
            "m2_spawn_pipeline[{label}] iterations={iterations} total_ms={:.2} avg_ms={:.2} p99_ms={:.2} spawned_entities={entities}",
            elapsed.as_secs_f64() * 1000.0,
            average.as_secs_f64() * 1000.0,
            p99.as_secs_f64() * 1000.0,
        );
        assert!(entities > 0, "expected spawned entities for {label}");
        assert!(
            p99.as_secs_f64() * 1000.0 <= p99_budget_ms,
            "expected {label} p99 <= {p99_budget_ms:.2}ms, got {:.2}ms",
            p99.as_secs_f64() * 1000.0,
        );
    }
}

fn measure_headless_m2_spawn_pipeline(
    model_path: &Path,
    iterations: usize,
) -> (Vec<Duration>, usize) {
    let mut samples = Vec::with_capacity(iterations);
    let mut final_entity_count = 0;
    for _ in 0..iterations {
        let start = Instant::now();
        let mut app = game_engine::test_harness::headless_app_with(configure_live_test_app);
        let spawned = spawn_live_character(&mut app, model_path);
        final_entity_count = spawned_entity_count(app.world(), spawned.model_root);
        assert!(final_entity_count > 0, "expected spawned model descendants");
        samples.push(start.elapsed());
    }
    (samples, final_entity_count)
}

fn spawned_entity_count(world: &World, root: Entity) -> usize {
    let mut count = 1;
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            count += children.len();
            stack.extend(children.iter());
        }
    }
    count
}

fn spawn_live_character(app: &mut App, character_path: &Path) -> m2_scene::SpawnedAnimatedStaticM2 {
    let creature_display_map = CreatureDisplayMap;
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
        state
            .get_mut(world)
            .expect("live equipment spawn system state");
    let mut ctx = live_m2_scene_spawn_context(
        &creature_display_map,
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
    );
    let spawned = m2_scene::spawn_animated_static_m2_parts(
        &mut ctx,
        character_path,
        Transform::from_scale(Vec3::splat(1.1)),
    )
    .expect("spawned humanmale_hd");
    state.apply(world);
    app.update();
    spawned
}

fn live_m2_scene_spawn_context<'a, 'w, 's>(
    creature_display_map: &'a CreatureDisplayMap,
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    images: &'a mut Assets<Image>,
    inv_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
) -> m2_scene::M2SceneSpawnContext<'a, 'w, 's> {
    m2_scene::M2SceneSpawnContext {
        commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes,
            materials,
            effect_materials,
            skybox_materials: None,
            images,
            inverse_bindposes: inv_bp,
        },
        creature_display_map,
    }
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
    app.world_mut()
        .trigger(EquipmentChanged { entity: model_root });
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
    app.world_mut()
        .trigger(EquipmentChanged { entity: model_root });
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
    app.world_mut()
        .trigger(EquipmentChanged { entity: model_root });
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
