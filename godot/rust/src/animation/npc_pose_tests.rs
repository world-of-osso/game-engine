//! Replicated creature poses (`UnitPose`) held while still, under the locomotion of
//! replicated motion, on the Stockade's HumanMale HD model.
use super::{AnimationState, MIN_MOVEMENT_BLEND_MS};
use crate::world::creature_animation_change;
use game_engine_core::m2;
use shared::components::CreatureMotion;
use std::{fs, path::PathBuf, sync::OnceLock};

/// HumanMale HD 1011653 (Stockade Guard display 2989, Petty Criminal display 35069) with
/// its external `.anim` sequences (Sit 97, Sleep 100).
pub(super) fn human_male_model() -> &'static m2::Model {
    static MODEL: OnceLock<m2::Model> = OnceLock::new();
    MODEL.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        let read = |name: &str| fs::read(root.join(name)).expect("authored fixture");
        m2::parse_model_with_skeleton(
            &read("1011653.m2"),
            &read("101165300.skin"),
            Some(&read("1011653.skel")),
            |fdid| fs::read(root.join(format!("{fdid}.anim"))).ok(),
        )
        .expect("HumanMale HD")
    })
}

pub(super) fn human_male_hd() -> AnimationState {
    AnimationState::new(human_male_model()).expect("animated HumanMale HD")
}

fn current_id(player: &AnimationState) -> u16 {
    player.sequences[player.current].id
}

/// One server snapshot: select its clip only when it differs from the applied one.
fn receive(
    player: &mut AnimationState,
    applied: &mut Option<u16>,
    motion: CreatureMotion,
    pose_anim: Option<u16>,
) -> bool {
    let Some(id) = creature_animation_change(*applied, Some(motion), pose_anim) else {
        return false;
    };
    *applied = Some(id);
    player.update_locomotion(id, false, id == 5).unwrap()
}

/// Largest bone origin distance between two sampled poses.
pub(super) fn pose_distance(
    a: &[godot::builtin::Transform3D],
    b: &[godot::builtin::Transform3D],
) -> f32 {
    a.iter()
        .zip(b)
        .map(|(a, b)| a.origin.distance_to(b.origin) + (a.basis.col_a() - b.basis.col_a()).length())
        .fold(0.0, f32::max)
}

/// A Stockade Guard holds Ready1H (26, emote 333) while still, walks (4) when the server
/// moves it and crossfades back into Ready1H when it stops, without a pose snap.
#[test]
fn guard_holds_ready1h_still_and_walks_when_moving() {
    let mut player = human_male_hd();
    let mut applied = None;
    let ready = Some(26);
    assert!(receive(
        &mut player,
        &mut applied,
        CreatureMotion::Still,
        ready
    ));
    assert_eq!(current_id(&player), 26);
    player.advance(1000.0).unwrap();
    assert!(!receive(
        &mut player,
        &mut applied,
        CreatureMotion::Still,
        ready
    ));

    for (motion, id) in [(CreatureMotion::Walk, 4), (CreatureMotion::Still, 26)] {
        let before = player.poses();
        assert!(
            receive(&mut player, &mut applied, motion, ready),
            "{motion:?}"
        );
        assert_eq!(current_id(&player), id);
        let blend = player.transition.as_ref().unwrap().duration_ms;
        assert!(blend >= MIN_MOVEMENT_BLEND_MS, "{motion:?} blend {blend}");
        assert!(
            pose_distance(&before, &player.poses()) < 1e-4,
            "{motion:?} crossfade starts from the outgoing pose"
        );
        player.advance(f64::from(blend) + 50.0).unwrap();
        assert!(player.transition.is_none());
    }
}

/// A sleeping (100) or sitting (97) Petty Criminal lies or sits: its keyframes come from
/// the external `.anim` files and move the body well away from Stand.
#[test]
fn criminal_sleep_and_sit_poses_leave_the_stand_pose() {
    let mut stand = human_male_hd();
    stand.advance(500.0).unwrap();
    let standing = stand.poses();
    for anim in [100, 97] {
        let mut player = human_male_hd();
        let mut applied = None;
        assert!(receive(
            &mut player,
            &mut applied,
            CreatureMotion::Still,
            Some(anim)
        ));
        assert_eq!(current_id(&player), anim);
        player.advance(500.0).unwrap();
        assert!(player.transition.is_none());
        let distance = pose_distance(&standing, &player.poses());
        assert!(distance > 0.3, "{anim}: pose moved only {distance}");
    }
}

/// A creature spawned in the Dead stand state (`UnitPose` → Dead 6) on HumanMale HD, which
/// has no Dead clip: `AnimationData.Fallback` 6 → 1 plays Death once and lies at its end
/// instead of failing with "M2 animation ID 6 has no base variation".
#[test]
fn a_corpse_without_a_dead_clip_lies_at_the_end_of_death() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
    let fallbacks = game_engine_core::spell_visual::read_animation_fallbacks(&root).unwrap();
    let mut player = human_male_hd();
    let err = player.update_locomotion(6, false, false).unwrap_err();
    assert_eq!(err, "M2 animation ID 6 has no base variation");

    let pose = player.resolve_clip(6, &fallbacks);
    assert_eq!(pose, Some(super::ANIM_DEATH));
    let mut applied = None;
    assert!(receive(
        &mut player,
        &mut applied,
        CreatureMotion::Still,
        pose
    ));
    assert_eq!(current_id(&player), super::ANIM_DEATH);
    let duration = f64::from(player.sequences[player.current].duration);
    player.advance(duration + 2000.0).unwrap();
    assert_eq!(current_id(&player), super::ANIM_DEATH);
    assert_eq!(player.time_ms, duration, "holds the last Death frame");
    // The next unchanged snapshot keeps it lying.
    assert!(!receive(
        &mut player,
        &mut applied,
        CreatureMotion::Still,
        pose
    ));
    assert_eq!(player.time_ms, duration);
}
