//! A player's stand state pose on HumanMale HD 1011653: sitting down plays SitGroundDown
//! 96 into SitGround 97, standing up plays SitGroundUp 98 into Stand, moving cuts the
//! up clip short, and a player first seen sitting holds SitGround without sitting down.
use super::{AnimationState, MIN_MOVEMENT_BLEND_MS, npc_pose_tests::human_male_hd};

const STAND: u16 = 0;
const RUN: u16 = 5;
const SIT_DOWN: u16 = 96;
const SIT: u16 = 97;
const SIT_UP: u16 = 98;

fn current_id(player: &AnimationState) -> u16 {
    player.sequences[player.current].id
}

fn duration(player: &AnimationState) -> f64 {
    f64::from(player.sequences[player.current].duration)
}

/// Advance in 50 ms frames, updating locomotion to `movement` after each, as the client
/// does every frame; the clip of each frame.
fn frames(player: &mut AnimationState, movement: u16, count: usize) -> Vec<u16> {
    (0..count)
        .map(|_| {
            player.advance(50.0).unwrap();
            player.update_locomotion(movement, false, false).unwrap();
            current_id(player)
        })
        .collect()
}

fn standing() -> AnimationState {
    let mut player = human_male_hd();
    player.update_locomotion(STAND, false, false).unwrap();
    player.advance(500.0).unwrap();
    player
}

#[test]
fn sitting_down_plays_sit_ground_down_then_holds_sit_ground() {
    let mut player = standing();
    player.pose_transition = true;
    assert!(player.update_locomotion(SIT, false, false).unwrap());
    assert_eq!(current_id(&player), SIT_DOWN);
    assert!(!player.looping);
    let blend = player.transition.as_ref().unwrap().duration_ms;
    assert!(blend >= MIN_MOVEMENT_BLEND_MS, "blend {blend}");
    let down_frames = (duration(&player) / 50.0).ceil() as usize;
    let clips = frames(&mut player, SIT, down_frames + 2);
    let first_sit = clips.iter().position(|&id| id == SIT).unwrap();
    assert!(clips[..first_sit].iter().all(|&id| id == SIT_DOWN));
    assert!(clips[first_sit..].iter().all(|&id| id == SIT));
    assert!(first_sit + 1 >= down_frames, "sat after {first_sit} frames");
    assert!(player.looping);
}

#[test]
fn standing_up_plays_sit_ground_up_then_stands() {
    let mut player = standing();
    player.select_animation_id(SIT, true).unwrap();
    player.advance(500.0).unwrap();
    player.pose_transition = true;
    assert!(player.update_locomotion(STAND, false, false).unwrap());
    assert_eq!(current_id(&player), SIT_UP);
    assert!(!player.looping);
    let up_frames = (duration(&player) / 50.0).ceil() as usize;
    let clips = frames(&mut player, STAND, up_frames + 2);
    let first_stand = clips.iter().position(|&id| id == STAND).unwrap();
    assert!(clips[..first_stand].iter().all(|&id| id == SIT_UP));
    assert!(
        first_stand + 1 >= up_frames,
        "stood after {first_stand} frames"
    );
    assert!(player.looping);
}

#[test]
fn moving_cuts_the_up_clip_short_with_a_crossfade() {
    let mut player = standing();
    player.select_animation_id(SIT, true).unwrap();
    player.advance(500.0).unwrap();
    player.pose_transition = true;
    player.update_locomotion(STAND, false, false).unwrap();
    assert_eq!(frames(&mut player, STAND, 2), [SIT_UP, SIT_UP]);
    assert_eq!(frames(&mut player, RUN, 1), [RUN]);
    assert!(player.transition.is_some());
}

/// Without a stand state change (a player first seen already sitting, or a creature's
/// pose) the model crossfades straight into SitGround.
#[test]
fn an_unarmed_pose_crossfades_straight_into_its_loop() {
    let mut player = standing();
    assert!(player.update_locomotion(SIT, false, false).unwrap());
    assert_eq!(current_id(&player), SIT);
    assert!(player.looping);
    assert!(player.transition.is_some());
    assert_eq!(frames(&mut player, STAND, 1), [STAND]);
}

/// The arm lasts one update: a stand state change that finds the player moving plays
/// no transition later.
#[test]
fn a_pose_transition_arm_lasts_one_update() {
    let mut player = standing();
    player.pose_transition = true;
    player.update_locomotion(RUN, false, true).unwrap();
    assert_eq!(current_id(&player), RUN);
    assert_eq!(frames(&mut player, SIT, 1), [SIT]);
}
