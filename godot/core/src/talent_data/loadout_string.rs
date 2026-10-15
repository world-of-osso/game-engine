//! Retail Loadout v2. Field/order provenance: Blizzard_ClassTalentImportExport.lua
//! 8–45,69–103,113–159,287–321,324–412; ExportUtil.lua 6–69,120–152.
//! Numeric version: installed TalentLoadoutManager/core/ImportExportV2.lua:5
//! and Simulationcraft/core.lua:63 (source hashes in docs/specs/talent-loadouts.md).
use super::{
    rule_data::{TraitNode, TraitNodeType, TraitTree},
    rules::{ConfigEntry, Purchase, TraitContext, build_config, granted_entries},
};

const VERSION: u32 = 2;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadoutError {
    BadString,
    Version,
    WrongSpec,
    InvalidNodes,
}
impl LoadoutError {
    /// Installed Retail GlobalStrings.csv IDs49511,49512,49514,49515.
    pub fn text(self) -> &'static str {
        match self {
            Self::BadString => "Error: Invalid loadout code",
            Self::Version => {
                "Error: The loadout you are trying to import was saved from an old version"
            }
            Self::WrongSpec => {
                "Error: The loadout you are trying to import is for a different specialization"
            }
            Self::InvalidNodes => "Failed to import loadout",
        }
    }
}

#[derive(Default)]
struct Stream {
    bits: Vec<bool>,
    cursor: usize,
}
impl Stream {
    fn write(&mut self, width: usize, value: u32) {
        self.bits
            .extend((0..width).map(|bit| value & (1 << bit) != 0));
    }
    fn read(&mut self, width: usize) -> Result<u32, LoadoutError> {
        let bits = self
            .bits
            .get(self.cursor..self.cursor + width)
            .ok_or(LoadoutError::BadString)?;
        let value = bits
            .iter()
            .enumerate()
            .fold(0, |value, (bit, set)| value | (u32::from(*set) << bit));
        self.cursor += width;
        Ok(value)
    }
    fn from_text(text: &str) -> Result<Self, LoadoutError> {
        let mut stream = Self::default();
        for byte in text.bytes() {
            let value = ALPHABET
                .iter()
                .position(|&candidate| candidate == byte)
                .ok_or(LoadoutError::BadString)?;
            stream.write(6, value as u32);
        }
        Ok(stream)
    }
    fn text(&self) -> String {
        self.bits
            .chunks(6)
            .map(|bits| {
                let value = bits
                    .iter()
                    .enumerate()
                    .fold(0, |value, (bit, set)| value | (usize::from(*set) << bit));
                char::from(ALPHABET[value])
            })
            .collect()
    }
}
fn choice(node: &TraitNode) -> bool {
    matches!(
        node.kind,
        TraitNodeType::Selection | TraitNodeType::SubTreeSelection
    )
}
fn max_ranks(node: &TraitNode) -> u32 {
    if node.kind == TraitNodeType::Tiered {
        node.entries.iter().map(|entry| entry.max_ranks).sum()
    } else {
        node.entries
            .iter()
            .map(|entry| entry.max_ranks)
            .max()
            .unwrap_or(0)
    }
}

