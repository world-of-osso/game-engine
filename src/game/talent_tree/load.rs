//! Joins the trait DB2 CSV exports into [`TalentTree`]s, one per class tree.
//! Link lists keep the link table's ID order, as the server's loader does.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;

use super::{
    AtlasCrop, CLASS_SKILL_LINES, ClassNames, CondKind, EdgeKind, NodeKind, TalentCurrency,
    TalentEntry, TalentGroup, TalentNode, TalentSpec, TalentSubTree, TalentTree, TraitCond,
    TraitCost,
};
use crate::spell_catalog::csv_records::CsvTable;

struct Row<'r> {
    table: &'r CsvTable,
    record: &'r [Cow<'r, str>],
    columns: &'r [usize],
}

impl Row<'_> {
    fn text(&self, column: usize) -> Result<&str, String> {
        self.record
            .get(self.columns[column])
            .map(|field| field.as_ref())
            .ok_or_else(|| format!("{} has a short record", self.table.path().display()))
    }

    fn get<T: FromStr>(&self, column: usize) -> Result<T, String> {
        let raw = self.text(column)?;
        raw.parse().map_err(|_| {
            format!(
                "{} column {}: bad value {raw:?}",
                self.table.path().display(),
                self.columns[column]
            )
        })
    }
}

fn for_each_row(
    dir: &Path,
    table: &str,
    columns: &[&str],
    mut visit: impl FnMut(&Row) -> Result<(), String>,
) -> Result<(), String> {
    let table = CsvTable::read(&dir.join(format!("{table}.csv")))?;
    let columns = columns
        .iter()
        .map(|name| table.column(name))
        .collect::<Result<Vec<_>, _>>()?;
    for record in table.records() {
        visit(&Row {
            table: &table,
            record: &record,
            columns: &columns,
        })?;
    }
    Ok(())
}

/// (link ID, owner ID, linked ID) rows of a two-column link table.
fn load_links(
    dir: &Path,
    table: &str,
    owner: &str,
    linked: &str,
) -> Result<Vec<(u32, u32, u32)>, String> {
    let mut links = Vec::new();
    for_each_row(dir, table, &["ID", owner, linked], |row| {
        links.push((row.get(0)?, row.get(1)?, row.get(2)?));
        Ok(())
    })?;
    links.sort_unstable_by_key(|link| link.0);
    Ok(links)
}

fn group_links<T: Clone>(
    links: &[(u32, u32, u32)],
    lookup: &HashMap<u32, T>,
) -> HashMap<u32, Vec<T>> {
    let mut grouped: HashMap<u32, Vec<T>> = HashMap::new();
    for &(_, owner, linked) in links {
        if let Some(value) = lookup.get(&linked) {
            grouped.entry(owner).or_default().push(value.clone());
        }
    }
    grouped
}

fn node_kind(value: u32) -> Result<NodeKind, String> {
    Ok(match value {
        0 => NodeKind::Single,
        1 => NodeKind::Tiered,
        2 => NodeKind::Selection,
        3 => NodeKind::SubTreeSelection,
        _ => return Err(format!("unknown TraitNode.Type {value}")),
    })
}

fn edge_kind(value: u32) -> Result<EdgeKind, String> {
    Ok(match value {
        2 => EdgeKind::SufficientForAvailability,
        3 => EdgeKind::RequiredForAvailability,
        _ => return Err(format!("unsupported TraitEdge.Type {value}")),
    })
}

fn cond_kind(value: u32) -> Result<CondKind, String> {
    Ok(match value {
        0 => CondKind::Available,
        1 => CondKind::Visible,
        2 => CondKind::Granted,
        3 => CondKind::Increased,
        4 => CondKind::DisplayError,
        5 => CondKind::RanksAllowed,
        _ => return Err(format!("unknown TraitCond.CondType {value}")),
    })
}

