//! Load the same conditions, costs, edges and currency sources as server trait_data.
use super::rule_data::*;
use super::{Projection, Row, Tables};
use std::{collections::BTreeMap, path::Path};

pub fn load_trait_rules(dir: &Path, tree_id: u32) -> Result<TraitTree, String> {
    let tables = Tables::load(dir)?;
    let projection = Projection::build(&tables, 0)?;
    let mut sets = BTreeMap::<u32, Vec<u32>>::new();
    for row in tables.rows("SpecSetMember") {
        sets.entry(row.number("SpecSet")?)
            .or_default()
            .push(row.number("ChrSpecializationID")?);
    }
    let conditions = |ids: Option<&Vec<u32>>| -> Result<Vec<TraitCond>, String> {
        ids.into_iter()
            .flatten()
            .map(|id| read_condition(tables.row("TraitCond", *id)?, &sets))
            .collect()
    };
    let costs = |ids: Option<&Vec<u32>>| -> Result<Vec<TraitCost>, String> {
        ids.into_iter()
            .flatten()
            .map(|id| {
                let row = tables.row("TraitCost", *id)?;
                Ok(TraitCost {
                    currency_id: row.number("TraitCurrencyID")?,
                    amount: row.number("Amount")?,
                })
            })
            .collect()
    };
    let mut nodes = Vec::new();
    for row in tables.rows("TraitNode") {
        if row.number("TraitTreeID")? != tree_id {
            continue;
        }
        let id = row.number("ID")?;
        let mut entries = Vec::new();
        for entry_id in projection.entries.get(&id).into_iter().flatten() {
            let entry = tables.row("TraitNodeEntry", *entry_id)?;
            let definition_id = entry.number("TraitDefinitionID")?;
            let spell_id = if definition_id == 0 {
                0
            } else {
                tables
                    .row("TraitDefinition", definition_id)?
                    .number("SpellID")?
            };
            entries.push(TraitNodeEntry {
                id: *entry_id,
                definition_id,
                spell_id,
                overrides_spell_id: 0,
                passive: false,
                max_ranks: entry.number("MaxRanks")?,
                entry_type: entry.number("NodeEntryType")?,
                sub_tree_id: entry.number("TraitSubTreeID")?,
                conds: conditions(projection.entry_conditions.get(entry_id))?,
                costs: costs(projection.entry_costs.get(entry_id))?,
            });
        }
        let mut parents = Vec::new();
        for edge in tables.rows("TraitEdge") {
            if edge.number("RightTraitNodeID")? != id {
                continue;
            }
            let kind = match edge.number("Type")? {
                2 => TraitEdgeType::SufficientForAvailability,
                3 => TraitEdgeType::RequiredForAvailability,
                _ => continue,
            };
            parents.push((edge.number("LeftTraitNodeID")?, kind));
        }
        let kind = match row.number("Type")? {
            0 => TraitNodeType::Single,
            1 => TraitNodeType::Tiered,
            2 => TraitNodeType::Selection,
            3 => TraitNodeType::SubTreeSelection,
            other => return Err(format!("Unknown TraitNode.Type {other}")),
        };
        nodes.push(TraitNode {
            id,
            pos_x: row.position("PosX")? as i32,
            pos_y: row.position("PosY")? as i32,
            kind,
            flags: row.number("Flags")?,
            sub_tree_id: row.number("TraitSubTreeID")?,
            entries,
            groups: projection.groups.get(&id).cloned().unwrap_or_default(),
            parents,
            conds: conditions(projection.node_conditions.get(&id))?,
            costs: costs(projection.node_costs.get(&id))?,
        });
    }
    let mut groups = Vec::new();
    for row in tables.rows("TraitNodeGroup") {
        if row.number("TraitTreeID")? != tree_id {
            continue;
        }
        let id = row.number("ID")?;
        groups.push(TraitNodeGroup {
            id,
            conds: conditions(projection.group_conditions.get(&id))?,
            costs: costs(projection.group_costs.get(&id))?,
        });
    }
    let mut links = tables
        .rows("TraitTreeXTraitCurrency")
        .filter_map(|row| match row.number("TraitTreeID") {
            Ok(id) if id == tree_id => Some(Ok(row)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, String>>()?;
    links.sort_by_key(|row| row.number("_Index").expect("validated CSV index"));
    let mut currencies = Vec::new();
    for link in links {
        let id = link.number("TraitCurrencyID")?;
        let mut sources = Vec::new();
        for row in tables.rows("TraitCurrencySource") {
            if row.number("TraitCurrencyID")? != id {
                continue;
            }
            sources.push(CurrencySource {
                amount: row
                    .text("Amount")?
                    .parse::<i32>()
                    .map_err(|error| format!("TraitCurrencySource Amount: {error}"))?,
                quest_id: row.number("QuestID")?,
                achievement_id: row.number("AchievementID")?,
                player_level: row.number("PlayerLevel")?,
                node_entry_id: row.number("TraitNodeEntryID")?,
            });
        }
        currencies.push(TraitCurrency {
            id,
            flags: tables.row("TraitCurrency", id)?.number("Flags")?,
            sources,
        });
    }
    Ok(TraitTree {
        tree_id,
        currencies,
        nodes,
        groups,
        sub_trees: Vec::new(),
    })
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
