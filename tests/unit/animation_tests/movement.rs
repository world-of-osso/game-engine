use super::*;

#[test]
fn switch_jump_uses_land_run_when_moving_forward() {
    let mut player = M2AnimPlayer {
        current_seq_idx: 1,
        time_ms: 200.0,
        looping: true,
        transition: None,
    };
    let movement = MovementState {
        direction: MoveDirection::Forward,
        running: true,
        jumping: false,
        autorun: false,
        swimming: false,
    };
    let sequences = vec![
        sequence(ANIM_STAND, 1000),
        sequence(ANIM_JUMP, 400),
        sequence(ANIM_JUMP_END, 200),
        sequence(ANIM_JUMP_LAND_RUN, 250),
    ];

    switch_jump(&mut player, &movement, Some(ANIM_JUMP), &sequences);

    assert_eq!(
        sequences[player.current_seq_idx].id, ANIM_JUMP_LAND_RUN,
        "forward landing should prefer JumpLandRun when available"
    );
}

#[test]
fn switch_jump_falls_back_to_jump_end_without_land_run() {
    let mut player = M2AnimPlayer {
        current_seq_idx: 1,
        time_ms: 200.0,
        looping: true,
        transition: None,
    };
    let movement = MovementState {
        direction: MoveDirection::Forward,
        running: true,
        jumping: false,
        autorun: false,
        swimming: false,
    };
    let sequences = vec![
        sequence(ANIM_STAND, 1000),
        sequence(ANIM_JUMP, 400),
        sequence(ANIM_JUMP_END, 200),
    ];

    switch_jump(&mut player, &movement, Some(ANIM_JUMP), &sequences);

    assert_eq!(
        sequences[player.current_seq_idx].id, ANIM_JUMP_END,
        "models without JumpLandRun should keep the generic landing anim"
    );
}

#[test]
fn override_registry_empty_by_default() {
    let reg = AnimOverrideRegistry::default();
    assert!(reg.is_empty());
    assert!(reg.get(100).is_none());
}

#[test]
fn override_insert_and_get() {
    let mut reg = AnimOverrideRegistry::default();
    reg.insert(
        2098,
        AnimOverride {
            anim_id: ANIM_SPELL_CAST_DIRECTED,
            looping: false,
        },
    );
    let entry = reg.get(2098).unwrap();
    assert_eq!(entry.anim_id, ANIM_SPELL_CAST_DIRECTED);
    assert!(!entry.looping);
}

#[test]
fn override_resolve_uses_override() {
    let mut reg = AnimOverrideRegistry::default();
    reg.insert(
        2098,
        AnimOverride {
            anim_id: ANIM_ATTACK_1H,
            looping: false,
        },
    );
    assert_eq!(reg.resolve(2098, CastAnimKind::Directed), ANIM_ATTACK_1H);
}

#[test]
fn override_resolve_falls_back_to_default() {
    let reg = AnimOverrideRegistry::default();
    assert_eq!(
        reg.resolve(9999, CastAnimKind::Directed),
        ANIM_SPELL_CAST_DIRECTED
    );
    assert_eq!(reg.resolve(9999, CastAnimKind::Channel), ANIM_CHANNEL);
}

#[test]
fn override_resolve_looping() {
    let mut reg = AnimOverrideRegistry::default();
    reg.insert(
        200,
        AnimOverride {
            anim_id: ANIM_CHANNEL,
            looping: true,
        },
    );
    assert!(reg.resolve_looping(200, CastAnimKind::Directed));
    assert!(!reg.resolve_looping(999, CastAnimKind::Directed));
    assert!(reg.resolve_looping(999, CastAnimKind::Channel));
}

#[test]
fn override_insert_replaces() {
    let mut reg = AnimOverrideRegistry::default();
    reg.insert(
        100,
        AnimOverride {
            anim_id: 1,
            looping: false,
        },
    );
    reg.insert(
        100,
        AnimOverride {
            anim_id: 2,
            looping: true,
        },
    );
    assert_eq!(reg.len(), 1);
    assert_eq!(reg.get(100).unwrap().anim_id, 2);
    assert!(reg.get(100).unwrap().looping);
}

#[test]
fn cancel_movement_returns_run_when_running() {
    let (anim, blend) = cancel_anim_params(AnimCancelReason::Movement, true, true);
    assert_eq!(anim, ANIM_RUN);
    assert!((blend - 200.0).abs() < 1.0);
}

#[test]
fn swim_direction_mapping_uses_swim_sequences() {
    assert_eq!(
        direction_to_anim_id(MoveDirection::None, true, true),
        ANIM_SWIM_IDLE
    );
    assert_eq!(
        direction_to_anim_id(MoveDirection::Forward, true, true),
        ANIM_SWIM
    );
    assert_eq!(
        direction_to_anim_id(MoveDirection::Left, true, true),
        ANIM_SWIM_LEFT
    );
    assert_eq!(
        direction_to_anim_id(MoveDirection::Right, true, true),
        ANIM_SWIM_RIGHT
    );
    assert_eq!(
        direction_to_anim_id(MoveDirection::Backward, true, true),
        ANIM_SWIM_BACKWARDS
    );
}

