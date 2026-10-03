//! Retail mirror timers (`Blizzard_MirrorTimer/MirrorTimer.lua`): the breath, fatigue and
//! feign-death bars the server starts, pauses and stops (`MIRROR_TIMER_START`, `_PAUSE`,
//! `_STOP`). The server owns the values; the client only counts a running bar down between
//! messages, the way `GetMirrorTimerProgress` does.

/// Retail mirror timer types, in `GetMirrorTimerInfo` index order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MirrorTimerKind {
    Exhaustion,
    Breath,
    FeignDeath,
}

impl MirrorTimerKind {
    /// `MirrorTimerAtlas[timer]` bar texture.
    pub fn atlas(self) -> &'static str {
        match self {
            Self::Exhaustion => "ui-castingbar-filling-standard",
            Self::Breath => "ui-castingbar-filling-applyingcrafting",
            Self::FeignDeath => "ui-castingbar-filling-channel",
        }
    }

    /// `EXHAUSTION_LABEL` (GlobalStrings 12172) and `BREATH_LABEL` (11176); feign death
    /// shows its spell name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Exhaustion => "Fatigue",
            Self::Breath => "Breath",
            Self::FeignDeath => "Feign Death",
        }
    }
}

/// `SMSG_START_MIRROR_TIMER` / `MIRROR_TIMER_START`: `value` of `max_value` milliseconds,
/// changing by `scale` milliseconds per millisecond (-1 counts down) unless `paused`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MirrorTimerStart {
    pub kind: MirrorTimerKind,
    pub value: i32,
    pub max_value: i32,
    pub scale: f32,
    pub paused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MirrorTimer {
    pub kind: MirrorTimerKind,
    /// Current value in milliseconds.
    pub value: f32,
    pub max_value: f32,
    pub scale: f32,
    pub paused: bool,
}

impl MirrorTimer {
    /// The shown `StatusBar` fill, `value / max_value`.
    pub fn fraction(&self) -> f32 {
        if self.max_value <= 0.0 {
            return 0.0;
        }
        (self.value / self.max_value).clamp(0.0, 1.0)
    }
}

/// Retail keeps three timer frames; a timer takes the first free one and keeps it until it
/// stops (`MirrorTimerContainerMixin:GetAvailableTimer`).
pub const MIRROR_TIMER_FRAMES: usize = 3;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MirrorTimersData {
    pub frames: [Option<MirrorTimer>; MIRROR_TIMER_FRAMES],
}

impl MirrorTimersData {
    pub fn start(&mut self, start: MirrorTimerStart) {
        let timer = MirrorTimer {
            kind: start.kind,
            value: start.value as f32,
            max_value: start.max_value as f32,
            scale: start.scale,
            paused: start.paused,
        };
        let slot = self
            .slot_of(start.kind)
            .or_else(|| self.frames.iter().position(Option::is_none));
        if let Some(slot) = slot {
            self.frames[slot] = Some(timer);
        }
    }

    pub fn pause(&mut self, kind: MirrorTimerKind, paused: bool) {
        if let Some(slot) = self.slot_of(kind)
            && let Some(timer) = self.frames[slot].as_mut()
        {
            timer.paused = paused;
        }
    }

    pub fn stop(&mut self, kind: MirrorTimerKind) {
        if let Some(slot) = self.slot_of(kind) {
            self.frames[slot] = None;
        }
    }

    pub fn tick(&mut self, dt: f32) {
        for timer in self.frames.iter_mut().flatten() {
            if !timer.paused {
                timer.value = (timer.value + timer.scale * dt * 1000.0).clamp(0.0, timer.max_value);
            }
        }
    }

    pub fn timer(&self, kind: MirrorTimerKind) -> Option<&MirrorTimer> {
        self.frames
            .iter()
            .flatten()
            .find(|timer| timer.kind == kind)
    }

    fn slot_of(&self, kind: MirrorTimerKind) -> Option<usize> {
        self.frames
            .iter()
            .position(|timer| timer.is_some_and(|timer| timer.kind == kind))
    }
}
