//! Raid target icons (Retail `SetRaidTarget` / `GetRaidTargetIndex`): the server keeps
//! the group's icons, or a solo player's own, and sends `RaidTargetIcons` after every
//! change. The `RAIDTARGET1..8` bindings and the target menu's icon entries toggle an
//! icon on the target (`SetRaidTargetIcon`), `RAIDTARGETNONE` clears it.

use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;

use crate::GameClient;
use crate::frame_error::{FrameError, SessionError};

/// `SetRaidTargetIcon(unit, index)`: the icon `SetRaidTarget` gets, 0 when the unit
/// already carries `index`.
pub(crate) fn raid_target_request(current: Option<u8>, index: u8) -> u8 {
    if current == Some(index) { 0 } else { index }
}

impl GameClient {
    /// `GetRaidTargetIndex(unit)` for a unit's server entity bits.
    pub(super) fn raid_target_of(&self, unit: u64) -> Option<u8> {
        self.account.raid_targets.icon_of(unit)
    }

    /// `SetRaidTargetIcon(unit, index)`, or `SetRaidTarget(unit, 0)` for 0.
    pub(super) fn set_raid_icon(&mut self, unit: u64, index: u8) -> Result<(), SessionError> {
        let icon = match index {
            0 => 0,
            index => raid_target_request(self.raid_target_of(unit), index),
        };
        self.account.send_raid_target(unit, icon)
    }

    /// `RAIDTARGET1..8` / `RAIDTARGETNONE` (Bindings_Standard.xml:1573-1599).
    pub(super) fn update_raid_target_keys(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld
            || self.game_menu_ui.is_some()
            || !self.account.session.gameplay_input_allowed()
        {
            return Ok(());
        }
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        let pressed = InputAction::ALL.into_iter().find_map(|action| {
            let icon = action.raid_target_icon()?;
            self.client_options
                .bindings
                .is_just_pressed(action, &input)
                .then_some(icon)
        });
        match (pressed, self.targeting_target()) {
            (Some(icon), Some(target)) => Ok(self.set_raid_icon(target, icon)?),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SetRaidTargetIcon` (TargetFrame.lua:688-694): the unit's own icon clears it.
    #[test]
    fn assigning_the_units_own_icon_clears_it() {
        assert_eq!(raid_target_request(None, 8), 8);
        assert_eq!(raid_target_request(Some(1), 8), 8);
        assert_eq!(raid_target_request(Some(8), 8), 0);
    }
}
