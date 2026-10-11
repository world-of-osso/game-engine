//! Retail PlayerCountdown: Timer.lua's bar/medium/large stages and Timer.xml pulses.
use game_engine_core::ui_sound_kits::{
    UI_COUNTDOWN_BAR_STATE_FINISHED, UI_COUNTDOWN_BAR_STATE_STARTS, UI_COUNTDOWN_FINISHED,
    UI_COUNTDOWN_MEDIUM_NUMBER_FINISHED, UI_COUNTDOWN_TIMER,
};
use shared::protocol::GroupCountdownUpdate;

/// Timer.lua's PlayerCountdown mediumMarker and largeMarker.
pub const MEDIUM_MARKER: f32 = 31.0;
pub const LARGE_MARKER: f32 = 11.0;

#[derive(Clone, Debug, PartialEq)]
pub struct CountdownView {
    pub update: GroupCountdownUpdate,
    pub pulse_elapsed: f32,
}

impl CountdownView {
    pub fn new(update: GroupCountdownUpdate) -> Self {
        Self {
            update,
            pulse_elapsed: 0.0,
        }
    }

    pub fn seconds(&self) -> u32 {
        self.update.time_remaining_secs.ceil() as u32
    }

    pub fn bar_visible(&self) -> bool {
        self.update.time_remaining_secs >= MEDIUM_MARKER
    }

    pub fn large(&self) -> bool {
        self.update.time_remaining_secs < LARGE_MARKER
    }

    pub fn start_sounds(&self) -> Vec<u32> {
        if self.bar_visible() {
            return vec![UI_COUNTDOWN_BAR_STATE_STARTS];
        }
        let mut sounds = Vec::new();
        if self.large() {
            sounds.push(UI_COUNTDOWN_MEDIUM_NUMBER_FINISHED);
        }
        sounds.push(UI_COUNTDOWN_TIMER);
        sounds
    }

    pub fn tick(&mut self, delta: f32) -> Vec<u32> {
        let was_bar = self.bar_visible();
        let was_large = self.large();
        let previous = self.seconds();
        self.update.time_remaining_secs = (self.update.time_remaining_secs - delta).max(0.0);
        self.pulse_elapsed += delta;
        if self.seconds() == 0 {
            return vec![UI_COUNTDOWN_FINISHED];
        }
        self.advance_number_sounds(previous, was_bar, was_large)
    }

    fn advance_number_sounds(&mut self, previous: u32, was_bar: bool, was_large: bool) -> Vec<u32> {
        let mut sounds = Vec::new();
        if was_bar && !self.bar_visible() {
            sounds.push(UI_COUNTDOWN_BAR_STATE_FINISHED);
        }
        if !was_large && self.large() {
            sounds.push(UI_COUNTDOWN_MEDIUM_NUMBER_FINISHED);
        }
        if !self.bar_visible() && self.seconds() != previous {
            self.pulse_elapsed = 0.0;
            sounds.push(UI_COUNTDOWN_TIMER);
        }
        sounds
    }
}
