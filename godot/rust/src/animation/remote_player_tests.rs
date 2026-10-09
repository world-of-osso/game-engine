//! Another player's replicated `PlayerMotion` (Retail `MovementFlags`) driving its model
//! through the local player's selector and jump sequence, on HumanMale HD.
use super::npc_pose_tests::{human_male_hd, pose_distance};
use super::{AnimationState, MIN_MOVEMENT_BLEND_MS};
use crate::world::{Locomotion, player_motion_locomotion, remote_player_locomotion};
use shared::components::PlayerMotion;

#[test]
fn ghoststate_remote_corpse_holds_death_and_ghost_alive_resume_motion() {
    use crate::{death_flow::ghost_transparency, world::death_animation_change};
    use shared::death::DeathState;
    let mut player = human_male_hd();
    let mut applied = false;
    let running = Some(PlayerMotion(FORWARD));
    for life in [
        DeathState::Alive,
        DeathState::Dead,
        DeathState::Ghost,
        DeathState::Alive,
    ] {
        let dead = life == DeathState::Dead;
        if death_animation_change(applied, dead) {
            player.play_death();
        }
        applied = dead;
        for _ in 0..120 {
            player.advance(FRAME_MS).unwrap();
            if let Some(motion) = remote_player_locomotion(true, false, running, Some(life)) {
                player
                    .update_locomotion(motion.animation_id, motion.jumping, motion.running_forward)
                    .unwrap();
            }
        }
        assert_eq!(
            current_id(&player),
            if dead {
                super::ANIM_DEATH
            } else {
                game_engine_core::movement_animation_data::ANIM_RUN
            }
        );
        assert_eq!(
            ghost_transparency(life == DeathState::Ghost),
            if life == DeathState::Ghost { 0.45 } else { 0.0 }
        );
        if dead {
            // Finish the authored Death and (when present) Dead clips before testing hold.
            for _ in 0..2 {
                let finish_ms = f64::from(player.sequences[player.current].duration)
                    + f64::from(MIN_MOVEMENT_BLEND_MS);
                player.advance(finish_ms).unwrap();
            }
            let held = player.poses();
            player.advance(FRAME_MS).unwrap();
            assert_eq!(pose_distance(&held, &player.poses()), 0.0);
            assert!(
                !death_animation_change(applied, dead),
                "repeated corpse state cannot restart death"
            );
            assert!(
                death_animation_change(false, dead),
                "replacement visual starts the current corpse pose"
            );
        }
    }
}

const FORWARD: u64 = PlayerMotion::FORWARD;
const BACKWARD: u64 = PlayerMotion::BACKWARD;
const LEFT: u64 = PlayerMotion::STRAFE_LEFT;
const RIGHT: u64 = PlayerMotion::STRAFE_RIGHT;
const WALK: u64 = PlayerMotion::WALKING;
const FALLING: u64 = PlayerMotion::FALLING;
const JUMPING: u64 = PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED;
const SWIM: u64 = PlayerMotion::SWIMMING;

/// One 60 fps client frame.
const FRAME_MS: f64 = 1000.0 / 60.0;

fn locomotion(animation_id: u16, jumping: bool, running_forward: bool) -> Locomotion {
    Locomotion {
        animation_id,
        jumping,
        running_forward,
    }
}

fn current_id(player: &AnimationState) -> u16 {
    player.sequences[player.current].id
}

/// Drive the model from the newest flags, as `WorldUnits` does after each clock advance.
fn drive(player: &mut AnimationState, flags: u64) -> bool {
    let Locomotion {
        animation_id,
        jumping,
        running_forward,
    } = player_motion_locomotion(PlayerMotion(flags));
    player
        .update_locomotion(animation_id, jumping, running_forward)
        .unwrap()
}

/// One client frame: advance the clock, then drive the model.
fn frame(player: &mut AnimationState, flags: u64) -> bool {
    player.advance(FRAME_MS).unwrap();
    drive(player, flags)
}

