use super::*;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::ecs::system::RunSystemOnce;
use bevy::state::app::StatesPlugin;
use std::time::Duration;

fn animated_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        StatesPlugin,
        bevy::transform::TransformPlugin,
    ));
    app.insert_state(crate::game_state::GameState::M2Debug);
    app.add_plugins(crate::animation::AnimationPlugin);
    app.init_asset::<Mesh>()
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<M2EffectMaterial>()
        .init_asset::<SkinnedMeshInverseBindposes>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    app
}

fn spawn_display(app: &mut App, display_id: u32, scale: f32) -> (Entity, Entity) {
    app.world_mut()
        .run_system_once(move |mut commands: Commands, mut assets: NpcSpawnAssets| {
            let npc = commands.spawn(Transform::default()).id();
            let root = spawn_npc_visual_root(&mut commands, npc, scale);
            let mut spawn_assets = crate::m2_spawn::SpawnAssets {
                meshes: &mut assets.meshes,
                materials: &mut assets.materials,
                effect_materials: &mut assets.effect_materials,
                skybox_materials: None,
                images: &mut assets.images,
                inverse_bindposes: &mut assets.inv_bp,
            };
            assert!(try_spawn_npc_model(
                &mut commands,
                &mut spawn_assets,
                root,
                npc,
                Some(&ModelDisplay { display_id }),
                Some(&CreatureDisplayMap),
                scale,
            ));
            (npc, root)
        })
        .unwrap()
}

fn assert_display_idle_moves(app: &mut App, display_id: u32) {
    let (npc, root) = spawn_display(app, display_id, 0.6);
    let owner = assert_animated_hierarchy(app, npc, root);
    assert_idle_bone_motion(app, owner);
}

fn assert_animated_hierarchy(app: &mut App, npc: Entity, root: Entity) -> Entity {
    let owners: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<crate::animation::M2AnimData>>()
        .iter(app.world())
        .collect();
    assert_eq!(
        owners.len(),
        1,
        "NPC spawn must attach its animation runtime"
    );
    let owner = owners[0];
    assert_eq!(app.world().get::<ChildOf>(owner).unwrap().parent(), root);
    assert_eq!(app.world().get::<ChildOf>(root).unwrap().parent(), npc);
    assert!(app.world().get::<crate::camera::Player>(owner).is_none());
    assert!(app.world().get::<crate::camera::Player>(root).is_none());
    assert!(
        app.world()
            .get::<crate::equipment::Equipment>(owner)
            .is_none_or(|equipment| equipment.slots.is_empty())
    );
    let root_transform = app.world().get::<Transform>(root).unwrap();
    assert_eq!(root_transform.scale, Vec3::splat(0.6));
    assert!((root_transform.rotation * Vec3::X - Vec3::Z).length() < 0.0001);
    owner
}

fn assert_idle_bone_motion(app: &mut App, owner: Entity) {
    for _ in 0..4 {
        app.update();
    }
    let joints = app
        .world()
        .get::<crate::animation::M2AnimData>(owner)
        .unwrap()
        .joint_entities
        .clone();
    let before: Vec<_> = joints
        .iter()
        .map(|joint| *app.world().get::<Transform>(*joint).unwrap())
        .collect();
    for _ in 0..3 {
        app.update();
    }
    assert!(
        joints
            .iter()
            .zip(before)
            .any(|(joint, before)| { *app.world().get::<Transform>(*joint).unwrap() != before }),
        "actual Bevy playback must change at least one idle bone pose"
    );
}

#[test]
fn npc_animated_spawn_advances_real_idle_bones_and_preserves_display_skin() {
    let mut app = animated_app();
    // Northshire sheep: nonzero explicit display texture, distinct from humanoid defaults.
    let display_id = 503;
    assert_display_idle_moves(&mut app, display_id);
    let skin = CreatureDisplayMap.get_skin_fdids(display_id).unwrap();
    let model_path =
        crate::asset::asset_cache::model(CreatureDisplayMap.get_fdid(display_id).unwrap()).unwrap();
    let expected = crate::asset::m2::load_m2(&model_path, &skin).unwrap();
    assert!(
        expected
            .batches
            .iter()
            .any(|batch| batch.texture_fdid == Some(skin[0]))
    );
    let (pixels, width, height) =
        crate::asset::blp::load_blp_rgba(&crate::asset::asset_cache::texture(skin[0]).unwrap())
            .unwrap();
    let expected_image = crate::rgba_image(pixels, width, height);
    let images = app.world().resource::<Assets<Image>>();
    assert!(
        images.iter().any(|(_, image)| image.texture_descriptor.size
            == expected_image.texture_descriptor.size
            && image.data == expected_image.data),
        "explicit creature-display skin must reach spawned image assets"
    );
}

#[test]
fn npc_animated_human_hd_spawn_advances_authored_stand_pose() {
    let path = crate::asset::asset_cache::model(1011653).unwrap();
    let model = crate::asset::m2::load_m2(&path, &[0; 3]).unwrap();
    let stand = model
        .sequences
        .iter()
        .position(|sequence| sequence.id == 0)
        .unwrap();
    let moving_tracks = model
        .bone_tracks
        .iter()
        .filter(|track| {
            track
                .translation
                .sequences
                .get(stand)
                .is_some_and(|(_, values)| values.windows(2).any(|pair| pair[0] != pair[1]))
                || track
                    .rotation
                    .sequences
                    .get(stand)
                    .is_some_and(|(_, values)| values.windows(2).any(|pair| pair[0] != pair[1]))
        })
        .count();
    eprintln!(
        "HumanMaleHD: bones={} sequences={} stand_index={} duration={} moving_stand_tracks={} bounds={:?}..{:?}",
        model.bones.len(),
        model.sequences.len(),
        stand,
        model.sequences[stand].duration,
        moving_tracks,
        model.bounding_box_min,
        model.bounding_box_max
    );
    assert!(
        moving_tracks > 0,
        "authored HumanMaleHD Stand tracks must be available"
    );
    assert_display_idle_moves(&mut animated_app(), 3167);
}

#[test]
fn npc_visual_facing_maps_model_forward_to_logical_forward() {
    let mut world = World::new();
    for yaw in [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
        let root = world
            .run_system_once(move |mut commands: Commands| {
                let npc = commands
                    .spawn(Transform::from_rotation(Quat::from_rotation_y(yaw)))
                    .id();
                spawn_npc_visual_root(&mut commands, npc, 1.0)
            })
            .unwrap();
        let local = world.get::<Transform>(root).unwrap();
        let actual = Quat::from_rotation_y(yaw) * local.rotation * Vec3::X;
        let expected = Quat::from_rotation_y(yaw) * Vec3::Z;
        assert!(
            (actual - expected).length() < 0.0001,
            "yaw {yaw}: {actual} != {expected}"
        );
    }
}
