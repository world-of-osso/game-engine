//! Replicated creature motion driving authored locomotion clips.
use super::{AnimationState, MIN_MOVEMENT_BLEND_MS};
use crate::world::{creature_locomotion_change, creature_motion_animation_id};
use game_engine_core::m2;
use shared::components::CreatureMotion;
use std::{fs, path::PathBuf};

/// Riverpaw gnoll FDID 3886641 (creature display 384): authored Stand 0, Walk 4, Run 5.
fn riverpaw() -> AnimationState {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let read = |name| fs::read(root.join(name)).expect("authored fixture");
    let model = m2::parse_model(&read("3886641.m2"), &read("388664100.skin")).expect("gnoll");
    AnimationState::new(&model).expect("animated gnoll")
}

fn current_id(player: &AnimationState) -> u16 {
    player.sequences[player.current].id
}

/// One server snapshot: select the motion's clip only when it differs from the applied one.
fn receive(player: &mut AnimationState, applied: &mut Option<u16>, motion: CreatureMotion) -> bool {
    let Some(id) = creature_locomotion_change(*applied, Some(motion)) else {
        return false;
    };
    *applied = Some(id);
    player.update_locomotion(id, false, id == 5).unwrap()
}

#[test]
fn replicated_motion_maps_through_the_shared_direction_selector() {
    assert_eq!(creature_motion_animation_id(CreatureMotion::Still), 0);
    assert_eq!(creature_motion_animation_id(CreatureMotion::Walk), 4);
    assert_eq!(creature_motion_animation_id(CreatureMotion::Run), 5);
    assert_eq!(creature_locomotion_change(None, None), None);
    assert_eq!(
        creature_locomotion_change(Some(4), Some(CreatureMotion::Walk)),
        None
    );
    assert_eq!(
        creature_locomotion_change(Some(4), Some(CreatureMotion::Run)),
        Some(5)
    );
}

#[test]
fn wandering_creature_walks_runs_and_stands_with_continuous_crossfades() {
    let mut player = riverpaw();
    let mut applied = None;
    assert!(!receive(&mut player, &mut applied, CreatureMotion::Still));
    assert_eq!(current_id(&player), 0);
    player.advance(400.0).unwrap();

    for (motion, id) in [
        (CreatureMotion::Walk, 4),
        (CreatureMotion::Run, 5),
        (CreatureMotion::Still, 0),
    ] {
        let before = player.poses();
        assert!(receive(&mut player, &mut applied, motion), "{motion:?}");
        assert_eq!(current_id(&player), id);
        let blend = player.transition.as_ref().unwrap().duration_ms;
        assert!(blend >= MIN_MOVEMENT_BLEND_MS, "{motion:?} blend {blend}");
        let after = player.poses();
        assert!(
            before
                .iter()
                .zip(&after)
                .all(|(a, b)| a.origin.distance_to(b.origin) < 1e-4),
            "{motion:?} crossfade starts from the outgoing pose"
        );
        // Snapshots repeating the motion every 50 ms neither restart the clip nor the fade.
        for step in 1..=2 {
            player.advance(50.0).unwrap();
            assert!(!receive(&mut player, &mut applied, motion));
            assert_eq!(current_id(&player), id);
            assert_eq!(player.time_ms, 50.0 * f64::from(step));
            assert_eq!(
                player.transition.as_ref().unwrap().elapsed_ms,
                50.0 * step as f32
            );
        }
        player.advance(f64::from(blend) - 100.0).unwrap();
        assert!(
            player.transition.is_none(),
            "{motion:?} crossfade completes"
        );
    }
}
