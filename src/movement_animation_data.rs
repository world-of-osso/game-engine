//! Bevy-free original movement-direction animation selection.

use crate::movement_input_data::MoveDirection;

pub(crate) const ANIM_STAND: u16 = 0;
pub(crate) const ANIM_WALK: u16 = 4;
pub(crate) const ANIM_RUN: u16 = 5;
pub(crate) const ANIM_SHUFFLE_LEFT: u16 = 11;
pub(crate) const ANIM_SHUFFLE_RIGHT: u16 = 12;
pub(crate) const ANIM_WALK_BACKWARDS: u16 = 13;
pub(crate) const ANIM_SWIM_IDLE: u16 = 41;
pub(crate) const ANIM_SWIM: u16 = 42;
pub(crate) const ANIM_SWIM_LEFT: u16 = 43;
pub(crate) const ANIM_SWIM_RIGHT: u16 = 44;
pub(crate) const ANIM_SWIM_BACKWARDS: u16 = 45;

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
