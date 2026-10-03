use super::{AnimationState, MIN_MOVEMENT_BLEND_MS};
use game_engine_core::{asset::m2_format::m2_anim, m2};

fn sequence(id: u16, duration: u32, blend_time: u16) -> m2::Sequence {
    m2::Sequence {
        id,
        variation_id: 0,
        duration,
        movespeed: 0.0,
        flags: 0,
        blend_time,
        frequency: 1,
        replay: [0, 0],
        variation_next: -1,
        bounds: [[0.0; 3]; 2],
    }
}

fn track<T>(values: Vec<T>) -> m2::AnimTrack<T> {
    m2_anim::AnimTrack {
        interpolation_type: 1,
        global_sequence: -1,
        sequences: values
            .into_iter()
            .map(|value| (vec![0], vec![value]))
            .collect(),
    }
}

fn player(with_running_landing: bool) -> AnimationState {
    let mut sequences = vec![
        sequence(0, 500, 100),
        sequence(5, 500, 200),
        sequence(37, 200, 80),
        sequence(38, 300, 80),
        sequence(39, 250, 80),
    ];
    if with_running_landing {
        sequences.push(sequence(187, 250, 80));
    }
    let translation = track(
        (0..sequences.len())
            .map(|index| [index as f32 * 2.0, 0.0, 0.0])
            .collect(),
    );
    AnimationState {
        sequence_animated: vec![false; sequences.len()],
        release_ms: vec![None; sequences.len()],
        action_events: vec![Vec::new(); sequences.len()],
        fired_events: Vec::new(),
        sequences,
        global_sequences: Vec::new(),
        global_ms: 0.0,
        tracks: vec![m2::BoneAnimTracks {
            translation,
            rotation: track(vec![
                [0, 0, 0, 32767];
                if with_running_landing { 6 } else { 5 }
            ]),
            scale: track(vec![
                [1.0, 1.0, 1.0];
                if with_running_landing { 6 } else { 5 }
            ]),
        }]
        .into(),
        local_pivots: vec![godot::builtin::Vector3::ZERO],
        current: 0,
        time_ms: 0.0,
        looping: true,
        transition: None,
        random_state: 0,
        action: None,
        upper_body: vec![false],
        legs_free: true,
        locomotion_speed: None,
        pose_transition: false,
    }
}

fn current_id(player: &AnimationState) -> u16 {
    player.sequences[player.current].id
}

fn assert_pose_continuous(
    player: &mut AnimationState,
    movement: u16,
    jumping: bool,
    forward: bool,
) {
    let before = player.poses()[0].origin;
    assert!(
        player
            .update_locomotion(movement, jumping, forward)
            .unwrap()
    );
    assert!(before.distance_to(player.poses()[0].origin) < 0.0001);
    assert!(player.transition.as_ref().unwrap().duration_ms >= MIN_MOVEMENT_BLEND_MS);
}

#[test]
fn early_landing_waits_for_start_and_airborne_loop_does_not_restart() {
    let mut player = player(true);
    assert_pose_continuous(&mut player, 0, true, false);
    assert_eq!(current_id(&player), 37);
    player.advance(75.0).unwrap();
    let time = player.time_ms;
    let pose = player.poses()[0].origin;
    assert!(!player.update_locomotion(0, false, false).unwrap());
    assert_eq!(player.time_ms, time);
    assert_eq!(player.poses()[0].origin, pose);
    player.advance(125.0).unwrap();
    assert_pose_continuous(&mut player, 0, false, false);
    assert_eq!(current_id(&player), 38);
    assert!(player.looping);
    player.advance(650.0).unwrap();
    let loop_time = player.time_ms;
    assert!(!player.update_locomotion(0, true, false).unwrap());
    assert_eq!(current_id(&player), 38);
    assert_eq!(player.time_ms, loop_time);
    assert_pose_continuous(&mut player, 0, false, false);
    assert_eq!(current_id(&player), 39);
    assert!(!player.looping);
}

#[test]
fn landing_holds_until_complete_then_resumes_latest_movement() {
    let mut player = player(true);
    player.update_locomotion(0, true, false).unwrap();
    player.advance(200.0).unwrap();
    player.update_locomotion(0, true, false).unwrap();
    player.advance(40.0).unwrap();
    assert_pose_continuous(&mut player, 0, false, false);
    assert_eq!(current_id(&player), 39);
    player.advance(125.0).unwrap();
    let landing_time = player.time_ms;
    let blend_elapsed = player.transition.as_ref().unwrap().elapsed_ms;
    assert!(!player.update_locomotion(5, true, true).unwrap());
    assert_eq!(current_id(&player), 39);
    assert_eq!(player.time_ms, landing_time);
    assert_eq!(
        player.transition.as_ref().unwrap().elapsed_ms,
        blend_elapsed
    );
    player.advance(125.0).unwrap();
    assert_pose_continuous(&mut player, 5, false, true);
    assert_eq!(current_id(&player), 5);
    assert!(player.looping);
    let resumed = player.time_ms;
    assert!(!player.update_locomotion(5, false, true).unwrap());
    assert_eq!(player.time_ms, resumed);
}

#[test]
fn running_landing_requires_forward_run_and_available_clip() {
    for (available, forward, expected) in [(true, true, 187), (true, false, 39), (false, true, 39)]
    {
        let mut player = player(available);
        player.update_locomotion(0, true, false).unwrap();
        player.advance(200.0).unwrap();
        player.update_locomotion(0, true, false).unwrap();
        assert_pose_continuous(&mut player, 5, false, forward);
        assert_eq!(current_id(&player), expected);
        player.advance(75.0).unwrap();
        let time = player.time_ms;
        assert!(!player.update_locomotion(0, false, false).unwrap());
        assert_eq!(player.time_ms, time);
        assert_eq!(current_id(&player), expected);
        player.advance(175.0).unwrap();
        assert_pose_continuous(&mut player, 0, false, false);
        assert_eq!(current_id(&player), 0);
    }
}

#[test]
fn missing_required_jump_clip_reports_error_without_changing_pose() {
    let mut player = player(true);
    player.sequences.retain(|sequence| sequence.id != 37);
    let pose = player.poses();
    let error = player.update_locomotion(0, true, false).unwrap_err();
    assert!(error.contains("37"));
    assert_eq!(current_id(&player), 0);
    assert_eq!(player.poses(), pose);
}
