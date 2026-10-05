//! Nameplate cast bars as Retail's `NamePlateCastingBarMixin` runs them
//! (Blizzard_NamePlates/Blizzard_NamePlateCastingBar.lua over
//! Blizzard_UIPanels_Game/Shared/CastingBarFrame.lua `CastingBarMixin`).
//!
//! The replicated `CastState` stands in for `UnitCastingInfo` / `UnitChannelInfo` and
//! its appearance for `UNIT_SPELLCAST_START` / `_CHANNEL_START`; `SpellGo` is
//! `UNIT_SPELLCAST_STOP` of a resolved cast and `SpellFailure` (`SMSG_SPELL_FAILURE`) is
//! `UNIT_SPELLCAST_INTERRUPTED` / `_FAILED`. A replicated channel that ends is
//! `UNIT_SPELLCAST_CHANNEL_STOP`. A cast's end always comes as one of the two messages,
//! so a removed cast keeps running until it does. Engine-free: the host draws `CastBar`.

use std::collections::HashMap;

use game_engine_ui_model::casting_bar_frame_component::CastingBarState;
use shared::casting::{CastState, CastType};
use shared::spell_data::CastFailReason;

/// `CastingBarFrameAnimsTemplate` `FadeOutAnim`: alpha 1 → 0 over 0.3 s after 0.2 s.
const FADE_DELAY: f32 = 0.2;
const FADE_SECS: f32 = 0.3;
/// `HoldFadeOutAnim`: alpha 1 for 1.0 s, then 1 → 0 over 0.3 s.
const HOLD_SECS: f32 = 1.0;

/// GlobalStrings `INTERRUPTED`, `FAILED`, `SPELL_INTERRUPTED_BY`.
const INTERRUPTED: &str = "Interrupted";
const FAILED: &str = "Failed";
const INTERRUPTED_BY: &str = "Interrupted: ";

/// `CastingBarType` of a nameplate bar (no trade skills or empowered casts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BarType {
    Standard,
    Channel,
    Uninterruptable,
    Interrupted,
}

/// `CastingBarMixin:ShowSpark` normal/interrupted tint choice; nameplates draw B's soft glow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spark {
    Pip,
    PipRed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Fade {
    None,
    /// Seconds into `FadeOutAnim`.
    Out(f32),
    /// Seconds into `HoldFadeOutAnim`.
    HoldOut(f32),
}

/// What interrupted a cast, for `GetInterruptText`: the interrupter's name and its class
/// colour (`RAID_CLASS_COLORS`), when it has one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Interrupter {
    pub name: String,
    pub color: Option<[f32; 3]>,
}

/// The bar's text: the spell name, or a result with an optionally coloured name.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BarText {
    pub text: String,
    pub name: Option<Interrupter>,
}

/// One unit's cast bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CastBar {
    pub spell_id: u32,
    /// A channel (drains) rather than a cast (fills).
    pub channel: bool,
    pub bar_type: BarType,
    /// `self.casting` / `self.channeling`.
    pub casting: bool,
    pub channeling: bool,
    pub value: f32,
    pub max_value: f32,
    pub text: BarText,
    /// `UpdateIconShown`: the spell icon for interruptible casts, the shield otherwise.
    pub icon_shown: bool,
    pub shield_shown: bool,
    /// `UpdateBarFillTexture(isFull)`.
    pub full: bool,
    pub spark: Option<Spark>,
    fade: Fade,
    /// Last replicated progress, to tell resyncs and recasts apart.
    seen: Option<Seen>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Seen {
    elapsed: f32,
    duration: f32,
    pushbacks: u8,
    interruptible: bool,
}

impl Seen {
    fn of(cast: &CastState) -> Self {
        Self {
            elapsed: cast.elapsed,
            duration: cast.duration,
            pushbacks: cast.pushback_count,
            interruptible: cast.interruptible,
        }
    }
}

impl CastBar {
    /// `HandleCastStart`.
    fn start(cast: &CastState) -> Self {
        let channel = cast.cast_type == CastType::Channel;
        let mut bar = Self {
            spell_id: cast.spell_id,
            channel,
            bar_type: effective_type(channel, cast.interruptible),
            casting: !channel,
            channeling: channel,
            value: 0.0,
            max_value: cast.duration,
            text: BarText {
                text: cast.spell_name.clone(),
                name: None,
            },
            icon_shown: false,
            shield_shown: false,
            full: false,
            spark: Some(Spark::Pip),
            fade: Fade::None,
            seen: None,
        };
        bar.resync(cast);
        bar.update_icon_shown();
        bar
    }

