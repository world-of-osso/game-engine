//! Ordinary replica projection for HARNESS_PULL_COVERAGE_KNOWN_VISIBLE v2.
//! Decoder progress and observed deaths are local facts, not server coverage authority.
use std::collections::{BTreeSet, HashMap, HashSet};

use game_engine_network::replica::{Replica, Unit};
use godot::prelude::*;
use serde::Serialize;
use shared::{
    components::{
        CombatStatus, Health, ModelDisplay, MovementControl, Npc, Position, UnitAuras,
        UnitFactionTemplate, UnitLevel, UnitScale, WorldArrival,
    },
    level_scaling::LevelScaling,
};

use crate::faction_reaction::{FactionTemplateEntry, Reaction, reaction};

type EntryEpoch = (u64, u32, u32, u32);

#[derive(Default)]
pub(crate) struct PullUnits {
    entry: Option<EntryEpoch>,
    update_baseline: u64,
    full_update: bool,
    living: HashSet<u64>,
    dying_entries: BTreeSet<u32>,
}

fn entry_epoch(replica: &Replica, player: Option<u64>, map: Option<u32>) -> Option<EntryEpoch> {
    let player = player?;
    let unit = replica.unit(player)?;
    Some((
        player,
        map?,
        unit.get::<WorldArrival>()?.0,
        unit.get::<MovementControl>()?.epoch,
    ))
}

impl PullUnits {
    pub(crate) fn observe(
        &mut self,
        replica: &Replica,
        player: Option<u64>,
        map: Option<u32>,
        updates: u64,
    ) {
        let epoch = entry_epoch(replica, player, map);
        if epoch != self.entry || epoch.is_none() || updates < self.update_baseline {
            *self = Self {
                entry: epoch,
                update_baseline: updates,
                ..Self::default()
            };
        }
        self.full_update = self.entry.is_some() && updates > self.update_baseline;
        if self.entry.is_some() {
            self.observe_deaths(replica);
        }
    }

    fn observe_deaths(&mut self, replica: &Replica) {
        for unit in replica.units() {
            let (Some(npc), Some(health)) = (unit.get::<Npc>(), unit.get::<Health>()) else {
                continue;
            };
            if health.current > 0.0 {
                self.living.insert(unit.server_id);
            } else if health.current == 0.0 && self.living.remove(&unit.server_id) {
                self.dying_entries.insert(npc.template_id);
            }
        }
    }

    pub(crate) fn full_update_after_entry(&self) -> bool {
        self.full_update
    }

    pub(crate) fn observed_dying_entries(&self) -> Vec<u32> {
        self.dying_entries.iter().copied().collect()
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct PullUnit {
    id: u64,
    entry: u32,
    position: [f32; 3],
    health: f32,
    combat: bool,
    hostile: bool,
    faction: u32,
    level: u8,
    scale: f32,
    display: u32,
    scaling: Option<LevelScaling>,
    auras: Vec<PullAura>,
}

#[derive(Debug, Serialize)]
struct PullAura {
    spell_id: u32,
    stacks: u8,
}

fn require_component<C: 'static>(unit: Unit<'_>, name: &str) -> Result<C, String>
where
    C: Clone,
{
    unit.get::<C>()
        .cloned()
        .ok_or_else(|| format!("Pull unit {} missing {name}", unit.server_id))
}

fn project_geometry(unit: Unit<'_>) -> Result<([f32; 3], f32, f32), String> {
    let position = require_component::<Position>(unit, "position")?;
    let position = [position.x, position.y, position.z];
    let health = require_component::<Health>(unit, "health")?.current;
    let scale = require_component::<UnitScale>(unit, "scale")?.0;
    let valid_scale = scale.is_finite() && scale > 0.0;
    let valid_position = position.iter().all(|v| v.is_finite());
    if !health.is_finite() || !valid_scale || !valid_position {
        return Err(format!(
            "Pull unit {} non-finite position/life/scale",
            unit.server_id
        ));
    }
    Ok((position, health, scale))
}

fn project_auras(unit: Unit<'_>) -> Result<Vec<PullAura>, String> {
    Ok(require_component::<UnitAuras>(unit, "auras")?
        .auras
        .into_iter()
        .map(|aura| PullAura {
            spell_id: aura.spell_id,
            stacks: aura.stacks,
        })
        .collect())
}

fn project_unit(
    unit: Unit<'_>,
    tank: &FactionTemplateEntry,
    templates: &HashMap<u32, FactionTemplateEntry>,
) -> Result<PullUnit, String> {
    let faction = require_component::<UnitFactionTemplate>(unit, "faction")?.0;
    let template = templates
        .get(&faction)
        .ok_or_else(|| format!("Pull faction {faction} unavailable"))?;
    let (position, health, scale) = project_geometry(unit)?;
    Ok(PullUnit {
        id: unit.server_id,
        entry: require_component::<Npc>(unit, "NPC")?.template_id,
        position,
        health,
        scale,
        faction,
        hostile: reaction(Some(template), Some(tank)) == Reaction::Hostile,
        combat: require_component::<CombatStatus>(unit, "combat")?.0,
        level: require_component::<UnitLevel>(unit, "level")?.0,
        display: require_component::<ModelDisplay>(unit, "display")?.display_id,
        scaling: unit.get::<LevelScaling>().copied(),
        auras: project_auras(unit)?,
    })
}

pub(crate) fn collect_pull_units(
    replica: &Replica,
    tank: u64,
    templates: &HashMap<u32, FactionTemplateEntry>,
) -> Result<Vec<PullUnit>, String> {
    let tank_unit = replica.unit(tank).ok_or("Pull tank not received")?;
    let faction = require_component::<UnitFactionTemplate>(tank_unit, "tank faction")?.0;
    let tank_template = templates
        .get(&faction)
        .ok_or("Pull tank faction template unavailable")?;
    let mut units = replica
        .units()
        .filter(|unit| unit.has::<Npc>())
        .map(|unit| project_unit(unit, tank_template, templates))
        .collect::<Result<Vec<_>, _>>()?;
    units.sort_by_key(|unit| unit.id);
    Ok(units)
}

#[derive(Serialize)]
struct PullSnapshot {
    epoch: [u32; 3],
    full_update_after_entry: bool,
    observed_dying_entries: Vec<u32>,
    units: Vec<PullUnit>,
}

impl crate::GameClient {
    pub(super) fn pull_units_snapshot(&mut self, tank: u64) -> VarDictionary {
        let mut result = VarDictionary::new();
        match self.read_pull_units(tank) {
            Ok(json) => result.set("state_json", json),
            Err(error) => result.set("error", error),
        }
        result
    }

    fn read_pull_units(&mut self, tank: u64) -> Result<String, String> {
        if !self.account.is_connected() {
            return Err("Pull snapshot requires connected ordinary client".into());
        }
        let (_, map, arrival, teleport) = self
            .pull_units
            .entry
            .ok_or("Pull entry/teleport epoch unavailable")?;
        let templates = self.nameplates.templates(&self.data_root)?;
        let snapshot = PullSnapshot {
            epoch: [map, arrival, teleport],
            full_update_after_entry: self.pull_units.full_update_after_entry(),
            observed_dying_entries: self.pull_units.observed_dying_entries(),
            units: collect_pull_units(&self.replica, tank, templates)?,
        };
        serde_json::to_string(&snapshot).map_err(|error| error.to_string())
    }
}
