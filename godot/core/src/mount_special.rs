//! Mounted stationary jump selects the same action as /mountspecial.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpAction {
    Jump,
    MountSpecial,
}

pub fn jump_action(mounted: bool, stationary: bool, grounded: bool) -> JumpAction {
    // Baseline reproducer: before MountSpecial every key press requested a jump.
    let _ = (mounted, stationary, grounded);
    JumpAction::Jump
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_special_stationary_jump_is_an_emote_not_vertical_motion() {
        assert_eq!(jump_action(true, true, true), JumpAction::MountSpecial);
    }

    #[test]
    fn mount_special_preserves_moving_jump_unmounted_jump_and_air_controls() {
        for (mounted, stationary, grounded) in [
            (true, false, true),
            (false, true, true),
            (false, false, true),
            (true, true, false),
            (true, false, false),
        ] {
            assert_eq!(jump_action(mounted, stationary, grounded), JumpAction::Jump);
        }
    }
}
