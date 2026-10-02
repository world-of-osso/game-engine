//! Retail GlobalStrings text of rejected casts (`CastFailed`), shared by the
//! Bevy and Godot clients.

use shared::components::PowerType;
use shared::spell_data::CastFailReason;

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
        // SPELL_FAILED_INTERRUPTED, SPELL_FAILED_INTERRUPTED_COMBAT
        CastFailReason::Interrupted | CastFailReason::InterruptedCombat => "Interrupted",
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
