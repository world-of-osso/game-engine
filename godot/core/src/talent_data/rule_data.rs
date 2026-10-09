//! Rule data used by the server-compatible client validator.

/// `TraitNode.Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitNodeType {
    Single,
    /// Entries are ranked one after another (apex talents).
    Tiered,
    /// Exactly one entry may be chosen.
    Selection,
    /// Exactly one entry may be chosen; each entry names a hero `TraitSubTree`.
    SubTreeSelection,
}

/// `TraitEdge.Type`. Only these two gate availability; the other DB2 values
/// (VisualOnly, deprecated, MutuallyExclusive) do not occur on class trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitEdgeType {
    /// Any one filled parent with this edge type makes the node available.
    SufficientForAvailability,
    /// This parent must be filled.
    RequiredForAvailability,
}

/// `TraitCond.CondType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitCondType {
    Available,
    Visible,
    Granted,
    Increased,
    /// UI error text only.
    DisplayError,
    /// Holding at least `granted_ranks` ranks requires the condition.
    RanksAllowed,
}

/// `TraitCond.Flags` IsGate.
pub const COND_FLAG_GATE: u32 = 0x1;
/// `TraitCond.Flags` IsSufficient: meeting this condition passes its whole list.
pub const COND_FLAG_SUFFICIENT: u32 = 0x4;

#[derive(Debug, Clone, PartialEq)]
pub struct TraitCond {
    pub id: u32,
    pub kind: TraitCondType,
    pub flags: u32,
    pub granted_ranks: u32,
    pub quest_id: u32,
    pub achievement_id: u32,
    /// `SpecSetMember.ChrSpecializationID` of `SpecSetID`; empty when `SpecSetID` is 0.
    pub spec_set: Vec<u32>,
    pub spec_set_id: u32,
    pub node_group_id: u32,
    pub node_id: u32,
    pub node_entry_id: u32,
    pub currency_id: u32,
    pub spent_amount_required: u32,
    pub required_level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitCost {
    pub currency_id: u32,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitNodeEntry {
    pub id: u32,
    pub definition_id: u32,
    /// `TraitDefinition.SpellID`; 0 when the entry has no definition spell.
    pub spell_id: u32,
    pub overrides_spell_id: u32,
    /// SPELL_ATTR0_PASSIVE of `spell_id`.
    pub passive: bool,
    pub max_ranks: u32,
    pub entry_type: u32,
    /// Hero sub-tree chosen by a `SubTreeSelection` entry; 0 otherwise.
    pub sub_tree_id: u32,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitNode {
    pub id: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub kind: TraitNodeType,
    pub flags: u32,
    /// Hero sub-tree the node belongs to; 0 for class and spec nodes.
    pub sub_tree_id: u32,
    /// `TraitNodeXTraitNodeEntry` order.
    pub entries: Vec<TraitNodeEntry>,
    /// `TraitNodeGroup` IDs, ascending.
    pub groups: Vec<u32>,
    /// `TraitEdge.LeftTraitNodeID` with the edge type.
    pub parents: Vec<(u32, TraitEdgeType)>,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

impl TraitNode {
    pub fn entry(&self, entry_id: u32) -> Option<&TraitNodeEntry> {
        self.entries.iter().find(|entry| entry.id == entry_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitNodeGroup {
    pub id: u32,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

/// One `TraitCurrencySource` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencySource {
    pub amount: i32,
    pub quest_id: u32,
    pub achievement_id: u32,
    pub player_level: u32,
    pub node_entry_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitCurrency {
    pub id: u32,
    /// `TraitCurrency.Flags`: 0x4 class-icon (class points), 0x8 spec-icon (spec points).
    pub flags: u32,
    pub sources: Vec<CurrencySource>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitSubTree {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitTree {
    pub tree_id: u32,
    /// `TraitTreeXTraitCurrency` order: class, spec, then hero currencies.
    pub currencies: Vec<TraitCurrency>,
    /// Sorted by ID.
    pub nodes: Vec<TraitNode>,
    /// Sorted by ID.
    pub groups: Vec<TraitNodeGroup>,
    pub sub_trees: Vec<TraitSubTree>,
}

impl TraitTree {
    pub fn node(&self, node_id: u32) -> Option<&TraitNode> {
        let index = self
            .nodes
            .binary_search_by_key(&node_id, |node| node.id)
            .ok()?;
        Some(&self.nodes[index])
    }

    pub fn group(&self, group_id: u32) -> Option<&TraitNodeGroup> {
        let index = self
            .groups
            .binary_search_by_key(&group_id, |group| group.id)
            .ok()?;
        Some(&self.groups[index])
    }
}
