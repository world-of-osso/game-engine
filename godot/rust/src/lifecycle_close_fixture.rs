//! Debug-only real GameClient regression for template registration without mounted UI.
use crate::{GameClient, frame_error::FrameError};
use godot::prelude::*;

#[godot_api(secondary)]
impl GameClient {
    #[func]
    fn probe_registered_window_closes(&mut self) -> VarDictionary {
        match self.probe_pending_registered_close() {
            Ok(state) => state,
            Err(error) => {
                let mut state = VarDictionary::new();
                state.set("error", error.to_string());
                state
            }
        }
    }

    fn probe_pending_registered_close(&mut self) -> Result<VarDictionary, FrameError> {
        if self.trading_post.ui.is_some() || self.trading_post.view.visible {
            return Err("close fixture requires a fresh unmounted GameClient".into());
        }
        let mut state = VarDictionary::new();
        state.set("empty_handled", self.close_all_windows()?);
        self.trading_post.view.visible = true;
        state.set("pending_before", self.trading_post.view.visible);
        state.set("ui_mounted", self.trading_post.ui.is_some());
        state.set("pending_handled", self.close_all_windows()?);
        state.set("pending_after", self.trading_post.view.visible);
        state.set("repeat_handled", self.close_all_windows()?);
        Ok(state)
    }
}
