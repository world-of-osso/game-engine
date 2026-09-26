//! Unit tooltip content (docs/specs/unit-tooltip.md): Retail `GameTooltip` unit
//! lines for NPCs and players, and below an NPC's lines the two sections that
//! deliberately deviate from Retail: its loot table drops and its vendor items,
//! each marked against the account's appearance collection.

use shared::faction_reaction::Reaction;
use shared::protocol::{CreatureTooltip, TooltipItem};
use shared::transmog::AppearanceCollection;

use super::{
    ItemMark, TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_LABEL_COLOR, TOOLTIP_WHITE, TooltipAnchor,
    TooltipFrameState, TooltipLineState, TooltipRecord, parse_rgba, quality_color, trim_number,
};

/// `FACTION_BAR_COLORS` by `UnitReaction` (SharedColorConstants.lua:3-13): 2 red
/// (hostile), 4 yellow (neutral), 5 green (friendly); the client-provided
/// `FACTION_*_COLOR` values as wow-ui-sim defines them.
const FACTION_RED: [f32; 4] = [0.8, 0.13, 0.13, 1.0];
const FACTION_YELLOW: [f32; 4] = [0.8, 0.73, 0.13, 1.0];
const FACTION_GREEN: [f32; 4] = [0.13, 0.8, 0.13, 1.0];

/// A section shows every item up to this many; beyond it, `SHOWN_WHEN_TRUNCATED`
/// entries and a "+N more" line (user requirement).
const MAX_LISTED: usize = 6;
const SHOWN_WHEN_TRUNCATED: usize = 5;

/// `GameTooltip_UnitColor` for a non-player unit: `FACTION_BAR_COLORS[UnitReaction]`.
pub(super) fn npc_name_color(reaction: Reaction) -> [f32; 4] {
    match reaction {
        Reaction::Hostile => FACTION_RED,
        Reaction::Neutral => FACTION_YELLOW,
        Reaction::Friendly => FACTION_GREEN,
    }
}

/// `GameTooltip_UnitColor` for a player: red when both can attack each other,
/// white otherwise (PvP flagging is not modelled).
pub(super) fn player_name_color(reaction: Reaction) -> [f32; 4] {
    match reaction {
        Reaction::Hostile => FACTION_RED,
        Reaction::Neutral | Reaction::Friendly => TOOLTIP_WHITE,
    }
}

/// Retail `CreatureType` names (CreatureType.db2); 10 "Not specified" and
/// unknown types show no type.
pub(super) fn creature_type_name(creature_type: u8) -> Option<&'static str> {
    Some(match creature_type {
        1 => "Beast",
        2 => "Dragonkin",
        3 => "Demon",
        4 => "Elemental",
        5 => "Giant",
        6 => "Undead",
        7 => "Humanoid",
        8 => "Critter",
        9 => "Mechanical",
        11 => "Totem",
        12 => "Non-combat Pet",
        13 => "Gas Cloud",
        14 => "Wild Pet",
        15 => "Aberration",
        _ => return None,
    })
}

/// What an NPC tooltip reads from the hovered unit.
pub(super) struct NpcTooltipInput<'a> {
    /// Creature template entry (`Npc::template_id`).
    pub entry: u32,
    pub name: &'a str,
    pub reaction: Reaction,
    pub level: Option<u8>,
    /// Name of the unit's reputation faction (`Faction.ReputationIndex` >= 0).
    pub faction: Option<&'a str>,
    /// The server's answer for the unit's creature entry, once it arrived.
    pub data: Option<&'a CreatureTooltip>,
}

pub(super) fn npc_tooltip(
    input: &NpcTooltipInput,
    collection: &AppearanceCollection,
) -> TooltipFrameState {
    let mut lines = Vec::new();
    if let Some(subname) = input.data.map(|data| data.subname.as_str())
        && !subname.is_empty()
    {
        lines.push(TooltipLineState::colored(subname, TOOLTIP_WHITE));
    }
    if let Some(level) = input.level {
        let kind = input
            .data
            .and_then(|data| creature_type_name(data.creature_type));
        // UNIT_TYPE_LEVEL_TEMPLATE "Level %d %s" / UNIT_LEVEL_TEMPLATE "Level %d".
        let text = match kind {
            Some(kind) => format!("Level {level} {kind}"),
            None => format!("Level {level}"),
        };
        lines.push(TooltipLineState::colored(text, TOOLTIP_WHITE));
    }
    if let Some(faction) = input.faction {
        lines.push(TooltipLineState::colored(faction, TOOLTIP_WHITE));
    }
    if let Some(data) = input.data {
        let drops: Vec<SectionItem> = data
            .drops
            .iter()
            .map(|drop| SectionItem::new(&drop.item, Some(drop.chance), collection))
            .collect();
        let sold: Vec<SectionItem> = data
            .vendor_items
            .iter()
            .map(|item| SectionItem::new(item, None, collection))
            .collect();
        lines.extend(section_lines("Drops", &drops));
        lines.extend(section_lines("Sells", &sold));
    }
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: input.name.to_string(),
        title_color: npc_name_color(input.reaction),
        lines,
        record: Some(TooltipRecord::Creature(input.entry)),
        // GameTooltip:SetWorldCursor (GameTooltip.lua:977-1020) and
        // UnitFrame_UpdateTooltip (UnitFrame.lua:390-391): the default anchor.
        anchor: TooltipAnchor::Default,
    }
}

