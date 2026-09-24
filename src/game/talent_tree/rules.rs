//! Display-side mirror of game-server `trait_config` (a TrinityCore `TraitMgr`
//! port): granted entries, spent currency and config validation. The server is
//! authoritative; this only decides what the talent window lets the player try.
//!
//! Difference from the server: the owned currency comes from the last
//! `TraitConfigSnapshot` (`unspent` plus what its config spends) instead of the
//! `TraitCurrencySource` rows, so no level-to-points table is mirrored.

use std::collections::HashMap;

use super::{
    COND_FLAG_GATE, COND_FLAG_SUFFICIENT, CondKind, EdgeKind, NodeKind, TalentEntry, TalentNode,
    TalentTree, TraitCond, TraitCost,
};

/// The config's specialization and the player's level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitContext {
    pub spec_id: u32,
    pub level: u8,
}

/// One node entry of a config: `ranks` bought, `granted` free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigEntry {
    pub node_id: u32,
    pub entry_id: u32,
    pub ranks: u32,
    pub granted: u32,
}

impl ConfigEntry {
    pub fn total(&self) -> u32 {
        self.ranks + self.granted
    }
}

#[derive(Debug, Default)]
struct Spent {
    total: u32,
    /// (gate requirement of the spending node, amount).
    by_gate: Vec<(u32, u32)>,
}

type SpentMap = HashMap<u32, Spent>;

/// Ranks granted for free at this spec and level, capped at max ranks.
pub fn granted_entries(tree: &TalentTree, ctx: TraitContext) -> Vec<ConfigEntry> {
    let checker = Checker::new(tree, ctx, &[]);
    let mut granted: Vec<ConfigEntry> = Vec::new();
    let mut grant = |node: &TalentNode, entry_id: u32, ranks: u32| {
        let max = node.entry(entry_id).map_or(0, |entry| entry.max_ranks);
        match granted
            .iter_mut()
            .find(|e| e.node_id == node.id && e.entry_id == entry_id)
        {
            Some(existing) => existing.granted = (existing.granted + ranks).min(max),
            None => granted.push(ConfigEntry {
                node_id: node.id,
                entry_id,
                ranks: 0,
                granted: ranks.min(max),
            }),
        }
    };
    for node in &tree.nodes {
        let available = |entry_id| checker.node_meets_conditions(node, entry_id).is_ok();
        for entry in &node.entries {
            if available(entry.id) {
                for cond in checker.met_grants(&entry.conds) {
                    grant(node, entry.id, cond.granted_ranks);
                }
            }
        }
        let node_grants = checker.met_grants(&node.conds);
        let group_grants = node
            .groups
            .iter()
            .filter_map(|&id| tree.group(id))
            .flat_map(|group| checker.met_grants(&group.conds));
        for cond in node_grants.chain(group_grants) {
            for entry in node.entries.iter().filter(|entry| available(entry.id)) {
                grant(node, entry.id, cond.granted_ranks);
            }
        }
    }
    granted
}

/// Whether the node's Visible conditions pass for this spec and level.
pub fn node_visible(tree: &TalentTree, ctx: TraitContext, node: &TalentNode) -> bool {
    let checker = Checker::new(tree, ctx, &[]);
    node.entries.iter().any(|entry| {
        checker
            .meets_conditions_of_type(node, entry.id, CondKind::Visible)
            .is_ok()
    })
}

/// Spent currency per currency ID (costs times bought ranks).
pub fn spent_totals(tree: &TalentTree, entries: &[ConfigEntry]) -> HashMap<u32, u32> {
    spent_currencies(tree, entries)
        .into_iter()
        .map(|(currency, spent)| (currency, spent.total))
        .collect()
}

/// Hero sub-tree chosen by a `SubTreeSelection` node, if any.
pub fn selected_sub_tree(tree: &TalentTree, entries: &[ConfigEntry]) -> Option<u32> {
    selected_sub_trees(tree, entries).first().copied()
}

