//! Spell and aura tooltips: `GameTooltip:SetSpellByID` (spellbook, action bar) and
//! `GameTooltip:SetUnitAura` (BuffFrame, TargetFrame auras), from the spell catalog with
//! the GlobalStrings formats named on each function.

use game_engine_core::spell_catalog::CatalogSpell;
use shared::components::PowerType;

use super::{GameTooltip, TooltipRecord};
use crate::aura_display_data::AuraInstance;
use crate::cast_failed_text::power_display_name;
use crate::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipLineState, TooltipPresentation,
    description_lines,
};

/// `RED_FONT_COLOR`.
const RED_FONT_COLOR: [f32; 4] = [1.0, 0.125, 0.125, 1.0];
/// Ranges up to this many yards read `MELEE_RANGE`.
const MELEE_YARDS: f32 = 5.0;

/// What a spell tooltip reads besides the catalog row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellTooltipInput {
    /// The rendered `Description_lang` (docs/reference/spell-description-tokens.md).
    pub description: String,
    /// `SPELLBOOK_AVAILABLE_AT`: the level of a spellbook spell not learned yet.
    pub available_at: Option<u32>,
    /// Seconds left on the spell's own cooldown (not the global cooldown).
    pub cooldown_remaining: Option<f32>,
}

/// Name, then cost | range, cast time | cooldown, the remaining cooldown, the level
/// requirement and the gold description. A passive spell shows `SPELL_PASSIVE` instead of
/// the cost and timing rows.
pub fn spell_tooltip(spell: &CatalogSpell, input: &SpellTooltipInput) -> GameTooltip {
    let mut lines = Vec::new();
    if !spell.subtext.is_empty() {
        lines.push(TooltipLineState::colored(
            spell.subtext.to_string(),
            TOOLTIP_WHITE,
        ));
    }
    if spell.passive {
        lines.push(TooltipLineState::colored("Passive", TOOLTIP_WHITE));
    } else {
        lines.extend(detail_pairs(spell));
    }
    if let Some(remaining) = input.cooldown_remaining.filter(|secs| *secs > 0.0) {
        lines.push(TooltipLineState::colored(
            cooldown_remaining_text(remaining),
            TOOLTIP_WHITE,
        ));
    }
    if let Some(level) = input.available_at {
        lines.push(TooltipLineState::colored(
            format!("Level {level}"),
            RED_FONT_COLOR,
        ));
    }
    lines.extend(description_lines(
        &input.description,
        TOOLTIP_DESCRIPTION_COLOR,
    ));
    GameTooltip::new(
        TooltipPresentation {
            title: spell.name.to_string(),
            title_color: TOOLTIP_WHITE,
            lines,
            ..TooltipPresentation::hidden()
        },
        Some(TooltipRecord::Spell(spell.id)),
    )
}

/// A spell the catalog does not know: its ID as the name.
pub fn unknown_spell_tooltip(spell_id: u32) -> GameTooltip {
    GameTooltip::new(
        TooltipPresentation {
            title: format!("Spell {spell_id}"),
            title_color: TOOLTIP_WHITE,
            ..TooltipPresentation::hidden()
        },
        Some(TooltipRecord::Spell(spell_id)),
    )
}

fn detail_pairs(spell: &CatalogSpell) -> Vec<TooltipLineState> {
    [
        (cost_text(spell), range_text(spell)),
        (cast_text(spell), recast_text(spell)),
    ]
    .into_iter()
    .filter(|(left, right)| !left.is_empty() || !right.is_empty())
    .map(|(left, right)| TooltipLineState::pair(left, right))
    .collect()
}

