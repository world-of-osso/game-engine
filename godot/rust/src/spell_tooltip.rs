//! Spell tooltip over hovered spellbook items and main bar buttons: GameTooltip lines
//! from the spell catalog (GlobalStrings `RAGE_COST`, `MELEE_RANGE`, `SPELL_RANGE`,
//! `SPELL_CAST_TIME_*`, `SPELL_RECAST_TIME_*`) and the description rendered by the
//! shared token renderer (docs/reference/spell-description-tokens.md).

use game_engine_core::spell_catalog::{CatalogSpell, SpellTextContext};
use game_engine_ui_model::spell_tooltip_component::{PADDING, SpellTooltipState, TEXT_SIZE};
use godot::classes::FontFile;
use godot::prelude::*;
use shared::components::PowerType;
use ui_toolkit::widgets::font_string::GameFont;

use crate::{GameClient, ui::RegistryUi};

/// `GameTooltip` default minimum width used for spells.
const TOOLTIP_W: f32 = 300.0;
const ANCHOR_GAP: f32 = 4.0;
/// Rage and runic power costs are stored in tenths.
const TENTHS_POWERS: [i8; 2] = [1, 6];
/// SPELL_RANGE uses "Melee Range" up to this many yards.
const MELEE_YARDS: f32 = 5.0;

