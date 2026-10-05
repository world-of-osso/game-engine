//! In-world experience bar (docs/specs/xp-bar.md) over the shared `ExperienceBar` screen:
//! the newest owner-only `PlayerXpUpdate`, with the XP text while the pointer is over it.

use game_engine_session::SessionScreen;
use game_engine_ui_model::game_tooltip::hud::xp_tooltip;
use game_engine_ui_model::xp_bar_component::XpBarState;
use godot::prelude::*;
use shared::protocol::PlayerXpUpdate;

use crate::tooltips::{HoveredFrame, HoveredTooltip};
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};

#[derive(Default)]
pub(crate) struct XpBarHud {
    ui: Option<Gd<RegistryUi>>,
}

impl XpBarHud {
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }

    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        match self.ui.as_mut() {
            Some(ui) => visit(ui),
            None => Ok(()),
        }
    }
}

fn xp_bar_state(update: PlayerXpUpdate, hovered: bool) -> XpBarState {
    XpBarState {
        xp: update.xp,
        next_level_xp: update.next_level_xp,
        rested_xp: update.rested_xp,
        hovered,
    }
}

impl GameClient {
    /// No bar outside the world or before the server's first `PlayerXpUpdate`.
    pub(super) fn update_xp_bar(&mut self) -> Result<(), FrameError> {
        let in_world = self.account.session.screen == SessionScreen::InWorld;
        let Some(update) = self.account.xp.filter(|_| in_world) else {
            self.xp_bar.free_ui();
            return Ok(());
        };
        let hovered = self
            .hovered_ui_frame()?
            .is_some_and(|hit| self.xp_bar.ui.as_ref() == Some(&hit.ui));
        Ok(self.sync_xp_bar(xp_bar_state(update, hovered))?)
    }

    /// The exhaustion tooltip while the pointer is anywhere over the bar.
    pub(crate) fn xp_bar_tooltip(&mut self, hit: &HoveredFrame) -> Option<HoveredTooltip> {
        if self.xp_bar.ui.as_ref() != Some(&hit.ui) {
            return None;
        }
        let update = self.account.xp?;
        Some(HoveredTooltip::text(xp_tooltip(
            update.xp,
            update.next_level_xp,
            update.rested_xp,
        )))
    }

    fn sync_xp_bar(&mut self, state: XpBarState) -> Result<(), String> {
        if let Some(ui) = self.xp_bar.ui.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("ExperienceBarUI");
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(self.effective_ui_scale())?;
        let shown = ui.bind_mut().show_xp_bar(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.xp_bar.ui = Some(ui);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xp_bar_state_carries_the_owner_update_and_hover() {
        let update = PlayerXpUpdate {
            xp: 250,
            next_level_xp: 1000,
            rested_xp: 400,
        };
        assert_eq!(
            xp_bar_state(update, true),
            XpBarState {
                xp: 250,
                next_level_xp: 1000,
                rested_xp: 400,
                hovered: true,
            }
        );
        // The server's level-cap update keeps its zero requirement, which hides the bar.
        let capped = PlayerXpUpdate {
            xp: 0,
            next_level_xp: 0,
            rested_xp: 0,
        };
        assert_eq!(xp_bar_state(capped, false).next_level_xp, 0);
    }
}
