use super::*;

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
