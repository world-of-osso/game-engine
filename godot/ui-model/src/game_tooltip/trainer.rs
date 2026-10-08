//! Trainer services compose the shared spell/item tooltips with existing recipe metadata.
use super::{
    GameTooltip,
    item::{item_game_tooltip, named_item},
};
use crate::item_catalog::item_catalog_entry;
use crate::item_tooltip::RED_FONT_COLOR;
use crate::professions::Recipe;
use crate::tooltip_presentation::{TOOLTIP_WHITE, TooltipLineState, item_id_line};
use crate::trainer_frame::TrainerView;
use shared::protocol::TrainerService;

/// Non-recipes keep the complete spell tooltip, including the live caster context.
/// Recipes describe their output item; the service's Spell record remains the final ID.
pub fn trainer_service_content(
    mut tooltip: GameTooltip,
    recipe: Option<&Recipe>,
    player_level: Option<u16>,
) -> Result<GameTooltip, String> {
    let Some(recipe) = recipe else {
        return Ok(tooltip);
    };
    let (item_id, count) = recipe.output;
    let item = item_catalog_entry(item_id).ok_or_else(|| {
        format!(
            "Trainer recipe {}: output item {item_id} missing from ItemCatalog",
            recipe.spell_id
        )
    })?;
    let slot = named_item(item_id, &item.name, item.quality, count);
    let mut content = item_game_tooltip(&slot, player_level).content;
    content.lines.push(item_id_line(item_id));
    content.lines.extend(reagent_lines(recipe)?);
    tooltip.content = content;
    Ok(tooltip)
}

fn reagent_lines(recipe: &Recipe) -> Result<Vec<TooltipLineState>, String> {
    if recipe.reagents.is_empty() {
        return Ok(Vec::new());
    }
    let reagents = recipe
        .reagents
        .iter()
        .map(|&(item_id, count)| {
            let item = item_catalog_entry(item_id).ok_or_else(|| {
                format!(
                    "Trainer recipe {}: reagent item {item_id} missing from ItemCatalog",
                    recipe.spell_id
                )
            })?;
            Ok(TooltipLineState::colored(
                format!("{} ({count})", item.name),
                TOOLTIP_WHITE,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(
        std::iter::once(TooltipLineState::colored("Reagents:", TOOLTIP_WHITE))
            .chain(reagents)
            .collect(),
    )
}

impl TrainerView {
    /// Current service requirements and Retail row ownership, independent of selection.
    pub fn service_tooltip(
        &self,
        spell: u32,
        tooltip: GameTooltip,
        owner: [f32; 4],
    ) -> Option<GameTooltip> {
        let service = self
            .book
            .visible_services()
            .find(|service| service.spell_id == spell)?;
        let mut tooltip = self.book.service_tooltip(spell, tooltip, owner)?;
        tooltip
            .content
            .lines
            .extend(service_requirements(self, service));
        Some(tooltip)
    }
}

fn requirement(text: String, met: bool) -> TooltipLineState {
    TooltipLineState::colored(text, if met { TOOLTIP_WHITE } else { RED_FONT_COLOR })
}

fn service_requirements(view: &TrainerView, service: &TrainerService) -> Vec<TooltipLineState> {
    let level = (service.req_level > 1).then(|| {
        requirement(
            format!("Requires Level {}", service.req_level),
            view.player_level >= service.req_level,
        )
    });
    let skill = skill_requirement(view, service);
    let abilities = service.req_abilities.iter().map(|id| {
        requirement(
            format!("Requires {}", view.display.spell_name(*id)),
            view.known_spells.contains(id),
        )
    });
    level.into_iter().chain(skill).chain(abilities).collect()
}

fn skill_requirement(view: &TrainerView, service: &TrainerService) -> Option<TooltipLineState> {
    if service.req_skill_line == 0 {
        return None;
    }
    let name = view
        .display
        .skills
        .get(&service.req_skill_line)
        .cloned()
        .unwrap_or_else(|| format!("Skill {}", service.req_skill_line));
    let met = view.ranks.iter().any(|line| {
        line.skill_line == service.req_skill_line && line.rank >= service.req_skill_rank
    });
    Some(requirement(
        format!("Requires {name} ({})", service.req_skill_rank),
        met,
    ))
}