    /// Takes the replicated progress (`UNIT_SPELLCAST_DELAYED` / `_CHANNEL_UPDATE`).
    fn resync(&mut self, cast: &CastState) {
        self.max_value = cast.duration;
        self.value = if self.channeling {
            cast.duration - cast.elapsed
        } else {
            cast.elapsed
        };
        self.seen = Some(Seen::of(cast));
    }

    fn update_icon_shown(&mut self) {
        let interruptible = self.bar_type != BarType::Uninterruptable;
        // `HideIconWhenNotInterruptible`; `showShield` is set for nameplates.
        self.icon_shown = interruptible;
        self.shield_shown = !interruptible;
    }

    /// The alpha `FadeOutAnim` / `HoldFadeOutAnim` leave.
    pub fn alpha(&self) -> f32 {
        match self.fade {
            Fade::None => 1.0,
            Fade::Out(t) => 1.0 - ((t - FADE_DELAY) / FADE_SECS).clamp(0.0, 1.0),
            Fade::HoldOut(t) => 1.0 - ((t - HOLD_SECS) / FADE_SECS).clamp(0.0, 1.0),
        }
    }

    /// Fill fraction of the bar's width.
    pub fn fraction(&self) -> f32 {
        if self.max_value <= 0.0 {
            return 0.0;
        }
        (self.value / self.max_value).clamp(0.0, 1.0)
    }

    fn fading(&self) -> bool {
        !matches!(self.fade, Fade::None)
    }

    /// `FinishSpell`: a cast filled up or a channel ran out.
    fn finish(&mut self) {
        if !self.channeling {
            self.value = self.max_value;
        }
        self.full = true;
        self.spark = None;
        self.fade = Fade::Out(0.0);
        self.casting = false;
        self.channeling = false;
    }

    /// `HandleInterruptOrSpellFailed`.
    fn interrupt(&mut self, text: BarText) {
        self.bar_type = BarType::Interrupted;
        self.full = true;
        self.spark = Some(Spark::PipRed);
        self.text = text;
        self.casting = false;
        self.channeling = false;
        // Nameplate bars play no `InterruptSparkAnim` (`playCastFX` unset), so the bar
        // keeps its value under the red fill while it holds.
        self.fade = Fade::HoldOut(0.0);
    }

    /// `OnUpdate` plus the running fade; `false` once the bar has faded out
    /// (`CastingBarAnim_OnFadeOutFinish` hides it).
    fn advance(&mut self, dt: f32) -> bool {
        if self.casting {
            self.value += dt;
            if self.value >= self.max_value {
                self.finish();
            }
        } else if self.channeling {
            self.value -= dt;
            if self.value <= 0.0 {
                self.finish();
            }
        }
        match &mut self.fade {
            Fade::None => true,
            Fade::Out(t) => {
                *t += dt;
                *t < FADE_DELAY + FADE_SECS
            }
            Fade::HoldOut(t) => {
                *t += dt;
                *t < HOLD_SECS + FADE_SECS
            }
        }
    }
}

/// A HUD bar (`PlayerCastingBarFrame`, `TargetFrameSpellBar`) drawn from `bar`, from
/// its start through the finish or interrupt fade. `CAST_BAR_CAST_TIME` shows only while
/// casting or channeling (`UpdateCastTimeTextShown`, CastingBarFrame.lua:840-850).
pub(crate) fn casting_bar_state(bar: &CastBar, icon_fdid: Option<u32>) -> CastingBarState {
    let timer_text = if bar.casting {
        format!("{:.1}", (bar.max_value - bar.value).max(0.0))
    } else if bar.channeling {
        format!("{:.1}", bar.value.max(0.0))
    } else {
        String::new()
    };
    let spell_name = match &bar.text.name {
        Some(name) => format!("{}{}", bar.text.text, name.name),
        None => bar.text.text.clone(),
    };
    CastingBarState {
        visible: true,
        spell_name,
        icon_fdid,
        timer_text,
        progress: bar.fraction(),
        is_channel: bar.channel,
        is_interruptible: bar.bar_type != BarType::Uninterruptable,
        is_interrupted: bar.bar_type == BarType::Interrupted,
        alpha: bar.alpha(),
    }
}

/// `GetEffectiveType`.
fn effective_type(channel: bool, interruptible: bool) -> BarType {
    if !interruptible {
        BarType::Uninterruptable
    } else if channel {
        BarType::Channel
    } else {
        BarType::Standard
    }
}

