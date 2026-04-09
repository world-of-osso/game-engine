use super::*;

#[test]
fn emote_anim_state_uses_expected_anim_ids() {
    assert_eq!(
        EmoteAnimState::new(shared::protocol::EmoteKind::Wave).anim_id(),
        ANIM_WAVE
    );
    assert_eq!(
        EmoteAnimState::new(shared::protocol::EmoteKind::Dance).anim_id(),
        ANIM_DANCE
    );
    assert_eq!(
        EmoteAnimState::new(shared::protocol::EmoteKind::Sit).anim_id(),
        ANIM_SIT_GROUND
    );
    assert_eq!(
        EmoteAnimState::new(shared::protocol::EmoteKind::Sleep).anim_id(),
        ANIM_SLEEP
    );
    assert_eq!(
        EmoteAnimState::new(shared::protocol::EmoteKind::Kneel).anim_id(),
        ANIM_KNEEL
    );
}

#[test]
fn pose_emotes_loop_until_interrupted() {
    assert!(EmoteAnimState::new(shared::protocol::EmoteKind::Sit).loops_until_interrupted());
    assert!(EmoteAnimState::new(shared::protocol::EmoteKind::Sleep).loops_until_interrupted());
    assert!(EmoteAnimState::new(shared::protocol::EmoteKind::Kneel).loops_until_interrupted());
    assert!(!EmoteAnimState::new(shared::protocol::EmoteKind::Wave).loops_until_interrupted());
}

#[test]
fn apply_emote_animation_starts_then_clears_finished_wave() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, apply_emote_animation);

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
                bones: vec![],
                spherical_billboards: vec![],
                sequences: vec![sequence(ANIM_STAND, 1000), sequence(ANIM_WAVE, 500)],
                bone_tracks: vec![],
                joint_entities: vec![],
            },
            EmoteAnimState::new(shared::protocol::EmoteKind::Wave),
        ))
        .id();

    app.update();

    let player = app.world().get::<M2AnimPlayer>(entity).unwrap();
    assert_eq!(
        player.current_seq_idx, 1,
        "wave should become the active sequence"
    );
    assert!(app.world().get::<EmoteAnimState>(entity).is_some());

    app.world_mut()
        .entity_mut(entity)
        .get_mut::<M2AnimPlayer>()
        .unwrap()
        .time_ms = 500.0;
    app.update();

    let player = app.world().get::<M2AnimPlayer>(entity).unwrap();
    assert_eq!(
        player.current_seq_idx, 0,
        "finished wave should return to stand"
    );
    assert!(app.world().get::<EmoteAnimState>(entity).is_none());
}

#[test]
fn apply_emote_animation_keeps_sit_pose_until_movement_interrupts() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, apply_emote_animation);

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
                bones: vec![],
                spherical_billboards: vec![],
                sequences: vec![
                    sequence(ANIM_STAND, 1000),
                    sequence(ANIM_SIT_GROUND, 1000),
                    sequence(ANIM_RUN, 1000),
                ],
                bone_tracks: vec![],
                joint_entities: vec![],
            },
            MovementState::default(),
            EmoteAnimState::new(shared::protocol::EmoteKind::Sit),
        ))
        .id();

    app.update();

    let player = app.world().get::<M2AnimPlayer>(entity).unwrap();
    assert_eq!(player.current_seq_idx, 1);
    assert!(player.looping, "sit pose should loop");
    assert!(app.world().get::<EmoteAnimState>(entity).is_some());

    app.world_mut()
        .entity_mut(entity)
        .get_mut::<MovementState>()
        .unwrap()
        .direction = MoveDirection::Forward;
    app.update();

    let player = app.world().get::<M2AnimPlayer>(entity).unwrap();
    assert_eq!(player.current_seq_idx, 2, "movement should return to run");
    assert!(app.world().get::<EmoteAnimState>(entity).is_none());
}