/// Server `validate`, with the budget checked against `owned`.
pub fn validate(
    tree: &TalentTree,
    ctx: TraitContext,
    entries: &[ConfigEntry],
    owned: &HashMap<u32, i32>,
) -> Result<(), String> {
    let checker = Checker::new(tree, ctx, entries);
    for entry in entries {
        checker.check_entry(entry)?;
    }
    check_sub_trees(tree, entries)?;
    check_budget(owned, &checker.spent)
}

fn selected_sub_trees(tree: &TalentTree, entries: &[ConfigEntry]) -> Vec<u32> {
    entries
        .iter()
        .filter_map(|config_entry| {
            let node = tree.node(config_entry.node_id)?;
            if node.kind != NodeKind::SubTreeSelection || config_entry.total() == 0 {
                return None;
            }
            node.entry(config_entry.entry_id)
                .map(|entry| entry.sub_tree_id)
        })
        .collect()
}

fn check_sub_trees(tree: &TalentTree, entries: &[ConfigEntry]) -> Result<(), String> {
    let selected = selected_sub_trees(tree, entries);
    if selected.len() > 1 {
        return Err(format!(
            "only one hero talent tree may be selected, got {selected:?}"
        ));
    }
    for entry in entries.iter().filter(|entry| entry.ranks > 0) {
        let Some(node) = tree.node(entry.node_id) else {
            continue;
        };
        if node.sub_tree_id != 0 && selected.first() != Some(&node.sub_tree_id) {
            return Err(format!(
                "node {} belongs to hero talent tree {}, which is not selected",
                node.id, node.sub_tree_id
            ));
        }
    }
    Ok(())
}

fn check_budget(owned: &HashMap<u32, i32>, spent: &SpentMap) -> Result<(), String> {
    let mut spent: Vec<_> = spent.iter().filter(|(_, spent)| spent.total > 0).collect();
    spent.sort_by_key(|(currency, _)| **currency);
    for (currency, spent) in spent {
        let owned = owned.get(currency).copied().unwrap_or(0);
        if owned < spent.total as i32 {
            return Err(format!(
                "config spends {} points of currency {currency}, {owned} owned",
                spent.total
            ));
        }
    }
    Ok(())
}

fn gate_requirements(tree: &TalentTree, node: &TalentNode) -> HashMap<u32, u32> {
    let group_conds = node
        .groups
        .iter()
        .filter_map(|&id| tree.group(id))
        .flat_map(|group| &group.conds);
    let mut gates: HashMap<u32, u32> = HashMap::new();
    for cond in node.conds.iter().chain(group_conds) {
        if cond.flags & COND_FLAG_GATE != 0 {
            let gate = gates.entry(cond.currency_id).or_default();
            *gate = (*gate).max(cond.spent_amount_required);
        }
    }
    gates
}

/// Group, entry and node costs of one node entry.
pub fn entry_costs<'a>(
    tree: &'a TalentTree,
    node: &'a TalentNode,
    entry_id: u32,
) -> impl Iterator<Item = &'a TraitCost> + 'a {
    let group_costs = node
        .groups
        .iter()
        .filter_map(|&id| tree.group(id))
        .flat_map(|group| &group.costs);
    let own_costs = node
        .entry(entry_id)
        .into_iter()
        .flat_map(|entry| &entry.costs);
    group_costs.chain(own_costs).chain(&node.costs)
}

fn spent_currencies(tree: &TalentTree, entries: &[ConfigEntry]) -> SpentMap {
    let mut spent = SpentMap::new();
    for config_entry in entries.iter().filter(|entry| entry.ranks > 0) {
        let Some(node) = tree.node(config_entry.node_id) else {
            continue;
        };
        let gates = gate_requirements(tree, node);
        for cost in entry_costs(tree, node, config_entry.entry_id) {
            let amount = cost.amount * config_entry.ranks;
            let currency = spent.entry(cost.currency_id).or_default();
            currency.total += amount;
            let gate = gates.get(&cost.currency_id).copied().unwrap_or(0);
            match currency.by_gate.iter_mut().find(|(g, _)| *g == gate) {
                Some((_, sum)) => *sum += amount,
                None => currency.by_gate.push((gate, amount)),
            }
        }
    }
    spent
}

#[derive(Default)]
struct CondListResult {
    sufficient: bool,
    failure: Option<String>,
}

