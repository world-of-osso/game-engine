//! Bevy-free original movement-direction animation selection.

use crate::movement_input_data::MoveDirection;

pub const ANIM_STAND: u16 = 0;
pub const ANIM_WALK: u16 = 4;
pub const ANIM_RUN: u16 = 5;
pub const ANIM_SHUFFLE_LEFT: u16 = 11;
pub const ANIM_SHUFFLE_RIGHT: u16 = 12;
pub const ANIM_WALK_BACKWARDS: u16 = 13;
pub const ANIM_SWIM_IDLE: u16 = 41;
pub const ANIM_SWIM: u16 = 42;
pub const ANIM_SWIM_LEFT: u16 = 43;
pub const ANIM_SWIM_RIGHT: u16 = 44;
pub const ANIM_SWIM_BACKWARDS: u16 = 45;

/// Map movement direction to the original WoW animation ID.
pub fn direction_to_anim_id(dir: MoveDirection, running: bool, swimming: bool) -> u16 {
    if swimming {
        return match dir {
            MoveDirection::None => ANIM_SWIM_IDLE,
            MoveDirection::Forward => ANIM_SWIM,
            MoveDirection::Backward => ANIM_SWIM_BACKWARDS,
            MoveDirection::Left => ANIM_SWIM_LEFT,
            MoveDirection::Right => ANIM_SWIM_RIGHT,
        };
    }
    match dir {
        MoveDirection::None => ANIM_STAND,
        MoveDirection::Forward => {
            if running {
                ANIM_RUN
            } else {
                ANIM_WALK
            }
        }
        MoveDirection::Backward => ANIM_WALK_BACKWARDS,
        MoveDirection::Left => ANIM_SHUFFLE_LEFT,
        MoveDirection::Right => ANIM_SHUFFLE_RIGHT,
    }
}

/// Playback rate of a movement clip authored for `movespeed` yd/s (M2 sequence
/// `movespeed`) on a unit moving at `speed` yd/s, so its feet keep pace with the ground:
/// a Run authored at 7 yd/s plays at 3/7 on a unit slowed to 3 yd/s. A clip without a
/// movespeed (Stand, emotes) or a unit with no known speed plays at 1.
pub fn locomotion_playback_rate(movespeed: f32, speed: Option<f32>) -> f32 {
    match speed {
        Some(speed) if movespeed > 0.0 && speed > 0.0 => speed / movespeed,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_clips_play_at_the_units_speed_over_their_authored_speed() {
        // Orc male HD (949470.skel) Run: movespeed 7.0; Blackrock Spy chilled 3, free 6.
        assert_eq!(locomotion_playback_rate(7.0, Some(3.0)), 3.0 / 7.0);
        assert_eq!(locomotion_playback_rate(7.0, Some(6.0)), 6.0 / 7.0);
        // Sheep (1377131.m2) Walk: movespeed 1.1111; wandering at 2.5 yd/s.
        assert!((locomotion_playback_rate(1.111_111_2, Some(2.5)) - 2.25).abs() < 1e-5);
    }

    #[test]
    fn clips_without_a_movespeed_or_units_without_a_speed_play_at_one() {
        assert_eq!(locomotion_playback_rate(0.0, Some(6.0)), 1.0);
        assert_eq!(locomotion_playback_rate(7.0, None), 1.0);
        assert_eq!(locomotion_playback_rate(7.0, Some(0.0)), 1.0);
    }
}
