//! Owner-only watched reputation HUD, independent of Journeys reward-catalog readiness.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::reputation_status_bar::ReputationBarState;
use godot::prelude::*;
use shared::protocol_snapshots::WatchedReputationSnapshot;

impl GameClient {
    pub(super) fn update_reputation_bar(&mut self) -> Result<(), FrameError> {
        let in_world = self.account.session.screen == SessionScreen::InWorld;
        let watched = self
            .reputation
            .as_ref()
            .filter(|_| in_world)
            .and_then(|snapshot| snapshot.watched.clone());
        let Some(watched) = watched else {
            if let Some(ui) = self.reputation_bar.take() {
                ui.free();
            }
            return Ok(());
        };
        let hovered = self
            .hovered_ui_frame()?
            .is_some_and(|hit| self.reputation_bar.as_ref() == Some(&hit.ui));
        let xp_visible = self.account.xp.is_some_and(|xp| xp.next_level_xp > 0);
        self.sync_reputation_bar(bar_state(watched, xp_visible, hovered))?;
        Ok(())
    }

    fn sync_reputation_bar(&mut self, state: ReputationBarState) -> Result<(), String> {
        if let Some(ui) = self.reputation_bar.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("ReputationBarUI");
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(self.effective_ui_scale())?;
        let shown = ui.bind_mut().show_reputation_status_bar(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.reputation_bar = Some(ui);
        Ok(())
    }
}

fn bar_state(
    watched: WatchedReputationSnapshot,
    xp_visible: bool,
    hovered: bool,
) -> ReputationBarState {
    ReputationBarState {
        watched: Some(watched),
        xp_visible,
        hovered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reputation_bar_state_preserves_authoritative_values_and_xp_priority() {
        let watched = WatchedReputationSnapshot {
            faction_id: 2503,
            name: "Maruuk Centaur".into(),
            value: 1000,
            maximum: 2500,
            capped: false,
            is_major_faction: true,
            reaction: shared::reputation::Standing::Neutral,
        };
        let state = bar_state(watched.clone(), true, true);
        assert_eq!(state.watched, Some(watched));
        assert!(state.xp_visible && state.hovered);
    }
}
