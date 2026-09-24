//! Client-side class trait (talent) trees from the pinned 12.1.0.69933 DB2 CSVs.
//!
//! Holds only what the talent window displays and what its local validity check
//! needs. The rules in [`rules`] mirror game-server `trait_config` for display;
//! the server stays authoritative (`CommitTraitConfig`).

mod cache;
mod load;
pub mod rules;
pub mod session;

#[cfg(test)]
mod real_data_tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use serde::{Deserialize, Serialize};

use crate::spell_catalog::SPELL_DB2_BUILD;

/// ChrClasses ID -> class skill line (`SkillLine.CategoryID` 7), as the server loads them.
pub const CLASS_SKILL_LINES: [(u8, u32); 10] = [
    (1, 840),
    (2, 800),
    (3, 795),
    (4, 921),
    (5, 804),
    (6, 796),
    (7, 924),
    (8, 904),
    (9, 849),
    (11, 798),
];

/// `TraitNode.Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Single,
    /// Entries are ranked one after another (apex talents).
    Tiered,
    /// Exactly one entry may be chosen.
    Selection,
    /// Exactly one entry may be chosen; each entry names a hero sub-tree.
    SubTreeSelection,
}

/// `TraitEdge.Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    SufficientForAvailability,
    RequiredForAvailability,
}

/// `TraitCond.CondType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CondKind {
    Available,
    Visible,
    Granted,
    Increased,
    DisplayError,
    RanksAllowed,
}

