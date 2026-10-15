//! One action resolves the authored next spell, then uses ordinary server-validated casting.
use std::collections::BTreeSet;

use game_engine_core::assisted_combat::{
    AssistedFacts, SINGLE_BUTTON_ASSISTANT, evaluate_rule, is_spell_usable,
};
use game_engine_core::cooldown_assist::Recommendation;
use game_engine_core::spell_catalog::CatalogSpell;
use godot::prelude::*;
use shared::components::{CombatStatus, Health, Position, UnitPowers, UnitRunes};

use super::cooldown_viewer::ViewerLoad;
use crate::GameClient;
use crate::faction_reaction::Reaction;
use crate::frame_error::report_once;

/// Inputs and first eligible authored step from one native evaluation, for replay.
#[derive(serde::Serialize)]
pub(super) struct AssistedEvaluation {
    specialization: u32,
    target: u64,
    facts: AssistedFacts,
    selection: Result<Recommendation, String>,
}

impl GameClient {
    pub(super) fn update_assisted_combat(&mut self) {
        self.spells.cooldowns.assisted_evaluation = None;
        if let Err(error) = self.spells.cooldowns.poll_data(&self.data_root) {
            report_once(&format!("Assisted Combat data: {error}"));
        }
        self.spells.cooldowns.assisted_available = self.authored_rotation_available();
        let decision = self.evaluate_and_record_assisted_recommendation();
        let hud = &mut self.spells.cooldowns;
        match decision {
            Ok(recommendation) => {
                hud.recommendation = recommendation;
                hud.assisted_failure.clear();
            }
            Err(error) => {
                hud.recommendation = Recommendation::None;
                hud.assisted_failure = error;
            }
        }
    }

