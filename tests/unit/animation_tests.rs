use super::*;
use crate::animation::billboard::compute_bone_stages;
use crate::asset::m2_anim::AnimTrack;

fn single_key_vec3_track(value: [f32; 3]) -> AnimTrack<[f32; 3]> {
    AnimTrack {
        interpolation_type: 0,
        global_sequence: -1,
        sequences: vec![(vec![0], vec![value])],
    }
}

fn single_key_rot_track() -> AnimTrack<[i16; 4]> {
    AnimTrack {
        interpolation_type: 0,
        global_sequence: -1,
        sequences: vec![(vec![0], vec![[0, 0, 0, i16::MAX]])],
    }
}

fn stationary_bone(translation: [f32; 3]) -> BoneAnimTracks {
    BoneAnimTracks {
        translation: single_key_vec3_track(translation),
        rotation: single_key_rot_track(),
        scale: single_key_vec3_track([1.0, 1.0, 1.0]),
    }
}

fn stand_sequence() -> M2AnimSequence {
    M2AnimSequence {
        id: 0,
        variation_id: 0,
        duration: 1000,
        movespeed: 0.0,
        flags: 0,
        blend_time: 0,
        next_animation: -1,
    }
}

fn sequence(anim_id: u16, duration: u32) -> M2AnimSequence {
    M2AnimSequence {
        id: anim_id,
        variation_id: 0,
        duration,
        movespeed: 0.0,
        flags: 0,
        blend_time: 150,
        next_animation: -1,
    }
}

fn stand_sequence_with_next(
    duration: u32,
    variation_id: u16,
    next_animation: i16,
) -> M2AnimSequence {
    M2AnimSequence {
        id: 0,
        variation_id,
        duration,
        movespeed: 0.0,
        flags: 0,
        blend_time: 150,
        next_animation,
    }
}

fn single_root_bone() -> Vec<M2Bone> {
    vec![M2Bone {
        key_bone_id: -1,
        flags: 0,
        parent_bone_id: -1,
        submesh_id: 0,
        pivot: [0.0, 0.0, 0.0],
    }]
}

#[test]
fn apply_animation_updates_each_model_with_its_own_data() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, apply_animation);

    let joint_a = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();
    let joint_b = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();

    let player_a = M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 0.0,
        looping: true,
        transition: None,
    };
    let player_b = M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 0.0,
        looping: true,
        transition: None,
    };
    let data_a = M2AnimData {
        bones: single_root_bone(),
        spherical_billboards: vec![false],
        sequences: vec![stand_sequence()],
        bone_tracks: vec![stationary_bone([1.0, 2.0, 3.0])],
        joint_entities: vec![joint_a],
    };
    let data_b = M2AnimData {
        bones: single_root_bone(),
        spherical_billboards: vec![false],
        sequences: vec![stand_sequence()],
        bone_tracks: vec![stationary_bone([4.0, 5.0, 6.0])],
        joint_entities: vec![joint_b],
    };

    app.world_mut().spawn((player_a, data_a));
    app.world_mut().spawn((player_b, data_b));

    app.update();

    let transform_a = app.world().get::<Transform>(joint_a).unwrap();
    let transform_b = app.world().get::<Transform>(joint_b).unwrap();
    assert_eq!(transform_a.translation, Vec3::new(1.0, 3.0, -2.0));
    assert_eq!(transform_b.translation, Vec3::new(4.0, 6.0, -5.0));
}

#[test]
fn animation_plugin_runs_on_char_select() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    app.insert_state(GameState::CharSelect);
    app.add_plugins(AnimationPlugin);

    let joint = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();
    app.world_mut().spawn((
        M2AnimPlayer {
            current_seq_idx: 0,
            time_ms: 0.0,
            looping: true,
            transition: None,
        },
        M2AnimData {
            bones: single_root_bone(),
            spherical_billboards: vec![false],
            sequences: vec![stand_sequence()],
            bone_tracks: vec![stationary_bone([1.0, 2.0, 3.0])],
            joint_entities: vec![joint],
        },
    ));

    app.update();

    let transform = app.world().get::<Transform>(joint).unwrap();
    assert_eq!(
        transform.translation,
        Vec3::new(1.0, 3.0, -2.0),
        "char-select models should sample their idle pose instead of staying in rest pose"
    );
}