/// What a player tooltip reads from the hovered unit.
pub(super) struct PlayerTooltipInput<'a> {
    pub name: &'a str,
    pub reaction: Reaction,
    pub guild: Option<&'a str>,
    pub level: Option<u8>,
    pub race: &'a str,
    pub class: &'a str,
}

pub(super) fn player_tooltip(input: &PlayerTooltipInput) -> TooltipFrameState {
    let mut lines = Vec::new();
    if let Some(guild) = input.guild {
        lines.push(TooltipLineState::colored(
            format!("<{guild}>"),
            TOOLTIP_WHITE,
        ));
    }
    let level = input
        .level
        .map_or_else(|| "??".to_string(), |level| level.to_string());
    lines.push(TooltipLineState::colored(
        format!("Level {level} {} {} (Player)", input.race, input.class),
        TOOLTIP_WHITE,
    ));
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: input.name.to_string(),
        title_color: player_name_color(input.reaction),
        lines,
        record: None,
        anchor: TooltipAnchor::Default,
    }
}

/// One listed item: its quality colour, its drop chance (vendor items have
/// none) and whether its appearance is collected (`None`: it has no appearance).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SectionItem {
    pub name: String,
    pub quality: u8,
    pub chance: Option<f32>,
    pub collected: Option<bool>,
}

impl SectionItem {
    fn new(item: &TooltipItem, chance: Option<f32>, collection: &AppearanceCollection) -> Self {
        Self {
            name: item.name.clone(),
            quality: item.quality,
            chance,
            collected: item.appearance_id.map(|id| collection.has(id)),
        }
    }

    fn mark(&self) -> ItemMark {
        match self.collected {
            Some(true) => ItemMark::Collected,
            Some(false) => ItemMark::Uncollected,
            None => ItemMark::Unmarked,
        }
    }
}

/// A header and the items, uncollected first, then by highest chance (vendor
/// items keep their slot order). More than `MAX_LISTED` items show the first
/// `SHOWN_WHEN_TRUNCATED` and "+N more", with "(k collected)" when the hidden
/// items include any with an appearance. No items, no section.
pub(super) fn section_lines(header: &str, items: &[SectionItem]) -> Vec<TooltipLineState> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut ordered: Vec<&SectionItem> = items.iter().collect();
    ordered.sort_by(|a, b| {
        let rank = |item: &SectionItem| u8::from(item.collected != Some(false));
        rank(a)
            .cmp(&rank(b))
            .then_with(|| b.chance.unwrap_or(0.0).total_cmp(&a.chance.unwrap_or(0.0)))
    });
    let shown = if ordered.len() > MAX_LISTED {
        SHOWN_WHEN_TRUNCATED
    } else {
        ordered.len()
    };
    let mut lines = vec![TooltipLineState::colored(header, TOOLTIP_DESCRIPTION_COLOR)];
    lines.extend(ordered[..shown].iter().map(|item| item_line(item)));
    let hidden = &ordered[shown..];
    if !hidden.is_empty() {
        lines.push(TooltipLineState::colored(
            more_text(hidden),
            TOOLTIP_LABEL_COLOR,
        ));
    }
    lines
}

fn item_line(item: &SectionItem) -> TooltipLineState {
    TooltipLineState {
        right_text: item.chance.map(chance_text).unwrap_or_default(),
        right_color: TOOLTIP_WHITE,
        item_mark: Some(item.mark()),
        ..TooltipLineState::colored(item.name.clone(), parse_rgba(quality_color(item.quality)))
    }
}

/// "+N more", after Retail `QUEST_HUB_TOOLTIP_MORE_QUESTS_REMAINING` ("+%d more"),
/// then "(k collected)" when any hidden item has an appearance.
fn more_text(hidden: &[&SectionItem]) -> String {
    let count = hidden.len();
    if hidden.iter().all(|item| item.collected.is_none()) {
        return format!("+{count} more");
    }
    let collected = hidden
        .iter()
        .filter(|item| item.collected == Some(true))
        .count();
    format!("+{count} more ({collected} collected)")
}

/// "60%", "20.1%", "0.04%"; below 0.01 % "<0.01%".
pub(super) fn chance_text(chance: f32) -> String {
    if chance < 0.01 {
        return "<0.01%".to_string();
    }
    if chance >= 1.0 {
        return format!("{}%", trim_number(chance));
    }
    let text = format!("{chance:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}%")
}

#[cfg(test)]
#[path = "unit_tooltip_tests.rs"]
mod tests;
