//! Read-only local DB2 talent projection; provenance: docs/wiki/systems/talents.md.
use crate::spell_catalog::csv_records::CsvTable;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::Path;

#[path = "talent_data/rule_data.rs"]
pub mod rule_data;
#[path = "talent_data/rule_loader.rs"]
mod rule_loader;
#[path = "talent_data/rules.rs"]
pub mod rules;
pub use rule_loader::load_trait_rules;

const VISIBLE: u32 = 1;
const GRANTED: u32 = 2;
const SUFFICIENT: u32 = 4;
const CLASS_CURRENCY: u32 = 4;
const SPEC_CURRENCY: u32 = 8;
const SUBTREE_SELECTION: u32 = 3;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentEntry {
    pub id: u32,
    pub spell_id: u32,
    pub override_icon: u32,
    pub max_ranks: u32,
    pub entry_type: u32,
    pub subtree_id: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentNode {
    pub id: u32,
    pub position: [f32; 2],
    pub node_type: u32,
    pub flags: u32,
    pub granted_ranks: u32,
    pub entries: Vec<TalentEntry>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentEdge {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub visual_style: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentTree {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub icon_atlas_element_id: u32,
    pub nodes: Vec<TalentNode>,
    pub edges: Vec<TalentEdge>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentPage {
    pub tree_id: u32,
    pub class: TalentTree,
    pub spec: TalentTree,
    pub heroes: Vec<TalentTree>,
    pub hero_selection: Option<TalentNode>,
    /// Provenance only: allocations are never applied to this read-only projection.
    pub starter_loadout_id: Option<u32>,
}

struct Row {
    source: String,
    fields: HashMap<String, String>,
}
impl Row {
    fn text(&self, field: &str) -> Result<&str, String> {
        self.fields
            .get(field)
            .map(String::as_str)
            .ok_or_else(|| format!("{}: missing column {field}", self.source))
    }
    fn number(&self, field: &str) -> Result<u32, String> {
        self.text(field)?
            .parse()
            .map_err(|err| format!("{}: {field}: {err}", self.source))
    }
    fn position(&self, field: &str) -> Result<f32, String> {
        self.text(field)?
            .parse()
            .map_err(|err| format!("{}: {field}: {err}", self.source))
    }
}
struct Tables(HashMap<&'static str, BTreeMap<u32, Row>>);
impl Tables {
    fn load(dir: &Path) -> Result<Self, String> {
        let names = [
            "ChrClasses",
            "ChrSpecialization",
            "SkillLine",
            "SpecSetMember",
            "SkillLineXTraitTree",
            "TraitTree",
            "TraitNode",
            "TraitNodeEntry",
            "TraitDefinition",
            "TraitEdge",
            "TraitNodeGroup",
            "TraitNodeGroupXTraitNode",
            "TraitNodeGroupXTraitCond",
            "TraitNodeGroupXTraitCost",
            "TraitNodeXTraitNodeEntry",
            "TraitNodeXTraitCond",
            "TraitNodeEntryXTraitCond",
            "TraitCond",
            "TraitCost",
            "TraitCurrency",
            "TraitCurrencySource",
            "TraitTreeXTraitCurrency",
            "TraitNodeXTraitCost",
            "TraitNodeEntryXTraitCost",
            "TraitSubTree",
            "TraitTreeLoadout",
            "TraitTreeLoadoutEntry",
        ];
        let tables = names
            .into_iter()
            .map(|name| Ok((name, read_rows(&dir.join(format!("{name}.csv")))?)))
            .collect::<Result<_, String>>()?;
        Ok(Self(tables))
    }
    fn rows(&self, name: &str) -> impl Iterator<Item = &Row> {
        self.0[name].values()
    }
    fn row(&self, name: &str, id: u32) -> Result<&Row, String> {
        self.0[name]
            .get(&id)
            .ok_or_else(|| format!("{name}: missing referenced row {id}"))
    }
    fn links(&self, name: &str, from: &str, to: &str) -> Result<BTreeMap<u32, Vec<u32>>, String> {
        let mut links = BTreeMap::<u32, Vec<u32>>::new();
        for row in self.rows(name) {
            links
                .entry(row.number(from)?)
                .or_default()
                .push(row.number(to)?);
        }
        Ok(links)
    }
}
fn read_rows(path: &Path) -> Result<BTreeMap<u32, Row>, String> {
    let table = CsvTable::read(path)?;
    // CsvTable keeps RFC4180 quoting, including localized embedded newlines.
    let names = table.headers();
    let mut rows = BTreeMap::new();
    for record in table.records() {
        let row = Row {
            source: path.display().to_string(),
            fields: names
                .iter()
                .zip(record)
                .map(|(name, value)| (name.clone(), value.into_owned()))
                .collect(),
        };
        let id = row.number("ID")?;
        if rows.insert(id, row).is_some() {
            return Err(format!("{}: duplicate ID {id}", path.display()));
        }
    }
    Ok(rows)
}

struct Projection<'a> {
    tables: &'a Tables,
    spec_sets: BTreeSet<u32>,
    groups: BTreeMap<u32, Vec<u32>>,
    node_conditions: BTreeMap<u32, Vec<u32>>,
    group_conditions: BTreeMap<u32, Vec<u32>>,
    entry_conditions: BTreeMap<u32, Vec<u32>>,
    group_costs: BTreeMap<u32, Vec<u32>>,
    node_costs: BTreeMap<u32, Vec<u32>>,
    entry_costs: BTreeMap<u32, Vec<u32>>,
    entries: BTreeMap<u32, Vec<u32>>,
}
impl<'a> Projection<'a> {
    fn build(tables: &'a Tables, spec: u32) -> Result<Self, String> {
        let mut spec_sets = BTreeSet::new();
        for row in tables.rows("SpecSetMember") {
            if row.number("ChrSpecializationID")? == spec {
                spec_sets.insert(row.number("SpecSet")?);
            }
        }
        let mut ordered = tables
            .rows("TraitNodeXTraitNodeEntry")
            .map(|row| Ok((row.number("_Index")?, row)))
            .collect::<Result<Vec<_>, String>>()?;
        ordered.sort_by_key(|(index, _)| *index);
        let mut entries = BTreeMap::<u32, Vec<u32>>::new();
        for (_, row) in ordered {
            entries
                .entry(row.number("TraitNodeID")?)
                .or_default()
                .push(row.number("TraitNodeEntryID")?);
        }
        Ok(Self {
            tables,
            spec_sets,
            entries,
            groups: tables.links(
                "TraitNodeGroupXTraitNode",
                "TraitNodeID",
                "TraitNodeGroupID",
            )?,
            node_conditions: tables.links("TraitNodeXTraitCond", "TraitNodeID", "TraitCondID")?,
            group_conditions: tables.links(
                "TraitNodeGroupXTraitCond",
                "TraitNodeGroupID",
                "TraitCondID",
            )?,
            entry_conditions: tables.links(
                "TraitNodeEntryXTraitCond",
                "TraitNodeEntryID",
                "TraitCondID",
            )?,
            group_costs: tables.links(
                "TraitNodeGroupXTraitCost",
                "TraitNodeGroupID",
                "TraitCostID",
            )?,
            node_costs: tables.links("TraitNodeXTraitCost", "TraitNodeID", "TraitCostID")?,
            entry_costs: tables.links(
                "TraitNodeEntryXTraitCost",
                "TraitNodeEntryID",
                "TraitCostID",
            )?,
        })
    }
    fn conditions(&self, node: u32) -> Result<Vec<&Row>, String> {
        let group_ids = self.groups.get(&node).into_iter().flatten();
        let group_conditions =
            group_ids.flat_map(|id| self.group_conditions.get(id).into_iter().flatten());
        let direct = self.node_conditions.get(&node).into_iter().flatten();
        direct
            .chain(group_conditions)
            .map(|id| self.tables.row("TraitCond", *id))
            .collect()
    }
    fn matches_spec(&self, cond: &Row) -> Result<bool, String> {
        let set = cond.number("SpecSetID")?;
        Ok(set == 0 || self.spec_sets.contains(&set))
    }
    fn visible(&self, conditions: &[&Row]) -> Result<bool, String> {
        let mut has_sufficient = false;
        let mut sufficient_met = false;
        for cond in conditions {
            if cond.number("CondType")? != VISIBLE {
                continue;
            }
            let met = self.matches_spec(cond)?;
            if cond.number("Flags")? & SUFFICIENT != 0 {
                has_sufficient = true;
                sufficient_met |= met;
            } else if !met {
                return Ok(false);
            }
        }
        Ok(!has_sufficient || sufficient_met)
    }
    fn granted_ranks(&self, conditions: &[&Row]) -> Result<u32, String> {
        let mut ranks = 0;
        for cond in conditions {
            if cond.number("CondType")? == GRANTED && self.matches_spec(cond)? {
                // This slice has no quest/achievement/level purchases or active hero choice.
                let unconditional =
                    cond.number("QuestID")? == 0 && cond.number("AchievementID")? == 0;
                if unconditional && cond.number("RequiredLevel")? == 0 {
                    ranks = ranks.max(cond.number("GrantedRanks")?);
                }
            }
        }
        Ok(ranks)
    }
    fn entry(&self, id: u32) -> Result<Option<TalentEntry>, String> {
        let row = self.tables.row("TraitNodeEntry", id)?;
        let conds = self
            .entry_conditions
            .get(&id)
            .into_iter()
            .flatten()
            .map(|id| self.tables.row("TraitCond", *id))
            .collect::<Result<Vec<_>, _>>()?;
        if !self.visible(&conds)? {
            return Ok(None);
        }
        let definition = row.number("TraitDefinitionID")?;
        let (spell_id, override_icon) = if definition == 0 {
            (0, 0)
        } else {
            let definition = self.tables.row("TraitDefinition", definition)?;
            (
                definition.number("SpellID")?,
                definition.number("OverrideIcon")?,
            )
        };
        Ok(Some(TalentEntry {
            id,
            spell_id,
            override_icon,
            max_ranks: row.number("MaxRanks")?,
            entry_type: row.number("NodeEntryType")?,
            subtree_id: row.number("TraitSubTreeID")?,
        }))
    }
    fn node(&self, row: &Row) -> Result<Option<TalentNode>, String> {
        let id = row.number("ID")?;
        let conds = self.conditions(id)?;
        if !self.visible(&conds)? {
            return Ok(None);
        }
        let entries = self
            .entries
            .get(&id)
            .into_iter()
            .flatten()
            .map(|id| self.entry(*id))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Err(format!("TraitNode {id}: no visible entries"));
        }
        let hero = row.number("TraitSubTreeID")? != 0;
        let granted_ranks = if hero { 0 } else { self.granted_ranks(&conds)? };
        Ok(Some(TalentNode {
            id,
            position: [row.position("PosX")?, row.position("PosY")?],
            node_type: row.number("Type")?,
            flags: row.number("Flags")?,
            granted_ranks,
            entries,
        }))
    }
    fn currency_side(&self, node: &TalentNode) -> Result<Option<u32>, String> {
        let group_costs = self
            .groups
            .get(&node.id)
            .into_iter()
            .flatten()
            .flat_map(|group| self.group_costs.get(group).into_iter().flatten());
        let node_costs = self.node_costs.get(&node.id).into_iter().flatten();
        let entry_costs = node
            .entries
            .iter()
            .flat_map(|entry| self.entry_costs.get(&entry.id).into_iter().flatten());
        let mut side = None;
        for id in node_costs.chain(group_costs).chain(entry_costs) {
            let cost = self.tables.row("TraitCost", *id)?;
            let currency = self
                .tables
                .row("TraitCurrency", cost.number("TraitCurrencyID")?)?;
            let flags = currency.number("Flags")? & (CLASS_CURRENCY | SPEC_CURRENCY);
            if flags != 0 {
                side = Some(flags);
            }
        }
        Ok(side)
    }
}

pub fn load_talent_page(dir: &Path, class: u32, spec: u32) -> Result<TalentPage, String> {
    let tables = Tables::load(dir)?;
    let class_name = tables.row("ChrClasses", class)?.text("Name_lang")?;
    let spec_row = tables.row("ChrSpecialization", spec)?;
    if spec_row.number("ClassID")? != class {
        return Err(format!("Spec {spec} does not belong to class {class}"));
    }
    let tree_id = find_class_tree(&tables, class_name)?;
    tables.row("TraitTree", tree_id)?;
    let projection = Projection::build(&tables, spec)?;
    let (nodes, subtrees) = project_nodes(&projection, tree_id)?;
    let edges = read_edges(&tables, &nodes)?;
    let sides = infer_main_sides(&projection, &nodes, &subtrees, &edges)?;
    let mut page = TalentPage {
        tree_id,
        class: select_tree(tree_id, class_name, &nodes, &edges, |node| {
            sides.get(&node.id) == Some(&CLASS_CURRENCY)
        }),
        spec: select_tree(
            tree_id,
            spec_row.text("Name_lang")?,
            &nodes,
            &edges,
            |node| sides.get(&node.id) == Some(&SPEC_CURRENCY),
        ),
        hero_selection: nodes
            .iter()
            .find(|node| node.node_type == SUBTREE_SELECTION)
            .cloned(),
        starter_loadout_id: find_starter_loadout(&tables, tree_id, spec)?,
        ..Default::default()
    };
    page.heroes = project_heroes(&tables, &page, &nodes, &subtrees, &edges)?;
    Ok(page)
}
fn find_class_tree(tables: &Tables, class_name: &str) -> Result<u32, String> {
    let mut skill = None;
    for row in tables.rows("SkillLine") {
        if row.number("CategoryID")? == 7 && row.text("DisplayName_lang")? == class_name {
            skill = Some(row.number("ID")?);
        }
    }
    let skill = skill.ok_or_else(|| format!("No class SkillLine for {class_name}"))?;
    for row in tables.rows("SkillLineXTraitTree") {
        if row.number("SkillLineID")? == skill && row.number("Variant")? == 0 {
            return row.number("TraitTreeID");
        }
    }
    Err(format!("No talent tree for SkillLine {skill}"))
}
fn project_nodes(
    projection: &Projection<'_>,
    tree: u32,
) -> Result<(Vec<TalentNode>, BTreeMap<u32, u32>), String> {
    let mut nodes = Vec::new();
    let mut subtrees = BTreeMap::new();
    for row in projection.tables.rows("TraitNode") {
        if row.number("TraitTreeID")? != tree {
            continue;
        }
        if let Some(node) = projection.node(row)? {
            subtrees.insert(node.id, row.number("TraitSubTreeID")?);
            nodes.push(node);
        }
    }
    Ok((nodes, subtrees))
}
fn read_edges(tables: &Tables, nodes: &[TalentNode]) -> Result<Vec<TalentEdge>, String> {
    let ids: BTreeSet<_> = nodes.iter().map(|node| node.id).collect();
    let mut edges = Vec::new();
    for row in tables.rows("TraitEdge") {
        let from = row.number("LeftTraitNodeID")?;
        let to = row.number("RightTraitNodeID")?;
        if ids.contains(&from) && ids.contains(&to) {
            edges.push(TalentEdge {
                id: row.number("ID")?,
                from,
                to,
                visual_style: row.number("VisualStyle")?,
            });
        }
    }
    Ok(edges)
}
fn infer_main_sides(
    projection: &Projection<'_>,
    nodes: &[TalentNode],
    subtrees: &BTreeMap<u32, u32>,
    edges: &[TalentEdge],
) -> Result<BTreeMap<u32, u32>, String> {
    let main: BTreeSet<_> = nodes
        .iter()
        .filter(|n| {
            subtrees[&n.id] == 0
                && n.node_type != SUBTREE_SELECTION
                && n.entries.iter().any(|entry| entry.max_ranks != 0)
        })
        .map(|n| n.id)
        .collect();
    let mut sides = BTreeMap::new();
    let mut queue = VecDeque::new();
    for node in nodes.iter().filter(|node| main.contains(&node.id)) {
        if let Some(side) = projection.currency_side(node)? {
            sides.insert(node.id, side);
            queue.push_back(node.id);
        }
    }
    let mut adjacent = BTreeMap::<u32, Vec<u32>>::new();
    for edge in edges
        .iter()
        .filter(|edge| main.contains(&edge.from) && main.contains(&edge.to))
    {
        adjacent.entry(edge.from).or_default().push(edge.to);
        adjacent.entry(edge.to).or_default().push(edge.from);
    }
    while let Some(id) = queue.pop_front() {
        for next in adjacent.get(&id).into_iter().flatten() {
            if !sides.contains_key(next) {
                sides.insert(*next, sides[&id]);
                queue.push_back(*next);
            }
        }
    }
    if sides.len() != main.len() {
        return Err("Talent nodes disconnected from class/spec currency groups".into());
    }
    Ok(sides)
}
fn select_tree(
    id: u32,
    name: &str,
    nodes: &[TalentNode],
    edges: &[TalentEdge],
    includes: impl Fn(&TalentNode) -> bool,
) -> TalentTree {
    let nodes: Vec<_> = nodes
        .iter()
        .filter(|node| includes(node))
        .cloned()
        .collect();
    let ids: BTreeSet<_> = nodes.iter().map(|node| node.id).collect();
    let edges = edges
        .iter()
        .filter(|edge| ids.contains(&edge.from) && ids.contains(&edge.to))
        .cloned()
        .collect();
    TalentTree {
        id,
        name: name.to_string(),
        nodes,
        edges,
        ..Default::default()
    }
}
fn project_heroes(
    tables: &Tables,
    page: &TalentPage,
    nodes: &[TalentNode],
    subtrees: &BTreeMap<u32, u32>,
    edges: &[TalentEdge],
) -> Result<Vec<TalentTree>, String> {
    // Retail HeroTalentsContainer/SelectionDialog use the spec-visible selection
    // node's entries, never every subtree containing a visible/unconditioned node.
    let eligible: BTreeSet<_> = page
        .hero_selection
        .iter()
        .flat_map(|node| &node.entries)
        .map(|entry| entry.subtree_id)
        .collect();
    eligible
        .into_iter()
        .map(|id| {
            let row = tables.row("TraitSubTree", id)?;
            let mut hero = select_tree(id, row.text("Name_lang")?, nodes, edges, |node| {
                subtrees[&node.id] == id
            });
            hero.description = row.text("Description_lang")?.to_string();
            hero.icon_atlas_element_id = row.number("UiTextureAtlasElementID")?;
            Ok(hero)
        })
        .collect()
}
fn find_starter_loadout(tables: &Tables, tree: u32, spec: u32) -> Result<Option<u32>, String> {
    for row in tables.rows("TraitTreeLoadout") {
        if row.number("TraitTreeID")? == tree && row.number("ChrSpecializationID")? == spec {
            let id = row.number("ID")?;
            // Read the provenance rows, never use NumPoints as learned state.
            for entry in tables.rows("TraitTreeLoadoutEntry") {
                if entry.number("TraitTreeLoadoutID")? == id {
                    entry.number("NumPoints")?;
                }
            }
            return Ok(Some(id));
        }
    }
    Ok(None)
}
