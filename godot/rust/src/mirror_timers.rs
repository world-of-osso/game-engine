//! Retail mirror timer bars (breath, fatigue, feign death) over the world.
//!
//! The server owns the values: its `MirrorTimerStart`, `MirrorTimerPause` and
//! `MirrorTimerStop` (TrinityCore `SMSG_*_MIRROR_TIMER`) start, pause and stop the bars, and
//! the client counts a running bar between messages.

use game_engine_session::SessionScreen;
use game_engine_ui_model::mirror_timer_data::{
    MirrorTimerKind, MirrorTimerStart, MirrorTimersData,
};
use godot::prelude::*;
use shared::protocol::{MirrorTimerPause, MirrorTimerStop};

/// Retail mirror timer index (`GetMirrorTimerInfo`, server `MirrorTimerType`).
pub(crate) fn mirror_timer_kind(index: i64) -> Result<MirrorTimerKind, String> {
    match index {
        0 => Ok(MirrorTimerKind::Exhaustion),
        1 => Ok(MirrorTimerKind::Breath),
        2 => Ok(MirrorTimerKind::FeignDeath),
        other => Err(format!("Unknown mirror timer {other}")),
    }
}

/// A server message on `MirrorTimerChannel`.
pub(crate) enum MirrorTimerMessage {
    Start(shared::protocol::MirrorTimerStart),
    Pause(MirrorTimerPause),
    Stop(MirrorTimerStop),
}

/// Apply one server message to the shown timers (`MIRROR_TIMER_START`, `_PAUSE`, `_STOP`).
pub(crate) fn apply_mirror_timer_message(
    timers: &mut MirrorTimersData,
    message: MirrorTimerMessage,
) -> Result<(), String> {
    match message {
        MirrorTimerMessage::Start(start) => timers.start(MirrorTimerStart {
            kind: mirror_timer_kind(start.timer.into())?,
            value: start.value_ms,
            max_value: start.max_value_ms,
            scale: start.scale,
            paused: start.paused,
        }),
        MirrorTimerMessage::Pause(pause) => {
            timers.pause(mirror_timer_kind(pause.timer.into())?, pause.paused)
        }
        MirrorTimerMessage::Stop(stop) => timers.stop(mirror_timer_kind(stop.timer.into())?),
    }
    Ok(())
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

    /// Apply a server mirror timer message; the bar overlay is created on the first one.
    pub(crate) fn receive_mirror_timer(
        &mut self,
        message: MirrorTimerMessage,
    ) -> Result<(), String> {
        self.attach_mirror_timer_ui()?;
        let ui = self
            .mirror_timer_ui
            .as_mut()
            .expect("mirror timer UI attached");
        let mut applied = Ok(());
        ui.bind_mut().update_mirror_timers(|timers| {
            applied = apply_mirror_timer_message(timers, message);
        })?;
        applied
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

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_ui_model::mirror_timer_data::MirrorTimersData;
    use shared::protocol::{
        MIRROR_TIMER_BREATH, MIRROR_TIMER_FATIGUE, MIRROR_TIMER_FEIGN_DEATH, MirrorTimerPause,
        MirrorTimerStop,
    };

    /// `Player::HandleDrowning`: a fresh 180 s breath timer draining at 1 ms per ms.
    fn breath_start() -> shared::protocol::MirrorTimerStart {
        shared::protocol::MirrorTimerStart {
            timer: MIRROR_TIMER_BREATH,
            value_ms: 180_000,
            max_value_ms: 180_000,
            scale: -1.0,
            paused: false,
            spell_id: 0,
        }
    }

    #[test]
    fn server_breath_start_drains_pauses_refills_and_stops_the_bar() {
        let mut timers = MirrorTimersData::default();
        apply_mirror_timer_message(&mut timers, MirrorTimerMessage::Start(breath_start())).unwrap();
        timers.tick(1.5);
        let breath = timers.timer(MirrorTimerKind::Breath).expect("breath shown");
        assert_eq!(breath.value, 178_500.0);
        assert_eq!(breath.kind.label(), "Breath");

        apply_mirror_timer_message(
            &mut timers,
            MirrorTimerMessage::Pause(MirrorTimerPause {
                timer: MIRROR_TIMER_BREATH,
                paused: true,
            }),
        )
        .unwrap();
        timers.tick(1.0);
        assert_eq!(
            timers.timer(MirrorTimerKind::Breath).unwrap().value,
            178_500.0
        );

        // Surfaced: the server resends it refilling at 10 ms per ms from 42.5 s.
        apply_mirror_timer_message(
            &mut timers,
            MirrorTimerMessage::Start(shared::protocol::MirrorTimerStart {
                value_ms: 42_500,
                scale: 10.0,
                ..breath_start()
            }),
        )
        .unwrap();
        timers.tick(0.25);
        assert_eq!(
            timers.timer(MirrorTimerKind::Breath).unwrap().value,
            45_000.0
        );

        apply_mirror_timer_message(
            &mut timers,
            MirrorTimerMessage::Stop(MirrorTimerStop {
                timer: MIRROR_TIMER_BREATH,
            }),
        )
        .unwrap();
        assert_eq!(timers.timer(MirrorTimerKind::Breath), None);
    }

    #[test]
    fn server_timer_types_select_fatigue_breath_and_feign_death_bars() {
        let mut timers = MirrorTimersData::default();
        for timer in [
            MIRROR_TIMER_FATIGUE,
            MIRROR_TIMER_BREATH,
            MIRROR_TIMER_FEIGN_DEATH,
        ] {
            apply_mirror_timer_message(
                &mut timers,
                MirrorTimerMessage::Start(shared::protocol::MirrorTimerStart {
                    timer,
                    ..breath_start()
                }),
            )
            .unwrap();
        }
        let shown: Vec<_> = timers
            .frames
            .iter()
            .flatten()
            .map(|timer| (timer.kind.label(), timer.kind.atlas()))
            .collect();
        assert_eq!(
            shown,
            [
                ("Fatigue", "ui-castingbar-filling-standard"),
                ("Breath", "ui-castingbar-filling-applyingcrafting"),
                ("Feign Death", "ui-castingbar-filling-channel"),
            ]
        );
        let unknown = apply_mirror_timer_message(
            &mut timers,
            MirrorTimerMessage::Stop(MirrorTimerStop { timer: 3 }),
        );
        assert_eq!(unknown, Err("Unknown mirror timer 3".into()));
    }
}