/// Trees of [`CLASS_SKILL_LINES`], keyed by tree ID with their class.
fn class_trees(dir: &Path) -> Result<HashMap<u32, u8>, String> {
    let mut trees = HashMap::new();
    for_each_row(
        dir,
        "SkillLineXTraitTree",
        &["SkillLineID", "TraitTreeID"],
        |row| {
            let skill_line: u32 = row.get(0)?;
            if let Some(&(class_id, _)) = CLASS_SKILL_LINES
                .iter()
                .find(|(_, line)| *line == skill_line)
            {
                trees.insert(row.get(1)?, class_id);
            }
            Ok(())
        },
    )?;
    Ok(trees)
}

fn load_spec_sets(dir: &Path) -> Result<HashMap<u32, Vec<u32>>, String> {
    let mut rows = Vec::new();
    for_each_row(
        dir,
        "SpecSetMember",
        &["ID", "SpecSet", "ChrSpecializationID"],
        |row| {
            rows.push((row.get::<u32>(0)?, row.get(1)?, row.get(2)?));
            Ok(())
        },
    )?;
    rows.sort_unstable_by_key(|row| row.0);
    let mut sets: HashMap<u32, Vec<u32>> = HashMap::new();
    for (_, set, spec) in rows {
        sets.entry(set).or_default().push(spec);
    }
    Ok(sets)
}

fn load_conds(dir: &Path) -> Result<HashMap<u32, TraitCond>, String> {
    let spec_sets = load_spec_sets(dir)?;
    let columns = [
        "ID",
        "CondType",
        "GrantedRanks",
        "QuestID",
        "AchievementID",
        "SpecSetID",
        "TraitNodeGroupID",
        "TraitNodeID",
        "TraitNodeEntryID",
        "TraitCurrencyID",
        "SpentAmountRequired",
        "Flags",
        "RequiredLevel",
    ];
    let mut conds = HashMap::new();
    for_each_row(dir, "TraitCond", &columns, |row| {
        let spec_set_id = row.get(5)?;
        let cond = TraitCond {
            id: row.get(0)?,
            kind: cond_kind(row.get(1)?)?,
            granted_ranks: row.get(2)?,
            quest_id: row.get(3)?,
            achievement_id: row.get(4)?,
            spec_set_id,
            spec_set: spec_sets.get(&spec_set_id).cloned().unwrap_or_default(),
            node_group_id: row.get(6)?,
            node_id: row.get(7)?,
            node_entry_id: row.get(8)?,
            currency_id: row.get(9)?,
            spent_amount_required: row.get(10)?,
            flags: row.get(11)?,
            required_level: row.get(12)?,
        };
        conds.insert(cond.id, cond);
        Ok(())
    })?;
    Ok(conds)
}

fn load_costs(dir: &Path) -> Result<HashMap<u32, TraitCost>, String> {
    let mut costs = HashMap::new();
    for_each_row(
        dir,
        "TraitCost",
        &["ID", "TraitCurrencyID", "Amount"],
        |row| {
            let cost = TraitCost {
                currency_id: row.get(1)?,
                amount: row.get(2)?,
            };
            costs.insert(row.get(0)?, cost);
            Ok(())
        },
    )?;
    Ok(costs)
}

/// Conditions and costs linked to nodes, groups or entries.
struct Links {
    conds: HashMap<u32, Vec<TraitCond>>,
    costs: HashMap<u32, Vec<TraitCost>>,
}

fn load_owner_links(
    dir: &Path,
    owner: &str,
    all_conds: &HashMap<u32, TraitCond>,
    all_costs: &HashMap<u32, TraitCost>,
) -> Result<Links, String> {
    let owner_column = format!("{owner}ID");
    let cond_links = load_links(
        dir,
        &format!("{owner}XTraitCond"),
        &owner_column,
        "TraitCondID",
    )?;
    let cost_links = load_links(
        dir,
        &format!("{owner}XTraitCost"),
        &owner_column,
        "TraitCostID",
    )?;
    Ok(Links {
        conds: group_links(&cond_links, all_conds),
        costs: group_links(&cost_links, all_costs),
    })
}

struct Definition {
    spell_id: u32,
    override_name: String,
    override_icon: u32,
}