/// The first unconditional cost: "20 Rage", "1% of base mana" (`POWER_TYPE` names).
pub fn cost_text(spell: &CatalogSpell) -> String {
    let Some(cost) = spell
        .powers
        .iter()
        .find(|cost| cost.required_aura_spell_id == 0)
    else {
        return String::new();
    };
    let Some(power) = PowerType::from_db(cost.power_type.into()) else {
        return String::new();
    };
    let name = power_display_name(power).unwrap_or("power");
    if cost.flat > 0 {
        let amount = cost.flat as f32 / power_display_divisor(power);
        return format!("{} {}", trim_number(amount), title_case(name));
    }
    if cost.pct > 0.0 {
        let base = if power == PowerType::Mana {
            "base"
        } else {
            "maximum"
        };
        return format!("{}% of {base} {name}", trim_number(cost.pct));
    }
    String::new()
}

/// Raw pool units per displayed point (`PowerType.DisplayModifier`).
fn power_display_divisor(power: PowerType) -> f32 {
    match power {
        PowerType::Rage
        | PowerType::RunicPower
        | PowerType::SoulShards
        | PowerType::LunarPower
        | PowerType::Pain => 10.0,
        PowerType::Insanity => 100.0,
        _ => 1.0,
    }
}

/// `MELEE_RANGE`, or `SPELL_RANGE` "%s yd range" with a "min-max" band; the hostile range
/// when the spell has one, else the friendly one. Self-only spells have no range.
pub fn range_text(spell: &CatalogSpell) -> String {
    let side = usize::from(spell.range.max_yd[0] <= 0.0);
    let (min, max) = (spell.range.min_yd[side], spell.range.max_yd[side]);
    if max <= 0.0 {
        String::new()
    } else if max <= MELEE_YARDS {
        "Melee Range".into()
    } else if min > 0.0 {
        format!("{}-{} yd range", trim_number(min), trim_number(max))
    } else {
        format!("{} yd range", trim_number(max))
    }
}

/// `SPELL_CAST_TIME_INSTANT` or `SPELL_CAST_TIME_SEC` "%.2g sec cast".
fn cast_text(spell: &CatalogSpell) -> String {
    if spell.cast_time_ms <= 0 {
        "Instant".into()
    } else {
        format!("{} sec cast", format_g2(spell.cast_time_ms as f32 / 1000.0))
    }
}

/// `SPELL_RECAST_TIME_CHARGES_*` "%.2g sec recharge" for spells with charges, else
/// `SPELL_RECAST_TIME_*` "%.2g sec cooldown"; none without a cooldown.
fn recast_text(spell: &CatalogSpell) -> String {
    if let Some(charges) = spell.charges.filter(|charges| charges.max_charges > 1) {
        return format!("{} recharge", g2_duration(charges.recovery_ms));
    }
    let ms = spell
        .cooldown
        .recovery_ms
        .max(spell.cooldown.category_recovery_ms);
    if ms == 0 {
        return String::new();
    }
    format!("{} cooldown", g2_duration(ms))
}

/// "20 sec", "1.5 min", "2 hour" with `%.2g` numbers.
fn g2_duration(ms: u32) -> String {
    let secs = ms as f32 / 1000.0;
    for (unit, label) in [(86_400.0, "day"), (3_600.0, "hour"), (60.0, "min")] {
        if secs >= unit {
            return format!("{} {label}", format_g2(secs / unit));
        }
    }
    format!("{} sec", format_g2(secs))
}

/// `ITEM_COOLDOWN_TIME_*`: "Cooldown remaining: 12 sec", whole units rounded up.
pub fn cooldown_remaining_text(secs: f32) -> String {
    let whole = |unit: f32| (secs / unit).ceil() as u32;
    if secs > 3_600.0 {
        format!(
            "Cooldown remaining: {} {}",
            whole(3_600.0),
            plural(whole(3_600.0), "hour")
        )
    } else if secs > 60.0 {
        format!("Cooldown remaining: {} min", whole(60.0))
    } else {
        format!("Cooldown remaining: {} sec", whole(1.0))
    }
}

