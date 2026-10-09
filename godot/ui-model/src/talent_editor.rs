//! Retail staged talent edits: SharedTalentUI purchase/refund/selection and rollback.
use crate::talents::TalentView;
use game_engine_core::talent_data::{
    rule_data::{TraitNodeType, TraitTree},
    rules::{Purchase, TraitContext, build_config, granted_entries},
};
use shared::protocol::{
    CommitTraitConfig, TraitCommitResult, TraitConfigSnapshot, TraitEntrySelection,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalentEditor {
    pub snapshot: Option<TraitConfigSnapshot>,
    pub error_text: Option<String>,
    pub choice_node: Option<u32>,
    pending: Vec<TraitEntrySelection>,
    level: u8,
}
impl TalentEditor {
    pub fn receive_snapshot(&mut self, mut snapshot: TraitConfigSnapshot) {
        snapshot
            .entries
            .sort_by_key(|entry| (entry.node_id, entry.entry_id));
        self.pending = snapshot.entries.clone();
        self.snapshot = Some(snapshot);
        self.choice_node = None;
        self.error_text = None;
    }
    pub fn receive_result(&mut self, result: TraitCommitResult) {
        self.error_text = if result.ok { None } else { result.reason };
    }
    pub fn rank(&self, node: u32, entry: u32) -> u8 {
        self.pending
            .iter()
            .find(|selected| selected.node_id == node && selected.entry_id == entry)
            .map_or(0, |selected| selected.rank)
    }
    pub fn node_rank(&self, node: u32) -> u32 {
        self.pending
            .iter()
            .filter(|entry| entry.node_id == node)
            .map(|entry| u32::from(entry.rank))
            .sum()
    }
    pub fn dirty(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.entries != self.pending)
    }
    pub fn undo(&mut self) {
        self.pending = self
            .snapshot
            .as_ref()
            .map_or_else(Vec::new, |snapshot| snapshot.entries.clone());
        self.choice_node = None;
        self.error_text = None;
    }
    pub fn apply(&self) -> Option<CommitTraitConfig> {
        let snapshot = self.snapshot.as_ref()?;
        self.dirty().then(|| CommitTraitConfig {
            spec_id: snapshot.spec_id,
            entries: self.pending.clone(),
        })
    }
    pub fn unspent(&self, view: &TalentView) -> Vec<(u32, i32)> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };
        let Some(tree) = &view.rules else {
            return snapshot.unspent.clone();
        };
        let committed = allocation_costs(tree, &snapshot.entries);
        let pending = allocation_costs(tree, &self.pending);
        snapshot
            .unspent
            .iter()
            .map(|&(id, amount)| {
                let delta = committed.get(&id).copied().unwrap_or(0)
                    - pending.get(&id).copied().unwrap_or(0);
                let bonus = entry_source_amount(tree, id, self.level, &self.pending)
                    - entry_source_amount(tree, id, self.level, &snapshot.entries);
                (id, amount + delta + bonus)
            })
            .collect()
    }
    fn valid(&self, view: &TalentView, level: u8, entries: &[TraitEntrySelection]) -> bool {
        let (Some(snapshot), Some(tree)) = (&self.snapshot, &view.rules) else {
            return false;
        };
        if tree.tree_id != snapshot.tree_id {
            return false;
        }
        let context = TraitContext {
            spec_id: snapshot.spec_id,
            level,
        };
        let granted = granted_entries(tree, context);
        let purchases: Vec<_> = entries
            .iter()
            .map(|entry| {
                let free = granted
                    .iter()
                    .find(|grant| {
                        grant.node_id == entry.node_id && grant.entry_id == entry.entry_id
                    })
                    .map_or(0, |grant| grant.granted);
                Purchase {
                    node_id: entry.node_id,
                    entry_id: entry.entry_id,
                    ranks: u32::from(entry.rank).saturating_sub(free),
                }
            })
            .filter(|entry| entry.ranks > 0)
            .collect();
        if build_config(tree, context, &purchases).is_err() {
            return false;
        }
        let mut candidate = self.clone();
        candidate.pending = entries.to_vec();
        candidate.level = level;
        candidate
            .unspent(view)
            .iter()
            .all(|&(_, amount)| amount >= 0)
    }
    fn purchased_entries(
        &self,
        view: &TalentView,
        node_id: u32,
        entry_id: u32,
    ) -> Option<Vec<TraitEntrySelection>> {
        let tree = view.rules.as_ref()?;
        let node = tree.node(node_id)?;
        let entry = node.entry(entry_id)?;
        if node.kind == TraitNodeType::Tiered {
            let earlier_tiers = node.entries.iter().take_while(|entry| entry.id != entry_id);
            if earlier_tiers
                .into_iter()
                .any(|entry| u32::from(self.rank(node_id, entry.id)) < entry.max_ranks)
            {
                return None;
            }
        }
        let mut entries = self.pending.clone();
        if matches!(
            node.kind,
            TraitNodeType::Selection | TraitNodeType::SubTreeSelection
        ) && self.rank(node_id, entry_id) == 0
        {
            entries.retain(|entry| entry.node_id != node_id);
        }
        let rank = entries
            .iter()
            .find(|entry| entry.node_id == node_id && entry.entry_id == entry_id)
            .map_or(0, |entry| u32::from(entry.rank));
        if rank >= entry.max_ranks {
            return None;
        }
        if let Some(selected) = entries
            .iter_mut()
            .find(|entry| entry.node_id == node_id && entry.entry_id == entry_id)
        {
            selected.rank = selected.rank.checked_add(1)?;
        } else {
            entries.push(TraitEntrySelection {
                node_id,
                entry_id,
                rank: 1,
            });
        }
        entries.sort_by_key(|entry| (entry.node_id, entry.entry_id));
        Some(entries)
    }
    pub fn can_purchase(&self, view: &TalentView, level: u8, node: u32, entry: u32) -> bool {
        self.purchased_entries(view, node, entry)
            .is_some_and(|entries| self.valid(view, level, &entries))
    }
    pub fn purchase(&mut self, view: &TalentView, level: u8, node: u32, entry: u32) -> bool {
        let Some(entries) = self.purchased_entries(view, node, entry) else {
            return false;
        };
        if !self.valid(view, level, &entries) {
            return false;
        }
        self.pending = entries;
        self.level = level;
        self.choice_node = None;
        self.error_text = None;
        true
    }
    fn refunded_entries(
        &self,
        view: &TalentView,
        level: u8,
        node_id: u32,
    ) -> Option<Vec<TraitEntrySelection>> {
        let snapshot = self.snapshot.as_ref()?;
        let tree = view.rules.as_ref()?;
        let node = tree.node(node_id)?;
        let granted = granted_entries(
            tree,
            TraitContext {
                spec_id: snapshot.spec_id,
                level,
            },
        );
        let entry_id = node.entries.iter().rev().find_map(|entry| {
            let free = granted
                .iter()
                .find(|grant| grant.node_id == node_id && grant.entry_id == entry.id)
                .map_or(0, |grant| grant.granted);
            (u32::from(self.rank(node_id, entry.id)) > free).then_some(entry.id)
        })?;
        let mut entries = self.pending.clone();
        let entry = entries
            .iter_mut()
            .find(|entry| entry.node_id == node_id && entry.entry_id == entry_id)?;
        entry.rank -= 1;
        entries.retain(|entry| entry.rank > 0);
        Some(entries)
    }
    pub fn refund(&mut self, view: &TalentView, level: u8, node_id: u32) -> bool {
        let Some(entries) = self.refunded_entries(view, level, node_id) else {
            return false;
        };
        if !self.valid(view, level, &entries) {
            return false;
        }
        self.pending = entries;
        self.level = level;
        self.choice_node = None;
        self.error_text = None;
        true
    }
    pub fn click(&mut self, view: &TalentView, level: u8, node_id: u32, right: bool) -> bool {
        if right {
            return self.refund(view, level, node_id);
        }
        let Some(node) = view.rules.as_ref().and_then(|tree| tree.node(node_id)) else {
            return false;
        };
        if matches!(
            node.kind,
            TraitNodeType::Selection | TraitNodeType::SubTreeSelection
        ) {
            if node
                .entries
                .iter()
                .any(|entry| self.can_purchase(view, level, node_id, entry.id))
            {
                self.choice_node = Some(node_id);
                return true;
            }
            return false;
        }
        let Some(entry) = node
            .entries
            .iter()
            .find(|entry| u32::from(self.rank(node_id, entry.id)) < entry.max_ranks)
        else {
            return false;
        };
        self.purchase(view, level, node_id, entry.id)
    }
    pub fn action(
        &mut self,
        view: &TalentView,
        level: u8,
        action: &str,
    ) -> Result<Option<CommitTraitConfig>, String> {
        match action {
            "talent:apply" => return Ok(self.apply()),
            "talent:undo" | "talent:reset" => {
                self.undo();
                return Ok(None);
            }
            "talent:close_choice" => {
                self.choice_node = None;
                return Ok(None);
            }
            _ => (),
        }
        let parts: Vec<_> = action.split(':').collect();
        match parts.as_slice() {
            ["talent", "node", node] => {
                self.click(view, level, parse_id(node)?, false);
            }
            ["talent", "refund", node] => {
                self.click(view, level, parse_id(node)?, true);
            }
            ["talent", "choice", node, entry] => {
                self.purchase(view, level, parse_id(node)?, parse_id(entry)?);
            }
            _ => return Err(format!("Unknown talent action: {action}")),
        }
        Ok(None)
    }
}
fn parse_id(raw: &str) -> Result<u32, String> {
    raw.parse()
        .map_err(|error| format!("Talent ID {raw}: {error}"))
}
fn allocation_costs(tree: &TraitTree, entries: &[TraitEntrySelection]) -> BTreeMap<u32, i32> {
    let mut spent = BTreeMap::new();
    for entry in entries {
        let Some(node) = tree.node(entry.node_id) else {
            continue;
        };
        let groups = node
            .groups
            .iter()
            .filter_map(|&id| tree.group(id))
            .flat_map(|group| &group.costs);
        let costs = groups
            .chain(
                node.entry(entry.entry_id)
                    .into_iter()
                    .flat_map(|entry| &entry.costs),
            )
            .chain(&node.costs);
        // Free ranks cancel between committed/pending allocations; they cannot be refunded.
        for cost in costs {
            *spent.entry(cost.currency_id).or_default() +=
                cost.amount as i32 * i32::from(entry.rank);
        }
    }
    spent
}
fn entry_source_amount(
    tree: &TraitTree,
    currency: u32,
    level: u8,
    entries: &[TraitEntrySelection],
) -> i32 {
    tree.currencies
        .iter()
        .filter(|c| c.id == currency)
        .flat_map(|c| &c.sources)
        .filter(|source| {
            source.node_entry_id != 0
                && source.quest_id == 0
                && source.achievement_id == 0
                && source.player_level <= u32::from(level)
                && entries
                    .iter()
                    .any(|entry| entry.entry_id == source.node_entry_id && entry.rank > 0)
        })
        .map(|source| source.amount)
        .sum()
}