fn load_definitions(dir: &Path) -> Result<HashMap<u32, Definition>, String> {
    let mut definitions = HashMap::new();
    for_each_row(
        dir,
        "TraitDefinition",
        &["ID", "SpellID", "OverrideName_lang", "OverrideIcon"],
        |row| {
            let definition = Definition {
                spell_id: row.get(1)?,
                override_name: row.text(2)?.to_string(),
                override_icon: row.get(3)?,
            };
            definitions.insert(row.get(0)?, definition);
            Ok(())
        },
    )?;
    Ok(definitions)
}

fn load_entries(
    dir: &Path,
    links: &Links,
    definitions: &HashMap<u32, Definition>,
) -> Result<HashMap<u32, TalentEntry>, String> {
    let mut entries = HashMap::new();
    let columns = ["ID", "TraitDefinitionID", "MaxRanks", "TraitSubTreeID"];
    for_each_row(dir, "TraitNodeEntry", &columns, |row| {
        let id: u32 = row.get(0)?;
        let definition = definitions.get(&row.get::<u32>(1)?);
        let entry = TalentEntry {
            id,
            spell_id: definition.map_or(0, |definition| definition.spell_id),
            override_name: definition
                .map(|definition| definition.override_name.clone())
                .unwrap_or_default(),
            override_icon: definition.map_or(0, |definition| definition.override_icon),
            passive: false,
            max_ranks: row.get(2)?,
            sub_tree_id: row.get(3)?,
            conds: links.conds.get(&id).cloned().unwrap_or_default(),
            costs: links.costs.get(&id).cloned().unwrap_or_default(),
        };
        entries.insert(id, entry);
        Ok(())
    })?;
    Ok(entries)
}

/// Node ID -> entries in `_Index` order.
fn load_node_entries(
    dir: &Path,
    node_ids: &HashSet<u32>,
    entries: &HashMap<u32, TalentEntry>,
) -> Result<HashMap<u32, Vec<TalentEntry>>, String> {
    let mut rows = Vec::new();
    for_each_row(
        dir,
        "TraitNodeXTraitNodeEntry",
        &["ID", "TraitNodeID", "TraitNodeEntryID", "_Index"],
        |row| {
            let node_id: u32 = row.get(1)?;
            if node_ids.contains(&node_id) {
                rows.push((node_id, row.get::<u32>(3)?, row.get::<u32>(0)?, row.get(2)?));
            }
            Ok(())
        },
    )?;
    rows.sort_unstable();
    let mut by_node: HashMap<u32, Vec<TalentEntry>> = HashMap::new();
    for (node_id, _, _, entry_id) in rows {
        let entry = entries
            .get(&entry_id)
            .ok_or_else(|| format!("node {node_id} names missing entry {entry_id}"))?;
        by_node.entry(node_id).or_default().push(entry.clone());
    }
    Ok(by_node)
}

/// Node ID -> (parent node, edge kind) in edge ID order.
fn load_parents(
    dir: &Path,
    node_ids: &HashSet<u32>,
) -> Result<HashMap<u32, Vec<(u32, EdgeKind)>>, String> {
    let mut rows = Vec::new();
    for_each_row(
        dir,
        "TraitEdge",
        &["ID", "LeftTraitNodeID", "RightTraitNodeID", "Type"],
        |row| {
            let child: u32 = row.get(2)?;
            let parent: u32 = row.get(1)?;
            if node_ids.contains(&child) && node_ids.contains(&parent) {
                rows.push((row.get::<u32>(0)?, child, parent, row.get::<u32>(3)?));
            }
            Ok(())
        },
    )?;
    rows.sort_unstable();
    let mut parents: HashMap<u32, Vec<(u32, EdgeKind)>> = HashMap::new();
    for (_, child, parent, kind) in rows {
        parents
            .entry(child)
            .or_default()
            .push((parent, edge_kind(kind)?));
    }
    Ok(parents)
}

