//! Retail mirror timer bars (breath, fatigue, feign death) over the world.
//!
//! The server sends no mirror timer messages yet; `MirrorTimerStart` is the client form of
//! retail `SMSG_START_MIRROR_TIMER`, and the host applies whatever starts, pauses and stops
//! it is given.

use game_engine_session::SessionScreen;
use game_engine_ui_model::mirror_timer_data::{MirrorTimerKind, MirrorTimerStart};
use godot::prelude::*;

/// Retail mirror timer index (`GetMirrorTimerInfo`, server `MirrorTimerType`).
pub(crate) fn mirror_timer_kind(index: i64) -> Result<MirrorTimerKind, String> {
    match index {
        0 => Ok(MirrorTimerKind::Exhaustion),
        1 => Ok(MirrorTimerKind::Breath),
        2 => Ok(MirrorTimerKind::FeignDeath),
        other => Err(format!("Unknown mirror timer {other}")),
    }
}

impl crate::GameClient {
    fn attach_mirror_timer_ui(&mut self) -> Result<(), String> {
        if self.mirror_timer_ui.is_some() {
            return Ok(());
        }
        let mut ui = crate::ui::RegistryUi::new_alloc();
        ui.set_name("MirrorTimers");
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_mirror_timers();
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        ui.set_visible(self.account.session.screen == SessionScreen::InWorld);
        self.mirror_timer_ui = Some(ui);
        Ok(())
    }

    pub(crate) fn start_mirror(&mut self, start: MirrorTimerStart) -> Result<(), String> {
        self.attach_mirror_timer_ui()?;
        let ui = self
            .mirror_timer_ui
            .as_mut()
            .expect("mirror timer UI attached");
        ui.bind_mut()
            .update_mirror_timers(|timers| timers.start(start))
    }

    pub(crate) fn stop_mirror(&mut self, kind: MirrorTimerKind) -> Result<(), String> {
        match self.mirror_timer_ui.as_mut() {
            Some(ui) => ui
                .bind_mut()
                .update_mirror_timers(|timers| timers.stop(kind)),
            None => Ok(()),
        }
    }

    /// Count running bars down while in the world; leaving it clears them.
    pub(crate) fn update_mirror_timers(&mut self, delta: f32) -> Result<(), String> {
        let in_world = self.account.session.screen == SessionScreen::InWorld;
        let Some(ui) = self.mirror_timer_ui.as_mut() else {
            return Ok(());
        };
        ui.set_visible(in_world);
        if in_world {
            ui.bind_mut()
                .update_mirror_timers(|timers| timers.tick(delta))
        } else {
            ui.bind_mut()
                .update_mirror_timers(|timers| *timers = Default::default())
        }
    }

    /// The shown fraction of `kind`'s bar, or None while it is not running.
    pub(crate) fn mirror_fraction(&self, kind: MirrorTimerKind) -> Option<f32> {
        let ui = self.mirror_timer_ui.as_ref()?;
        ui.bind().mirror_timer_fraction(kind)
    }
}
