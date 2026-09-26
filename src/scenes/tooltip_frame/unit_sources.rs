//! What the unit tooltip reads about the hovered unit (`HoveredUnit`): its
//! replicated components, the reaction and reputation faction from the
//! FactionTemplate/Faction DB2s, the server's cached creature data and the
//! account's appearance collection.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::components::{
    GuildMembership, Npc, Player as NetPlayer, UnitFactionTemplate, UnitLevel,
};

use super::TooltipFrameState;
use super::unit_tooltip::{NpcTooltipInput, PlayerTooltipInput, npc_tooltip, player_tooltip};
use crate::csv_util::parse_csv_line;
use crate::game::networking_unit_tooltip::{AccountAppearances, CreatureTooltipCache};
use crate::networking::LocalPlayer;
use crate::rendering::unit_frames::FactionTemplates;
use crate::rendering::unit_hover::HoveredUnit;
use game_engine::char_create_data::{class_by_id, race_by_id};
use game_engine::faction_reaction::{Reaction, reaction};

const FACTION_CSV: &str = "data/db2/12.1.0.69933/Faction.csv";

/// `Faction.Name_lang` of reputation factions (`ReputationIndex` >= 0), by
/// Faction id: the faction line Retail shows under an NPC ("Stormwind").
/// Factions without a reputation ("Creature", "Beast - Wolf") have no line.
#[derive(Resource, Default)]
pub(super) struct FactionNames(HashMap<u32, String>);

impl FactionNames {
    pub(super) fn load() -> Self {
        let parsed = std::fs::read_to_string(FACTION_CSV)
            .map_err(|err| format!("read {FACTION_CSV}: {err}"))
            .and_then(|text| parse_faction_csv(&text));
        match parsed {
            Ok(names) => Self(names),
            Err(err) => {
                error!("Unit tooltips show no faction line: {err}");
                Self::default()
            }
        }
    }

    fn get(&self, faction: u32) -> Option<&str> {
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
            names.insert(faction, field(name).to_string());
        }
    }
    Ok(names)
}

type UnitData<'a> = (
    Option<&'a Npc>,
    Option<&'a NetPlayer>,
    Option<&'a UnitLevel>,
    Option<&'a UnitFactionTemplate>,
    Option<&'a GuildMembership>,
);

#[derive(SystemParam)]
pub(super) struct UnitTooltipSources<'w, 's> {
    hovered: Res<'w, HoveredUnit>,
    units: Query<'w, 's, UnitData<'static>>,
    local: Query<'w, 's, Option<&'static UnitFactionTemplate>, With<LocalPlayer>>,
    templates: Option<Res<'w, FactionTemplates>>,
    factions: Option<Res<'w, FactionNames>>,
    cache: ResMut<'w, CreatureTooltipCache>,
    appearances: Res<'w, AccountAppearances>,
}

impl UnitTooltipSources<'_, '_> {
    /// Ask the server for the hovered NPC's data the first time it is hovered.
    pub(super) fn request_hovered(&mut self) {
        let entry = self
            .hovered
            .0
            .and_then(|unit| self.units.get(unit).ok())
            .and_then(|(npc, ..)| npc.map(|npc| npc.template_id));
        if let Some(entry) = entry {
            self.cache.request(entry);
        }
    }

    fn reaction_to(&self, faction: Option<&UnitFactionTemplate>) -> Reaction {
        let Some(templates) = self.templates.as_deref() else {
            return Reaction::Neutral;
        };
        let player = self.local.single().ok().flatten();
        reaction(templates.row(faction), templates.row(player))
    }

    fn faction_name(&self, faction: Option<&UnitFactionTemplate>) -> Option<&str> {
        let row = self.templates.as_deref()?.row(faction)?;
        self.factions.as_deref()?.get(row.faction)
    }

    pub(super) fn tooltip(&self) -> Option<TooltipFrameState> {
        let (npc, player, level, faction, guild) = self.units.get(self.hovered.0?).ok()?;
        let level = displayed_level(level);
        let reaction = self.reaction_to(faction);
        if let Some(player) = player {
            return Some(player_tooltip(&PlayerTooltipInput {
                name: &player.name,
                reaction,
                guild: guild.map(|guild| guild.guild_name.as_str()),
                level,
                race: race_by_id(player.race).map_or("Unknown", |race| race.name),
                class: class_by_id(player.class).map_or("Unknown", |class| class.name),
            }));
        }
        let npc = npc?;
        Some(npc_tooltip(
            &NpcTooltipInput {
                entry: npc.template_id,
                name: &npc.name,
                reaction,
                level,
                faction: self.faction_name(faction),
                data: self.cache.get(npc.template_id),
            },
            &self.appearances.0,
        ))
    }
}

/// The level the tooltip shows: the unit's replicated level, as the target frame
/// shows it. The one place to switch to the viewer-scaled level
/// (`shared::level_scaling::level_for_viewer`, branch levelscaling).
fn displayed_level(level: Option<&UnitLevel>) -> Option<u8> {
    level.map(|level| level.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_reputation_factions_are_named() {
        let text = std::fs::read_to_string(FACTION_CSV).expect("Faction.csv");

        let names = parse_faction_csv(&text).unwrap();

        assert_eq!(names.get(&72).map(String::as_str), Some("Stormwind"));
        assert_eq!(names.get(&47).map(String::as_str), Some("Ironforge"));
        assert_eq!(names.get(&7), None, "Creature");
        assert_eq!(names.get(&29), None, "Beast - Wolf");
        assert_eq!(names.get(&1), None, "PLAYER, Human");
    }
}