fn load_node_groups(dir: &Path, node_ids: &HashSet<u32>) -> Result<HashMap<u32, Vec<u32>>, String> {
    let mut groups: HashMap<u32, Vec<u32>> = HashMap::new();
    for_each_row(
        dir,
        "TraitNodeGroupXTraitNode",
        &["TraitNodeID", "TraitNodeGroupID"],
        |row| {
            let node_id: u32 = row.get(0)?;
            if node_ids.contains(&node_id) {
                groups.entry(node_id).or_default().push(row.get(1)?);
            }
            Ok(())
        },
    )?;
    for list in groups.values_mut() {
        list.sort_unstable();
        list.dedup();
    }
    Ok(groups)
}

/// Tree ID -> group IDs, ascending.
fn load_group_ids(dir: &Path, trees: &HashMap<u32, u8>) -> Result<HashMap<u32, Vec<u32>>, String> {
    let mut groups: HashMap<u32, Vec<u32>> = HashMap::new();
    for_each_row(dir, "TraitNodeGroup", &["ID", "TraitTreeID"], |row| {
        let tree_id: u32 = row.get(1)?;
        if trees.contains_key(&tree_id) {
            groups.entry(tree_id).or_default().push(row.get(0)?);
        }
        Ok(())
    })?;
    for list in groups.values_mut() {
        list.sort_unstable();
    }
    Ok(groups)
}

fn load_currencies(
    dir: &Path,
    trees: &HashMap<u32, u8>,
) -> Result<HashMap<u32, Vec<TalentCurrency>>, String> {
    let mut flags = HashMap::new();
    for_each_row(dir, "TraitCurrency", &["ID", "Flags"], |row| {
        flags.insert(row.get::<u32>(0)?, row.get::<u32>(1)?);
        Ok(())
    })?;
    let mut rows = Vec::new();
    for_each_row(
        dir,
        "TraitTreeXTraitCurrency",
        &["ID", "_Index", "TraitTreeID", "TraitCurrencyID"],
        |row| {
            let tree_id: u32 = row.get(2)?;
            if trees.contains_key(&tree_id) {
                rows.push((tree_id, row.get::<u32>(1)?, row.get::<u32>(0)?, row.get(3)?));
            }
            Ok(())
        },
    )?;
    rows.sort_unstable();
    let mut currencies: HashMap<u32, Vec<TalentCurrency>> = HashMap::new();
    for (tree_id, _, _, id) in rows {
        let flags = flags.get(&id).copied().unwrap_or(0);
        currencies
            .entry(tree_id)
            .or_default()
            .push(TalentCurrency { id, flags });
    }
    Ok(currencies)
}

fn load_sub_trees(
    dir: &Path,
    trees: &HashMap<u32, u8>,
) -> Result<HashMap<u32, Vec<TalentSubTree>>, String> {
    let mut sub_trees: HashMap<u32, Vec<TalentSubTree>> = HashMap::new();
    for_each_row(
        dir,
        "TraitSubTree",
        &["ID", "TraitTreeID", "Name_lang"],
        |row| {
            let tree_id: u32 = row.get(1)?;
            if trees.contains_key(&tree_id) {
                sub_trees.entry(tree_id).or_default().push(TalentSubTree {
                    id: row.get(0)?,
                    name: row.text(2)?.to_string(),
                });
            }
            Ok(())
        },
    )?;
    for list in sub_trees.values_mut() {
        list.sort_unstable_by_key(|sub_tree| sub_tree.id);
    }
    Ok(sub_trees)
}

fn load_specs(dir: &Path) -> Result<HashMap<u8, Vec<TalentSpec>>, String> {
    let mut specs: HashMap<u8, Vec<TalentSpec>> = HashMap::new();
    let columns = [
        "ID",
        "ClassID",
        "OrderIndex",
        "Name_lang",
        "SpellIconFileID",
    ];
    for_each_row(dir, "ChrSpecialization", &columns, |row| {
        let class_id: u8 = row.get(1)?;
        if CLASS_SKILL_LINES
            .iter()
            .any(|(class, _)| *class == class_id)
        {
            specs.entry(class_id).or_default().push(TalentSpec {
                id: row.get(0)?,
                order_index: row.get(2)?,
                name: row.text(3)?.to_string(),
                icon_fdid: row.get(4)?,
            });
        }
        Ok(())
    })?;
    for list in specs.values_mut() {
        list.sort_unstable_by_key(|spec| spec.order_index);
    }
    Ok(specs)
}