/// `GetInterruptText` for `UNIT_SPELLCAST_INTERRUPTED`; `FAILED` for `_FAILED`.
fn failure_text(reason: CastFailReason, interrupter: Option<Interrupter>) -> BarText {
    let interrupted = matches!(
        reason,
        CastFailReason::Interrupted | CastFailReason::InterruptedCombat
    );
    match (interrupted, interrupter) {
        (false, _) => BarText {
            text: FAILED.into(),
            name: None,
        },
        (true, Some(name)) if !name.name.is_empty() => BarText {
            text: INTERRUPTED_BY.into(),
            name: Some(name),
        },
        (true, _) => BarText {
            text: INTERRUPTED.into(),
            name: None,
        },
    }
}

/// Every unit's cast bar.
#[derive(Default)]
pub(crate) struct PlateCasts {
    bars: HashMap<u64, CastBar>,
}

impl PlateCasts {
    pub fn get(&self, unit: u64) -> Option<&CastBar> {
        self.bars.get(&unit)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &CastBar)> {
        self.bars.iter()
    }

    /// The unit's replicated cast this frame.
    pub fn observe(&mut self, unit: u64, cast: Option<&CastState>) {
        let Some(cast) = cast else {
            self.cast_gone(unit);
            return;
        };
        match self.bars.get_mut(&unit) {
            // The cast this bar showed, still replicated after a message or the bar's own
            // clock ended it: no new `UNIT_SPELLCAST_START`.
            Some(bar) if same_cast(bar, cast) && !(bar.casting || bar.channeling) => {}
            Some(bar) if same_cast(bar, cast) => {
                let seen = Seen::of(cast);
                if bar.seen != Some(seen) {
                    if bar
                        .seen
                        .is_some_and(|old| old.interruptible != seen.interruptible)
                    {
                        // `UNIT_SPELLCAST_INTERRUPTIBLE` / `_NOT_INTERRUPTIBLE`.
                        bar.bar_type = effective_type(bar.channeling, seen.interruptible);
                        bar.update_icon_shown();
                    }
                    bar.resync(cast);
                }
            }
            _ => {
                self.bars.insert(unit, CastBar::start(cast));
            }
        }
    }

    /// No replicated cast: a channel's `UNIT_SPELLCAST_CHANNEL_STOP` without an
    /// interrupter finishes it; a cast waits for its `SpellGo` or `SpellFailure`.
    fn cast_gone(&mut self, unit: u64) {
        let Some(bar) = self.bars.get_mut(&unit) else {
            return;
        };
        if bar.channeling {
            bar.finish();
        }
        // Whatever is replicated next is a new cast.
        bar.seen = None;
    }

    /// `SpellGo`: `UNIT_SPELLCAST_STOP` of the resolved cast.
    pub fn spell_go(&mut self, caster: u64, spell_id: u32) {
        if let Some(bar) = self.bars.get_mut(&caster)
            && bar.casting
            && bar.spell_id == spell_id
        {
            bar.finish();
        }
    }

    /// `SpellFailure`: interrupted or failed.
    pub fn spell_failure(
        &mut self,
        caster: u64,
        spell_id: u32,
        reason: CastFailReason,
        interrupter: Option<Interrupter>,
    ) {
        let Some(bar) = self.bars.get_mut(&caster) else {
            return;
        };
        if bar.spell_id != spell_id {
            return;
        }
        if bar.casting && !bar.fading() {
            bar.interrupt(failure_text(reason, interrupter));
        } else if bar.channel && bar.bar_type != BarType::Interrupted && interrupter.is_some() {
            // `UNIT_SPELLCAST_CHANNEL_STOP` with `interruptedBy`: the empowered path,
            // shown even when the channel has already started fading.
            bar.interrupt(failure_text(reason, interrupter));
        }
    }

    /// One frame; faded-out bars and bars of units without plates are dropped.
    pub fn advance(&mut self, dt: f32, units: impl Fn(u64) -> bool) {
        self.bars
            .retain(|unit, bar| units(*unit) && bar.advance(dt));
    }
}

/// The replicated cast is the one the bar showed: same spell and kind, replicated
/// without a gap, and no restart (progress going back without a further pushback).
fn same_cast(bar: &CastBar, cast: &CastState) -> bool {
    let channel = cast.cast_type == CastType::Channel;
    if bar.spell_id != cast.spell_id || channel != bar.channel {
        return false;
    }
    bar.seen.is_some_and(|seen| {
        cast.elapsed >= seen.elapsed - 1e-4 || cast.pushback_count > seen.pushbacks
    })
}

#[cfg(test)]
#[path = "nameplate_casts_tests.rs"]
mod tests;
