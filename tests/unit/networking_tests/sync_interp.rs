use super::*;

fn registered_interp_test_app(stage: crate::game::inworld_scene_stage::InWorldSceneStage) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_state(crate::game_state::GameState::InWorld);
    app.insert_resource(stage);
    app.add_systems(
        Update,
        interpolate_remote_entities
            .run_if(in_state(crate::game_state::GameState::InWorld))
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_npcs),
    );
    app
}

fn spawn_interpolation_fixture(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            InterpolationTarget {
                target: Vec3::new(10.0, 0.0, 0.0),
            },
            Transform::IDENTITY,
            RemoteEntity,
        ))
        .id()
}

#[test]
fn sync_updates_rotation_target() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            NetRotation {
                x: 0.0,
                y: 1.5,
                z: 0.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RotationTarget { yaw: 0.0 },
            RemoteEntity,
        ))
        .id();
    app.update();
    let rot = app.world().get::<RotationTarget>(entity).unwrap();
    assert!((rot.yaw - 1.5).abs() < 1e-6);
}

#[test]
fn sync_updates_interpolation_target() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RemoteEntity,
        ))
        .id();
    app.update();
    let interp = app.world().get::<InterpolationTarget>(entity).unwrap();
    assert_eq!(interp.target, Vec3::new(10.0, 20.0, 30.0));
}

#[test]
fn sync_skips_entities_without_remote_marker() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<InterpolationTarget>(entity)
            .unwrap()
            .target,
        Vec3::ZERO
    );
}

#[test]
fn sync_skips_local_player_even_with_remote_marker() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RemoteEntity,
            LocalPlayer,
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<InterpolationTarget>(entity)
            .unwrap()
            .target,
        Vec3::ZERO
    );
}

#[test]
fn interpolation_moves_toward_target() {
    let mut app = interp_test_app();
    let start = Vec3::ZERO;
    let target = Vec3::new(10.0, 0.0, 0.0);
    let entity = app
        .world_mut()
        .spawn((
            InterpolationTarget { target },
            Transform::from_translation(start),
            RemoteEntity,
        ))
        .id();

    app.update();
    app.update();

    let pos = app.world().get::<Transform>(entity).unwrap().translation;
    assert!(pos.x > 0.0);
    assert!(pos.x < 10.0);
    assert!(pos.y.abs() < 1e-6);
}

#[test]
fn registered_interpolation_does_not_move_remote_before_npcs() {
    for stage in [
        crate::game::inworld_scene_stage::InWorldSceneStage::Empty,
        crate::game::inworld_scene_stage::InWorldSceneStage::Character,
        crate::game::inworld_scene_stage::InWorldSceneStage::Skybox,
        crate::game::inworld_scene_stage::InWorldSceneStage::Terrain,
    ] {
        let mut app = registered_interp_test_app(stage);
        let entity = spawn_interpolation_fixture(&mut app);
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));

        app.update();

        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().translation,
            Vec3::ZERO,
            "remote interpolation must stay disabled at {stage:?}"
        );
    }
}

#[test]
fn registered_interpolation_moves_remote_at_npcs() {
    let mut app =
        registered_interp_test_app(crate::game::inworld_scene_stage::InWorldSceneStage::Npcs);
    let entity = spawn_interpolation_fixture(&mut app);
    app.update();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(0.1));

    app.update();

    let pos = app.world().get::<Transform>(entity).unwrap().translation;
    assert!(pos.x > 0.0);
    assert!(pos.x < 10.0);
}

#[test]
fn interpolation_skips_local_player_even_with_remote_marker() {
    let mut app = interp_test_app();
    let start = Vec3::ZERO;
    let target = Vec3::new(10.0, 20.0, 30.0);
    let entity = app
        .world_mut()
        .spawn((
            InterpolationTarget { target },
            Transform::from_translation(start),
            RemoteEntity,
            LocalPlayer,
        ))
        .id();

    app.update();
    app.update();

    let pos = app.world().get::<Transform>(entity).unwrap().translation;
    assert_eq!(pos, start);
}

#[derive(Resource, Default)]
struct TransformChangeCount(usize);

fn count_transform_changes(
    changed: Query<(), Changed<Transform>>,
    mut count: ResMut<TransformChangeCount>,
) {
    count.0 += changed.iter().count();
}

fn interpolation_change_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TransformChangeCount>();
    app.add_systems(
        Update,
        (interpolate_remote_entities, count_transform_changes).chain(),
    );
    app
}

#[test]
fn interpolation_does_not_dirty_transform_at_target() {
    let mut app = interpolation_change_test_app();
    let entity = app
        .world_mut()
        .spawn((
            InterpolationTarget { target: Vec3::ZERO },
            RotationTarget { yaw: 0.0 },
            Transform::IDENTITY,
            RemoteEntity,
        ))
        .id();

    app.update();
    app.world_mut().resource_mut::<TransformChangeCount>().0 = 0;
    app.update();

    assert_eq!(app.world().resource::<TransformChangeCount>().0, 0);
    assert_eq!(
        *app.world().get::<Transform>(entity).unwrap(),
        Transform::IDENTITY
    );
}
