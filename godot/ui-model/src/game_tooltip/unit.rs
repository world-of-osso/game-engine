//! Unit tooltips (docs/specs/unit-tooltip.md), ported from the Bevy
//! `scenes/tooltip_frame/unit_tooltip.rs`: Retail `GameTooltip` unit lines for NPCs and
//! players and, below an NPC's lines, the user-requested drops and vendor sections marked
//! against the account's appearance collection.

use std::collections::HashMap;
use std::path::Path;

use shared::components::CreatureClassification;
use shared::faction_reaction::Reaction;
use shared::protocol::{CreatureTooltip, TooltipItem};
use shared::transmog::AppearanceCollection;

use super::spell::trim_number;
use super::{GameTooltip, TooltipRecord};
use crate::csv_util::parse_csv_line;
use crate::merchant_data::quality_color;
use crate::tooltip_presentation::{
    ItemMark, TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_LABEL_COLOR, TOOLTIP_WHITE, TooltipBorder,
    TooltipLineState, TooltipPresentation, parse_rgba,
};

/// `FACTION_BAR_COLORS` by `UnitReaction` (SharedColorConstants.lua:3-13): 2 red (hostile),
/// 4 yellow (neutral), 5 green (friendly).
pub const FACTION_RED: [f32; 4] = [0.8, 0.13, 0.13, 1.0];
pub const FACTION_YELLOW: [f32; 4] = [0.8, 0.73, 0.13, 1.0];
pub const FACTION_GREEN: [f32; 4] = [0.13, 0.8, 0.13, 1.0];

/// A section lists every item up to this many; beyond it, `SHOWN_WHEN_TRUNCATED` items and
/// a "+N more" line (user requirement).
const MAX_LISTED: usize = 6;
const SHOWN_WHEN_TRUNCATED: usize = 5;

/// `GameTooltip_UnitColor` for an NPC: `FACTION_BAR_COLORS[UnitReaction]`.
pub fn npc_name_color(reaction: Reaction) -> [f32; 4] {
    match reaction {
        Reaction::Hostile => FACTION_RED,
        Reaction::Neutral => FACTION_YELLOW,
        Reaction::Friendly => FACTION_GREEN,
    }
}

/// `GameTooltip_UnitColor` for a player: red when hostile, else white (PvP flagging is not
/// modelled).
pub fn player_name_color(reaction: Reaction) -> [f32; 4] {
    match reaction {
        Reaction::Hostile => FACTION_RED,
        Reaction::Neutral | Reaction::Friendly => TOOLTIP_WHITE,
    }
}

/// Retail `CreatureType` names (CreatureType.db2); 10 "Not specified" and unknown types
/// show no type.
pub fn creature_type_name(creature_type: u8) -> Option<&'static str> {
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
pub struct NpcTooltipInput<'a> {
    /// Creature template entry (`Npc::template_id`).
    pub entry: u32,
    pub name: &'a str,
    pub reaction: Reaction,
    pub level: Option<u8>,
    pub classification: CreatureClassification,
    /// Name of the unit's reputation faction (`Faction.ReputationIndex` >= 0).
    pub faction: Option<&'a str>,
    /// The server's answer for the unit's creature entry, once it arrived.
    pub data: Option<&'a CreatureTooltip>,
}

/// Name in its reaction colour, subname, `UNIT_TYPE_LEVEL_TEMPLATE` "Level %d %s" (or
/// `UNIT_LEVEL_TEMPLATE` without a known type), the reputation faction, then the Drops and
/// Sells sections. `GameTooltip:SetWorldCursor` and `UnitFrame_UpdateTooltip` use the
/// default anchor.
pub fn npc_tooltip(input: &NpcTooltipInput, collection: &AppearanceCollection) -> GameTooltip {
    let mut lines = npc_identity_lines(input);
    if let Some(data) = input.data {
        lines.extend(npc_sections(data, collection));
    }
    GameTooltip::new(
        TooltipPresentation {
            title: input.name.to_owned(),
            title_color: npc_name_color(input.reaction),
            lines,
            border: TooltipBorder::Reaction(input.reaction),
            ..TooltipPresentation::hidden()
        },
        Some(TooltipRecord::Creature(input.entry)),
    )
}

