//! Authored Retail CooldownSet and AssistedCombat joins, not inferred spell lists.
//! See docs/wiki/investigations/cooldown-assist-data.md for provenance and gaps.

use crate::spell_catalog::csv_records::CsvTable;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CooldownEntry {
    pub id: u32,
    pub spell_id: u32,
    pub category: u32,
    pub order: u32,
    pub flags: u32,
    pub player_condition_id: u32,
    pub spell_category_id: u32,
    pub equip_slot: u32,
    pub field_12_1_0_68209_009: u32,
    pub linked_spells: Vec<CooldownLinkedSpell>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CooldownLinkedSpell {
    pub spell_id: u32,
    pub field_11_1_5_59571_001: u32,
}

impl CooldownEntry {
    pub fn hidden_by_default(&self) -> bool {
        self.flags & 2 != 0
    }

    pub fn hides_aura(&self) -> bool {
        self.flags & 1 != 0
    }
}

#[derive(Clone, Debug)]
struct CooldownSet {
    spec: u32,
    player_condition: u32,
    entries: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistedRule {
    pub id: u32,
    pub step_spell: u32,
    pub condition_type: u32,
    pub values: [u32; 3],
    pub field_11_1_7_60520_002: u32,
}

#[derive(Clone, Debug)]
struct AssistedStep {
    id: u32,
    spell: u32,
    order: u32,
    rules: Vec<(u32, AssistedRule)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recommendation {
    Spell(u32),
    /// Spec has no authored rotation. Never substitute another spec's list.
    Unavailable,
    /// An authored list exists but no step's predicates pass now.
    None,
}

#[derive(Clone, Debug, Default)]
pub struct CooldownAssistCatalog {
    sets: BTreeMap<u32, CooldownSet>,
    entries: BTreeMap<u32, CooldownEntry>,
    rotations: BTreeMap<u32, Vec<AssistedStep>>,
    charge_categories: BTreeMap<u32, u32>,
}

impl CooldownAssistCatalog {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut data = Self::default();
        data.load_cooldowns(dir)?;
        data.load_rotations(dir)?;
        data.load_charge_categories(dir)?;
        Ok(data)
    }

    pub fn player_condition_ids(&self) -> BTreeSet<u32> {
        self.sets
            .values()
            .map(|set| set.player_condition)
            .chain(self.entries.values().map(|entry| entry.player_condition_id))
            .filter(|id| *id != 0)
            .collect()
    }

    pub fn charge_category(&self, spell: u32) -> Option<u32> {
        self.charge_categories.get(&spell).copied()
    }

    fn load_charge_categories(&mut self, dir: &Path) -> Result<(), String> {
        for [_, spell, difficulty, category] in read_columns(
            dir,
            "SpellCategories",
            ["ID", "SpellID", "DifficultyID", "ChargeCategory"],
        )? {
            if difficulty == 0 && category != 0 {
                self.charge_categories.insert(spell, category);
            }
        }
        Ok(())
    }

    pub fn entry(&self, id: u32) -> Option<&CooldownEntry> {
        self.entries.get(&id)
    }

    /// Includes spec-agnostic authored sets. Caller supplies real PlayerCondition state;
    /// an unavailable predicate propagates, rather than becoming an invented default.
    pub fn cooldowns(
        &self,
        spec: u32,
        mut condition: impl FnMut(u32) -> Result<bool, String>,
    ) -> Result<Vec<&CooldownEntry>, String> {
        let mut ids = BTreeSet::new();
        for set in self
            .sets
            .values()
            .filter(|set| set.spec == spec || set.spec == 0)
        {
            if !passes_player_condition(set.player_condition, &mut condition)? {
                continue;
            }
            for id in &set.entries {
                let entry = &self.entries[id];
                if passes_player_condition(entry.player_condition_id, &mut condition)? {
                    ids.insert(*id);
                }
            }
        }
        let mut entries: Vec<_> = ids.into_iter().map(|id| &self.entries[&id]).collect();
        entries.sort_by_key(|entry| (entry.category, entry.order, entry.id));
        Ok(entries)
    }

    pub fn rotation_rules(&self, spec: u32) -> Vec<&AssistedRule> {
        self.rotations
            .get(&spec)
            .into_iter()
            .flatten()
            .filter(|step| !automation_only(step))
            .flat_map(|step| step.rules.iter().map(|(_, rule)| rule))
            .collect()
    }

    pub fn rotation_spells(&self, spec: u32) -> Vec<u32> {
        let mut seen = BTreeSet::new();
        self.rotations
            .get(&spec)
            .into_iter()
            .flatten()
            .filter(|step| !automation_only(step))
            .filter_map(|step| seen.insert(step.spell).then_some(step.spell))
            .collect()
    }

    /// First authored eligible step. Rule predicates are evaluated against the same
    /// authoritative state as casting; callers must not treat unsupported rules as true.
    /// Unknown rules on a potentially eligible higher step block the recommendation.
    pub fn recommend(
        &self,
        spec: u32,
        mut learned: impl FnMut(u32) -> bool,
        mut usable: impl FnMut(u32) -> Result<bool, String>,
        mut evaluate: impl FnMut(&AssistedRule) -> Result<bool, String>,
    ) -> Result<Recommendation, String> {
        let Some(steps) = self.rotations.get(&spec) else {
            return Ok(Recommendation::Unavailable);
        };
        for step in steps {
            if automation_only(step) || !learned(step.spell) {
                continue;
            }
            let readiness = usable(step.spell);
            let predicates = passes_step(step, &mut evaluate);
            match (readiness, predicates) {
                (Ok(false), _) | (_, Ok(false)) => continue,
                (Err(error), _) | (_, Err(error)) => return Err(error),
                (Ok(true), Ok(true)) => return Ok(Recommendation::Spell(step.spell)),
            }
        }
        Ok(Recommendation::None)
    }

    fn load_cooldowns(&mut self, dir: &Path) -> Result<(), String> {
        self.load_sets(dir)?;
        self.load_entries(dir)?;
        self.load_links(dir)
    }

    fn load_sets(&mut self, dir: &Path) -> Result<(), String> {
        for [id, spec, field, player_condition] in read_columns(
            dir,
            "CooldownSet",
            [
                "ID",
                "ChrSpecializationID",
                "Field_11_1_5_59571_002",
                "PlayerConditionID",
            ],
        )? {
            if field != 0 {
                return Err(format!("CooldownSet {id}: unidentified field {field}"));
            }
            self.sets.insert(
                id,
                CooldownSet {
                    spec,
                    player_condition,
                    entries: Vec::new(),
                },
            );
        }
        Ok(())
    }

    fn load_entries(&mut self, dir: &Path) -> Result<(), String> {
        for [
            id,
            set_id,
            spell_id,
            category,
            player_condition_id,
            order,
            flags,
            spell_category_id,
            equip_slot,
            field,
        ] in read_columns(
            dir,
            "CooldownSetSpell",
            [
                "ID",
                "CooldownSetID",
                "SpellID",
                "Category",
                "PlayerConditionID",
                "OrderIndex",
                "Flags",
                "SpellCategoryID",
                "EquipSlot",
                "Field_12_1_0_68209_009",
            ],
        )? {
            let set = self
                .sets
                .get_mut(&set_id)
                .ok_or_else(|| format!("CooldownSetSpell {id}: missing set {set_id}"))?;
            set.entries.push(id);
            self.entries.insert(
                id,
                CooldownEntry {
                    id,
                    spell_id,
                    category,
                    order,
                    flags,
                    player_condition_id,
                    spell_category_id,
                    equip_slot,
                    field_12_1_0_68209_009: field,
                    linked_spells: Vec::new(),
                },
            );
        }
        Ok(())
    }

    fn load_links(&mut self, dir: &Path) -> Result<(), String> {
        for [id, spell, field, entry_id] in read_columns(
            dir,
            "CooldownSetLinkedSpell",
            [
                "ID",
                "SpellID",
                "Field_11_1_5_59571_001",
                "CooldownSetSpellID",
            ],
        )? {
            let entry = self
                .entries
                .get_mut(&entry_id)
                .ok_or_else(|| format!("CooldownSetLinkedSpell {id}: missing entry {entry_id}"))?;
            entry.linked_spells.push(CooldownLinkedSpell {
                spell_id: spell,
                field_11_1_5_59571_001: field,
            });
        }
        Ok(())
    }

    fn load_rotations(&mut self, dir: &Path) -> Result<(), String> {
        let mut steps = Self::read_steps(dir)?;
        Self::load_rules(dir, &mut steps)?;
        for (_, (spec, mut step)) in steps {
            step.rules.sort_by_key(|(order, rule)| (*order, rule.id));
            self.rotations.entry(spec).or_default().push(step);
        }
        for steps in self.rotations.values_mut() {
            steps.sort_by_key(|step| (step.order, step.id));
        }
        Ok(())
    }

    fn read_steps(dir: &Path) -> Result<BTreeMap<u32, (u32, AssistedStep)>, String> {
        let combat: BTreeMap<_, _> =
            read_columns(dir, "AssistedCombat", ["ID", "ChrSpecializationID"])?
                .into_iter()
                .map(|[id, spec]| (id, spec))
                .collect();
        let mut steps = BTreeMap::new();
        for [id, spell, combat_id, order] in read_columns(
            dir,
            "AssistedCombatStep",
            ["ID", "SpellID", "AssistedCombatID", "OrderIndex"],
        )? {
            let spec = *combat
                .get(&combat_id)
                .ok_or_else(|| format!("AssistedCombatStep {id}: missing rotation {combat_id}"))?;
            steps.insert(
                id,
                (
                    spec,
                    AssistedStep {
                        id,
                        spell,
                        order,
                        rules: Vec::new(),
                    },
                ),
            );
        }
        Ok(steps)
    }

    fn load_rules(
        dir: &Path,
        steps: &mut BTreeMap<u32, (u32, AssistedStep)>,
    ) -> Result<(), String> {
        for [id, order, field, condition_type, v1, v2, v3, step_id] in read_columns(
            dir,
            "AssistedCombatRule",
            [
                "ID",
                "OrderIndex",
                "Field_11_1_7_60520_002",
                "ConditionType",
                "ConditionValue1",
                "ConditionValue2",
                "ConditionValue3",
                "AssistedCombatStepID",
            ],
        )? {
            let (_, step) = steps
                .get_mut(&step_id)
                .ok_or_else(|| format!("AssistedCombatRule {id}: missing step {step_id}"))?;
            step.rules.push((
                order,
                AssistedRule {
                    id,
                    step_spell: step.spell,
                    condition_type,
                    values: [v1, v2, v3],
                    field_11_1_7_60520_002: field,
                },
            ));
        }
        Ok(())
    }
}

fn passes_player_condition(
    id: u32,
    evaluate: &mut impl FnMut(u32) -> Result<bool, String>,
) -> Result<bool, String> {
    if id == 0 { Ok(true) } else { evaluate(id) }
}

fn automation_only(step: &AssistedStep) -> bool {
    // Primary reference: SimulationCraft AC_AUTOMATION_ONLY=70, excluded from SBA.
    step.rules.iter().any(|(_, rule)| rule.condition_type == 70)
}

fn passes_step(
    step: &AssistedStep,
    evaluate: &mut impl FnMut(&AssistedRule) -> Result<bool, String>,
) -> Result<bool, String> {
    let mut unknown = None;
    for (_, rule) in &step.rules {
        let result = if rule.field_11_1_7_60520_002 == 0 {
            evaluate(rule)
        } else {
            Err(format!(
                "rule {}: unidentified field {}",
                rule.id, rule.field_11_1_7_60520_002
            ))
        };
        match result {
            Ok(false) => return Ok(false),
            Ok(true) => {}
            Err(error) => {
                unknown.get_or_insert(error);
            }
        }
    }
    match unknown {
        Some(error) => Err(error),
        None => Ok(true),
    }
}

fn read_columns<const N: usize>(
    dir: &Path,
    table: &str,
    names: [&str; N],
) -> Result<Vec<[u32; N]>, String> {
    let path = dir.join(format!("{table}.csv"));
    let csv = CsvTable::read(&path)?;
    let indices = names
        .map(|name| csv.column(name))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut ids = BTreeSet::new();
    let mut rows = Vec::new();
    for record in csv.records() {
        let mut values = [0; N];
        for (n, index) in indices.iter().enumerate() {
            values[n] = record
                .get(*index)
                .ok_or_else(|| format!("{table}: missing {}", names[n]))?
                .parse()
                .map_err(|error| format!("{table} {}: {error}", names[n]))?;
        }
        if !ids.insert(values[0]) {
            return Err(format!("{table}: duplicate ID {}", values[0]));
        }
        rows.push(values);
    }
    Ok(rows)
}
