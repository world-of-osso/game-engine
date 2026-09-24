//! Local edits of a `TraitConfigSnapshot` config: add or remove one rank, kept
//! only when the result passes the mirrored [`rules::validate`].

use std::collections::HashMap;

use shared::protocol::{TraitConfigSnapshot, TraitEntrySelection};

use super::rules::{self, ConfigEntry, TraitContext};
use super::{NodeKind, TalentNode, TalentTree};

/// The snapshot's config split into bought and granted ranks, with the owned
/// currency implied by the snapshot.
pub struct TalentSession<'a> {
    pub tree: &'a TalentTree,
    pub ctx: TraitContext,
    /// Snapshot config.
    pub base: Vec<ConfigEntry>,
    owned: HashMap<u32, i32>,
}

impl<'a> TalentSession<'a> {
    pub fn new(tree: &'a TalentTree, level: u8, snapshot: &TraitConfigSnapshot) -> Self {
        let ctx = TraitContext {
            spec_id: snapshot.spec_id,
            level,
        };
        let base = snapshot_config(tree, ctx, &snapshot.entries);
        let spent = rules::spent_totals(tree, &base);
        let owned = snapshot
            .unspent
            .iter()
            .map(|&(currency, unspent)| {
                let spent = spent.get(&currency).copied().unwrap_or(0) as i32;
                (currency, unspent + spent)
            })
            .collect();
        Self {
            tree,
            ctx,
            base,
            owned,
        }
    }

    /// Owned minus spent per tree currency, in `TraitTreeXTraitCurrency` order.
    pub fn unspent(&self, config: &[ConfigEntry]) -> Vec<(u32, i32)> {
        let spent = rules::spent_totals(self.tree, config);
        self.tree
            .currencies
            .iter()
            .map(|currency| {
                let owned = self.owned.get(&currency.id).copied().unwrap_or(0);
                let spent = spent.get(&currency.id).copied().unwrap_or(0) as i32;
                (currency.id, owned - spent)
            })
            .collect()
    }

    pub fn is_valid(&self, config: &[ConfigEntry]) -> bool {
        rules::validate(self.tree, self.ctx, config, &self.owned).is_ok()
    }

    /// `config` plus one bought rank of `entry_id`, if the result is valid.
    /// Choosing another entry of a choice node replaces the bought choice.
    pub fn try_add(
        &self,
        config: &[ConfigEntry],
        node_id: u32,
        entry_id: u32,
    ) -> Option<Vec<ConfigEntry>> {
        let node = self.tree.node(node_id)?;
        node.entry(entry_id)?;
        let mut next = config.to_vec();
        if is_choice(node) {
            next.retain(|e| e.node_id != node_id || e.entry_id == entry_id || e.granted > 0);
        }
        match next
            .iter_mut()
            .find(|e| e.node_id == node_id && e.entry_id == entry_id)
        {
            Some(entry) => entry.ranks += 1,
            None => next.push(ConfigEntry {
                node_id,
                entry_id,
                ranks: 1,
                granted: 0,
            }),
        }
        self.is_valid(&next).then_some(next)
    }

    /// `config` minus one bought rank of `entry_id`, if the result is valid.
    pub fn try_remove(
        &self,
        config: &[ConfigEntry],
        node_id: u32,
        entry_id: u32,
    ) -> Option<Vec<ConfigEntry>> {
        let mut next = config.to_vec();
        let index = next
            .iter()
            .position(|e| e.node_id == node_id && e.entry_id == entry_id && e.ranks > 0)?;
        next[index].ranks -= 1;
        if next[index].total() == 0 {
            next.remove(index);
        }
        self.is_valid(&next).then_some(next)
    }

    /// Entry a click on the node's single button acts on: the only entry, or
    /// for tiered nodes the first entry not yet maxed.
    pub fn add_target(&self, config: &[ConfigEntry], node: &TalentNode) -> Option<u32> {
        match node.kind {
            NodeKind::Tiered => node
                .entries
                .iter()
                .find(|entry| total_ranks(config, node.id, entry.id) < entry.max_ranks)
                .map(|entry| entry.id),
            _ => node.entries.first().map(|entry| entry.id),
        }
    }

    /// Entry a right-click removes from: the last entry with bought ranks.
    pub fn remove_target(&self, config: &[ConfigEntry], node: &TalentNode) -> Option<u32> {
        node.entries
            .iter()
            .rev()
            .find(|entry| {
                config
                    .iter()
                    .any(|e| e.node_id == node.id && e.entry_id == entry.id && e.ranks > 0)
            })
            .map(|entry| entry.id)
    }
}

pub fn is_choice(node: &TalentNode) -> bool {
    matches!(node.kind, NodeKind::Selection | NodeKind::SubTreeSelection)
}

pub fn total_ranks(config: &[ConfigEntry], node_id: u32, entry_id: u32) -> u32 {
    config
        .iter()
        .filter(|e| e.node_id == node_id && e.entry_id == entry_id)
        .map(ConfigEntry::total)
        .sum()
}

/// Snapshot ranks are totals (granted plus bought); split them with the
/// locally computed granted ranks.
fn snapshot_config(
    tree: &TalentTree,
    ctx: TraitContext,
    entries: &[TraitEntrySelection],
) -> Vec<ConfigEntry> {
    let mut config = rules::granted_entries(tree, ctx);
    for selection in entries {
        let rank = u32::from(selection.rank);
        match config
            .iter_mut()
            .find(|e| e.node_id == selection.node_id && e.entry_id == selection.entry_id)
        {
            Some(entry) => {
                entry.granted = entry.granted.min(rank);
                entry.ranks = rank - entry.granted;
            }
            None => config.push(ConfigEntry {
                node_id: selection.node_id,
                entry_id: selection.entry_id,
                ranks: rank,
                granted: 0,
            }),
        }
    }
    config.retain(|entry| entry.total() > 0);
    config
}

/// `CommitTraitConfig.entries`: total ranks per entry, as the snapshot sends them.
pub fn commit_entries(config: &[ConfigEntry]) -> Vec<TraitEntrySelection> {
    config
        .iter()
        .filter(|entry| entry.total() > 0)
        .map(|entry| TraitEntrySelection {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            rank: entry.total() as u8,
        })
        .collect()
}
