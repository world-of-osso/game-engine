//! Loot window data model.
//!
//! Represents the loot window that appears when looting a creature, chest, or
//! other container. Items can be looted individually or all at once.

use bevy::prelude::*;

use crate::bag_data::ItemQuality;

/// A single item in the loot window.
#[derive(Clone, Debug, PartialEq)]
pub struct LootItem {
    pub slot_index: u32,
    pub item_id: u32,
    pub name: String,
    pub icon_fdid: u32,
    pub quality: ItemQuality,
    pub count: u32,
    /// Whether this item is quest-related (gold sparkle border).
    pub is_quest_item: bool,
}

/// Runtime loot window state.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LootWindowState {
    pub open: bool,
    pub source_name: String,
    pub items: Vec<LootItem>,
    /// Money in copper (gold/silver/copper display).
    pub money: u64,
}

impl LootWindowState {
    /// Open the loot window with items from a source.
    pub fn open(&mut self, source_name: String, items: Vec<LootItem>, money: u64) {
        self.open = true;
        self.source_name = source_name;
        self.items = items;
        self.money = money;
    }

    /// Close and clear the loot window.
    pub fn close(&mut self) {
        self.open = false;
        self.source_name.clear();
        self.items.clear();
        self.money = 0;
    }

    /// Loot a single item by slot index.
    pub fn loot_item(&mut self, slot_index: u32) -> Option<LootItem> {
        let pos = self.items.iter().position(|i| i.slot_index == slot_index)?;
        Some(self.items.remove(pos))
    }

    /// Loot all items (returns them and clears the list).
    pub fn loot_all(&mut self) -> Vec<LootItem> {
        let items = std::mem::take(&mut self.items);
        let money = self.money;
        self.money = 0;
        if items.is_empty() && money == 0 {
            self.close();
        }
        items
    }

    /// Whether there are items or money remaining.
    pub fn has_loot(&self) -> bool {
        !self.items.is_empty() || self.money > 0
    }

    /// Number of lootable items.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Format money as "Xg Ys Zc".
    pub fn money_text(&self) -> String {
        if self.money == 0 {
            return String::new();
        }
        let g = self.money / 10_000;
        let s = (self.money % 10_000) / 100;
        let c = self.money % 100;
        if g > 0 {
            format!("{g}g {s}s {c}c")
        } else if s > 0 {
            format!("{s}s {c}c")
        } else {
            format!("{c}c")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(slot: u32, name: &str, quality: ItemQuality) -> LootItem {
        LootItem {
            slot_index: slot,
            item_id: slot + 1000,
            name: name.into(),
            icon_fdid: slot + 100000,
            quality,
            count: 1,
            is_quest_item: false,
        }
    }

    fn sample_state() -> LootWindowState {
        let mut state = LootWindowState::default();
        state.open(
            "Defias Pillager".into(),
            vec![
                sample_item(0, "Linen Cloth", ItemQuality::Common),
                sample_item(1, "Cruel Barb", ItemQuality::Rare),
                LootItem {
                    is_quest_item: true,
                    ..sample_item(2, "Defias Head", ItemQuality::Common)
                },
            ],
            5030, // 50s 30c
        );
        state
    }

    #[test]
    fn open_populates_state() {
        let state = sample_state();
        assert!(state.open);
        assert_eq!(state.source_name, "Defias Pillager");
        assert_eq!(state.item_count(), 3);
        assert!(state.has_loot());
    }

    #[test]
    fn close_clears_everything() {
        let mut state = sample_state();
        state.close();
        assert!(!state.open);
        assert!(state.source_name.is_empty());
        assert_eq!(state.item_count(), 0);
        assert!(!state.has_loot());
    }

    #[test]
    fn loot_single_item() {
        let mut state = sample_state();
        let looted = state.loot_item(1).expect("should find slot 1");
        assert_eq!(looted.name, "Cruel Barb");
        assert_eq!(state.item_count(), 2);
    }

    #[test]
    fn loot_item_not_found() {
        let mut state = sample_state();
        assert!(state.loot_item(99).is_none());
    }

    #[test]
    fn loot_all_returns_items() {
        let mut state = sample_state();
        let items = state.loot_all();
        assert_eq!(items.len(), 3);
        assert!(state.items.is_empty());
        assert_eq!(state.money, 0);
    }

    #[test]
    fn money_text_formatting() {
        let state = sample_state();
        assert_eq!(state.money_text(), "50s 30c");
    }

    #[test]
    fn money_text_empty_when_zero() {
        let state = LootWindowState::default();
        assert_eq!(state.money_text(), "");
    }

    #[test]
    fn has_loot_money_only() {
        let mut state = LootWindowState::default();
        state.money = 100;
        state.open = true;
        assert!(state.has_loot());
    }

    #[test]
    fn quest_item_flag() {
        let state = sample_state();
        assert!(state.items[2].is_quest_item);
        assert!(!state.items[0].is_quest_item);
    }

    #[test]
    fn default_state_is_closed() {
        let state = LootWindowState::default();
        assert!(!state.open);
        assert!(!state.has_loot());
    }
}
