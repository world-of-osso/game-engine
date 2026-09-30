//! Combat/spell action clips layered over locomotion on the HumanMale HD model.
use super::npc_pose_tests::{human_male_hd, pose_distance};
use super::{ActionPriority, AnimationState};
use godot::builtin::Transform3D;
use std::{collections::HashMap, fs, path::PathBuf};

const STAND: u16 = 0;
const RUN: u16 = 5;
const ATTACK_1H: u16 = 17;
const ATTACK_2H: u16 = 18;
const READY_SPELL_DIRECTED: u16 = 51;
const SPELL_CAST_DIRECTED: u16 = 53;
const FRAME_MS: f64 = 1000.0 / 60.0;

fn advance(player: &mut AnimationState, frames: usize) {
    for _ in 0..frames {
        player.advance(FRAME_MS).unwrap();
    }
}

/// The action layer's (upper-body, lower-body) weight.
fn weights(player: &AnimationState) -> (f32, f32) {
    let action = player.action.as_ref().expect("action layer");
    (action.upper, action.lower)
}

fn split_distance(a: &[Transform3D], b: &[Transform3D], upper: &[bool]) -> (f32, f32) {
    let mut distances = (0.0f32, 0.0f32);
    for ((a, b), &is_upper) in a.iter().zip(b).zip(upper) {
        let distance = pose_distance(std::slice::from_ref(a), std::slice::from_ref(b));
        if is_upper {
            distances.0 = distances.0.max(distance);
        } else {
            distances.1 = distances.1.max(distance);
        }
    }
    distances
}

/// `AnimationData.Fallback` of the pinned 12.1.0.69933 export.
fn fallbacks() -> HashMap<u16, u16> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/db2/12.1.0.69933/AnimationData.csv");
    fs::read_to_string(path)
        .expect("AnimationData export")
        .lines()
        .skip(1)
        .map(|line| {
            let mut cells = line.split(',');
            let id = cells.next().unwrap().parse().unwrap();
            (id, cells.next().unwrap().parse().unwrap())
        })
        .collect()
}

/// A melee swing fades in from Stand without a pop, moves the body, plays once and
/// fades back into the same Stand an unswinging unit shows.
#[test]
fn swing_crossfades_in_plays_once_and_returns_to_the_locomotion_pose() {
    let mut swinging = human_male_hd();
    let mut still = human_male_hd();
    swinging.update_locomotion(STAND, false, false).unwrap();
    still.update_locomotion(STAND, false, false).unwrap();
    swinging
        .play_action(ATTACK_1H, false, ActionPriority::Combat)
        .unwrap();
    assert!(pose_distance(&swinging.poses(), &still.poses()) < 1e-4);
    assert_eq!(swinging.action_id(), Some(ATTACK_1H));

    // Attack1H's 150 ms blend: one 60 fps frame in, the swing weighs 1/9.
    advance(&mut swinging, 1);
    let (upper, lower) = weights(&swinging);
    assert!((upper - FRAME_MS as f32 / 150.0).abs() < 1e-4 && upper == lower);
    advance(&mut swinging, 29);
    advance(&mut still, 30);
    assert_eq!(weights(&swinging), (1.0, 1.0));
    assert!(pose_distance(&swinging.poses(), &still.poses()) > 0.05);

    // Attack1H lasts 1000 ms, then fades out over its 150 ms blend time.
    advance(&mut swinging, 45);
    advance(&mut still, 45);
    assert_eq!(swinging.action_id(), None);
    assert!(swinging.action.is_none());
    assert!(pose_distance(&swinging.poses(), &still.poses()) < 1e-4);
}

/// A unit swinging on the run keeps its legs on the run cycle; the swing drives the
/// upper body only.
#[test]
fn swing_while_running_keeps_the_legs_on_the_run_cycle() {
    let mut swinging = human_male_hd();
    let mut running = human_male_hd();
    for player in [&mut swinging, &mut running] {
        player.update_locomotion(RUN, false, true).unwrap();
        advance(player, 12);
    }
    swinging
        .play_action(ATTACK_1H, false, ActionPriority::Combat)
        .unwrap();
    for _ in 0..18 {
        for player in [&mut swinging, &mut running] {
            player.advance(FRAME_MS).unwrap();
            player.update_locomotion(RUN, false, true).unwrap();
        }
    }
    let (upper, lower) = split_distance(&swinging.poses(), &running.poses(), &swinging.upper_body);
    assert!(upper > 0.05, "upper body follows the swing ({upper})");
    assert!(lower < 1e-4, "legs stay on Run ({lower})");
}