/// `TraitCond.Flags` IsGate.
pub const COND_FLAG_GATE: u32 = 0x1;
/// `TraitCond.Flags` IsSufficient.
pub const COND_FLAG_SUFFICIENT: u32 = 0x4;
/// `TraitCurrency.Flags` class-icon: class points.
pub const CURRENCY_FLAG_CLASS: u32 = 0x4;
/// `TraitCurrency.Flags` spec-icon: spec points.
pub const CURRENCY_FLAG_SPEC: u32 = 0x8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitCond {
    pub id: u32,
    pub kind: CondKind,
    pub flags: u32,
    pub granted_ranks: u32,
    pub quest_id: u32,
    pub achievement_id: u32,
    pub spec_set_id: u32,
    /// `SpecSetMember.ChrSpecializationID` of `spec_set_id`.
    pub spec_set: Vec<u32>,
    pub node_group_id: u32,
    pub node_id: u32,
    pub node_entry_id: u32,
    pub currency_id: u32,
    pub spent_amount_required: u32,
    pub required_level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitCost {
    pub currency_id: u32,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TalentEntry {
    pub id: u32,
    /// `TraitDefinition.SpellID`; 0 when the definition has none.
    pub spell_id: u32,
    /// `TraitDefinition.OverrideName_lang`; empty when the spell name applies.
    pub override_name: String,
    /// `TraitDefinition.OverrideIcon`; 0 when the spell icon applies.
    pub override_icon: u32,
    /// SPELL_ATTR0_PASSIVE of `spell_id` (`SpellMisc.Attributes_0 & 0x40`).
    pub passive: bool,
    pub max_ranks: u32,
    /// Hero sub-tree chosen by a `SubTreeSelection` entry; 0 otherwise.
    pub sub_tree_id: u32,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TalentNode {
    pub id: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub kind: NodeKind,
    /// Hero sub-tree the node belongs to; 0 for class and spec nodes.
    pub sub_tree_id: u32,
    /// `TraitNodeXTraitNodeEntry._Index` order.
    pub entries: Vec<TalentEntry>,
    /// `TraitNodeGroup` IDs, ascending.
    pub groups: Vec<u32>,
    /// `TraitEdge.LeftTraitNodeID` with the edge type.
    pub parents: Vec<(u32, EdgeKind)>,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

impl TalentNode {
    pub fn entry(&self, entry_id: u32) -> Option<&TalentEntry> {
        self.entries.iter().find(|entry| entry.id == entry_id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TalentGroup {
    pub id: u32,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalentCurrency {
    pub id: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TalentSubTree {
    pub id: u32,
    pub name: String,
}

/// A `ChrSpecialization` row of the tree's class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TalentSpec {
    pub id: u32,
    pub name: String,
    pub order_index: u32,
    pub icon_fdid: u32,
}

impl TalentSpec {
    /// `OrderIndex` 4 is the pre-level-10 Initial spec, which cannot be chosen.
    pub fn selectable(&self) -> bool {
        self.order_index != INITIAL_SPEC_ORDER_INDEX
    }
}

const INITIAL_SPEC_ORDER_INDEX: u32 = 4;
/// Server `SPEC_UNLOCK_LEVEL`: `SetSpecialization` is rejected below it.
pub const SPEC_UNLOCK_LEVEL: u8 = 10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TalentTree {
    pub class_id: u8,
    pub tree_id: u32,
    /// `TraitTreeXTraitCurrency` order.
    pub currencies: Vec<TalentCurrency>,
    /// Sorted by ID.
    pub nodes: Vec<TalentNode>,
    /// Sorted by ID.
    pub groups: Vec<TalentGroup>,
    pub sub_trees: Vec<TalentSubTree>,
    /// `OrderIndex` order.
    pub specs: Vec<TalentSpec>,
}

impl TalentTree {
    pub fn node(&self, node_id: u32) -> Option<&TalentNode> {
        let index = self
            .nodes
            .binary_search_by_key(&node_id, |node| node.id)
            .ok()?;
        Some(&self.nodes[index])
    }

    pub fn group(&self, group_id: u32) -> Option<&TalentGroup> {
        let index = self
            .groups
            .binary_search_by_key(&group_id, |group| group.id)
            .ok()?;
        Some(&self.groups[index])
    }

    pub fn currency_flags(&self, currency_id: u32) -> u32 {
        self.currencies
            .iter()
            .find(|currency| currency.id == currency_id)
            .map_or(0, |currency| currency.flags)
    }

    pub fn sub_tree_name(&self, sub_tree_id: u32) -> Option<&str> {
        self.sub_trees
            .iter()
            .find(|sub_tree| sub_tree.id == sub_tree_id)
            .map(|sub_tree| sub_tree.name.as_str())
    }
}

/// A `UiTextureAtlasMember` crop of its atlas texture.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtlasCrop {
    pub fdid: u32,
    /// Normalized (left, right, top, bottom).
    pub tex_coords: [f32; 4],
    pub width: f32,
    pub height: f32,
}

/// `ChrClasses` `Filename` (e.g. `PALADIN`) and `Name_lang`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassNames {
    pub file: String,
    pub name: String,
}

/// The ten character-creation class trees, sorted by tree ID, plus the Retail
/// `talents-*` atlas members (`UiTextureAtlasMember.CommittedName`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TalentTreeData {
    trees: Vec<TalentTree>,
    art: HashMap<String, AtlasCrop>,
    classes: HashMap<u8, ClassNames>,
}

impl TalentTreeData {
    pub fn new(
        mut trees: Vec<TalentTree>,
        art: HashMap<String, AtlasCrop>,
        classes: HashMap<u8, ClassNames>,
    ) -> Self {
        trees.sort_by_key(|tree| tree.tree_id);
        Self {
            trees,
            art,
            classes,
        }
    }

    pub fn class_name(&self, class_id: u8) -> Option<&str> {
        self.classes.get(&class_id).map(|names| names.name.as_str())
    }

    pub fn art(&self, name: &str) -> Option<AtlasCrop> {
        self.art.get(name).copied()
    }

    /// `talents-background-<class>-<spec>`, e.g. `talents-background-paladin-retribution`.
    pub fn spec_background(&self, tree: &TalentTree, spec_id: u32) -> Option<AtlasCrop> {
        let class = self.classes.get(&tree.class_id)?.file.to_lowercase();
        let spec = tree.specs.iter().find(|spec| spec.id == spec_id)?;
        let spec = spec.name.to_lowercase().replace([' ', '-'], "");
        self.art(&format!("talents-background-{class}-{spec}"))
    }

    pub fn tree(&self, tree_id: u32) -> Option<&TalentTree> {
        let index = self
            .trees
            .binary_search_by_key(&tree_id, |tree| tree.tree_id)
            .ok()?;
        Some(&self.trees[index])
    }

    pub fn trees(&self) -> &[TalentTree] {
        &self.trees
    }
}

#[derive(Debug, Default)]
pub enum TalentTreesState {
    #[default]
    Loading,
    Ready(TalentTreeData),
    Failed(String),
}

#[derive(Resource, Debug, Default)]
pub struct TalentTrees {
    pub state: TalentTreesState,
}

impl TalentTrees {
    pub fn data(&self) -> Option<&TalentTreeData> {
        match &self.state {
            TalentTreesState::Ready(data) => Some(data),
            _ => None,
        }
    }
}

pub fn talent_source_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("db2").join(SPELL_DB2_BUILD)
}

pub struct TalentTreePaths {
    pub data_dir: PathBuf,
    pub cache_path: PathBuf,
}

impl TalentTreePaths {
    pub fn for_data_dir(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            cache_path: data_dir
                .join("cache")
                .join(format!("talent_trees-{SPELL_DB2_BUILD}.bin")),
        }
    }
}

/// Trees from `data/db2/<build>/`, art from `data/UiTextureAtlas*.csv`; read
/// from the cache when it is fresh, otherwise rebuilt and cached.
pub fn load_talent_trees(paths: &TalentTreePaths) -> Result<TalentTreeData, String> {
    let source_dir = talent_source_dir(&paths.data_dir);
    let key = cache::cache_key(&paths.data_dir, &source_dir)?;
    if let Some(data) = cache::read_cache(&paths.cache_path, &key)? {
        return Ok(data);
    }
    let trees = load::load_trees(&source_dir)?;
    let art = load::load_atlas_crops(&paths.data_dir, "talents-")?;
    let classes = load::load_class_names(&source_dir)?;
    let data = TalentTreeData::new(trees, art, classes);
    cache::write_cache(&paths.cache_path, &key, &data)?;
    Ok(data)
}

#[derive(Resource)]
struct TalentTreesLoadTask(Task<Result<TalentTreeData, String>>);

pub struct TalentTreePlugin;

impl Plugin for TalentTreePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TalentTrees>()
            .add_systems(Startup, spawn_talent_tree_load)
            .add_systems(
                Update,
                finish_talent_tree_load.run_if(resource_exists::<TalentTreesLoadTask>),
            );
    }
}

fn spawn_talent_tree_load(mut commands: Commands) {
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let started = std::time::Instant::now();
        let result = load_talent_trees(&TalentTreePaths::for_data_dir(Path::new("data")));
        if let Ok(data) = &result {
            info!(
                "Talent trees loaded {} class trees in {:?}",
                data.trees().len(),
                started.elapsed()
            );
        }
        result
    });
    commands.insert_resource(TalentTreesLoadTask(task));
}

fn finish_talent_tree_load(
    mut commands: Commands,
    mut task: ResMut<TalentTreesLoadTask>,
    mut trees: ResMut<TalentTrees>,
) {
    let Some(result) = check_ready(&mut task.0) else {
        return;
    };
    commands.remove_resource::<TalentTreesLoadTask>();
    trees.state = match result {
        Ok(data) => TalentTreesState::Ready(data),
        Err(err) => {
            error!("Talent tree load failed: {err}");
            TalentTreesState::Failed(err)
        }
    };
}