/// `%.2g`: at most two significant digits, no trailing zeros.
fn two_significant(value: f32) -> String {
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

fn cost_text(spell: &CatalogSpell) -> String {
    let Some(cost) = spell
        .powers
        .iter()
        .find(|cost| cost.required_aura_spell_id == 0)
    else {
        return String::new();
    };
    if cost.flat <= 0 {
        return String::new();
    }
    let amount = if TENTHS_POWERS.contains(&cost.power_type) {
        cost.flat / 10
    } else {
        cost.flat
    };
    let name = PowerType::from_db(i32::from(cost.power_type))
        .and_then(game_engine_ui_model::cast_failed_text::power_display_name)
        .map(|name| {
            let mut chars = name.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    format!("{amount} {name}")
}

fn range_text(spell: &CatalogSpell) -> String {
    let [min, max] = [spell.range.min_yd[0], spell.range.max_yd[0]];
    if max <= 0.0 {
        String::new()
    } else if max <= MELEE_YARDS {
        "Melee Range".into()
    } else if min > 0.0 {
        format!("{}-{} yd range", two_significant(min), two_significant(max))
    } else {
        format!("{} yd range", two_significant(max))
    }
}

fn cast_text(spell: &CatalogSpell) -> String {
    if spell.cast_time_ms <= 0 {
        "Instant".into()
    } else {
        format!(
            "{} sec cast",
            two_significant(spell.cast_time_ms as f32 / 1000.0)
        )
    }
}

fn cooldown_text(spell: &CatalogSpell) -> String {
    let ms = spell
        .cooldown
        .recovery_ms
        .max(spell.cooldown.category_recovery_ms);
    let secs = ms as f32 / 1000.0;
    if ms == 0 {
        String::new()
    } else if secs > 60.0 {
        format!("{} min cooldown", two_significant(secs / 60.0))
    } else {
        format!("{} sec cooldown", two_significant(secs))
    }
}

/// GameTooltip detail lines of an active spell; passives show none.
pub(crate) fn tooltip_details(spell: &CatalogSpell) -> Vec<(String, String)> {
    if spell.passive {
        return vec![("Passive".into(), String::new())];
    }
    let rows = [
        (cost_text(spell), range_text(spell)),
        (cast_text(spell), cooldown_text(spell)),
    ];
    rows.into_iter()
        .filter(|(left, right)| !left.is_empty() || !right.is_empty())
        .collect()
}

/// Greedy word wrap at `width`; `\n` breaks lines, `|c`/`|r` colour escapes are dropped.
pub(crate) fn wrap_text(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let plain = strip_color_escapes(text);
    let mut lines = Vec::new();
    for paragraph in plain.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && measure(&candidate) > width {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn strip_color_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '|' {
            match chars.peek() {
                Some('c') => {
                    chars.next();
                    for _ in 0..8 {
                        chars.next();
                    }
                    continue;
                }
                Some('r') => {
                    chars.next();
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

impl GameClient {
    fn tooltip_font(&self) -> Result<Gd<FontFile>, String> {
        crate::ui::assets::load_font(GameFont::FrizQuadrata)
    }

    fn spell_tooltip_state(&self) -> Result<SpellTooltipState, String> {
        let hovered = self
            .hovered_book_spell()
            .map(|(id, level, rect)| (id, level, rect, false))
            .or_else(|| {
                self.hovered_bar_spell()
                    .map(|(id, rect)| (id, None, rect, true))
            });
        let Some((spell_id, available_at, [x, y, w, _], on_bar)) = hovered else {
            return Ok(SpellTooltipState::default());
        };
        let Some(spell) = self.spells.catalog().and_then(|data| data.get(spell_id)) else {
            return Ok(SpellTooltipState::default());
        };
        let context = SpellTextContext {
            known_spells: self.account.spells.known().to_vec(),
            auras: Vec::new(),
            spec_id: self.account.spells.spec(),
        };
        let description = self
            .spells
            .catalog()
            .and_then(|data| data.render_description(spell_id, &context))
            .unwrap_or_default();
        let font = self.tooltip_font()?;
        let measure = |text: &str| {
            font.get_string_size_ex(text)
                .font_size(TEXT_SIZE as i32)
                .done()
                .x
        };
        let mut state = SpellTooltipState {
            visible: true,
            origin: [0.0, 0.0],
            width: TOOLTIP_W,
            name: spell.name.to_string(),
            details: tooltip_details(spell),
            description: wrap_text(&description, TOOLTIP_W - 2.0 * PADDING, measure),
            requirement: available_at.map(|level| format!("Level {level}")),
        };
        let screen = self
            .base()
            .get_viewport()
            .map_or(Vector2::new(1280.0, 720.0), |viewport| {
                viewport.get_visible_rect().size
            });
        let height = state.height();
        // Spellbook items: to the right of the icon; action buttons: above the bar.
        let (left, top) = if on_bar {
            (x, y - height - ANCHOR_GAP)
        } else {
            (x + w + ANCHOR_GAP, y)
        };
        state.origin = [
            left.clamp(0.0, (screen.x - TOOLTIP_W).max(0.0)),
            top.clamp(0.0, (screen.y - height).max(0.0)),
        ];
        Ok(state)
    }

    pub(crate) fn sync_spell_tooltip(&mut self) -> Result<(), String> {
        let state = self.spell_tooltip_state()?;
        if let Some(ui) = self.spells.tooltip_ui.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("SpellTooltipUI");
        ui.set_layer(8);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_spell_tooltip(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.spells.tooltip_ui = Some(ui);
        Ok(())
    }

    /// Tooltip text for automation: name, details and description lines.
    pub(crate) fn spell_tooltip_lines(&self) -> Vec<String> {
        let Ok(state) = self.spell_tooltip_state() else {
            return Vec::new();
        };
        if !state.visible {
            return Vec::new();
        }
        std::iter::once(state.name)
            .chain(
                state
                    .details
                    .into_iter()
                    .map(|(left, right)| format!("{left}|{right}")),
            )
            .chain(state.requirement)
            .chain(state.description)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_core::spell_catalog::{SpellCooldown, SpellPowerCost, SpellRange};

    fn spell() -> CatalogSpell {
        CatalogSpell {
            id: 1464,
            name: "Slam".into(),
            ..Default::default()
        }
    }

    #[test]
    fn slam_costs_twenty_rage_at_melee_range_instantly() {
        let mut slam = spell();
        slam.powers = vec![SpellPowerCost {
            power_type: 1,
            flat: 200,
            pct: 0.0,
            required_aura_spell_id: 0,
        }]
        .into();
        slam.range = SpellRange {
            min_yd: [0.0, 0.0],
            max_yd: [5.0, 5.0],
        };
        assert_eq!(
            tooltip_details(&slam),
            [
                ("20 Rage".to_owned(), "Melee Range".to_owned()),
                ("Instant".to_owned(), String::new()),
            ]
        );
    }

    #[test]
    fn charge_shows_its_range_band_and_cooldown() {
        let mut charge = spell();
        charge.range = SpellRange {
            min_yd: [8.0, 8.0],
            max_yd: [25.0, 25.0],
        };
        charge.cooldown = SpellCooldown {
            recovery_ms: 20_000,
            category_recovery_ms: 0,
            gcd_ms: 0,
        };
        charge.cast_time_ms = 1_500;
        assert_eq!(
            tooltip_details(&charge),
            [
                (String::new(), "8-25 yd range".to_owned()),
                ("1.5 sec cast".to_owned(), "20 sec cooldown".to_owned()),
            ]
        );
        charge.cooldown.recovery_ms = 120_000;
        assert_eq!(cooldown_text(&charge), "2 min cooldown");
    }

    #[test]
    fn words_wrap_at_the_width_and_colour_escapes_drop() {
        // 10 px per character.
        let measure = |text: &str| text.chars().count() as f32 * 10.0;
        let lines = wrap_text(
            "Slams an opponent, causing 12 Physical damage.\n\n|cFFFFFFFFGenerates 10 Rage.|r",
            160.0,
            measure,
        );
        assert_eq!(
            lines,
            [
                "Slams an",
                "opponent,",
                "causing 12",
                "Physical damage.",
                "",
                "Generates 10",
                "Rage.",
            ]
        );
    }
}