#[test]
fn switch_animation_uses_swim_idle_when_stationary_in_water() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, switch_animation);

    let entity = app
        .world_mut()
        .spawn((
            M2AnimPlayer {
                current_seq_idx: 0,
                time_ms: 0.0,
                looping: true,
                transition: None,
            },
            M2AnimData {
                bones: Vec::new(),
                spherical_billboards: Vec::new(),
                sequences: vec![
                    M2AnimSequence {
                        id: ANIM_STAND,
                        variation_id: 0,
                        duration: 1000,
                        movespeed: 0.0,
                        flags: 0,
                        blend_time: 150,
                        next_animation: -1,
                    },
                    M2AnimSequence {
                        id: ANIM_SWIM_IDLE,
                        variation_id: 0,
                        duration: 1000,
                        movespeed: 0.0,
                        flags: 0,
                        blend_time: 150,
                        next_animation: -1,
                    },
                ],
                bone_tracks: Vec::new(),
                joint_entities: Vec::new(),
            },
            MovementState {
                swimming: true,
                ..Default::default()
            },
        ))
        .id();

    app.update();

    let player = app
        .world()
        .get::<M2AnimPlayer>(entity)
        .expect("anim player");
    let data = app.world().get::<M2AnimData>(entity).expect("anim data");
    assert_eq!(data.sequences[player.current_seq_idx].id, ANIM_SWIM_IDLE);
}

#[test]
fn turn_in_place_direction_tracks_yaw_delta() {
    assert_eq!(
        turn_in_place_direction(0.0, 0.04),
        Some(TurnDirection::Left)
    );
    assert_eq!(
        turn_in_place_direction(0.0, -0.04),
        Some(TurnDirection::Right)
    );
    assert_eq!(turn_in_place_direction(0.0, 0.005), None);
}

#[test]
fn switch_animation_uses_turn_left_when_idle_and_rotating() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, (sync_turn_in_place_state, switch_animation).chain());

    let entity = app
        .world_mut()
        .spawn((
            Transform::from_rotation(Quat::from_rotation_y(0.0)),
            M2AnimPlayer {
                current_seq_idx: 0,
                time_ms: 0.0,
                looping: true,
                transition: None,
            },
            M2AnimData {
                bones: Vec::new(),
                spherical_billboards: Vec::new(),
                sequences: vec![
                    sequence(ANIM_STAND, 1000),
                    sequence(ANIM_SHUFFLE_LEFT, 250),
                    sequence(ANIM_SHUFFLE_RIGHT, 250),
                ],
                bone_tracks: Vec::new(),
                joint_entities: Vec::new(),
            },
            MovementState::default(),
        ))
        .id();

    app.update();

    app.world_mut()
        .entity_mut(entity)
        .insert(Transform::from_rotation(Quat::from_rotation_y(0.08)));
    app.update();

    let player = app
        .world()
        .get::<M2AnimPlayer>(entity)
        .expect("anim player");
    let data = app.world().get::<M2AnimData>(entity).expect("anim data");
    assert_eq!(data.sequences[player.current_seq_idx].id, ANIM_SHUFFLE_LEFT);
}

#[test]
fn cancel_movement_returns_walk_when_walking() {
    let (anim, _) = cancel_anim_params(AnimCancelReason::Movement, true, false);
    assert_eq!(anim, ANIM_WALK);
}

#[test]
fn cancel_interrupt_returns_stand_fast() {
    let (anim, blend) = cancel_anim_params(AnimCancelReason::Interrupt, false, false);
    assert_eq!(anim, ANIM_STAND);
    assert!((blend - 100.0).abs() < 1.0);
}

#[test]
fn cancel_complete_while_stationary_returns_stand() {
    let (anim, _) = cancel_anim_params(AnimCancelReason::Complete, false, false);
    assert_eq!(anim, ANIM_STAND);
}

#[test]
fn cancel_complete_while_moving_returns_run() {
    let (anim, _) = cancel_anim_params(AnimCancelReason::Complete, true, true);
    assert_eq!(anim, ANIM_RUN);
}

#[test]
fn cancel_complete_while_walking_returns_walk() {
    let (anim, _) = cancel_anim_params(AnimCancelReason::Complete, true, false);
    assert_eq!(anim, ANIM_WALK);
}

#[test]
fn interrupt_blend_faster_than_normal() {
    let (_, fast) = cancel_anim_params(AnimCancelReason::Interrupt, false, false);
    let (_, normal) = cancel_anim_params(AnimCancelReason::Complete, false, false);
    assert!(fast < normal);
}