struct Checker<'a> {
    tree: &'a TalentTree,
    ctx: TraitContext,
    entries: &'a [ConfigEntry],
    spent: SpentMap,
}

impl<'a> Checker<'a> {
    fn new(tree: &'a TalentTree, ctx: TraitContext, entries: &'a [ConfigEntry]) -> Self {
        Self {
            tree,
            ctx,
            entries,
            spent: spent_currencies(tree, entries),
        }
    }

    fn check_entry(&self, config_entry: &ConfigEntry) -> Result<(), String> {
        let node = self
            .tree
            .node(config_entry.node_id)
            .ok_or_else(|| format!("node {} is not in the tree", config_entry.node_id))?;
        let entry = node.entry(config_entry.entry_id).ok_or_else(|| {
            format!(
                "entry {} does not belong to node {}",
                config_entry.entry_id, node.id
            )
        })?;
        if config_entry.total() > entry.max_ranks {
            return Err(format!(
                "entry {} of node {} has {} ranks, max {}",
                entry.id,
                node.id,
                config_entry.total(),
                entry.max_ranks
            ));
        }
        if matches!(node.kind, NodeKind::Selection | NodeKind::SubTreeSelection) {
            let chosen = self
                .entries
                .iter()
                .filter(|other| other.node_id == node.id)
                .count();
            if chosen != 1 {
                return Err(format!(
                    "choice node {} must have exactly one entry, got {chosen}",
                    node.id
                ));
            }
        }
        self.node_meets_conditions(node, entry.id)
            .map_err(|reason| format!("node {} is not available: {reason}", node.id))?;
        self.check_parents(node)
    }

    fn is_filled(&self, node: &TalentNode) -> bool {
        let maxed = |entry: &TalentEntry| {
            self.entries.iter().any(|config_entry| {
                config_entry.node_id == node.id
                    && config_entry.entry_id == entry.id
                    && config_entry.total() == entry.max_ranks
            })
        };
        match node.kind {
            NodeKind::Selection => node.entries.iter().any(maxed),
            _ => node.entries.iter().all(maxed),
        }
    }

    fn check_parents(&self, node: &TalentNode) -> Result<(), String> {
        if node.parents.is_empty() {
            return Ok(());
        }
        let mut any_filled = false;
        for &(parent_id, kind) in &node.parents {
            let filled = self
                .tree
                .node(parent_id)
                .is_some_and(|parent| self.is_filled(parent));
            if filled {
                any_filled = true;
            } else if kind == EdgeKind::RequiredForAvailability {
                return Err(format!(
                    "node {} requires node {parent_id} fully ranked",
                    node.id
                ));
            }
        }
        if any_filled {
            return Ok(());
        }
        let parents: Vec<u32> = node.parents.iter().map(|(id, _)| *id).collect();
        Err(format!(
            "node {} requires one of nodes {parents:?} fully ranked",
            node.id
        ))
    }

    fn node_meets_conditions(&self, node: &TalentNode, entry_id: u32) -> Result<(), String> {
        [
            CondKind::Visible,
            CondKind::Available,
            CondKind::RanksAllowed,
        ]
        .into_iter()
        .try_for_each(|kind| self.meets_conditions_of_type(node, entry_id, kind))
    }

    fn meets_conditions_of_type(
        &self,
        node: &TalentNode,
        entry_id: u32,
        kind: CondKind,
    ) -> Result<(), String> {
        let node_ranks = self.ranks_where(|entry| entry.node_id == node.id);
        let entry_ranks =
            self.ranks_where(|entry| entry.node_id == node.id && entry.entry_id == entry_id);
        let mut lists = vec![self.check_cond_list(&node.conds, kind, node_ranks)];
        for group in node.groups.iter().filter_map(|&id| self.tree.group(id)) {
            let group_ranks = self.ranks_where(|entry| {
                self.tree
                    .node(entry.node_id)
                    .is_some_and(|other| other.groups.contains(&group.id))
            });
            lists.push(self.check_cond_list(&group.conds, kind, group_ranks));
        }
        if let Some(entry) = node.entry(entry_id) {
            lists.push(self.check_cond_list(&entry.conds, kind, entry_ranks));
        }
        if lists.iter().any(|list| list.sufficient) {
            return Ok(());
        }
        match lists.into_iter().find_map(|list| list.failure) {
            Some(reason) => Err(reason),
            None => Ok(()),
        }
    }

