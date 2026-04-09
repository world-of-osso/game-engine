use bevy::prelude::*;

/// Texture FDIDs for bag frames and slots.
pub mod textures {
    /// Backpack background texture.
    pub const BACKPACK_BG: u32 = 130981;
    /// Backpack button icon.
    pub const BACKPACK_BUTTON: u32 = 130716;
    /// Container frame background: 1×4 grid.
    pub const BAG_BG_1X4: u32 = 130986;
    /// Container frame background: 2×4 grid.
    pub const BAG_BG_2X4: u32 = 130990;
    /// Container frame background: 3×4 grid.
    pub const BAG_BG_3X4: u32 = 130994;
    /// Container frame background: 4×4 grid.
    pub const BAG_BG_4X4: u32 = 130998;
    /// Default bag icon (small pouch).
    pub const BAG_ICON_DEFAULT: u32 = 133622;
    /// Medium bag icon.
    pub const BAG_ICON_MEDIUM: u32 = 133625;
}

/// Texture FDIDs for the bank frame.
pub mod bank_textures {
    /// Bank frame main chrome texture.
    pub const FRAME_CHROME: u32 = 130703;
    /// Bank frame top-left corner.
    pub const CORNER_TOP_LEFT: u32 = 130701;
    /// Bank frame top-right corner.
    pub const CORNER_TOP_RIGHT: u32 = 130702;
    /// Bank frame bottom-left corner.
    pub const CORNER_BOTTOM_LEFT: u32 = 130699;
    /// Bank frame bottom-right corner.
    pub const CORNER_BOTTOM_RIGHT: u32 = 130700;
    /// Bank background fill.
    pub const BACKGROUND: u32 = 590155;
    /// Item slot background texture.
    pub const SLOT_BACKGROUND: u32 = 130862;
    /// Lock icon for locked/unpurchased slots.
    pub const LOCK_ICON: u32 = 130944;
}

/// Returns the appropriate container background FDID for a given row count.
pub fn bag_background_for_rows(rows: usize) -> u32 {
    match rows {
        0 | 1 => textures::BAG_BG_1X4,
        2 => textures::BAG_BG_2X4,
        3 => textures::BAG_BG_3X4,
        _ => textures::BAG_BG_4X4,
    }
}

/// Item quality tiers, used for slot border color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ItemQuality {
    Poor,
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl ItemQuality {
    /// RGBA color string for slot border overlay.
    pub fn border_color(self) -> &'static str {
        match self {
            Self::Poor => "0.62,0.62,0.62,1.0",
            Self::Common => "1.0,1.0,1.0,0.0",
            Self::Uncommon => "0.12,1.0,0.0,1.0",
            Self::Rare => "0.0,0.44,0.87,1.0",
            Self::Epic => "0.64,0.21,0.93,1.0",
            Self::Legendary => "1.0,0.5,0.0,1.0",
        }
    }

    /// Whether this quality should show a colored border (Common is invisible).
    pub fn has_visible_border(self) -> bool {
        !matches!(self, Self::Common)
    }
}

/// Contents of a single inventory slot.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InventorySlot {
    /// Icon texture FDID (0 = empty slot).
    pub icon_fdid: u32,
    /// Stack count (0 or 1 = hide count display).
    pub count: u32,
    /// Item quality for border color.
    pub quality: ItemQuality,
    /// Item name (for tooltips).
    pub name: String,
}

impl InventorySlot {
    pub fn is_empty(&self) -> bool {
        self.icon_fdid == 0
    }
}

/// A bag in the player's inventory.
#[derive(Clone, Debug, PartialEq)]
pub struct BagInfo {
    /// Bag index (0 = backpack, 1–4 = equipped bags).
    pub index: usize,
    /// Display name (e.g. "Backpack", "Mooncloth Bag").
    pub name: String,
    /// Total slot capacity.
    pub size: usize,
    /// Bag icon FDID.
    pub icon_fdid: u32,
}

/// Runtime inventory state for all bags.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct InventoryState {
    pub bags: Vec<BagInfo>,
    /// Slots indexed by `[bag_index][slot_index]`.
    pub slots: Vec<Vec<InventorySlot>>,
}

impl Default for InventoryState {
    fn default() -> Self {
        let backpack = BagInfo {
            index: 0,
            name: "Backpack".into(),
            size: 16,
            icon_fdid: 0,
        };
        Self {
            bags: vec![backpack],
            slots: vec![vec![InventorySlot::default(); 16]],
        }
    }
}

impl InventoryState {
    pub fn bag_slot_count(&self, bag_index: usize) -> usize {
        self.slots.get(bag_index).map_or(0, |s| s.len())
    }

    pub fn slot(&self, bag_index: usize, slot_index: usize) -> Option<&InventorySlot> {
        self.slots.get(bag_index)?.get(slot_index)
    }

    pub fn total_free_slots(&self) -> usize {
        self.slots
            .iter()
            .flat_map(|bag| bag.iter())
            .filter(|s| s.is_empty())
            .count()
    }

    pub fn total_slots(&self) -> usize {
        self.slots.iter().map(|bag| bag.len()).sum()
    }

    /// Set an item in a specific bag slot (from server update).
    pub fn set_item(&mut self, bag_index: usize, slot_index: usize, item: InventorySlot) {
        if let Some(bag) = self.slots.get_mut(bag_index)
            && let Some(slot) = bag.get_mut(slot_index)
        {
            *slot = item;
        }
    }