#[test]
fn flags_select_the_local_players_animation_ids() {
    for (flags, expected) in [
        (0, locomotion(0, false, false)),
        (WALK, locomotion(0, false, false)),
        (FORWARD, locomotion(5, false, true)),
        (FORWARD | WALK, locomotion(4, false, false)),
        (BACKWARD, locomotion(13, false, false)),
        (LEFT, locomotion(11, false, false)),
        (RIGHT, locomotion(12, false, false)),
        // Forward outranks a strafe, as the local `compute_movement_input` does.
        (FORWARD | LEFT, locomotion(5, false, true)),
        (SWIM, locomotion(41, false, false)),
        // Running forward, as the local `update_player_animation` passes it while swimming.
        (SWIM | FORWARD, locomotion(42, false, true)),
        (SWIM | LEFT, locomotion(43, false, false)),
        (SWIM | RIGHT, locomotion(44, false, false)),
        (SWIM | BACKWARD, locomotion(45, false, false)),
        (FALLING, locomotion(40, false, false)),
        (FORWARD | FALLING, locomotion(40, false, true)),
        (SWIM | FALLING, locomotion(41, false, false)),
        (JUMPING, locomotion(0, true, false)),
        (FORWARD | JUMPING, locomotion(5, true, true)),
        (FORWARD | WALK | JUMPING, locomotion(4, true, false)),
    ] {
        assert_eq!(
            player_motion_locomotion(PlayerMotion(flags)),
            expected,
            "flags {flags:#x}"
        );
    }
}

/// Only other players follow their replicated flags; the local player keeps its own
/// predicted movement, and creatures carry no `PlayerMotion`.
#[test]
fn only_remote_players_follow_replicated_motion() {
    let running = Some(PlayerMotion(FORWARD));
    assert_eq!(
        remote_player_locomotion(true, false, running, None),
        Some(locomotion(5, false, true))
    );
    assert_eq!(remote_player_locomotion(true, true, running, None), None);
    assert_eq!(remote_player_locomotion(false, false, running, None), None);
    assert_eq!(remote_player_locomotion(true, false, None, None), None);
}

#[test]
fn remote_player_walks_runs_strafes_backpedals_and_stands_with_continuous_crossfades() {
    let mut player = human_male_hd();
    assert!(!frame(&mut player, 0));
    assert_eq!(current_id(&player), 0);
    for (flags, id) in [
        (FORWARD | WALK, 4),
        (FORWARD, 5),
        (LEFT, 11),
        (RIGHT, 12),
        (BACKWARD, 13),
        (0, 0),
    ] {
        player.advance(FRAME_MS).unwrap();
        let before = player.poses();
        assert!(drive(&mut player, flags), "{flags:#x}");
        assert_eq!(current_id(&player), id);
        let blend = player.transition.as_ref().unwrap().duration_ms;
        assert!(blend >= MIN_MOVEMENT_BLEND_MS, "{flags:#x} blend {blend}");
        assert!(
            pose_distance(&before, &player.poses()) < 1e-4,
            "{flags:#x} crossfade starts from the outgoing pose"
        );
        // The same flags on every later frame neither restart the clip nor its fade.
        for step in 1..=5 {
            assert!(!frame(&mut player, flags), "{flags:#x} frame {step}");
            assert_eq!(current_id(&player), id);
            assert!((player.time_ms - FRAME_MS * f64::from(step)).abs() < 1e-6);
            let elapsed = player.transition.as_ref().unwrap().elapsed_ms;
            assert!((f64::from(elapsed) - FRAME_MS * f64::from(step)).abs() < 1e-3);
        }
        player.advance(f64::from(blend)).unwrap();
        assert!(
            player.transition.is_none(),
            "{flags:#x} crossfade completes"
        );
    }
}

#[test]
fn locomotion_remote_water_entry_preempts_air_and_swims_each_direction() {
    let mut player = human_male_hd();
    drive(&mut player, JUMPING);
    player.advance(75.0).unwrap();
    for (flags, expected) in [
        (SWIM | FORWARD, 42),
        (SWIM, 41),
        (SWIM | BACKWARD, 45),
        (SWIM | LEFT, 43),
        (SWIM | RIGHT, 44),
        (FORWARD, 5),
        (0, 0),
    ] {
        let before = player.poses();
        assert!(drive(&mut player, flags));
        assert_eq!(current_id(&player), expected);
        assert!(pose_distance(&before, &player.poses()) < 1e-4);
        let blend = player.transition.as_ref().unwrap().duration_ms;
        assert!(blend >= MIN_MOVEMENT_BLEND_MS);
        for _ in 0..3 {
            assert!(!frame(&mut player, flags));
        }
        assert!((player.transition.as_ref().unwrap().elapsed_ms - 50.0).abs() < 1e-3);
    }
    player.advance(500.0).unwrap();
    assert!(player.transition.is_none());
}