/// Replacing a swing mid-play crossfades from the pose it reached.
#[test]
fn replacing_a_swing_mid_play_continues_from_the_reached_pose() {
    let mut player = human_male_hd();
    player
        .play_action(ATTACK_1H, false, ActionPriority::Combat)
        .unwrap();
    advance(&mut player, 6);
    let reached = player.poses();
    let blend = weights(&player);
    assert!(blend.0 > 0.5 && blend.0 < 1.0, "{blend:?}");
    player
        .play_action(ATTACK_2H, false, ActionPriority::Combat)
        .unwrap();
    // Same pose, same layer weight: the fade-in carries on instead of restarting.
    assert!(pose_distance(&reached, &player.poses()) < 1e-4);
    assert_eq!(weights(&player), blend);
    assert_eq!(player.action_id(), Some(ATTACK_2H));
}

/// A precast loop is held past its clip length, re-requesting it keeps its time and
/// blend, and stopping it fades back to locomotion.
#[test]
fn held_precast_loop_persists_until_stopped() {
    let mut player = human_male_hd();
    player
        .play_action(READY_SPELL_DIRECTED, true, ActionPriority::Spell)
        .unwrap();
    // ReadySpellDirected lasts 533 ms.
    advance(&mut player, 70);
    assert_eq!(player.action_id(), Some(READY_SPELL_DIRECTED));
    let before = player.poses();
    player
        .play_action(READY_SPELL_DIRECTED, true, ActionPriority::Spell)
        .unwrap();
    assert!(pose_distance(&before, &player.poses()) < 1e-4);
    player.stop_action(READY_SPELL_DIRECTED);
    assert_eq!(player.action_id(), None);
    advance(&mut player, 10);
    assert!(player.action.is_none());
}

/// A clip the model lacks plays its `AnimationData.Fallback`; a chain ending in Stand
/// plays nothing.
#[test]
fn missing_clips_follow_animation_data_fallbacks() {
    let player = human_male_hd();
    let fallbacks = fallbacks();
    // AttackUnarmedOff (117) → AttackOff (87).
    assert_eq!(player.resolve_clip(117, &fallbacks), Some(87));
    assert_eq!(player.resolve_clip(ATTACK_1H, &fallbacks), Some(ATTACK_1H));
    // FireBow (47) falls back to Stand.
    assert_eq!(player.resolve_clip(47, &fallbacks), None);
}

/// A melee swing arriving mid-cast leaves the spell's clip playing; a spell clip
/// replaces a swing.
#[test]
fn spell_clips_are_not_cut_short_by_melee_swings() {
    let mut player = human_male_hd();
    player
        .play_action(READY_SPELL_DIRECTED, true, ActionPriority::Spell)
        .unwrap();
    assert!(
        !player
            .play_action(ATTACK_1H, false, ActionPriority::Combat)
            .unwrap()
    );
    assert_eq!(player.action_id(), Some(READY_SPELL_DIRECTED));
    player.stop_action(READY_SPELL_DIRECTED);
    assert!(
        player
            .play_action(ATTACK_1H, false, ActionPriority::Combat)
            .unwrap()
    );
    assert!(
        player
            .play_action(READY_SPELL_DIRECTED, true, ActionPriority::Spell)
            .unwrap()
    );
    assert_eq!(player.action_id(), Some(READY_SPELL_DIRECTED));
}