#[test]
fn animation_plugin_runs_on_char_create() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    app.insert_state(GameState::CharCreate);
    app.add_plugins(AnimationPlugin);

    let joint = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();
    app.world_mut().spawn((
        M2AnimPlayer {
            current_seq_idx: 0,
            time_ms: 0.0,
            looping: true,
            transition: None,
        },
        M2AnimData {
            bones: single_root_bone(),
            spherical_billboards: vec![false],
            sequences: vec![stand_sequence()],
            bone_tracks: vec![stationary_bone([1.0, 2.0, 3.0])],
            joint_entities: vec![joint],
        },
    ));

    app.update();

    let transform = app.world().get::<Transform>(joint).unwrap();
    assert_eq!(
        transform.translation,
        Vec3::new(1.0, 3.0, -2.0),
        "char-create models should sample their idle pose instead of staying in rest pose"
    );
}

#[test]
fn animation_plugin_runs_on_inworld_selection_debug() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    app.insert_state(GameState::InWorldSelectionDebug);
    app.add_plugins(AnimationPlugin);

    let joint = app
        .world_mut()
        .spawn((Transform::IDENTITY, BonePivot(Vec3::ZERO)))
        .id();
    app.world_mut().spawn((
        M2AnimPlayer {
            current_seq_idx: 0,
            time_ms: 0.0,
            looping: true,
            transition: None,
        },
        M2AnimData {
            bones: single_root_bone(),
            spherical_billboards: vec![false],
            sequences: vec![stand_sequence()],
            bone_tracks: vec![stationary_bone([1.0, 2.0, 3.0])],
            joint_entities: vec![joint],
        },
    ));

    app.update();

    let transform = app.world().get::<Transform>(joint).unwrap();
    assert_eq!(
        transform.translation,
        Vec3::new(1.0, 3.0, -2.0),
        "in-world selection debug models should sample their idle pose"
    );
}

#[test]
fn looping_stand_advances_into_next_authored_variant() {
    let mut player = M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 900.0,
        looping: true,
        transition: None,
    };
    let data = M2AnimData {
        bones: vec![],
        spherical_billboards: vec![],
        sequences: vec![
            stand_sequence_with_next(1000, 0, 1),
            stand_sequence_with_next(2000, 1, -1),
        ],
        bone_tracks: vec![],
        joint_entities: vec![],
    };

    advance_player_time(&mut player, &data, 200.0);

    assert_eq!(
        player.current_seq_idx, 1,
        "should follow the authored idle chain"
    );
    assert_eq!(
        player.time_ms, 100.0,
        "should preserve overflow into the next variant"
    );
    assert!(
        player.transition.is_some(),
        "switching stand variants should crossfade"
    );
}

#[test]
fn spherical_billboard_does_not_propagate_to_descendants() {
    let bones = vec![
        M2Bone {
            key_bone_id: -1,
            flags: 0x8,
            parent_bone_id: -1,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
        M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: 0,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
        M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: 1,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
    ];
    assert_eq!(
        propagate_spherical_billboards(&bones),
        vec![true, false, false]
    );
}

#[test]
fn non_billboard_child_world_pose_is_camera_stable() {
    let bones = vec![
        M2Bone {
            key_bone_id: -1,
            flags: 0x8,
            parent_bone_id: -1,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
        M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: 0,
            submesh_id: 0,
            pivot: [1.0, 0.0, 0.0],
        },
    ];
    let sbb = vec![true, false];

    let mut pre_a = vec![Mat4::IDENTITY; 2];
    let mut post_a = vec![Mat4::IDENTITY; 2];
    let (_, pp_a, pp_post_a) = compute_bone_stages(
        &bones,
        &sbb,
        0,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::IDENTITY),
        &pre_a,
        &post_a,
    )
    .unwrap();
    pre_a[0] = pp_a;
    post_a[0] = pp_post_a;
    let (_, _, child_post_a) = compute_bone_stages(
        &bones,
        &sbb,
        1,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::IDENTITY),
        &pre_a,
        &post_a,
    )
    .unwrap();

    let mut pre_b = vec![Mat4::IDENTITY; 2];
    let mut post_b = vec![Mat4::IDENTITY; 2];
    let (_, pp_b, pp_post_b) = compute_bone_stages(
        &bones,
        &sbb,
        0,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::from_rotation_y(1.1)),
        &pre_b,
        &post_b,
    )
    .unwrap();
    pre_b[0] = pp_b;
    post_b[0] = pp_post_b;
    let (_, _, child_post_b) = compute_bone_stages(
        &bones,
        &sbb,
        1,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::from_rotation_y(1.1)),
        &pre_b,
        &post_b,
    )
    .unwrap();

    let (_, child_rot_a, child_pos_a) = child_post_a.to_scale_rotation_translation();
    let (_, child_rot_b, child_pos_b) = child_post_b.to_scale_rotation_translation();
    assert!(child_pos_a.distance(child_pos_b) < 1.0e-5);
    assert!(child_rot_a.dot(child_rot_b).abs() > 0.9999);
}