#[test]
fn remote_idle_turn_uses_replicated_facing_with_continuous_crossfades() {
    use crate::world::remote_player_facing_locomotion;
    let mut player = human_male_hd();
    let mut previous = None;
    let pi = std::f32::consts::PI;
    for (flags, facing, flying, expected) in [
        (0, 0.0, false, 0),
        (0, 0.12, false, 11),
        (0, 0.0, false, 12),
        (0, 0.0, false, 0),
        (FORWARD, pi - 0.01, false, 5),
        (0, -pi + 0.02, false, 11),
        (0, pi - 0.01, false, 12),
        (SWIM, 0.0, false, 41),
        (0, 0.12, true, 0),
        (FORWARD, 0.24, false, 5),
        (0, 0.24, false, 0),
    ] {
        player.advance(75.0).unwrap();
        let selected = remote_player_facing_locomotion(
            player_motion_locomotion(PlayerMotion(flags)),
            previous,
            facing,
            flying,
        );
        previous = Some(facing);
        assert_eq!(selected.animation_id, expected);
        let before = player.poses();
        let changed = player
            .update_locomotion(
                selected.animation_id,
                selected.jumping,
                selected.running_forward,
            )
            .unwrap();
        assert_eq!(current_id(&player), expected);
        assert!(pose_distance(&before, &player.poses()) < 1e-4);
        if changed {
            assert!(player.transition.as_ref().unwrap().duration_ms >= MIN_MOVEMENT_BLEND_MS);
        }
    }
    player.advance(500.0).unwrap();
    assert!(player.transition.is_none());
}

#[test]
fn remote_player_airborne_walkoff_falls_and_lands_mid_crossfade() {
    let mut player = human_male_hd();
    drive(&mut player, FORWARD);
    player.advance(400.0).unwrap();
    let before = player.poses();
    assert!(drive(&mut player, FORWARD | FALLING));
    assert_eq!(current_id(&player), 40);
    assert!(player.transition.as_ref().unwrap().duration_ms >= MIN_MOVEMENT_BLEND_MS);
    assert!(pose_distance(&before, &player.poses()) < 1e-4);
    for _ in 0..3 {
        assert!(!frame(&mut player, FORWARD | FALLING));
        assert_eq!(current_id(&player), 40);
    }
    // Touchdown interrupts the Fall fade, preserving the currently blended pose.
    let before = player.poses();
    assert!(drive(&mut player, FORWARD));
    assert_eq!(current_id(&player), 5);
    assert!(pose_distance(&before, &player.poses()) < 1e-4);
    assert!(player.transition.as_ref().unwrap().duration_ms >= MIN_MOVEMENT_BLEND_MS);
    for _ in 0..30 {
        assert!(!frame(&mut player, FORWARD));
        assert_eq!(current_id(&player), 5);
    }
    assert!(player.transition.is_none());
}

/// A running jump: JumpStart 37 plays out, Jump 38 loops while `FALLING` stays set, the
/// landing (running JumpLandRun 187 when authored, else JumpEnd 39) follows its clear,
/// then Run 5 resumes and Stand 0 follows the stop, every switch crossfaded.
#[test]
fn remote_jump_starts_loops_and_lands_from_the_falling_flag() {
    let mut player = human_male_hd();
    frame(&mut player, FORWARD);
    player.advance(400.0).unwrap();
    let mut seen = vec![current_id(&player)];
    let mut record = |player: &mut AnimationState, flags: u64, frames: usize| {
        for _ in 0..frames {
            player.advance(FRAME_MS).unwrap();
            let before = player.poses();
            if drive(player, flags) {
                seen.push(current_id(player));
                assert!(pose_distance(&before, &player.poses()) < 1e-4, "{seen:?}");
                let blend = player.transition.as_ref().unwrap().duration_ms;
                assert!(blend >= MIN_MOVEMENT_BLEND_MS, "{seen:?} blend {blend}");
            }
        }
    };
    record(&mut player, FORWARD | JUMPING, 120);
    record(&mut player, FORWARD, 120);
    record(&mut player, 0, 30);
    let landing = if player.sequences.iter().any(|s| s.id == 187) {
        187
    } else {
        39
    };
    assert_eq!(seen, vec![5, 37, 38, landing, 5, 0]);
}