    /// Clear a bag slot (item removed, sold, moved, etc.).
    pub fn clear_slot(&mut self, bag_index: usize, slot_index: usize) {
        self.set_item(bag_index, slot_index, InventorySlot::default());
    }

    /// Add a new bag to the inventory (e.g. equipping a bag item).
    pub fn add_bag(&mut self, info: BagInfo) {
        let size = info.size;
        self.bags.push(info);
        self.slots.push(vec![InventorySlot::default(); size]);
    }

    /// Find the first empty slot across all bags. Returns (bag_index, slot_index).
    pub fn first_empty_slot(&self) -> Option<(usize, usize)> {
        for (bi, bag) in self.slots.iter().enumerate() {
            for (si, slot) in bag.iter().enumerate() {
                if slot.is_empty() {
                    return Some((bi, si));
                }
            }
        }
        None
    }

    /// Count of a specific item by name across all bags.
    pub fn count_item(&self, name: &str) -> u32 {
        self.slots
            .iter()
            .flat_map(|bag| bag.iter())
            .filter(|s| s.name == name)
            .map(|s| s.count.max(1))
            .sum()
    }

    /// Replace the entire inventory (bulk sync from server).
    pub fn replace_all(&mut self, bags: Vec<BagInfo>, slots: Vec<Vec<InventorySlot>>) {
        self.bags = bags;
        self.slots = slots;
    }

    /// Swap two items between any two slots (drag-and-drop).
    /// Both slots can be in the same or different bags.
    pub fn swap_slots(
        &mut self,
        from_bag: usize,
        from_slot: usize,
        to_bag: usize,
        to_slot: usize,
    ) -> bool {
        if from_bag == to_bag && from_slot == to_slot {
            return false;
        }
        let from_valid = self
            .slots
            .get(from_bag)
            .is_some_and(|b| from_slot < b.len());
        let to_valid = self.slots.get(to_bag).is_some_and(|b| to_slot < b.len());
        if !from_valid || !to_valid {
            return false;
        }
        if from_bag == to_bag {
            self.slots[from_bag].swap(from_slot, to_slot);
        } else {
            let from_item = std::mem::take(&mut self.slots[from_bag][from_slot]);
            let to_item = std::mem::take(&mut self.slots[to_bag][to_slot]);
            self.slots[from_bag][from_slot] = to_item;
            self.slots[to_bag][to_slot] = from_item;
        }
        true
    }

    /// Move an item from one slot to an empty slot (shortcut for swap with empty).
    pub fn move_to_empty(
        &mut self,
        from_bag: usize,
        from_slot: usize,
        to_bag: usize,
        to_slot: usize,
    ) -> bool {
        let target_empty = self.slot(to_bag, to_slot).is_some_and(|s| s.is_empty());
        if !target_empty {
            return false;
        }
        self.swap_slots(from_bag, from_slot, to_bag, to_slot)
    }

    /// Try to stack an item onto a matching item in the target slot.
    /// Returns true if stacking succeeded (same item name, combined count).
    pub fn try_stack(
        &mut self,
        from_bag: usize,
        from_slot: usize,
        to_bag: usize,
        to_slot: usize,
    ) -> bool {
        let from = self.slot(from_bag, from_slot).cloned();
        let to = self.slot(to_bag, to_slot).cloned();
        let (Some(from_item), Some(to_item)) = (from, to) else {
            return false;
        };
        if from_item.name.is_empty() || from_item.name != to_item.name {
            return false;
        }
        let combined = from_item.count.max(1) + to_item.count.max(1);
        self.slots[to_bag][to_slot].count = combined;
        self.clear_slot(from_bag, from_slot);
        true
    }
}

/// A bank bag slot that may or may not be purchased.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BankBagSlot {
    pub purchased: bool,
    pub bag_name: String,
    pub bag_size: usize,
    pub icon_fdid: u32,
}

/// Runtime state for the player's bank.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct BankState {
    /// Main bank slots (28).
    pub slots: Vec<InventorySlot>,
    /// Bank bag slots (7), each may be purchased or locked.
    pub bag_slots: Vec<BankBagSlot>,
    /// Reagent bank slots (98), only `reagent_unlocked` are usable.
    pub reagent_slots: Vec<InventorySlot>,
    /// Number of reagent slots the player has purchased.
    pub reagent_unlocked: usize,
}

impl Default for BankState {
    fn default() -> Self {
        Self {
            slots: vec![InventorySlot::default(); 28],
            bag_slots: vec![BankBagSlot::default(); 7],
            reagent_slots: vec![InventorySlot::default(); 98],
            reagent_unlocked: 0,
        }
    }
}

impl BankState {
    pub fn main_slot(&self, index: usize) -> Option<&InventorySlot> {
        self.slots.get(index)
    }

    pub fn reagent_slot(&self, index: usize) -> Option<&InventorySlot> {
        self.reagent_slots.get(index)
    }

    pub fn is_reagent_slot_locked(&self, index: usize) -> bool {
        index >= self.reagent_unlocked
    }

    pub fn is_bag_slot_purchased(&self, index: usize) -> bool {
        self.bag_slots.get(index).is_some_and(|s| s.purchased)
    }

    pub fn purchased_bag_count(&self) -> usize {
        self.bag_slots.iter().filter(|s| s.purchased).count()
    }
}

#[cfg(test)]
#[path = "bag_data_tests/mod.rs"]
mod tests;