/// `UNIT_TYPE_LEVEL_TEMPLATE` "Level %d %s", for elites `UNIT_TYPE_PLUS_LEVEL_TEMPLATE`
/// "Level %d Elite %s", for world bosses `UNIT_TYPE_LETHAL_LEVEL_TEMPLATE` "Level ?? %s"
/// (GlobalStrings 10997-10999; without a type `UNIT_LEVEL_TEMPLATE`,
/// `UNIT_PLUS_LEVEL_TEMPLATE`, `UNIT_LETHAL_LEVEL_TEMPLATE`, 10253-10255). Rares put
/// `MAP_LEGEND_RARE` "Rare" or `MAP_LEGEND_RAREELITE` "Rare Elite" where elites put "Elite".
pub fn npc_level_text(
    level: u8,
    classification: CreatureClassification,
    kind: Option<&str>,
) -> String {
    let level = match classification {
        CreatureClassification::WorldBoss => "??".to_owned(),
        _ => level.to_string(),
    };
    let rank = match classification {
        CreatureClassification::Elite => Some("Elite"),
        CreatureClassification::Rare => Some("Rare"),
        CreatureClassification::RareElite => Some("Rare Elite"),
        _ => None,
    };
    ["Level", &level]
        .into_iter()
        .chain(rank)
        .chain(kind)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Subname, level and type, reputation faction.
fn npc_identity_lines(input: &NpcTooltipInput) -> Vec<TooltipLineState> {
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
        let text = npc_level_text(level, input.classification, kind);
        lines.push(TooltipLineState::colored(text, TOOLTIP_WHITE));
    }
    if let Some(faction) = input.faction {
        lines.push(TooltipLineState::colored(faction, TOOLTIP_WHITE));
    }
    lines
}

fn npc_sections(
    data: &CreatureTooltip,
    collection: &AppearanceCollection,
) -> Vec<TooltipLineState> {
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
    let mut lines = section_lines("Drops", &drops);
    lines.extend(section_lines("Sells", &sold));
    lines
}

/// What a player tooltip reads from the hovered unit.
pub struct PlayerTooltipInput<'a> {
    pub name: &'a str,
    pub reaction: Reaction,
    pub guild: Option<&'a str>,
    pub level: Option<u8>,
    pub race: &'a str,
    pub class: &'a str,
    /// `ChrClasses` id of `class`.
    pub class_id: u8,
}

/// Name, "<Guild>", "Level %d %s %s (Player)".
pub fn player_tooltip(input: &PlayerTooltipInput) -> GameTooltip {
    let mut lines = Vec::new();
    if let Some(guild) = input.guild {
        lines.push(TooltipLineState::colored(
            format!("<{guild}>"),
            TOOLTIP_WHITE,
        ));
    }
    let level = input
        .level
        .map_or_else(|| "??".to_owned(), |level| level.to_string());
    lines.push(TooltipLineState::colored(
        format!("Level {level} {} {} (Player)", input.race, input.class),
        TOOLTIP_WHITE,
    ));
    GameTooltip::new(
        TooltipPresentation {
            title: input.name.to_owned(),
            title_color: player_name_color(input.reaction),
            lines,
            border: TooltipBorder::Class(input.class_id),
            ..TooltipPresentation::hidden()
        },
        None,
    )
}

/// One listed item: its quality colour, its drop chance (vendor items have none) and
/// whether its appearance is collected (`None`: it has no appearance).
#[derive(Clone, Debug, PartialEq)]
struct SectionItem {
    name: String,
    quality: u8,
    chance: Option<f32>,
    collected: Option<bool>,
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

/// A header and the items, uncollected first, then by highest chance (vendor items keep
/// their slot order). More than `MAX_LISTED` items show the first `SHOWN_WHEN_TRUNCATED` and
/// "+N more", with "(k collected)" when the hidden items include any with an appearance.
fn section_lines(header: &str, items: &[SectionItem]) -> Vec<TooltipLineState> {
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

/// "+N more" (Retail `QUEST_HUB_TOOLTIP_MORE_QUESTS_REMAINING`), then "(k collected)" when
/// any hidden item has an appearance.
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
pub fn chance_text(chance: f32) -> String {
    if chance < 0.01 {
        return "<0.01%".to_owned();
    }
    if chance >= 1.0 {
        return format!("{}%", trim_number(chance));
    }
    let text = format!("{chance:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}%")
}

/// `Faction.Name_lang` of reputation factions (`ReputationIndex` >= 0), by Faction id: the
/// faction line Retail shows under an NPC ("Stormwind"). Factions without a reputation
/// ("Creature", "Beast - Wolf") have no line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FactionNames(HashMap<u32, String>);

impl FactionNames {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|err| format!("read {}: {err}", path.display()))?;
        parse_faction_csv(&text).map(Self)
    }

    pub fn get(&self, faction: u32) -> Option<&str> {
        self.0.get(&faction).map(String::as_str)
    }
}