/// `GameTooltip:SetUnitAura`: the white name, the gold rendered aura description and the
/// gold `SPELL_TIME_REMAINING_*` line ("12 minutes remaining") for timed auras.
pub fn aura_tooltip(aura: &AuraInstance) -> GameTooltip {
    let mut lines = description_lines(&aura.description, TOOLTIP_DESCRIPTION_COLOR);
    if !aura.is_permanent() {
        lines.push(TooltipLineState::colored(
            time_remaining_text(aura.remaining),
            TOOLTIP_DESCRIPTION_COLOR,
        ));
    }
    GameTooltip::new(
        TooltipPresentation {
            title: aura.name.clone(),
            title_color: TOOLTIP_WHITE,
            lines,
            ..TooltipPresentation::hidden()
        },
        Some(TooltipRecord::Spell(aura.spell_id)),
    )
}

/// `SPELL_TIME_REMAINING_DAYS/HOURS/MIN/SEC` in the largest whole unit, rounded up.
pub fn time_remaining_text(secs: f32) -> String {
    let secs = secs.max(0.0);
    for (unit, label) in [(86_400.0, "day"), (3_600.0, "hour"), (60.0, "minute")] {
        if secs >= unit {
            let count = (secs / unit).ceil() as u32;
            return format!("{count} {} remaining", plural(count, label));
        }
    }
    let count = secs.ceil() as u32;
    format!("{count} {} remaining", plural(count, "second"))
}

/// The `|4singular:plural;` escape.
fn plural(count: u32, singular: &str) -> String {
    if count == 1 {
        singular.to_owned()
    } else {
        format!("{singular}s")
    }
}

/// `%.2g`: at most two significant digits, no trailing zeros.
fn format_g2(value: f32) -> String {
    let magnitude = if value.abs() >= 1.0 {
        value.abs().log10().floor() as i32 + 1
    } else {
        1
    };
    let decimals = (2 - magnitude).max(0) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

/// One decimal at most, none when whole.
pub(super) fn trim_number(value: f32) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").map(str::to_owned).unwrap_or(text)
}