#[test]
fn spherical_billboard_bone_tracks_camera_motion() {
    let bones = vec![M2Bone {
        key_bone_id: -1,
        flags: 0x8,
        parent_bone_id: -1,
        submesh_id: 0,
        pivot: [0.0, 0.0, 0.0],
    }];
    let sbb = vec![true];

    fn cam_bb_rot(camera_rotation: Quat) -> Quat {
        let forward = camera_rotation * -Vec3::Z;
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();
        Quat::from_mat3(&Mat3::from_cols(right, up, forward))
            * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
    }

    fn root_stage(bones: &[M2Bone], sbb: &[bool], camera_rotation: Quat) -> Mat4 {
        let pre = [Mat4::IDENTITY];
        let post = [Mat4::IDENTITY];
        let (_, _, stage) = compute_bone_stages(
            bones,
            sbb,
            0,
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            Some(cam_bb_rot(camera_rotation)),
            &pre,
            &post,
        )
        .unwrap();
        stage
    }

    let focus = Vec3::Y * 0.5;
    let camera_rot_a = Transform::from_translation(Vec3::new(2.2, 1.1, 2.8))
        .looking_at(focus, Vec3::Y)
        .rotation;
    let camera_rot_b = Transform::from_translation(Vec3::new(-2.4, 1.7, 1.9))
        .looking_at(focus, Vec3::Y)
        .rotation;

    let stage_a = root_stage(&bones, &sbb, camera_rot_a);
    let stage_b = root_stage(&bones, &sbb, camera_rot_b);
    let (_, rot_a, pos_a) = stage_a.to_scale_rotation_translation();
    let (_, rot_b, pos_b) = stage_b.to_scale_rotation_translation();
    assert!(
        pos_a.distance(pos_b) < 1.0e-6,
        "billboard pivot translation should remain stable when camera moves"
    );
    assert!(
        rot_a.dot(rot_b).abs() < 0.999,
        "billboard orientation should update when camera moves"
    );
}

// --- Cast animation ---

#[test]
fn cast_anim_directed_id() {
    assert_eq!(CastAnimKind::Directed.cast_anim_id(), 51);
}

#[test]
fn cast_anim_omni_id() {
    assert_eq!(CastAnimKind::Omni.cast_anim_id(), 52);
}

#[test]
fn cast_anim_channel_id() {
    assert_eq!(CastAnimKind::Channel.cast_anim_id(), 76);
}

#[test]
fn ready_anim_directed_id() {
    assert_eq!(CastAnimKind::Directed.ready_anim_id(), 55);
}

#[test]
fn ready_anim_omni_id() {
    assert_eq!(CastAnimKind::Omni.ready_anim_id(), 56);
}

#[test]
fn cast_anim_state_lifecycle() {
    let mut state = CastAnimState::new(CastAnimKind::Directed, 2.5);
    assert!(!state.is_finished());
    assert_eq!(state.kind, CastAnimKind::Directed);
    state.tick(1.0);
    assert!(!state.is_finished());
    state.tick(2.0);
    assert!(state.is_finished());
}

#[test]
fn cast_anim_state_tick_clamps() {
    let mut state = CastAnimState::new(CastAnimKind::Omni, 0.5);
    state.tick(999.0);
    assert_eq!(state.remaining, 0.0);
    assert!(state.is_finished());
}

#[test]
fn cast_anim_default_is_directed() {
    assert_eq!(CastAnimKind::default(), CastAnimKind::Directed);
}

// --- Channel animation loop ---

#[test]
fn channel_is_looping() {
    assert!(CastAnimKind::Channel.is_looping());
    assert!(!CastAnimKind::Directed.is_looping());
    assert!(!CastAnimKind::Omni.is_looping());
}