fn parse_faction_csv(text: &str) -> Result<HashMap<u32, String>, String> {
    let mut lines = text.lines();
    let header = parse_csv_line(lines.next().ok_or("Faction.csv is empty")?);
    let column = |name: &str| {
        header
            .iter()
            .position(|column| column == name)
            .ok_or_else(|| format!("Faction.csv has no {name} column"))
    };
    let (id, name, reputation) = (
        column("ID")?,
        column("Name_lang")?,
        column("ReputationIndex")?,
    );
    let mut names = HashMap::new();
    for line in lines {
        let fields = parse_csv_line(line);
        let field = |index: usize| fields.get(index).map(String::as_str).unwrap_or("");
        let has_reputation = field(reputation)
            .parse::<i32>()
            .is_ok_and(|index| index >= 0);
        if let (true, Ok(faction)) = (has_reputation, field(id).parse::<u32>()) {
            names.insert(faction, field(name).to_owned());
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::TooltipDrop;

    fn item(item_id: u32, name: &str, quality: u8, appearance_id: Option<u32>) -> TooltipItem {
        TooltipItem {
            item_id,
            name: name.into(),
            quality,
            appearance_id,
        }
    }

    fn drop(item: TooltipItem, chance: f32) -> TooltipDrop {
        TooltipDrop { item, chance }
    }

    /// Defias Thug (creature 38) with the drops the server lists for it.
    fn defias_thug() -> CreatureTooltip {
        CreatureTooltip {
            entry: 38,
            subname: String::new(),
            creature_type: 7,
            drops: vec![
                drop(item(1251, "Linen Bandage", 1, None), 1.5),
                drop(item(2589, "Linen Cloth", 1, None), 0.9),
                drop(item(2073, "Red Burlap Bandana", 1, None), 60.0),
                drop(item(2287, "Darnassian Bleu", 1, None), 20.1),
                drop(item(1179, "Refreshing Spring Water", 1, None), 9.9),
                drop(item(5571, "Small Red Pouch", 1, None), 0.04),
                drop(item(1411, "Withered Staff", 0, None), 0.02),
                drop(item(2138, "Sharpened Letter Opener", 2, None), 0.005),
                drop(item(2057, "Pitted Defias Shortsword", 2, Some(4_212)), 2.0),
            ],
            vendor_items: Vec::new(),
        }
    }

    fn rows(tooltip: &GameTooltip) -> Vec<(Option<ItemMark>, &str, &str)> {
        tooltip
            .content
            .lines
            .iter()
            .map(|line| {
                (
                    line.item_mark,
                    line.left_text.as_str(),
                    line.right_text.as_str(),
                )
            })
            .collect()
    }

    fn thug_input(data: &CreatureTooltip) -> NpcTooltipInput<'_> {
        NpcTooltipInput {
            entry: 38,
            name: "Defias Thug",
            reaction: Reaction::Hostile,
            level: Some(3),
            classification: CreatureClassification::Normal,
            faction: None,
            data: Some(data),
        }
    }

    /// Timber (world.db creature_template 1132: level 10, rank 4, type 1 Beast).
    #[test]
    fn timber_reads_level_10_rare_beast() {
        super::super::set_test_data_root();
        let data = CreatureTooltip {
            entry: 1_132,
            subname: String::new(),
            creature_type: 1,
            drops: Vec::new(),
            vendor_items: Vec::new(),
        };
        let input = NpcTooltipInput {
            entry: 1_132,
            name: "Timber",
            reaction: Reaction::Hostile,
            level: Some(10),
            classification: CreatureClassification::Rare,
            faction: None,
            data: Some(&data),
        };
        let tooltip = npc_tooltip(&input, &AppearanceCollection::default());
        assert_eq!(rows(&tooltip), [(None, "Level 10 Rare Beast", "")]);
    }

    #[test]
    fn elite_rare_elite_and_world_boss_level_lines() {
        use CreatureClassification::{Elite, RareElite, WorldBoss};
        // Hogger (448): level 11 elite humanoid.
        assert_eq!(
            npc_level_text(11, Elite, Some("Humanoid")),
            "Level 11 Elite Humanoid"
        );
        // Bruegal Ironknuckle (1720): level 25 rare elite, before the type arrives.
        assert_eq!(npc_level_text(25, RareElite, None), "Level 25 Rare Elite");
        // Azuregos (6109): level 63 dragonkin, rank 3.
        assert_eq!(
            npc_level_text(63, WorldBoss, Some("Dragonkin")),
            "Level ?? Dragonkin"
        );
    }

    #[test]
    fn defias_thug_lists_uncollected_first_then_by_chance_and_truncates() {
        super::super::set_test_data_root();
        let data = defias_thug();
        let tooltip = npc_tooltip(&thug_input(&data), &AppearanceCollection::default());
        assert_eq!(tooltip.content.title, "Defias Thug");
        assert_eq!(tooltip.content.title_color, FACTION_RED);
        assert_eq!(tooltip.record, Some(TooltipRecord::Creature(38)));
        use ItemMark::{Uncollected, Unmarked};
        assert_eq!(
            rows(&tooltip),
            [
                (None, "Level 3 Humanoid", ""),
                (None, "Drops", ""),
                (Some(Uncollected), "Pitted Defias Shortsword", "2%"),
                (Some(Unmarked), "Red Burlap Bandana", "60%"),
                (Some(Unmarked), "Darnassian Bleu", "20.1%"),
                (Some(Unmarked), "Refreshing Spring Water", "9.9%"),
                (Some(Unmarked), "Linen Bandage", "1.5%"),
                (None, "+4 more", ""),
            ]
        );
        assert_eq!(tooltip.content.lines[3].left_color, TOOLTIP_WHITE);
        assert_eq!(
            tooltip.content.lines[2].left_color,
            parse_rgba(quality_color(2))
        );
    }

    #[test]
    fn a_collected_appearance_shows_the_check_and_loses_its_priority() {
        super::super::set_test_data_root();
        let data = defias_thug();
        let mut collection = AppearanceCollection::default();
        collection.learn(4_212);
        let tooltip = npc_tooltip(&thug_input(&data), &collection);
        let rows = rows(&tooltip);
        assert_eq!(
            rows[2],
            (Some(ItemMark::Unmarked), "Red Burlap Bandana", "60%")
        );
        assert_eq!(
            rows[5],
            (Some(ItemMark::Collected), "Pitted Defias Shortsword", "2%")
        );
        assert_eq!(rows[7], (None, "+4 more", ""));
    }

    #[test]
    fn before_the_server_answers_the_npc_shows_name_level_and_faction() {
        super::super::set_test_data_root();
        let input = NpcTooltipInput {
            entry: 1_423,
            name: "Stormwind Guard",
            reaction: Reaction::Friendly,
            level: Some(30),
            classification: CreatureClassification::Normal,
            faction: Some("Stormwind"),
            data: None,
        };
        let tooltip = npc_tooltip(&input, &AppearanceCollection::default());
        assert_eq!(tooltip.content.title_color, FACTION_GREEN);
        assert_eq!(
            rows(&tooltip),
            [(None, "Level 30", ""), (None, "Stormwind", "")]
        );
    }

    #[test]
    fn vendor_items_keep_slot_order_without_chances() {
        super::super::set_test_data_root();
        let data = CreatureTooltip {
            entry: 54,
            subname: "Armorer".into(),
            creature_type: 7,
            drops: Vec::new(),
            vendor_items: vec![
                item(2117, "Thin Cloth Shoes", 1, Some(7_015)),
                item(159, "Refreshing Spring Water", 1, None),
            ],
        };
        let input = NpcTooltipInput {
            entry: 54,
            name: "Corina Steele",
            reaction: Reaction::Friendly,
            level: Some(10),
            classification: CreatureClassification::Normal,
            faction: Some("Stormwind"),
            data: Some(&data),
        };
        let tooltip = npc_tooltip(&input, &AppearanceCollection::default());
        assert_eq!(
            rows(&tooltip),
            [
                (None, "Armorer", ""),
                (None, "Level 10 Humanoid", ""),
                (None, "Stormwind", ""),
                (None, "Sells", ""),
                (Some(ItemMark::Uncollected), "Thin Cloth Shoes", ""),
                (Some(ItemMark::Unmarked), "Refreshing Spring Water", ""),
            ]
        );
    }

    #[test]
    fn players_show_guild_level_race_and_class_and_no_record() {
        super::super::set_test_data_root();
        let input = PlayerTooltipInput {
            name: "Tradea",
            reaction: Reaction::Friendly,
            guild: Some("Osso"),
            level: Some(12),
            race: "Human",
            class: "Mage",
            class_id: 8,
        };
        let tooltip = player_tooltip(&input);
        assert_eq!(tooltip.content.title_color, TOOLTIP_WHITE);
        assert_eq!(tooltip.record, None);
        assert_eq!(
            rows(&tooltip),
            [
                (None, "<Osso>", ""),
                (None, "Level 12 Human Mage (Player)", "")
            ]
        );
    }

    #[test]
    fn chances_read_like_the_spec() {
        assert_eq!(chance_text(60.0), "60%");
        assert_eq!(chance_text(20.1), "20.1%");
        assert_eq!(chance_text(0.04), "0.04%");
        assert_eq!(chance_text(0.005), "<0.01%");
    }

    #[test]
    fn only_reputation_factions_are_named() {
        let names = FactionNames::load(&crate::paths::resolve_data_path(
            "db2/12.1.0.69933/Faction.csv",
        ))
        .expect("Faction.csv");
        assert_eq!(names.get(72), Some("Stormwind"));
        assert_eq!(names.get(7), None, "Creature has no reputation");
    }
}