/// Includes every node of the class tree, not just the current spec projection.
/// Zero hash is explicitly permitted for third-party generators by Retail Lua16–17.
pub fn encode(
    tree: &TraitTree,
    spec: u32,
    entries: &[ConfigEntry],
) -> Result<String, LoadoutError> {
    if spec > u16::MAX.into() {
        return Err(LoadoutError::WrongSpec);
    }
    if entries.iter().any(|entry| {
        tree.node(entry.node_id)
            .and_then(|node| node.entry(entry.entry_id))
            .is_none()
    }) {
        return Err(LoadoutError::InvalidNodes);
    }
    let mut stream = Stream::default();
    stream.write(8, VERSION);
    stream.write(16, spec);
    for _ in 0..16 {
        stream.write(8, 0);
    }
    for node in &tree.nodes {
        let selected: Vec<_> = entries
            .iter()
            .filter(|entry| entry.node_id == node.id && entry.total() > 0)
            .collect();
        let purchased: u32 = selected.iter().map(|entry| entry.ranks).sum();
        stream.write(1, u32::from(!selected.is_empty()));
        if selected.is_empty() {
            continue;
        }
        stream.write(1, u32::from(purchased > 0));
        if purchased == 0 {
            continue;
        }
        if purchased > max_ranks(node) || purchased > 63 {
            return Err(LoadoutError::InvalidNodes);
        }
        let partial = purchased != max_ranks(node);
        stream.write(1, u32::from(partial));
        if partial {
            stream.write(6, purchased);
        }
        stream.write(1, u32::from(choice(node)));
        if choice(node) {
            if selected.len() != 1 {
                return Err(LoadoutError::InvalidNodes);
            }
            let index = node
                .entries
                .iter()
                .position(|entry| entry.id == selected[0].entry_id)
                .ok_or(LoadoutError::InvalidNodes)?;
            if index > 3 {
                return Err(LoadoutError::InvalidNodes);
            }
            stream.write(2, index as u32);
        }
    }
    Ok(stream.text())
}

/// Hash bytes are consumed but not compared: C_Traits.GetTreeHash is unavailable
/// locally. Lead-approved compatibility boundary; strict node/rule checks remain.
pub fn decode(
    tree: &TraitTree,
    context: TraitContext,
    text: &str,
) -> Result<Vec<ConfigEntry>, LoadoutError> {
    let mut stream = Stream::from_text(text)?;
    if stream.bits.len() < 152 {
        return Err(LoadoutError::BadString);
    }
    if stream.read(8)? != VERSION {
        return Err(LoadoutError::Version);
    }
    if stream.read(16)? != context.spec_id {
        return Err(LoadoutError::WrongSpec);
    }
    for _ in 0..16 {
        stream.read(8)?;
    }
    let grants = granted_entries(tree, context);
    let mut purchases = Vec::new();
    for node in &tree.nodes {
        if stream.read(1)? == 0 {
            continue;
        }
        if stream.read(1)? == 0 {
            if !grants
                .iter()
                .any(|entry| entry.node_id == node.id && entry.granted == 1)
            {
                return Err(LoadoutError::InvalidNodes);
            }
            continue;
        }
        let partial = stream.read(1)? != 0;
        let ranks = if partial {
            stream.read(6)?
        } else {
            max_ranks(node)
        };
        let is_choice = stream.read(1)? != 0;
        if is_choice != choice(node)
            || ranks == 0
            || ranks > max_ranks(node)
            || (partial && ranks == max_ranks(node))
        {
            return Err(LoadoutError::InvalidNodes);
        }
        let index = if is_choice {
            stream.read(2)? as usize
        } else {
            0
        };
        let selected = node.entries.get(index).ok_or(LoadoutError::InvalidNodes)?;
        if node.kind == TraitNodeType::Tiered {
            let mut remaining = ranks;
            for entry in &node.entries {
                let ranks = remaining.min(entry.max_ranks);
                if ranks > 0 {
                    purchases.push(Purchase {
                        node_id: node.id,
                        entry_id: entry.id,
                        ranks,
                    });
                }
                remaining -= ranks;
            }
        } else {
            purchases.push(Purchase {
                node_id: node.id,
                entry_id: selected.id,
                ranks,
            });
        }
    }
    // Only the final base64 symbol's zero padding may remain.
    if stream.bits.len() - stream.cursor > 5 || stream.bits[stream.cursor..].iter().any(|bit| *bit)
    {
        return Err(LoadoutError::BadString);
    }
    build_config(tree, context, &purchases).map_err(|_| LoadoutError::InvalidNodes)
}