    fn ranks_where(&self, filter: impl Fn(&ConfigEntry) -> bool) -> u32 {
        self.entries
            .iter()
            .filter(|entry| filter(entry))
            .map(|entry| entry.ranks)
            .sum()
    }

    fn check_cond_list(&self, conds: &[TraitCond], kind: CondKind, ranks: u32) -> CondListResult {
        let mut result = CondListResult::default();
        for cond in conds.iter().filter(|cond| cond.kind == kind) {
            if kind == CondKind::RanksAllowed && ranks < cond.granted_ranks {
                continue;
            }
            match self.meets_cond(cond) {
                Err(reason) => {
                    result.failure.get_or_insert(reason);
                }
                Ok(()) if cond.flags & COND_FLAG_SUFFICIENT != 0 => {
                    result.sufficient = true;
                    break;
                }
                Ok(()) => {}
            }
        }
        result
    }

    fn met_grants(&self, conds: &'a [TraitCond]) -> impl Iterator<Item = &'a TraitCond> + '_ {
        conds
            .iter()
            .filter(|cond| cond.kind == CondKind::Granted && self.meets_cond(cond).is_ok())
    }

    fn meets_cond(&self, cond: &TraitCond) -> Result<(), String> {
        if cond.quest_id != 0 {
            return Err(format!(
                "condition {} requires quest {}",
                cond.id, cond.quest_id
            ));
        }
        if cond.achievement_id != 0 {
            return Err(format!(
                "condition {} requires achievement {}",
                cond.id, cond.achievement_id
            ));
        }
        if cond.spec_set_id != 0 && !cond.spec_set.contains(&self.ctx.spec_id) {
            return Err(format!(
                "condition {} requires a spec in spec set {}",
                cond.id, cond.spec_set_id
            ));
        }
        let targets_node = cond.node_group_id != 0 || cond.node_id != 0 || cond.node_entry_id != 0;
        if cond.currency_id != 0 {
            if targets_node {
                self.meets_spent_requirement(cond)?;
            }
        } else if targets_node {
            self.meets_rank_requirement(cond)?;
        }
        if cond.required_level != 0 && u32::from(self.ctx.level) < cond.required_level {
            return Err(format!(
                "condition {} requires level {}",
                cond.id, cond.required_level
            ));
        }
        Ok(())
    }

    fn meets_spent_requirement(&self, cond: &TraitCond) -> Result<(), String> {
        let spent_before: u32 = self.spent.get(&cond.currency_id).map_or(0, |spent| {
            spent
                .by_gate
                .iter()
                .filter(|(gate, _)| *gate < cond.spent_amount_required)
                .map(|(_, amount)| amount)
                .sum()
        });
        if spent_before < cond.spent_amount_required {
            return Err(format!(
                "condition {} requires {} points of currency {} spent in earlier rows, {spent_before} spent",
                cond.id, cond.spent_amount_required, cond.currency_id
            ));
        }
        Ok(())
    }

    fn meets_rank_requirement(&self, cond: &TraitCond) -> Result<(), String> {
        let group = self.ranks_where(|entry| {
            cond.node_group_id != 0
                && self
                    .tree
                    .node(entry.node_id)
                    .is_some_and(|node| node.groups.contains(&cond.node_group_id))
        });
        let node = self.ranks_where(|entry| entry.node_id == cond.node_id);
        let entry = self.ranks_where(|entry| entry.entry_id == cond.node_entry_id);
        let required = cond.spent_amount_required;
        if required != 0 && group < required && node < required && entry < required {
            return Err(format!(
                "condition {} requires {required} ranks in group {}, node {} or entry {}",
                cond.id, cond.node_group_id, cond.node_id, cond.node_entry_id
            ));
        }
        if required == 0 && group != 0 && node != 0 && entry != 0 {
            return Err(format!(
                "condition {} requires no ranks in group {}, node {} or entry {}",
                cond.id, cond.node_group_id, cond.node_id, cond.node_entry_id
            ));
        }
        Ok(())
    }
}