/// SpellCastDirected fires HumanMale HD's `$CSL` at 200 ms: a cast started at frame 0
/// holds its missile through frame 11 (183 ms) and has released it by frame 13 (217 ms);
/// frame 12 lands on 200 ms up to float rounding.
/// The precast loop has no release event, so nothing waits on it.
#[test]
fn cast_clip_releases_missiles_at_its_release_event() {
    let mut player = human_male_hd();
    player.update_locomotion(STAND, false, false).unwrap();
    assert!(!player.awaits_missile_release());
    player
        .play_action(READY_SPELL_DIRECTED, true, ActionPriority::Spell)
        .unwrap();
    assert!(!player.awaits_missile_release());
    advance(&mut player, 30);
    player
        .play_action(SPELL_CAST_DIRECTED, false, ActionPriority::Spell)
        .unwrap();
    assert!(player.awaits_missile_release());
    advance(&mut player, 11);
    assert!(player.awaits_missile_release());
    advance(&mut player, 2);
    assert!(!player.awaits_missile_release());
    assert_eq!(player.action_id(), Some(SPELL_CAST_DIRECTED));
}

/// A cast clip stopped before its event no longer holds a missile back.
#[test]
fn stopping_the_cast_clip_before_its_event_stops_awaiting_the_release() {
    let mut player = human_male_hd();
    player.update_locomotion(STAND, false, false).unwrap();
    player
        .play_action(SPELL_CAST_DIRECTED, false, ActionPriority::Spell)
        .unwrap();
    advance(&mut player, 3);
    player.stop_action(SPELL_CAST_DIRECTED);
    assert!(!player.awaits_missile_release());
}

/// SpellCastDirected reports HumanMale HD's `$SCD` (the spell-cast-directed voice) once
/// its clip time passes 200 ms, and only once.
#[test]
fn cast_clip_reports_its_cast_voice_event() {
    let mut player = human_male_hd();
    player.update_locomotion(STAND, false, false).unwrap();
    player
        .play_action(SPELL_CAST_DIRECTED, false, ActionPriority::Spell)
        .unwrap();
    advance(&mut player, 11);
    assert!(player.take_fired_events().is_empty());
    advance(&mut player, 2);
    assert_eq!(player.take_fired_events(), vec![*b"$SCD"]);
    advance(&mut player, 30);
    assert!(player.take_fired_events().is_empty());
}

/// Attack1H reports HumanMale HD's `$CSS` (weapon swoosh) and, 100 ms later, `$CAH`
/// (where the swing lands), each once.
#[test]
fn attack_clip_reports_its_swoosh_then_its_hit() {
    let mut player = human_male_hd();
    player.update_locomotion(STAND, false, false).unwrap();
    player
        .play_action(ATTACK_1H, false, ActionPriority::Combat)
        .unwrap();
    let mut fired = Vec::new();
    for frame in 0..60 {
        advance(&mut player, 1);
        fired.extend(
            player
                .take_fired_events()
                .into_iter()
                .map(|event| (frame, event)),
        );
    }
    let [(swoosh_frame, swoosh), (hit_frame, hit)] = fired[..] else {
        panic!("expected $CSS then $CAH, got {fired:?}");
    };
    assert_eq!((&swoosh, &hit), (b"$CSS", b"$CAH"));
    // 100 ms at 60 fps: the 6th or 7th frame after, by where the clip clock lands.
    assert!((6..=7).contains(&(hit_frame - swoosh_frame)), "{fired:?}");
}

/// A hit reaction arriving mid-swing does not cut the swing short: its `$CSS` and
/// `$CAH` still fire; a swing replaces a playing reaction.
#[test]
fn hit_reactions_do_not_cut_a_swing_short() {
    const COMBAT_WOUND: u16 = 9;
    let mut player = human_male_hd();
    player.update_locomotion(STAND, false, false).unwrap();
    player
        .play_action(ATTACK_1H, false, ActionPriority::Combat)
        .unwrap();
    advance(&mut player, 2);
    assert!(
        !player
            .play_action(COMBAT_WOUND, false, ActionPriority::Reaction)
            .unwrap()
    );
    assert_eq!(player.action_id(), Some(ATTACK_1H));
    advance(&mut player, 40);
    assert_eq!(player.take_fired_events(), vec![*b"$CSS", *b"$CAH"]);
    player
        .play_action(COMBAT_WOUND, false, ActionPriority::Reaction)
        .unwrap();
    assert!(
        player
            .play_action(ATTACK_1H, false, ActionPriority::Combat)
            .unwrap()
    );
    assert_eq!(player.action_id(), Some(ATTACK_1H));
}
