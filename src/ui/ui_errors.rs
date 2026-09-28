//! Top-center error text (WoW `UIErrorsFrame` equivalent).
//!
//! Newest message first, at most [`MAX_ERROR_LINES`]. Re-adding a shown message refreshes
//! its timer in place instead of stacking a duplicate.

use bevy::prelude::*;
use shared::components::PowerType;

use super::ui_errors_data::UiErrorsData;
pub use super::ui_errors_data::{ERROR_FADE_SECS, ERROR_HOLD_SECS, ErrorLine, MAX_ERROR_LINES};
use shared::spell_data::CastFailReason;

#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct UiErrors(pub UiErrorsData);

impl std::ops::Deref for UiErrors {
    type Target = UiErrorsData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for UiErrors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Error text for a rejected cast. A server `detail` wins; otherwise Retail GlobalStrings
/// wording. `power` is the spell's power type when the client knows it.
pub fn cast_failed_text(
    reason: CastFailReason,
    detail: Option<&str>,
    power: Option<PowerType>,
) -> String {
    if let Some(detail) = detail {
        return detail.to_string();
    }
    let text = match reason {
        // ERR_GENERIC_NO_TARGET
        CastFailReason::NoTarget => "You have no target.",
        // ERR_OUT_OF_RANGE
        CastFailReason::OutOfRange => "Out of range.",
        // SPELL_FAILED_BAD_TARGETS
        CastFailReason::InvalidTarget => "Invalid target",
        CastFailReason::NotEnoughResource => return not_enough_power_text(power),
        // SPELL_FAILED_NOT_READY
        CastFailReason::OnCooldown | CastFailReason::OnGlobalCooldown => "Spell is not ready yet.",
        // ERR_PLAYER_DEAD
        CastFailReason::CasterDead => "You can't do that when you're dead.",
        // SPELL_FAILED_MOVING
        CastFailReason::CantCastWhileMoving => "Can't do that while moving",
        // SPELL_FAILED_TOO_CLOSE
        CastFailReason::TooClose => "Target too close.",
        // SPELL_FAILED_SPELL_IN_PROGRESS
        CastFailReason::SpellInProgress => "Another action is in progress.",
        // SPELL_FAILED_NO_CHARGES_REMAIN
        CastFailReason::NoChargesRemain => "No charges remain.",
        // SPELL_FAILED_UNIT_NOT_INFRONT
        CastFailReason::NotInFront => "Target needs to be in front of you.",
        // SPELL_FAILED_SILENCED, shown for interrupt school lockouts
        CastFailReason::SchoolLockedOut => "Can't do that while silenced",
        // Not a Retail string: the server cannot execute this spell.
        CastFailReason::NotSupported => "That spell isn't available yet.",
        // SPELL_FAILED_STUNNED
        CastFailReason::Stunned => "Can't do that while stunned",
        // SPELL_FAILED_SILENCED
        CastFailReason::Silenced => "Can't do that while silenced",
        // SPELL_FAILED_PACIFIED
        CastFailReason::Pacified => "Can't do that while pacified",
        // Not Retail strings: the server's detail carries the Retail text with the item
        // name ("Missing reagent: %s" SPELL_FAILED_REAGENTS, "Requires %s" SPELL_FAILED_TOTEMS).
        CastFailReason::Reagents => "Missing reagent",
        CastFailReason::Totems => "Requires a tool",
        // ERR_INV_FULL
        CastFailReason::InventoryFull => "Inventory is full.",
        // SPELL_FAILED_SUMMON_PENDING
        CastFailReason::SummonPending => "A summon is already pending",
        // SPELL_FAILED_LEVEL_REQUIREMENT
        CastFailReason::LevelRequirement => "You are not high enough level",
        // SPELL_FAILED_LOWLEVEL
        CastFailReason::TargetTooLowLevel => "Target is too low level",
    };
    text.to_string()
}

/// ERR_OUT_OF_{POWER} ("Not enough mana"); generic when the power type is unknown.
fn not_enough_power_text(power: Option<PowerType>) -> String {
    match power.and_then(power_display_name) {
        Some(name) => format!("Not enough {name}."),
        None => "Not enough power.".to_string(),
    }
}

pub fn power_display_name(power: PowerType) -> Option<&'static str> {
    Some(match power {
        PowerType::Mana => "mana",
        PowerType::Rage => "rage",
        PowerType::Focus => "focus",
        PowerType::Energy => "energy",
        PowerType::ComboPoints => "combo points",
        PowerType::Runes => "runes",
        PowerType::RunicPower => "runic power",
        PowerType::SoulShards => "soul shards",
        PowerType::LunarPower => "astral power",
        PowerType::HolyPower => "Holy Power",
        PowerType::Maelstrom => "maelstrom",
        PowerType::Chi => "chi",
        PowerType::Insanity => "insanity",
        PowerType::ArcaneCharges => "arcane charges",
        PowerType::Fury => "fury",
        PowerType::Pain => "pain",
        PowerType::Essence => "essence",
        PowerType::Alternate
        | PowerType::AlternateQuest
        | PowerType::AlternateEncounter
        | PowerType::AlternateMount => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_refreshes_instead_of_stacking() {
        let mut errors = UiErrors::default();
        errors.add("Out of range.");
        errors.tick(2.0);
        errors.add("Out of range.");
        assert_eq!(
            errors.lines,
            vec![ErrorLine {
                text: "Out of range.".into(),
                age: 0.0
            }]
        );
    }

    #[test]
    fn keeps_three_newest_lines_newest_first() {
        let mut errors = UiErrors::default();
        for text in ["a", "b", "c", "d"] {
            errors.add(text);
        }
        let texts: Vec<_> = errors.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["d", "c", "b"]);
    }

    #[test]
    fn lines_hold_then_fade_then_expire() {
        let mut errors = UiErrors::default();
        errors.add("Out of range.");
        errors.tick(ERROR_HOLD_SECS);
        assert_eq!(errors.lines[0].alpha(), 1.0);
        errors.tick(ERROR_FADE_SECS / 2.0);
        assert!((errors.lines[0].alpha() - 0.5).abs() < 0.01);
        errors.tick(ERROR_FADE_SECS);
        assert!(errors.lines.is_empty());
    }

    #[test]
    fn cast_fail_reasons_use_retail_wording() {
        let text = |reason| cast_failed_text(reason, None, None);
        assert_eq!(text(CastFailReason::OutOfRange), "Out of range.");
        assert_eq!(text(CastFailReason::InvalidTarget), "Invalid target");
        assert_eq!(text(CastFailReason::OnCooldown), "Spell is not ready yet.");
        assert_eq!(text(CastFailReason::NoTarget), "You have no target.");
        assert_eq!(text(CastFailReason::TooClose), "Target too close.");
        assert_eq!(
            text(CastFailReason::SchoolLockedOut),
            "Can't do that while silenced"
        );
        assert_eq!(
            text(CastFailReason::SpellInProgress),
            "Another action is in progress."
        );
        assert_eq!(text(CastFailReason::NoChargesRemain), "No charges remain.");
        assert_eq!(
            text(CastFailReason::NotInFront),
            "Target needs to be in front of you."
        );
        assert_eq!(
            text(CastFailReason::NotSupported),
            "That spell isn't available yet."
        );
        assert_eq!(text(CastFailReason::NotEnoughResource), "Not enough power.");
        assert_eq!(
            cast_failed_text(
                CastFailReason::NotEnoughResource,
                None,
                Some(PowerType::Rage)
            ),
            "Not enough rage."
        );
        assert_eq!(
            cast_failed_text(CastFailReason::OutOfRange, Some("Too far away"), None),
            "Too far away"
        );
    }
}
