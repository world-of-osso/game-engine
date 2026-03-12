use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

fn make_state(direction: MoveDirection) -> MovementState {
    MovementState {
        direction,
        ..Default::default()
    }
}

fn make_facing(yaw: f32) -> CharacterFacing {
    CharacterFacing { yaw }
}

#[test]
fn idle_produces_zero_direction() {
    let dir = movement_to_direction(&make_state(MoveDirection::None), &make_facing(0.0));
    assert_eq!(dir, [0.0, 0.0, 0.0]);
}

#[test]
fn forward_at_zero_yaw() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(0.0));
    // yaw=0: forward = [sin(0), 0, cos(0)] = [0, 0, 1]
    assert!(dir[0].abs() < 1e-6);
    assert_eq!(dir[1], 0.0);
    assert!((dir[2] - 1.0).abs() < 1e-6);
}

#[test]
fn forward_at_90_degrees() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(FRAC_PI_2));
    // yaw=π/2: forward = [sin(π/2), 0, cos(π/2)] = [1, 0, 0]
    assert!((dir[0] - 1.0).abs() < 1e-6);
    assert!((dir[2]).abs() < 1e-6);
}

#[test]
fn backward_is_opposite_of_forward() {
    let facing = make_facing(0.5);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let bwd = movement_to_direction(&make_state(MoveDirection::Backward), &facing);
    assert!((fwd[0] + bwd[0]).abs() < 1e-6);
    assert!((fwd[2] + bwd[2]).abs() < 1e-6);
}

#[test]
fn left_is_perpendicular_to_forward() {
    let facing = make_facing(0.0);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    // dot product should be zero (perpendicular)
    let dot = fwd[0] * left[0] + fwd[2] * left[2];
    assert!(dot.abs() < 1e-6);
}

#[test]
fn right_is_opposite_of_left() {
    let facing = make_facing(PI / 3.0);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    let right = movement_to_direction(&make_state(MoveDirection::Right), &facing);
    assert!((left[0] + right[0]).abs() < 1e-6);
    assert!((left[2] + right[2]).abs() < 1e-6);
}

#[test]
fn direction_is_unit_length() {
    for dir in [
        MoveDirection::Forward,
        MoveDirection::Backward,
        MoveDirection::Left,
        MoveDirection::Right,
    ] {
        let d = movement_to_direction(&make_state(dir), &make_facing(1.23));
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-6,
            "direction {dir:?} has length {len}"
        );
    }
}

#[test]
fn y_component_always_zero() {
    for yaw in [0.0, FRAC_PI_2, PI, -PI] {
        for dir in [
            MoveDirection::Forward,
            MoveDirection::Backward,
            MoveDirection::Left,
            MoveDirection::Right,
        ] {
            let d = movement_to_direction(&make_state(dir), &make_facing(yaw));
            assert_eq!(d[1], 0.0);
        }
    }
}

#[test]
fn sync_updates_rotation_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);

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
    assert!(
        (rot.yaw - 1.5).abs() < 1e-6,
        "rotation target should be 1.5, got {}",
        rot.yaw
    );
}

#[test]
fn sync_updates_interpolation_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);

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
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);

    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            // no RemoteEntity marker
        ))
        .id();

    app.update();

    let interp = app.world().get::<InterpolationTarget>(entity).unwrap();
    assert_eq!(interp.target, Vec3::ZERO);
}

#[test]
fn interpolation_moves_toward_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, interpolate_remote_entities);

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

    // First update has zero delta_time; run twice so time advances.
    app.update();
    app.update();

    let pos = app.world().get::<Transform>(entity).unwrap().translation;
    // Should have moved toward target but not reached it in one frame
    assert!(pos.x > 0.0, "should move toward target");
    assert!(pos.x < 10.0, "should not snap to target");
    assert!((pos.y).abs() < 1e-6, "y should stay zero");
}

#[test]
fn chat_log_caps_at_max() {
    let mut log = ChatLog::default();
    for i in 0..101 {
        log.messages
            .push((format!("player{i}"), format!("msg{i}"), ChatType::Say));
        if log.messages.len() > MAX_CHAT_LOG {
            log.messages.remove(0);
        }
    }
    assert_eq!(log.messages.len(), MAX_CHAT_LOG);
    assert_eq!(log.messages[0].0, "player1");
    assert_eq!(log.messages[99].0, "player100");
}

fn selected_with_name(name: &str) -> SelectedCharacterId {
    SelectedCharacterId {
        character_id: Some(1),
        character_name: Some(name.to_string()),
    }
}

#[test]
fn is_local_player_entity_matches_selected() {
    let selected = selected_with_name("Theron");
    assert!(is_local_player_entity("Theron", Some(&selected)));
}

#[test]
fn is_local_player_entity_rejects_different() {
    let selected = selected_with_name("Theron");
    assert!(!is_local_player_entity("OtherPlayer", Some(&selected)));
}

#[test]
fn is_local_player_entity_none_without_resource() {
    assert!(!is_local_player_entity("Theron", None));
}

#[test]
fn is_local_player_entity_none_when_not_selected() {
    let selected = SelectedCharacterId::default();
    assert!(!is_local_player_entity("Theron", Some(&selected)));
}
