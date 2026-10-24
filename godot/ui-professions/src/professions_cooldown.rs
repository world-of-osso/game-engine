//! Recipe cooldown display: `C_TradeSkillUI.GetRecipeCooldown` from the spell's own and
//! category clocks, and RecipeSchematicForm.lua's `COOLDOWN_REMAINING` text.

use game_engine_ui_model::auction_mail::text;

/// Whole seconds until the recipe is ready: the longer of its own and its category clock.
pub fn remaining_secs(own: Option<f32>, category: Option<f32>) -> Option<u32> {
    let remaining = own.into_iter().chain(category).fold(0.0_f32, f32::max);
    (remaining > 0.0).then(|| remaining.ceil() as u32)
}

/// RecipeSchematicForm.lua566 `COOLDOWN_REMAINING.." "..cooldownFormatter:Format(cooldown)`.
pub fn remaining_text(secs: u32) -> String {
    format!("{} {}", text("COOLDOWN_REMAINING"), format_seconds(secs))
}

/// SecondsFormatterMixin:Format with the schematic's formatter (lines 23–29): no
/// abbreviation, last unit floored, the first two nonzero units from days to seconds.
fn format_seconds(secs: u32) -> String {
    let mut left = secs;
    let mut units = Vec::new();
    for (interval, tag) in [
        (86_400, "D_DAYS"),
        (3_600, "D_HOURS"),
        (60, "D_MINUTES"),
        (1, "D_SECONDS"),
    ] {
        if units.len() == 2 {
            break;
        }
        if left >= interval {
            units.push(count_text(text(tag), left / interval));
            left %= interval;
        }
    }
    if units.is_empty() {
        return count_text(text("D_SECONDS"), 0);
    }
    units.join(text("TIME_UNIT_DELIMITER"))
}

/// `%d` and the `|4singular:plural;` escape that follows it.
fn count_text(template: &str, count: u32) -> String {
    let filled = template.replacen("%d", &count.to_string(), 1);
    let Some((head, rest)) = filled.split_once("|4") else {
        return filled;
    };
    let Some((forms, tail)) = rest.split_once(';') else {
        return filled;
    };
    let (singular, plural) = forms.split_once(':').unwrap_or((forms, forms));
    let form = if count == 1 { singular } else { plural };
    format!("{head}{form}{tail}")
}