struct NodeRow {
    tree_id: u32,
    id: u32,
    pos_x: i32,
    pos_y: i32,
    kind: NodeKind,
    sub_tree_id: u32,
}

fn load_node_rows(dir: &Path, trees: &HashMap<u32, u8>) -> Result<Vec<NodeRow>, String> {
    let mut nodes = Vec::new();
    let columns = [
        "ID",
        "TraitTreeID",
        "PosX",
        "PosY",
        "Type",
        "TraitSubTreeID",
    ];
    for_each_row(dir, "TraitNode", &columns, |row| {
        let tree_id: u32 = row.get(1)?;
        if trees.contains_key(&tree_id) {
            nodes.push(NodeRow {
                tree_id,
                id: row.get(0)?,
                pos_x: row.get(2)?,
                pos_y: row.get(3)?,
                kind: node_kind(row.get(4)?)?,
                sub_tree_id: row.get(5)?,
            });
        }
        Ok(())
    })?;
    nodes.sort_unstable_by_key(|node| node.id);
    Ok(nodes)
}

pub(super) fn load_trees(dir: &Path) -> Result<Vec<TalentTree>, String> {
    let class_of = class_trees(dir)?;
    let node_rows = load_node_rows(dir, &class_of)?;
    let node_ids: HashSet<u32> = node_rows.iter().map(|node| node.id).collect();
    let all_conds = load_conds(dir)?;
    let all_costs = load_costs(dir)?;
    let node_links = load_owner_links(dir, "TraitNode", &all_conds, &all_costs)?;
    let group_links = load_owner_links(dir, "TraitNodeGroup", &all_conds, &all_costs)?;
    let entry_links = load_owner_links(dir, "TraitNodeEntry", &all_conds, &all_costs)?;
    let definitions = load_definitions(dir)?;
    let entries = load_entries(dir, &entry_links, &definitions)?;
    let mut node_entries = load_node_entries(dir, &node_ids, &entries)?;
    mark_passive_entries(dir, &mut node_entries)?;
    let mut parents = load_parents(dir, &node_ids)?;
    let mut node_groups = load_node_groups(dir, &node_ids)?;
    let mut group_ids = load_group_ids(dir, &class_of)?;
    let mut currencies = load_currencies(dir, &class_of)?;
    let mut sub_trees = load_sub_trees(dir, &class_of)?;
    let mut specs = load_specs(dir)?;

    let mut trees: HashMap<u32, TalentTree> = class_of
        .iter()
        .map(|(&tree_id, &class_id)| {
            let groups = group_ids
                .remove(&tree_id)
                .unwrap_or_default()
                .into_iter()
                .map(|id| TalentGroup {
                    id,
                    conds: group_links.conds.get(&id).cloned().unwrap_or_default(),
                    costs: group_links.costs.get(&id).cloned().unwrap_or_default(),
                })
                .collect();
            let tree = TalentTree {
                class_id,
                tree_id,
                currencies: currencies.remove(&tree_id).unwrap_or_default(),
                nodes: Vec::new(),
                groups,
                sub_trees: sub_trees.remove(&tree_id).unwrap_or_default(),
                specs: specs.remove(&class_id).unwrap_or_default(),
            };
            (tree_id, tree)
        })
        .collect();
    for row in node_rows {
        let node = TalentNode {
            id: row.id,
            pos_x: row.pos_x,
            pos_y: row.pos_y,
            kind: row.kind,
            sub_tree_id: row.sub_tree_id,
            entries: node_entries.remove(&row.id).unwrap_or_default(),
            groups: node_groups.remove(&row.id).unwrap_or_default(),
            parents: parents.remove(&row.id).unwrap_or_default(),
            conds: node_links.conds.get(&row.id).cloned().unwrap_or_default(),
            costs: node_links.costs.get(&row.id).cloned().unwrap_or_default(),
        };
        if let Some(tree) = trees.get_mut(&row.tree_id) {
            tree.nodes.push(node);
        }
    }
    Ok(trees.into_values().collect())
}

