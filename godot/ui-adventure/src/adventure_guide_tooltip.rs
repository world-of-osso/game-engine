//! Retail set membership and ItemSetSpell thresholds appended to the shared item tooltip.
use crate::adventure_guide::View;
use game_engine_core::adventure_guide::{ItemSet, SetBonus};
use game_engine_ui_model::bag_data::InventoryState;
use game_engine_ui_model::tooltip_presentation::{TooltipLineState, description_lines};
use std::collections::{BTreeMap, BTreeSet};

const GRAY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GOLD: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

pub fn set_lines(
    view: &View,
    set: &ItemSet,
    inventory: &InventoryState,
    descriptions: &BTreeMap<u32, String>,
) -> Vec<TooltipLineState> {
    let equipped: BTreeSet<_> = inventory
        .equipment
        .values()
        .filter(|item| item.definition_source == shared::item_data::ItemDefinitionSource::Retail)
        .map(|item| item.item_id)
        .collect();
    let count = set.items.iter().filter(|id| equipped.contains(id)).count();
    let title = view.catalog.text["ITEM_SET_NAME"]
        .replacen("%s", &set.name, 1)
        .replacen("%d", &count.to_string(), 1)
        .replacen("%d", &set.items.len().to_string(), 1);
    let legacy = set.flags & 1 != 0; // TC ItemSetFlags::ITEM_SET_FLAG_LEGACY_INACTIVE.
    let mut lines = vec![
        TooltipLineState::new(String::new()),
        TooltipLineState::colored(title, GOLD),
    ];
    lines.extend(member_lines(view, set, &equipped));
    if legacy {
        lines.push(TooltipLineState::colored(
            view.catalog.text["ITEM_SET_LEGACY_INACTIVE_BONUS"].clone(),
            GRAY,
        ));
    }
    let bonuses: Vec<_> = view
        .catalog
        .set_bonuses
        .get(&set.id)
        .into_iter()
        .flatten()
        .collect();
    if view.spec == 0 && bonuses.iter().any(|bonus| bonus.spec != 0) {
        lines.push(TooltipLineState::colored(
            view.catalog.text["ITEM_SET_BONUS_NO_VALID_SPEC"].clone(),
            GRAY,
        ));
    }
    lines.extend(
        bonuses
            .iter()
            .filter(|bonus| bonus.spec == 0 || bonus.spec == view.spec)
            .flat_map(|bonus| bonus_lines(view, bonus, legacy, count, descriptions)),
    );
    lines
}

fn member_lines(view: &View, set: &ItemSet, equipped: &BTreeSet<u32>) -> Vec<TooltipLineState> {
    set.items
        .iter()
        .map(|id| {
            let name = match view.catalog.items.get(id) {
                Some(item) => item.name.clone(),
                None => format!("Item {id}: definition unavailable"),
            };
            let color = if equipped.contains(id) { WHITE } else { GRAY };
            TooltipLineState::colored(name, color)
        })
        .collect()
}

fn bonus_lines(
    view: &View,
    bonus: &SetBonus,
    legacy: bool,
    equipped: usize,
    descriptions: &BTreeMap<u32, String>,
) -> Vec<TooltipLineState> {
    let body = match descriptions.get(&bonus.spell) {
        Some(text) => text.clone(),
        None => format!("Spell {}: details unavailable", bonus.spell),
    };
    let text = view.catalog.text["ITEM_SET_BONUS_GRAY"]
        .replacen("%d", &bonus.threshold.to_string(), 1)
        .replacen("%s", &body, 1);
    let active = !legacy && bonus.trait_subtree == 0 && equipped >= bonus.threshold as usize;
    let mut lines = description_lines(&text, if active { GREEN } else { GRAY });
    if bonus.trait_subtree != 0 {
        lines.push(TooltipLineState::colored(
            format!(
                "Trait subtree {}: eligibility unavailable",
                bonus.trait_subtree
            ),
            GRAY,
        ));
    }
    lines
}