    pub(crate) fn assisted_in_combat(&self) -> bool {
        self.world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.get::<CombatStatus>())
            .is_some_and(|combat| combat.0)
    }

    fn authored_rotation_available(&self) -> bool {
        let ViewerLoad::Ready(data) = &self.spells.cooldowns.load else {
            return false;
        };
        self.account
            .spells
            .spec()
            .is_some_and(|spec| !data.catalog.rotation_spells(spec).is_empty())
    }

    fn evaluate_and_record_assisted_recommendation(&mut self) -> Result<Recommendation, String> {
        if !self.spells.cooldowns.assisted_available {
            return Ok(Recommendation::Unavailable);
        }
        let Some(target) = self.targeting_target() else {
            return Ok(Recommendation::None);
        };
        let hostile = self.reaction_to(target) != Reaction::Friendly;
        let Some(target_unit) = self.replica.unit(target) else {
            return Err("Assisted target not replicated".into());
        };
        if target_unit
            .get::<Health>()
            .is_some_and(|health| health.current <= 0.0)
        {
            return Ok(Recommendation::None);
        }
        let spec = self
            .account
            .spells
            .spec()
            .ok_or("Assisted specialization not received")?;
        let ViewerLoad::Ready(data) = &self.spells.cooldowns.load else {
            return Err("Assisted data not loaded".into());
        };
        let known = self.account.spells.known();
        let mut needed: BTreeSet<_> = data
            .catalog
            .rotation_spells(spec)
            .into_iter()
            .filter(|id| known.contains(id) || known.contains(&self.effective_spell(*id)))
            .collect();
        for rule in data.catalog.rotation_rules(spec) {
            if needed.contains(&rule.step_spell)
                && matches!(rule.condition_type, 1 | 2 | 14 | 56 | 65 | 66)
                && rule.values[0] != 0
            {
                needed.insert(rule.values[0]);
            }
        }
        let mut facts = self.read_assisted_facts(&needed, target)?;
        facts.target_hostile = Some(hostile);
        let selection = data.catalog.recommend(
            spec,
            |id| facts.known.contains(&id),
            |id| is_spell_usable(id, &facts),
            |rule| evaluate_rule(rule, &facts),
        );
        self.spells.cooldowns.assisted_evaluation = Some(AssistedEvaluation {
            specialization: spec,
            target,
            facts,
            selection: selection.clone(),
        });
        selection.map(|recommendation| match recommendation {
            Recommendation::Spell(id) => Recommendation::Spell(self.effective_spell(id)),
            other => other,
        })
    }

    fn read_assisted_facts(
        &self,
        needed: &BTreeSet<u32>,
        target: u64,
    ) -> Result<AssistedFacts, String> {
        let player = self
            .world
            .local_player_id()
            .ok_or("Assisted player not replicated")?;
        let catalog = self
            .spells
            .catalog()
            .ok_or("Assisted spell metadata not loaded")?;
        let player_position = self
            .replica
            .unit(player)
            .and_then(|unit| unit.get::<Position>())
            .ok_or("Assisted player position not received")?;
        let target_position = self
            .replica
            .unit(target)
            .and_then(|unit| unit.get::<Position>())
            .ok_or("Assisted target position not received")?;
        let distance = position_distance(player_position, target_position);
        let mut powers: std::collections::BTreeMap<_, _> = self
            .replica
            .unit(player)
            .and_then(|unit| unit.get::<UnitPowers>())
            .into_iter()
            .flat_map(|pools| pools.entries.iter())
            .map(|entry| (entry.power as u8, f64::from(entry.current)))
            .collect();
        if let Some(runes) = self
            .replica
            .unit(player)
            .and_then(|unit| unit.get::<UnitRunes>())
        {
            powers.insert(
                5,
                runes
                    .ready_in_ms
                    .iter()
                    .filter(|remaining| **remaining == 0)
                    .count() as f64,
            );
        }
        let health_pct = |unit| {
            self.replica
                .unit(unit)
                .and_then(|unit| unit.get::<Health>())
                .filter(|health| health.max > 0.0)
                .map(|health| f64::from(health.current) * 100.0 / f64::from(health.max))
        };
        let mut facts = AssistedFacts {
            powers,
            player_health_pct: health_pct(player),
            target_health_pct: health_pct(target),
            known: self.account.spells.known().iter().copied().collect(),
            player_auras: self
                .unit_aura_views_now(player)
                .iter()
                .map(|aura| aura.spell_id)
                .collect(),
            target_auras: self
                .unit_aura_views_now(target)
                .iter()
                .map(|aura| aura.spell_id)
                .collect(),
            ..Default::default()
        };
        for id in needed {
            let effective = self.effective_spell(*id);
            if facts.known.contains(&effective) {
                facts.known.insert(*id);
            }
            let spell = catalog
                .get(effective)
                .ok_or_else(|| format!("Assisted spell metadata {effective} absent"))?;
            facts
                .ready
                .insert(*id, self.read_assisted_readiness(effective, spell));
            if let ViewerLoad::Ready(data) = &self.spells.cooldowns.load
                && let Some(category) = data.catalog.charge_category(effective)
                && let Some(charges) = self.account.spells.charges(category)
            {
                facts.charges.insert(*id, charges.current);
            }
            facts.cooldowns_ms.insert(
                *id,
                self.account
                    .spells
                    .button_cooldown(effective, false)
                    .map_or(0.0, |clock| f64::from(clock.remaining) * 1000.0),
            );
            facts
                .affordable
                .insert(*id, self.account.spells.affordable(effective));
            let in_range = distance >= spell.range.min_yd[0] && distance <= spell.range.max_yd[0];
            facts.in_range.insert(*id, Ok(in_range));
        }
        Ok(facts)
    }

    fn read_assisted_readiness(&self, id: u32, spell: &CatalogSpell) -> Result<bool, String> {
        if self.account.spells.button_cooldown(id, false).is_some() {
            return Ok(false);
        }
        if spell.charges.is_none() {
            return Ok(true);
        }
        let ViewerLoad::Ready(data) = &self.spells.cooldowns.load else {
            return Err("Assisted charge categories not loaded".into());
        };
        let category = data
            .catalog
            .charge_category(id)
            .ok_or_else(|| format!("Assisted spell {id}: charge category absent"))?;
        let charges = self.account.spells.charges(category).ok_or_else(|| {
            format!("Assisted spell {id}: server charges category {category} not received")
        })?;
        Ok(charges.current > 0)
    }

    pub(crate) fn spell_priority_snapshot(&self, target: i64, requested: &[i32]) -> VarDictionary {
        let mut result = VarDictionary::new();
        let query = self
            .read_priority_state(target, requested)
            .and_then(|state| serde_json::to_string(&state).map_err(|error| error.to_string()));
        match query {
            Ok(state) => result.set("state_json", state.as_str()),
            Err(error) => result.set("error", error.as_str()),
        };
        result
    }

    fn read_priority_state(
        &self,
        target: i64,
        requested: &[i32],
    ) -> Result<serde_json::Value, String> {
        let target = u64::try_from(target)
            .ok()
            .filter(|id| *id > 0)
            .ok_or("Priority target must be positive")?;
        let needed = parse_priority_spell_ids(requested)?;
        // This read-only query does not load the UI's mutable faction cache.
        let facts = self.read_assisted_facts(&needed, target)?;
        let player = self
            .world
            .local_player_id()
            .ok_or("Priority player not replicated")?;
        let stats = self
            .replica
            .unit(player)
            .and_then(|unit| unit.get::<shared::components::DerivedStats>())
            .ok_or("Priority player derived stats not received")?;
        Ok(serde_json::json!({
            "target": target,
            "facts": facts,
            "player_haste_pct": stats.haste_pct,
            "spell_gcd_ms": self.read_priority_gcds(&needed)?,
            "target_cast": self.priority_target_cast(target),
        }))
    }

    fn read_priority_gcds(
        &self,
        needed: &BTreeSet<u32>,
    ) -> Result<std::collections::BTreeMap<u32, u32>, String> {
        let catalog = self
            .spells
            .catalog()
            .ok_or("Priority spell catalog not loaded")?;
        needed
            .iter()
            .map(|id| {
                let spell = catalog
                    .get(self.effective_spell(*id))
                    .ok_or_else(|| format!("Priority spell metadata {id} absent"))?;
                Ok((*id, spell.cooldown.gcd_ms))
            })
            .collect()
    }

    fn priority_target_cast(&self, target: u64) -> serde_json::Value {
        let bar = self.cast_bars.get(target);
        serde_json::json!({
            "casting": bar.is_some_and(|bar| bar.casting),
            "channeling": bar.is_some_and(|bar| bar.channeling),
            "interruptible": bar.is_some_and(|bar| {
                (bar.casting || bar.channeling)
                    && bar.bar_type != crate::nameplate_casts::BarType::Uninterruptable
            }),
            "spell_id": bar.filter(|bar| bar.casting || bar.channeling).map(|bar| bar.spell_id),
        })
    }

    pub(crate) fn assisted_snapshot(&self) -> VarDictionary {
        let hud = &self.spells.cooldowns;
        let mut state = VarDictionary::new();
        state.set("available", hud.assisted_available);
        state.set("in_combat", self.assisted_in_combat());
        state.set("failure_reason", hud.assisted_failure.as_str());
        state.set(
            "action_spell",
            if hud.assisted_available {
                SINGLE_BUTTON_ASSISTANT
            } else {
                0
            },
        );
        state.set(
            "next_spell",
            match hud.recommendation {
                Recommendation::Spell(id) => id,
                _ => 0,
            },
        );
        let rotation: Vec<_> = match &hud.load {
            ViewerLoad::Ready(data) => self
                .account
                .spells
                .spec()
                .map(|spec| data.catalog.rotation_spells(spec))
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let mut spells = Array::<u32>::new();
        for id in rotation {
            spells.push(id);
        }
        state.set("rotation_spells", &spells);
        self.write_assisted_evaluation(&mut state);
        state
    }

    fn write_assisted_evaluation(&self, state: &mut VarDictionary) {
        let Some(evaluation) = &self.spells.cooldowns.assisted_evaluation else {
            return;
        };
        match serde_json::to_string(evaluation) {
            Ok(encoded) => state.set("evaluation_json", encoded.as_str()),
            Err(error) => {
                let message = format!("Assisted evaluation snapshot: {error}");
                state.set("evaluation_error", message.as_str());
                report_once(&message);
            }
        }
    }
}

fn parse_priority_spell_ids(requested: &[i32]) -> Result<BTreeSet<u32>, String> {
    if requested.is_empty() {
        return Err("Priority query requires at least one spell".into());
    }
    requested
        .iter()
        .map(|id| {
            u32::try_from(*id)
                .ok()
                .filter(|id| *id > 0)
                .ok_or_else(|| format!("Priority spell must be positive: {id}"))
        })
        .collect()
}

fn position_distance(a: &Position, b: &Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}
