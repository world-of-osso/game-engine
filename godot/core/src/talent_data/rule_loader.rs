//! Load the same conditions, costs, edges and currency sources as server trait_data.
use super::rule_data::*;
use super::{Projection, Row, Tables};
use std::{collections::BTreeMap, path::Path};

pub fn load_trait_rules(dir: &Path, tree_id: u32) -> Result<TraitTree, String> {
    let tables = Tables::load(dir)?;
    let mut spec_sets = BTreeMap::<u32, Vec<u32>>::new();
    for row in tables.rows("SpecSetMember") {
        spec_sets
            .entry(row.number("SpecSet")?)
            .or_default()
            .push(row.number("ChrSpecializationID")?);
    }
    let loader = RuleLoader {
        tables: &tables,
        projection: Projection::build(&tables, 0)?,
        spec_sets,
    };
    Ok(TraitTree {
        tree_id,
        currencies: loader.read_currencies(tree_id)?,
        nodes: loader.read_nodes(tree_id)?,
        groups: loader.read_groups(tree_id)?,
        sub_trees: Vec::new(),
    })
}
struct RuleLoader<'a> {
    tables: &'a Tables,
    projection: Projection<'a>,
    spec_sets: BTreeMap<u32, Vec<u32>>,
}
impl RuleLoader<'_> {
    fn tree_rows(&self, table: &str, tree_id: u32) -> Result<Vec<&Row>, String> {
        self.tables
            .rows(table)
            .filter_map(|row| match row.number("TraitTreeID") {
                Ok(id) if id == tree_id => Some(Ok(row)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }
    fn read_conditions(&self, ids: Option<&Vec<u32>>) -> Result<Vec<TraitCond>, String> {
        ids.into_iter()
            .flatten()
            .map(|id| read_condition(self.tables.row("TraitCond", *id)?, &self.spec_sets))
            .collect()
    }
    fn read_costs(&self, ids: Option<&Vec<u32>>) -> Result<Vec<TraitCost>, String> {
        ids.into_iter()
            .flatten()
            .map(|id| {
                let row = self.tables.row("TraitCost", *id)?;
                Ok(TraitCost {
                    currency_id: row.number("TraitCurrencyID")?,
                    amount: row.number("Amount")?,
                })
            })
            .collect()
    }
    fn read_entries(&self, node_id: u32) -> Result<Vec<TraitNodeEntry>, String> {
        self.projection
            .entries
            .get(&node_id)
            .into_iter()
            .flatten()
            .map(|entry_id| {
                let row = self.tables.row("TraitNodeEntry", *entry_id)?;
                let definition_id = row.number("TraitDefinitionID")?;
                let spell_id = if definition_id == 0 {
                    0
                } else {
                    self.tables
                        .row("TraitDefinition", definition_id)?
                        .number("SpellID")?
                };
                Ok(TraitNodeEntry {
                    id: *entry_id,
                    definition_id,
                    spell_id,
                    overrides_spell_id: 0,
                    passive: false,
                    max_ranks: row.number("MaxRanks")?,
                    entry_type: row.number("NodeEntryType")?,
                    sub_tree_id: row.number("TraitSubTreeID")?,
                    conds: self.read_conditions(self.projection.entry_conditions.get(entry_id))?,
                    costs: self.read_costs(self.projection.entry_costs.get(entry_id))?,
                })
            })
            .collect()
    }
    fn read_parents(&self) -> Result<BTreeMap<u32, Vec<(u32, TraitEdgeType)>>, String> {
        let mut parents = BTreeMap::<u32, Vec<(u32, TraitEdgeType)>>::new();
        for edge in self.tables.rows("TraitEdge") {
            let kind = match edge.number("Type")? {
                2 => TraitEdgeType::SufficientForAvailability,
                3 => TraitEdgeType::RequiredForAvailability,
                _ => continue,
            };
            parents
                .entry(edge.number("RightTraitNodeID")?)
                .or_default()
                .push((edge.number("LeftTraitNodeID")?, kind));
        }
        Ok(parents)
    }
    fn read_nodes(&self, tree_id: u32) -> Result<Vec<TraitNode>, String> {
        let mut parents = self.read_parents()?;
        self.tree_rows("TraitNode", tree_id)?
            .into_iter()
            .map(|row| {
                let id = row.number("ID")?;
                let kind = match row.number("Type")? {
                    0 => TraitNodeType::Single,
                    1 => TraitNodeType::Tiered,
                    2 => TraitNodeType::Selection,
                    3 => TraitNodeType::SubTreeSelection,
                    other => return Err(format!("Unknown TraitNode.Type {other}")),
                };
                Ok(TraitNode {
                    id,
                    pos_x: row.position("PosX")? as i32,
                    pos_y: row.position("PosY")? as i32,
                    kind,
                    flags: row.number("Flags")?,
                    sub_tree_id: row.number("TraitSubTreeID")?,
                    entries: self.read_entries(id)?,
                    groups: self.projection.groups.get(&id).cloned().unwrap_or_default(),
                    parents: parents.remove(&id).unwrap_or_default(),
                    conds: self.read_conditions(self.projection.node_conditions.get(&id))?,
                    costs: self.read_costs(self.projection.node_costs.get(&id))?,
                })
            })
            .collect()
    }
    fn read_groups(&self, tree_id: u32) -> Result<Vec<TraitNodeGroup>, String> {
        self.tree_rows("TraitNodeGroup", tree_id)?
            .into_iter()
            .map(|row| {
                let id = row.number("ID")?;
                Ok(TraitNodeGroup {
                    id,
                    conds: self.read_conditions(self.projection.group_conditions.get(&id))?,
                    costs: self.read_costs(self.projection.group_costs.get(&id))?,
                })
            })
            .collect()
    }
    fn read_sources(&self, currency_id: u32) -> Result<Vec<CurrencySource>, String> {
        self.tables
            .rows("TraitCurrencySource")
            .filter_map(|row| match row.number("TraitCurrencyID") {
                Ok(id) if id == currency_id => Some(Ok(row)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .map(|row| {
                let row = row?;
                Ok(CurrencySource {
                    amount: row
                        .text("Amount")?
                        .parse::<i32>()
                        .map_err(|error| format!("TraitCurrencySource Amount: {error}"))?,
                    quest_id: row.number("QuestID")?,
                    achievement_id: row.number("AchievementID")?,
                    player_level: row.number("PlayerLevel")?,
                    node_entry_id: row.number("TraitNodeEntryID")?,
                })
            })
            .collect()
    }
    fn read_currencies(&self, tree_id: u32) -> Result<Vec<TraitCurrency>, String> {
        let mut links = self
            .tree_rows("TraitTreeXTraitCurrency", tree_id)?
            .into_iter()
            .map(|row| Ok((row.number("_Index")?, row.number("TraitCurrencyID")?)))
            .collect::<Result<Vec<_>, String>>()?;
        links.sort_by_key(|&(index, _)| index);
        links
            .into_iter()
            .map(|(_, id)| {
                Ok(TraitCurrency {
                    id,
                    flags: self.tables.row("TraitCurrency", id)?.number("Flags")?,
                    sources: self.read_sources(id)?,
                })
            })
            .collect()
    }
}
fn read_condition(row: &Row, sets: &BTreeMap<u32, Vec<u32>>) -> Result<TraitCond, String> {
    let kind = match row.number("CondType")? {
        0 => TraitCondType::Available,
        1 => TraitCondType::Visible,
        2 => TraitCondType::Granted,
        3 => TraitCondType::Increased,
        4 => TraitCondType::DisplayError,
        5 => TraitCondType::RanksAllowed,
        other => return Err(format!("Unknown TraitCond.CondType {other}")),
    };
    let spec_set_id = row.number("SpecSetID")?;
    Ok(TraitCond {
        id: row.number("ID")?,
        kind,
        flags: row.number("Flags")?,
        granted_ranks: row.number("GrantedRanks")?,
        quest_id: row.number("QuestID")?,
        achievement_id: row.number("AchievementID")?,
        spec_set: sets.get(&spec_set_id).cloned().unwrap_or_default(),
        spec_set_id,
        node_group_id: row.number("TraitNodeGroupID")?,
        node_id: row.number("TraitNodeID")?,
        node_entry_id: row.number("TraitNodeEntryID")?,
        currency_id: row.number("TraitCurrencyID")?,
        spent_amount_required: row.number("SpentAmountRequired")?,
        required_level: row.number("RequiredLevel")?,
    })
}
