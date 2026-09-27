//! Center-screen boss emotes (Retail `RaidWarningFrame`, Blizzard_RaidWarning/RaidWarning.lua):
//! a `RAID_BOSS_EMOTE` fades in over 0.2 s, holds 10 s (`DEFAULT_HOLD_TIME`, the
//! server sends no display time) and fades out over 3 s. At most `MAX_MESSAGES`
//! show (`maxMessageSlots`); a new one evicts the oldest.

use bevy::prelude::*;

pub const MAX_MESSAGES: usize = 4;
pub const FADE_IN_SECS: f32 = 0.2;
pub const HOLD_SECS: f32 = 10.0;
pub const FADE_OUT_SECS: f32 = 3.0;

#[derive(Clone, Debug, PartialEq)]
pub struct RaidWarningLine {
    pub text: String,
    pub color: [f32; 3],
    pub age: f32,
}

impl RaidWarningLine {
    pub fn alpha(&self) -> f32 {
        if self.age < FADE_IN_SECS {
            return self.age / FADE_IN_SECS;
        }
        let fade = (self.age - FADE_IN_SECS - HOLD_SECS) / FADE_OUT_SECS;
        (1.0 - fade.max(0.0)).clamp(0.0, 1.0)
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RaidWarnings {
    /// Oldest first.
    pub lines: Vec<RaidWarningLine>,
}

impl RaidWarnings {
    pub fn add(&mut self, text: impl Into<String>, color: [f32; 3]) {
        self.lines.push(RaidWarningLine {
            text: text.into(),
            color,
            age: 0.0,
        });
        if self.lines.len() > MAX_MESSAGES {
            self.lines.remove(0);
        }
    }

    pub fn tick(&mut self, dt: f32) {
        for line in &mut self.lines {
            line.age += dt;
        }
        self.lines
            .retain(|line| line.age < FADE_IN_SECS + HOLD_SECS + FADE_OUT_SECS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boss_emote_fades_in_holds_ten_seconds_and_fades_out() {
        let mut warnings = RaidWarnings::default();
        warnings.add("Hogger enrages!", [1.0, 0.867, 0.0]);
        warnings.tick(0.1);
        assert!((warnings.lines[0].alpha() - 0.5).abs() < 1e-4);
        warnings.tick(9.0);
        assert_eq!(warnings.lines[0].alpha(), 1.0);
        warnings.tick(2.6);
        assert!((warnings.lines[0].alpha() - 0.5).abs() < 1e-3);
        warnings.tick(1.6);
        assert!(warnings.lines.is_empty());
    }

    #[test]
    fn a_fifth_message_evicts_the_oldest() {
        let mut warnings = RaidWarnings::default();
        for text in ["a", "b", "c", "d", "e"] {
            warnings.add(text, [1.0, 1.0, 1.0]);
        }
        let texts: Vec<&str> = warnings
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(texts, ["b", "c", "d", "e"]);
    }
}