fn title_case(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_core::spell_catalog::{
        SpellCharges, SpellCooldown, SpellPowerCost, SpellRange,
    };

    fn rows(tooltip: &GameTooltip) -> Vec<(&str, &str)> {
        tooltip
            .content
            .lines
            .iter()
            .map(|line| (line.left_text.as_str(), line.right_text.as_str()))
            .collect()
    }

    fn slam() -> CatalogSpell {
        CatalogSpell {
            id: 1464,
            name: "Slam".into(),
            powers: vec![SpellPowerCost {
                power_type: 1,
                flat: 200,
                pct: 0.0,
                required_aura_spell_id: 0,
            }]
            .into(),
            range: SpellRange {
                min_yd: [0.0, 0.0],
                max_yd: [5.0, 5.0],
            },
            ..Default::default()
        }
    }

    #[test]
    fn slam_reads_rage_melee_range_instant_and_its_description() {
        super::super::set_test_data_root();
        let input = SpellTooltipInput {
            description: "Slams an opponent, causing 12 Physical damage.".into(),
            ..Default::default()
        };
        let tooltip = spell_tooltip(&slam(), &input);
        assert_eq!(tooltip.content.title, "Slam");
        assert_eq!(tooltip.record, Some(TooltipRecord::Spell(1464)));
        assert_eq!(
            rows(&tooltip),
            [
                ("20 Rage", "Melee Range"),
                ("Instant", ""),
                ("Slams an opponent, causing 12 Physical damage.", ""),
            ]
        );
        assert_eq!(
            tooltip.content.lines[2].left_color,
            TOOLTIP_DESCRIPTION_COLOR
        );
    }

    #[test]
    fn charge_shows_its_band_cast_cooldown_remaining_cooldown_and_level() {
        super::super::set_test_data_root();
        let charge = CatalogSpell {
            id: 100,
            name: "Charge".into(),
            range: SpellRange {
                min_yd: [8.0, 0.0],
                max_yd: [25.0, 0.0],
            },
            cooldown: SpellCooldown {
                recovery_ms: 20_000,
                category_recovery_ms: 0,
                gcd_ms: 0,
            },
            cast_time_ms: 1_500,
            ..Default::default()
        };
        let input = SpellTooltipInput {
            available_at: Some(2),
            cooldown_remaining: Some(11.2),
            ..Default::default()
        };
        let tooltip = spell_tooltip(&charge, &input);
        assert_eq!(
            rows(&tooltip),
            [
                ("", "8-25 yd range"),
                ("1.5 sec cast", "20 sec cooldown"),
                ("Cooldown remaining: 12 sec", ""),
                ("Level 2", ""),
            ]
        );
        assert_eq!(tooltip.content.lines[3].left_color, RED_FONT_COLOR);
    }

    #[test]
    fn mana_percent_costs_friendly_ranges_charges_and_long_cooldowns() {
        super::super::set_test_data_root();
        let flash = CatalogSpell {
            id: 19_750,
            name: "Flash of Light".into(),
            powers: vec![SpellPowerCost {
                power_type: 0,
                flat: 0,
                pct: 10.0,
                required_aura_spell_id: 0,
            }]
            .into(),
            range: SpellRange {
                min_yd: [0.0, 0.0],
                max_yd: [0.0, 40.0],
            },
            charges: Some(SpellCharges {
                max_charges: 2,
                recovery_ms: 90_000,
            }),
            ..Default::default()
        };
        let tooltip = spell_tooltip(&flash, &SpellTooltipInput::default());
        assert_eq!(
            rows(&tooltip),
            [
                ("10% of base mana", "40 yd range"),
                ("Instant", "1.5 min recharge"),
            ]
        );
        let shield_wall = CatalogSpell {
            cooldown: SpellCooldown {
                recovery_ms: 240_000,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(recast_text(&shield_wall), "4 min cooldown");
        assert_eq!(cooldown_remaining_text(95.0), "Cooldown remaining: 2 min");
        assert_eq!(
            cooldown_remaining_text(7_300.0),
            "Cooldown remaining: 3 hours"
        );
    }

    #[test]
    fn passive_spells_show_passive_instead_of_costs() {
        super::super::set_test_data_root();
        let passive = CatalogSpell {
            passive: true,
            ..slam()
        };
        let tooltip = spell_tooltip(&passive, &SpellTooltipInput::default());
        assert_eq!(rows(&tooltip), [("Passive", "")]);
    }

    fn arcane_intellect(remaining: f32) -> AuraInstance {
        AuraInstance {
            spell_id: 1459,
            name: "Arcane Intellect".into(),
            description: "Intellect increased by 5%.".into(),
            icon_fdid: 135_932,
            instance_id: 1,
            source: String::new(),
            from_local_player: true,
            from_player: true,
            duration: 3_600.0,
            remaining,
            stacks: 1,
            is_debuff: false,
            debuff_type: Default::default(),
        }
    }

    #[test]
    fn auras_show_their_description_and_the_time_remaining_counting_down() {
        super::super::set_test_data_root();
        let tooltip = aura_tooltip(&arcane_intellect(3_542.0));
        assert_eq!(tooltip.content.title, "Arcane Intellect");
        assert_eq!(tooltip.content.title_color, TOOLTIP_WHITE);
        assert_eq!(tooltip.record, Some(TooltipRecord::Spell(1459)));
        assert_eq!(
            rows(&tooltip),
            [
                ("Intellect increased by 5%.", ""),
                ("60 minutes remaining", ""),
            ]
        );
        let later = aura_tooltip(&arcane_intellect(61.0));
        assert_eq!(later.content.lines[1].left_text, "2 minutes remaining");
        let last = aura_tooltip(&arcane_intellect(0.4));
        assert_eq!(last.content.lines[1].left_text, "1 second remaining");
        let permanent = aura_tooltip(&AuraInstance {
            duration: 0.0,
            ..arcane_intellect(0.0)
        });
        assert_eq!(permanent.content.lines.len(), 1);
    }
}