#[test]
fn channel_state_should_loop_while_active() {
    let state = CastAnimState::channel(5.0);
    assert!(state.should_loop());
    assert_eq!(state.kind, CastAnimKind::Channel);
}

#[test]
fn channel_state_stops_looping_when_finished() {
    let mut state = CastAnimState::channel(0.5);
    state.tick(1.0);
    assert!(state.is_finished());
    assert!(!state.should_loop());
}

#[test]
fn directed_cast_does_not_loop() {
    let state = CastAnimState::new(CastAnimKind::Directed, 2.5);
    assert!(!state.should_loop());
}

#[test]
fn current_anim_id_cast_vs_hold() {
    let mut state = CastAnimState::new(CastAnimKind::Directed, 2.5);
    assert_eq!(state.current_anim_id(), ANIM_SPELL_CAST_DIRECTED);
    state.holding = true;
    assert_eq!(state.current_anim_id(), ANIM_READY_SPELL_DIRECTED);
}

#[test]
fn channel_current_anim_id_always_channel() {
    let state = CastAnimState::channel(5.0);
    assert_eq!(state.current_anim_id(), ANIM_CHANNEL);
    // Channel hold is also ANIM_CHANNEL
    let mut held = CastAnimState::channel(5.0);
    held.holding = true;
    assert_eq!(held.current_anim_id(), ANIM_CHANNEL);
}

// --- Attack animations ---

#[test]
fn attack_1h_anim_id() {
    assert_eq!(MeleeWeaponKind::OneHand.attack_anim_id(), ANIM_ATTACK_1H);
}

#[test]
fn attack_2h_anim_id() {
    assert_eq!(MeleeWeaponKind::TwoHand.attack_anim_id(), ANIM_ATTACK_2H);
}

#[test]
fn attack_off_anim_id() {
    assert_eq!(MeleeWeaponKind::OffHand.attack_anim_id(), ANIM_ATTACK_OFF);
}

#[test]
fn parry_1h_uses_1h_anim() {
    assert_eq!(MeleeWeaponKind::OneHand.parry_anim_id(), ANIM_PARRY_1H);
    assert_eq!(MeleeWeaponKind::OffHand.parry_anim_id(), ANIM_PARRY_1H);
}

#[test]
fn parry_2h_uses_2h_anim() {
    assert_eq!(MeleeWeaponKind::TwoHand.parry_anim_id(), ANIM_PARRY_2H);
}

#[test]
fn ready_stance_by_weapon() {
    assert_eq!(MeleeWeaponKind::OneHand.ready_anim_id(), ANIM_READY_1H);
    assert_eq!(MeleeWeaponKind::TwoHand.ready_anim_id(), ANIM_READY_2H);
    assert_eq!(MeleeWeaponKind::OffHand.ready_anim_id(), ANIM_READY_1H);
}

#[test]
fn attack_anim_state_lifecycle() {
    let mut state = AttackAnimState::new(MeleeWeaponKind::TwoHand, 2.0);
    assert!(!state.is_finished());
    assert_eq!(state.anim_id(), ANIM_ATTACK_2H);
    state.tick(1.0);
    assert!(!state.is_finished());
    state.tick(1.5);
    assert!(state.is_finished());
}

#[test]
fn attack_anim_ids_all_distinct() {
    let ids = [
        MeleeWeaponKind::OneHand.attack_anim_id(),
        MeleeWeaponKind::TwoHand.attack_anim_id(),
        MeleeWeaponKind::OffHand.attack_anim_id(),
    ];
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[0], ids[2]);
    assert_ne!(ids[1], ids[2]);
}

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

// --- Animation overrides ---

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
        2098, // Kick
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
    // Unregistered spell falls back to kind default
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

// --- Animation cancel ---

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
fn cancel_movement_returns_walk_when_walking() {
    let (anim, _) = cancel_anim_params(AnimCancelReason::Movement, true, false);
    assert_eq!(anim, ANIM_WALK);
}

#[test]
fn cancel_interrupt_returns_stand_fast() {
    let (anim, blend) = cancel_anim_params(AnimCancelReason::Interrupt, false, false);
    assert_eq!(anim, ANIM_STAND);
    assert!((blend - 100.0).abs() < 1.0); // fast blend
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
