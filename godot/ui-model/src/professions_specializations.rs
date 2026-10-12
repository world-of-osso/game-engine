//! Retail specialization draft. Server snapshots own committed knowledge and unlocks.
use shared::protocol::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecializationDraft {
    pub snapshot: Option<ProfessionSpecializationsSnapshot>,
    pub selected_tree: Option<u32>,
    pub selected_node: Option<u32>,
    pub purchases: BTreeMap<(u32, u32, u32), ProfessionTraitPurchase>,
    pub confirming_unlock: bool,
    pub pending: bool,
    pub status: String,
}
impl SpecializationDraft {
    pub fn tree(&self) -> Option<&ProfessionSpecTreeSnapshot> {
        self.snapshot
            .as_ref()?
            .trees
            .iter()
            .find(|t| Some(t.tree_id) == self.selected_tree)
    }
    pub fn node(&self) -> Option<&ProfessionSpecNodeSnapshot> {
        self.entry_for_node(self.selected_node?)
    }
    pub fn entry_for_node(&self, node_id: u32) -> Option<&ProfessionSpecNodeSnapshot> {
        let tree = self.tree()?;
        let entries: Vec<_> = tree
            .nodes
            .iter()
            .filter(|entry| entry.node_id == node_id)
            .collect();
        let unfinished: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| self.rank(tree.tree_id, entry) + entry.granted_rank < entry.max_rank)
            .collect();
        if let Some(entry) = unfinished.first() {
            return Some(*entry);
        }
        entries.last().copied()
    }
    pub fn path_nodes(&self) -> Vec<&ProfessionSpecNodeSnapshot> {
        let Some(tree) = self.tree() else {
            return vec![];
        };
        let mut ids = BTreeSet::new();
        tree.nodes
            .iter()
            .filter(|entry| entry.kind == 1 || entry.node_id == tree.root_node_id)
            .filter(|entry| ids.insert(entry.node_id))
            .filter_map(|entry| self.entry_for_node(entry.node_id))
            .collect()
    }
    pub fn display_ranks(&self, node_id: u32) -> Option<(u32, u32)> {
        self.path_ranks(node_id, true)
    }
    pub fn committed_display_ranks(&self, node_id: u32) -> Option<(u32, u32)> {
        self.path_ranks(node_id, false)
    }
    fn path_ranks(&self, node_id: u32, include_pending: bool) -> Option<(u32, u32)> {
        let tree = self.tree()?;
        let entries: Vec<_> = tree
            .nodes
            .iter()
            .filter(|entry| entry.node_id == node_id)
            .collect();
        let first = entries.first()?;
        let total = entries
            .iter()
            .map(|entry| {
                let purchased = if include_pending {
                    self.rank(tree.tree_id, entry)
                } else {
                    entry.rank
                };
                purchased + entry.granted_rank
            })
            .sum::<u32>();
        let maximum = entries.iter().map(|entry| entry.max_rank).sum::<u32>();
        // Mainline GetRanks excludes the first unlock tier of a profession path.
        let unlock = if first.kind == 1 { first.max_rank } else { 0 };
        Some((total.saturating_sub(unlock), maximum.saturating_sub(unlock)))
    }
    pub fn path_unlocked(&self, node_id: u32) -> bool {
        let Some(tree) = self.tree() else {
            return false;
        };
        if node_id == tree.root_node_id {
            return tree.unlocked;
        }
        tree.nodes
            .iter()
            .find(|entry| entry.node_id == node_id)
            .is_some_and(|entry| {
                entry.max_rank == 0 || self.rank(tree.tree_id, entry) + entry.granted_rank > 0
            })
    }
    pub fn rank(&self, tree: u32, node: &ProfessionSpecNodeSnapshot) -> u32 {
        self.purchases
            .get(&(tree, node.node_id, node.entry_id))
            .map_or(node.rank, |p| p.ranks)
    }
    fn spent_currency(&self, trait_currency_id: u32) -> u32 {
        let Some(snapshot) = &self.snapshot else {
            return 0;
        };
        snapshot
            .trees
            .iter()
            .flat_map(|tree| tree.nodes.iter().map(move |n| (tree.tree_id, n)))
            .map(|(tree, node)| {
                let ranks = self.rank(tree, node).saturating_sub(node.rank);
                let cost: u32 = node
                    .costs
                    .iter()
                    .filter(|c| c.trait_currency_id == trait_currency_id)
                    .map(|c| c.amount)
                    .sum();
                ranks.saturating_mul(cost)
            })
            .fold(0u32, u32::saturating_add)
    }
    pub fn remaining_knowledge(&self) -> u32 {
        let Some(snapshot) = &self.snapshot else {
            return 0;
        };
        let spent = snapshot
            .currencies
            .iter()
            .filter(|c| c.currency_id == snapshot.knowledge_currency_id)
            .map(|c| self.spent_currency(c.trait_currency_id))
            .fold(0u32, u32::saturating_add);
        snapshot.knowledge_available.saturating_sub(spent)
    }
    pub fn can_stage(&self) -> bool {
        if self.pending {
            return false;
        }
        let (Some(tree), Some(node)) = (self.tree(), self.node()) else {
            return false;
        };
        if !tree.unlocked
            || !node.available
            || self.rank(tree.tree_id, node) + node.granted_rank >= node.max_rank
        {
            return false;
        }
        self.can_afford(node)
    }
    fn can_afford(&self, node: &ProfessionSpecNodeSnapshot) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        let mut costs = BTreeMap::<u32, u32>::new();
        for cost in &node.costs {
            let amount = costs.entry(cost.trait_currency_id).or_default();
            *amount = amount.saturating_add(cost.amount);
        }
        costs.into_iter().all(|(trait_currency_id, amount)| {
            snapshot
                .currencies
                .iter()
                .find(|currency| currency.trait_currency_id == trait_currency_id)
                .is_some_and(|currency| {
                    currency
                        .available
                        .saturating_sub(self.spent_currency(trait_currency_id))
                        >= amount
                })
        })
    }
    pub fn stage_rank(&mut self) -> bool {
        if !self.can_stage() {
            return false;
        }
        let tree = self.tree().unwrap();
        let node = self.node().unwrap();
        let key = (tree.tree_id, node.node_id, node.entry_id);
        let purchase = ProfessionTraitPurchase {
            node_id: node.node_id,
            entry_id: node.entry_id,
            ranks: self.rank(tree.tree_id, node) + 1,
        };
        self.purchases.insert(key, purchase);
        true
    }
    pub fn commit_request(&mut self) -> Option<ProfessionSpecializationRequest> {
        if self.pending {
            return None;
        }
        let snapshot = self.snapshot.as_ref()?;
        let tree_id = self.selected_tree?;
        let has_changes = self.purchases.keys().any(|(tree, _, _)| *tree == tree_id);
        if !has_changes {
            return None;
        }
        let purchases = self.purchase_configuration(self.tree()?, None);
        let request = ProfessionSpecializationRequest::Commit {
            skill_line: snapshot.skill_line,
            tree_id,
            purchases,
        };
        self.pending = true;
        Some(request)
    }
    fn purchase_configuration(
        &self,
        tree: &ProfessionSpecTreeSnapshot,
        confirmed: Option<&ProfessionTraitPurchase>,
    ) -> Vec<ProfessionTraitPurchase> {
        tree.nodes
            .iter()
            .filter_map(|node| {
                let ranks = match confirmed {
                    Some(purchase)
                        if purchase.node_id == node.node_id
                            && purchase.entry_id == node.entry_id =>
                    {
                        purchase.ranks
                    }
                    _ => self.rank(tree.tree_id, node),
                };
                (ranks > 0).then_some(ProfessionTraitPurchase {
                    node_id: node.node_id,
                    entry_id: node.entry_id,
                    ranks,
                })
            })
            .collect()
    }
    pub fn can_unlock(&self) -> bool {
        !self.pending
            && self.tree().is_some_and(|t| {
                !t.unlocked
                    && t.nodes.iter().any(|node| {
                        node.node_id == t.root_node_id && node.available && self.can_afford(node)
                    })
            })
    }
    pub fn confirm_unlock(&mut self) -> Option<ProfessionSpecializationRequest> {
        if !self.confirming_unlock || !self.can_unlock() {
            return None;
        }
        let tree = self.tree()?;
        let node = self.entry_for_node(tree.root_node_id)?;
        let purchase = ProfessionTraitPurchase {
            node_id: node.node_id,
            entry_id: node.entry_id,
            ranks: node.rank + 1,
        };
        let request = ProfessionSpecializationRequest::Commit {
            skill_line: self.snapshot.as_ref()?.skill_line,
            tree_id: tree.tree_id,
            purchases: self.purchase_configuration(tree, Some(&purchase)),
        };
        self.confirming_unlock = false;
        self.pending = true;
        Some(request)
    }
    pub fn undo(&mut self) {
        if !self.pending {
            self.purchases
                .retain(|(tree, _, _), _| Some(*tree) != self.selected_tree);
        }
    }
    pub fn receive(&mut self, update: ProfessionSpecializationStateUpdate) {
        self.pending = false;
        self.status = update.error.clone().or(update.message).unwrap_or_default();
        if let Some(snapshot) = update.snapshot {
            if update.error.is_none() {
                self.purchases.clear();
            }
            self.snapshot = Some(snapshot);
            if self.tree().is_none() {
                self.selected_tree = self
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.trees.first())
                    .map(|t| t.tree_id);
                self.selected_node = self.tree().map(|t| t.root_node_id);
            }
        }
    }
    fn perk_tooltip_lines(
        &self,
        perk: &ProfessionSpecPerk,
    ) -> Option<Vec<crate::tooltip_presentation::TooltipLineState>> {
        use crate::tooltip_presentation::TooltipLineState;
        let path = self.entry_for_node(perk.path_node_id)?;
        let (committed, _) = self.committed_display_ranks(perk.path_node_id)?;
        let (staged, _) = self.display_ranks(perk.path_node_id)?;
        let state = if committed >= perk.required_points {
            "Earned"
        } else if staged >= perk.required_points {
            "Pending"
        } else {
            "Unearned"
        };
        let mut lines = vec![
            TooltipLineState::new(format!(
                "Requires {} points in {}",
                perk.required_points, path.name
            )),
            TooltipLineState::colored(state, [1.0; 4]),
        ];
        if perk.unlocks_subpath {
            lines.push(TooltipLineState::new("Unlocks a sub-specialization"));
        }
        Some(lines)
    }
    pub fn tooltip(&self, node_id: u32) -> Option<crate::game_tooltip::GameTooltip> {
        use crate::tooltip_presentation::{
            TOOLTIP_DESCRIPTION_COLOR, TooltipLineState, TooltipPresentation, description_lines,
        };
        let node = self.entry_for_node(node_id)?;
        let mut lines = description_lines(&node.description, TOOLTIP_DESCRIPTION_COLOR);
        if let Some(perk) = &node.perk {
            lines.extend(self.perk_tooltip_lines(perk)?);
        } else {
            let (rank, maximum) = self.display_ranks(node_id)?;
            lines.push(TooltipLineState::colored(
                format!("Rank {rank}/{maximum}"),
                [1.0; 4],
            ));
        }
        if !node.available {
            lines.push(TooltipLineState::colored(
                node.unavailable_reason.as_deref().unwrap_or("Locked"),
                [1.0, 0.1, 0.1, 1.0],
            ));
        }
        Some(crate::game_tooltip::GameTooltip::new(
            TooltipPresentation {
                title: node.name.clone(),
                lines,
                ..Default::default()
            },
            None,
        ))
    }
}
#[cfg(test)]
#[path = "professions_specializations_tests.rs"]
mod tests;
