//! The local player's XP bar numbers from the server's owner-only `PlayerXpUpdate`
//! (Retail `UnitXP`, `UnitXPMax`, `GetXPExhaustion`), and the Retail texts built from them.

use bevy::prelude::*;
use shared::protocol::{LogXpGain, PlayerXpUpdate, XpGainReason};

/// Latest `PlayerXpUpdate`; `None` until the server sends one on enter world.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ExperienceState(pub Option<PlayerXpUpdate>);

impl ExperienceState {
    /// XP numbers while the player can still level. The server sends `next_level_xp` 0 at
    /// the level cap, where Retail hides the bar (`GameRulesUtil.CanShowExperienceBar`).
    pub fn leveling(&self) -> Option<PlayerXpUpdate> {
        self.0.filter(|update| update.next_level_xp > 0)
    }
}

/// `UnitXP / UnitXPMax`, the status bar value.
pub fn fill_fraction(update: &PlayerXpUpdate) -> f32 {
    (update.xp as f32 / update.next_level_xp as f32).clamp(0.0, 1.0)
}

/// `GetRestState() == 1` (Rested): the player has a rested pool.
pub fn is_rested(update: &PlayerXpUpdate) -> bool {
    update.rested_xp > 0
}

/// `ExhaustionTickMixin:UpdateTickPosition` `widthRatio`: where rested XP ends as a fraction
/// of the level, unclamped (above 1 when the pool reaches past the level). `None` without
/// a rested pool.
pub fn rested_end_fraction(update: &PlayerXpUpdate) -> Option<f32> {
    is_rested(update)
        .then(|| (update.xp as f32 + update.rested_xp as f32) / update.next_level_xp as f32)
}

/// `XP_STATUS_BAR_TEXT` ("XP: %d/%d"), the bar text shown on hover.
pub fn bar_text(update: &PlayerXpUpdate) -> String {
    format!("XP: {}/{}", update.xp, update.next_level_xp)
}

/// `XP_TEXT` ("%s / %s  ( %d%% )") with `BreakUpLargeNumbers` and `math.ceil` percent,
/// the tooltip title (`ExhaustionTickMixin:ExhaustionToolTipText`).
pub fn tooltip_title(update: &PlayerXpUpdate) -> String {
    let percent = (update.xp as f64 * 100.0 / update.next_level_xp as f64).ceil();
    format!(
        "{} / {}  ( {percent}% )",
        break_up_large_number(update.xp),
        break_up_large_number(update.next_level_xp)
    )
}

/// `GetRestState()` name and multiplier percent, as `EXHAUST_TOOLTIP1` shows them
/// ("%s" then "%d%% of normal experience gained from monsters.").
pub fn rest_state(update: &PlayerXpUpdate) -> (&'static str, u32) {
    if is_rested(update) {
        ("Rested", 200)
    } else {
        ("Normal", 100)
    }
}

/// The chat line for one gain (Retail `CHAT_MSG_COMBAT_XP_GAIN` strings). Quest XP is
/// already announced by the quest complete notice (`ERR_QUEST_REWARD_EXP_I`), so it has none.
pub fn gain_chat_line(gain: &LogXpGain, victim: Option<&str>) -> Option<String> {
    let bonus = gain.original.saturating_sub(gain.amount);
    match (gain.reason, victim) {
        (XpGainReason::Quest, _) => None,
        // COMBATLOG_XPGAIN_EXHAUSTION1
        (XpGainReason::Kill, Some(name)) if bonus > 0 => Some(format!(
            "{name} dies, you gain {} experience. (+{bonus} exp Rested bonus)",
            gain.original
        )),
        // COMBATLOG_XPGAIN_FIRSTPERSON
        (XpGainReason::Kill, Some(name)) => Some(format!(
            "{name} dies, you gain {} experience.",
            gain.original
        )),
        // COMBATLOG_XPGAIN_FIRSTPERSON_UNNAMED
        _ => Some(format!("You gain {} experience.", gain.original)),
    }
}

/// `BreakUpLargeNumbers`: thousands separated by commas.
fn break_up_large_number(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(xp: u32, next_level_xp: u32, rested_xp: u32) -> PlayerXpUpdate {
        PlayerXpUpdate {
            xp,
            next_level_xp,
            rested_xp,
        }
    }

    fn kill(original: u32, amount: u32) -> LogXpGain {
        LogXpGain {
            victim: Some(7),
            original,
            amount,
            group_bonus: 1.0,
            reason: XpGainReason::Kill,
        }
    }

    #[test]
    fn fill_is_xp_over_the_level_requirement() {
        assert_eq!(fill_fraction(&update(1_000, 4_000, 0)), 0.25);
    }

    #[test]
    fn at_the_level_cap_there_is_no_bar() {
        assert_eq!(ExperienceState(Some(update(0, 0, 0))).leveling(), None);
        assert_eq!(ExperienceState(None).leveling(), None);
        assert_eq!(
            ExperienceState(Some(update(10, 400, 0))).leveling(),
            Some(update(10, 400, 0))
        );
    }

    #[test]
    fn rested_end_is_xp_plus_pool_over_the_requirement() {
        assert_eq!(rested_end_fraction(&update(1_000, 4_000, 1_000)), Some(0.5));
        assert_eq!(
            rested_end_fraction(&update(3_000, 4_000, 2_000)),
            Some(1.25)
        );
        assert_eq!(rested_end_fraction(&update(1_000, 4_000, 0)), None);
    }

    #[test]
    fn hover_and_tooltip_texts_use_the_retail_strings() {
        let xp = update(1_234, 4_000, 800);
        assert_eq!(bar_text(&xp), "XP: 1234/4000");
        assert_eq!(tooltip_title(&xp), "1,234 / 4,000  ( 31% )");
        assert_eq!(rest_state(&xp), ("Rested", 200));
        assert_eq!(rest_state(&update(1_234, 4_000, 0)), ("Normal", 100));
    }

    #[test]
    fn kill_gains_print_the_combat_log_line() {
        assert_eq!(
            gain_chat_line(&kill(90, 45), Some("Kobold Vermin")).as_deref(),
            Some("Kobold Vermin dies, you gain 90 experience. (+45 exp Rested bonus)")
        );
        assert_eq!(
            gain_chat_line(&kill(45, 45), Some("Kobold Vermin")).as_deref(),
            Some("Kobold Vermin dies, you gain 45 experience.")
        );
        assert_eq!(
            gain_chat_line(&kill(45, 45), None).as_deref(),
            Some("You gain 45 experience.")
        );
        let quest = LogXpGain {
            reason: XpGainReason::Quest,
            ..kill(450, 450)
        };
        assert_eq!(gain_chat_line(&quest, None), None);
    }
}