/// `UiTextureAtlasMember` rows named `prefix*`, cropped from their `UiTextureAtlas`.
pub(super) fn load_atlas_crops(
    data_dir: &Path,
    prefix: &str,
) -> Result<HashMap<String, AtlasCrop>, String> {
    let mut atlases = HashMap::new();
    for_each_row(
        data_dir,
        "UiTextureAtlas",
        &["ID", "FileDataID", "AtlasWidth", "AtlasHeight"],
        |row| {
            let size: (u32, f32, f32) = (row.get(1)?, row.get(2)?, row.get(3)?);
            atlases.insert(row.get::<u32>(0)?, size);
            Ok(())
        },
    )?;
    let columns = [
        "CommittedName",
        "UiTextureAtlasID",
        "CommittedLeft",
        "CommittedRight",
        "CommittedTop",
        "CommittedBottom",
    ];
    let mut crops = HashMap::new();
    for_each_row(data_dir, "UiTextureAtlasMember", &columns, |row| {
        let name = row.text(0)?;
        if !name.starts_with(prefix) {
            return Ok(());
        }
        let Some(&(fdid, width, height)) = atlases.get(&row.get::<u32>(1)?) else {
            return Ok(());
        };
        let [left, right, top, bottom]: [f32; 4] =
            [row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?];
        let crop = AtlasCrop {
            fdid,
            tex_coords: [left / width, right / width, top / height, bottom / height],
            width: right - left,
            height: bottom - top,
        };
        crops.insert(name.to_string(), crop);
        Ok(())
    })?;
    Ok(crops)
}

/// `ChrClasses` names of the character-creation classes.
pub(super) fn load_class_names(dir: &Path) -> Result<HashMap<u8, ClassNames>, String> {
    let mut classes = HashMap::new();
    for_each_row(dir, "ChrClasses", &["ID", "Filename", "Name_lang"], |row| {
        let class_id: u8 = row.get(0)?;
        if CLASS_SKILL_LINES
            .iter()
            .any(|(class, _)| *class == class_id)
        {
            let names = ClassNames {
                file: row.text(1)?.to_string(),
                name: row.text(2)?.to_string(),
            };
            classes.insert(class_id, names);
        }
        Ok(())
    })?;
    Ok(classes)
}

/// SPELL_ATTR0_PASSIVE in `SpellMisc.Attributes_0`.
const ATTR0_PASSIVE: u32 = 0x40;

/// Sets `passive` from the base-difficulty `SpellMisc` row of each entry spell.
/// `SpellMisc.csv` has no quoted fields, so a plain line split is enough and
/// keeps the 400k-row scan cheap.
fn mark_passive_entries(
    dir: &Path,
    node_entries: &mut HashMap<u32, Vec<TalentEntry>>,
) -> Result<(), String> {
    let path = dir.join("SpellMisc.csv");
    let text =
        std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let mut lines = text.lines();
    let headers: Vec<String> = lines
        .next()
        .ok_or_else(|| format!("{} has no header", path.display()))?
        .split(',')
        .map(str::to_string)
        .collect();
    let column = |name| crate::csv_util::header_index(&headers, name, &path);
    let (attrs, difficulty, spell) = (
        column("Attributes_0")?,
        column("DifficultyID")?,
        column("SpellID")?,
    );
    let wanted: HashSet<u32> = node_entries
        .values()
        .flatten()
        .map(|entry| entry.spell_id)
        .filter(|&id| id != 0)
        .collect();
    let mut passive = HashSet::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        let field = |index: usize| fields.get(index).and_then(|v| v.parse::<i64>().ok());
        let Some(spell_id) = field(spell).map(|id| id as u32) else {
            continue;
        };
        if field(difficulty) == Some(0)
            && wanted.contains(&spell_id)
            && field(attrs).is_some_and(|value| value as u32 & ATTR0_PASSIVE != 0)
        {
            passive.insert(spell_id);
        }
    }
    for entry in node_entries.values_mut().flatten() {
        entry.passive = passive.contains(&entry.spell_id);
    }
    Ok(())
}
