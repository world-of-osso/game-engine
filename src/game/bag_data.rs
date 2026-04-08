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
mod tests {
    use super::*;

    #[test]
    fn default_inventory_has_backpack() {
        let inv = InventoryState::default();
        assert_eq!(inv.bags.len(), 1);
        assert_eq!(inv.bags[0].name, "Backpack");
        assert_eq!(inv.bags[0].size, 16);
        assert_eq!(inv.bag_slot_count(0), 16);
    }

    #[test]
    fn empty_slot_detection() {
        let slot = InventorySlot::default();
        assert!(slot.is_empty());
        let filled = InventorySlot {
            icon_fdid: 12345,
            count: 1,
            quality: ItemQuality::Rare,
            name: "Hearthstone".into(),
        };
        assert!(!filled.is_empty());
    }

    #[test]
    fn quality_border_colors() {
        assert!(!ItemQuality::Common.has_visible_border());
        assert!(ItemQuality::Uncommon.has_visible_border());
        assert!(ItemQuality::Rare.has_visible_border());
        assert!(ItemQuality::Epic.has_visible_border());
        assert!(ItemQuality::Legendary.has_visible_border());
        assert!(ItemQuality::Poor.has_visible_border());
    }

    #[test]
    fn total_free_slots_counts_empty() {
        let mut inv = InventoryState::default();
        assert_eq!(inv.total_free_slots(), 16);
        inv.slots[0][0].icon_fdid = 100;
        inv.slots[0][1].icon_fdid = 200;
        assert_eq!(inv.total_free_slots(), 14);
    }

    #[test]
    fn total_slots_across_bags() {
        let mut inv = InventoryState::default();
        inv.bags.push(BagInfo {
            index: 1,
            name: "Mooncloth Bag".into(),
            size: 12,
            icon_fdid: 0,
        });
        inv.slots.push(vec![InventorySlot::default(); 12]);
        assert_eq!(inv.total_slots(), 28);
    }

    #[test]
    fn slot_access_out_of_bounds_returns_none() {
        let inv = InventoryState::default();
        assert!(inv.slot(0, 0).is_some());
        assert!(inv.slot(0, 15).is_some());
        assert!(inv.slot(0, 16).is_none());
        assert!(inv.slot(5, 0).is_none());
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::BACKPACK_BG, 0);
        assert_ne!(textures::BACKPACK_BUTTON, 0);
        assert_ne!(textures::BAG_BG_1X4, 0);
        assert_ne!(textures::BAG_BG_4X4, 0);
        assert_ne!(textures::BAG_ICON_DEFAULT, 0);
    }

    #[test]
    fn bank_texture_fdids_are_nonzero() {
        assert_ne!(bank_textures::FRAME_CHROME, 0);
        assert_ne!(bank_textures::CORNER_TOP_LEFT, 0);
        assert_ne!(bank_textures::BACKGROUND, 0);
        assert_ne!(bank_textures::SLOT_BACKGROUND, 0);
        assert_ne!(bank_textures::LOCK_ICON, 0);
    }

    #[test]
    fn bag_background_selects_correct_size() {
        assert_eq!(bag_background_for_rows(1), textures::BAG_BG_1X4);
        assert_eq!(bag_background_for_rows(2), textures::BAG_BG_2X4);
        assert_eq!(bag_background_for_rows(3), textures::BAG_BG_3X4);
        assert_eq!(bag_background_for_rows(4), textures::BAG_BG_4X4);
        assert_eq!(bag_background_for_rows(6), textures::BAG_BG_4X4);
    }

    // --- Bank state tests ---

    #[test]
    fn default_bank_has_28_main_slots() {
        let bank = BankState::default();
        assert_eq!(bank.slots.len(), 28);
        assert!(bank.main_slot(0).is_some());
        assert!(bank.main_slot(27).is_some());
        assert!(bank.main_slot(28).is_none());
    }

    #[test]
    fn default_bank_bag_slots_are_locked() {
        let bank = BankState::default();
        assert_eq!(bank.bag_slots.len(), 7);
        assert_eq!(bank.purchased_bag_count(), 0);
        assert!(!bank.is_bag_slot_purchased(0));
    }

    #[test]
    fn purchased_bag_slot_tracking() {
        let mut bank = BankState::default();
        bank.bag_slots[0].purchased = true;
        bank.bag_slots[0].bag_name = "Runecloth Bag".into();
        bank.bag_slots[0].bag_size = 14;
        assert!(bank.is_bag_slot_purchased(0));
        assert!(!bank.is_bag_slot_purchased(1));
        assert_eq!(bank.purchased_bag_count(), 1);
    }

    #[test]
    fn reagent_slot_locked_state() {
        let mut bank = BankState::default();
        assert_eq!(bank.reagent_slots.len(), 98);
        assert!(bank.is_reagent_slot_locked(0));
        bank.reagent_unlocked = 49;
        assert!(!bank.is_reagent_slot_locked(0));
        assert!(!bank.is_reagent_slot_locked(48));
        assert!(bank.is_reagent_slot_locked(49));
    }

    // --- Slot contents by bag index ---

    #[test]
    fn slot_contents_across_bags() {
        let mut inv = InventoryState::default();
        inv.bags.push(BagInfo {
            index: 1,
            name: "Netherweave Bag".into(),
            size: 16,
            icon_fdid: 0,
        });
        inv.slots.push(vec![InventorySlot::default(); 16]);
        // Place item in bag 0 slot 3
        inv.slots[0][3] = InventorySlot {
            icon_fdid: 111,
            name: "Hearthstone".into(),
            count: 1,
            quality: ItemQuality::Common,
        };
        // Place item in bag 1 slot 0
        inv.slots[1][0] = InventorySlot {
            icon_fdid: 222,
            name: "Ore".into(),
            count: 20,
            quality: ItemQuality::Uncommon,
        };
        assert_eq!(inv.slot(0, 3).unwrap().name, "Hearthstone");
        assert_eq!(inv.slot(1, 0).unwrap().name, "Ore");
        assert_eq!(inv.slot(1, 0).unwrap().count, 20);
        assert!(inv.slot(0, 0).unwrap().is_empty());
    }

    // --- Quality color mapping ---

    #[test]
    fn quality_border_color_values() {
        assert!(ItemQuality::Poor.border_color().starts_with("0.62"));
        assert!(ItemQuality::Uncommon.border_color().starts_with("0.12"));
        assert!(ItemQuality::Rare.border_color().starts_with("0.0,0.44"));
        assert!(ItemQuality::Epic.border_color().starts_with("0.64"));
        assert!(ItemQuality::Legendary.border_color().starts_with("1.0,0.5"));
    }

    #[test]
    fn quality_common_border_invisible() {
        // Common border alpha is 0.0
        assert!(ItemQuality::Common.border_color().ends_with("0.0"));
        assert!(!ItemQuality::Common.has_visible_border());
    }

    // --- Bag size variation ---

    #[test]
    fn varied_bag_sizes() {
        let mut inv = InventoryState::default(); // 16-slot backpack
        // Add a small 8-slot bag
        inv.bags.push(BagInfo {
            index: 1,
            name: "Small Bag".into(),
            size: 8,
            icon_fdid: 0,
        });
        inv.slots.push(vec![InventorySlot::default(); 8]);
        // Add a large 20-slot bag
        inv.bags.push(BagInfo {
            index: 2,
            name: "Large Bag".into(),
            size: 20,
            icon_fdid: 0,
        });
        inv.slots.push(vec![InventorySlot::default(); 20]);

        assert_eq!(inv.bag_slot_count(0), 16);
        assert_eq!(inv.bag_slot_count(1), 8);
        assert_eq!(inv.bag_slot_count(2), 20);
        assert_eq!(inv.total_slots(), 44);
        assert_eq!(inv.total_free_slots(), 44);
    }

    #[test]
    fn bag_slot_count_nonexistent_bag() {
        let inv = InventoryState::default();
        assert_eq!(inv.bag_slot_count(5), 0);
    }

    #[test]
    fn bag_background_for_zero_rows() {
        assert_eq!(bag_background_for_rows(0), textures::BAG_BG_1X4);
    }

    // --- BankFrame data: main vs reagent slot indexing ---

    #[test]
    fn main_and_reagent_slots_are_independent() {
        let mut bank = BankState::default();
        bank.slots[0] = InventorySlot {
            icon_fdid: 100,
            name: "Ore".into(),
            count: 20,
            quality: ItemQuality::Common,
        };
        bank.reagent_slots[0] = InventorySlot {
            icon_fdid: 200,
            name: "Herb".into(),
            count: 5,
            quality: ItemQuality::Uncommon,
        };
        // Different items in same index across main/reagent
        assert_eq!(bank.main_slot(0).unwrap().name, "Ore");
        assert_eq!(bank.reagent_slot(0).unwrap().name, "Herb");
        assert_eq!(bank.main_slot(0).unwrap().icon_fdid, 100);
        assert_eq!(bank.reagent_slot(0).unwrap().icon_fdid, 200);
    }

    #[test]
    fn main_slot_out_of_bounds() {
        let bank = BankState::default();
        assert!(bank.main_slot(27).is_some());
        assert!(bank.main_slot(28).is_none());
        assert!(bank.main_slot(100).is_none());
    }

    #[test]
    fn reagent_slot_out_of_bounds() {
        let bank = BankState::default();
        assert!(bank.reagent_slot(97).is_some());
        assert!(bank.reagent_slot(98).is_none());
    }

    #[test]
    fn reagent_slot_content_when_unlocked() {
        let mut bank = BankState::default();
        bank.reagent_unlocked = 10;
        bank.reagent_slots[5] = InventorySlot {
            icon_fdid: 300,
            name: "Flask".into(),
            count: 3,
            quality: ItemQuality::Rare,
        };
        let slot = bank.reagent_slot(5).unwrap();
        assert!(!slot.is_empty());
        assert_eq!(slot.name, "Flask");
        assert!(!bank.is_reagent_slot_locked(5));
        // Slot 5 is unlocked but slot 10 is locked
        assert!(bank.is_reagent_slot_locked(10));
    }

    // --- BankFrame data: bag purchase state transitions ---

    #[test]
    fn multiple_bag_purchases() {
        let mut bank = BankState::default();
        assert_eq!(bank.purchased_bag_count(), 0);

        bank.bag_slots[0].purchased = true;
        bank.bag_slots[0].bag_name = "Frostweave Bag".into();
        bank.bag_slots[0].bag_size = 20;
        assert_eq!(bank.purchased_bag_count(), 1);

        bank.bag_slots[1].purchased = true;
        bank.bag_slots[1].bag_name = "Netherweave Bag".into();
        bank.bag_slots[1].bag_size = 16;
        assert_eq!(bank.purchased_bag_count(), 2);

        bank.bag_slots[6].purchased = true;
        bank.bag_slots[6].bag_name = "Embersilk Bag".into();
        bank.bag_slots[6].bag_size = 22;
        assert_eq!(bank.purchased_bag_count(), 3);

        // Non-purchased slots still locked
        for i in 2..6 {
            assert!(!bank.is_bag_slot_purchased(i));
        }
    }

    #[test]
    fn bag_slot_purchased_out_of_bounds() {
        let bank = BankState::default();
        assert!(!bank.is_bag_slot_purchased(7));
        assert!(!bank.is_bag_slot_purchased(100));
    }

    #[test]
    fn reagent_unlock_boundary_at_full() {
        let mut bank = BankState::default();
        bank.reagent_unlocked = 98;
        // All slots unlocked
        for i in 0..98 {
            assert!(
                !bank.is_reagent_slot_locked(i),
                "slot {i} should be unlocked"
            );
        }
        // Index 98 doesn't exist
        assert!(bank.reagent_slot(98).is_none());
    }

    // --- Client-side bag contents (server sync) ---

    #[test]
    fn set_item_populates_slot() {
        let mut inv = InventoryState::default();
        inv.set_item(
            0,
            5,
            InventorySlot {
                icon_fdid: 135274,
                count: 20,
                quality: ItemQuality::Common,
                name: "Iron Ore".into(),
            },
        );
        let slot = inv.slot(0, 5).unwrap();
        assert_eq!(slot.name, "Iron Ore");
        assert_eq!(slot.count, 20);
        assert!(!slot.is_empty());
    }

    #[test]
    fn clear_slot_empties() {
        let mut inv = InventoryState::default();
        inv.set_item(
            0,
            0,
            InventorySlot {
                icon_fdid: 100,
                name: "Sword".into(),
                ..Default::default()
            },
        );
        assert!(!inv.slot(0, 0).unwrap().is_empty());
        inv.clear_slot(0, 0);
        assert!(inv.slot(0, 0).unwrap().is_empty());
    }

    #[test]
    fn set_item_out_of_bounds_no_panic() {
        let mut inv = InventoryState::default();
        inv.set_item(99, 0, InventorySlot::default()); // no crash
        inv.set_item(0, 99, InventorySlot::default()); // no crash
    }

    #[test]
    fn add_bag_extends_inventory() {
        let mut inv = InventoryState::default();
        assert_eq!(inv.bags.len(), 1);
        inv.add_bag(BagInfo {
            index: 1,
            name: "Netherweave Bag".into(),
            size: 16,
            icon_fdid: 133625,
        });
        assert_eq!(inv.bags.len(), 2);
        assert_eq!(inv.bag_slot_count(1), 16);
        assert_eq!(inv.total_slots(), 32);
    }

    #[test]
    fn first_empty_slot_finds_earliest() {
        let mut inv = InventoryState::default();
        inv.set_item(
            0,
            0,
            InventorySlot {
                icon_fdid: 1,
                name: "A".into(),
                ..Default::default()
            },
        );
        inv.set_item(
            0,
            1,
            InventorySlot {
                icon_fdid: 2,
                name: "B".into(),
                ..Default::default()
            },
        );
        assert_eq!(inv.first_empty_slot(), Some((0, 2)));
    }

    #[test]
    fn first_empty_slot_none_when_full() {
        let mut inv = InventoryState {
            bags: vec![BagInfo {
                index: 0,
                name: "Tiny".into(),
                size: 2,
                icon_fdid: 0,
            }],
            slots: vec![vec![
                InventorySlot {
                    icon_fdid: 1,
                    name: "A".into(),
                    ..Default::default()
                },
                InventorySlot {
                    icon_fdid: 2,
                    name: "B".into(),
                    ..Default::default()
                },
            ]],
        };
        assert!(inv.first_empty_slot().is_none());
    }

    #[test]
    fn count_item_across_bags() {
        let mut inv = InventoryState::default();
        inv.add_bag(BagInfo {
            index: 1,
            name: "Bag".into(),
            size: 4,
            icon_fdid: 0,
        });
        inv.set_item(
            0,
            0,
            InventorySlot {
                icon_fdid: 1,
                count: 20,
                name: "Iron Ore".into(),
                ..Default::default()
            },
        );
        inv.set_item(
            1,
            0,
            InventorySlot {
                icon_fdid: 1,
                count: 15,
                name: "Iron Ore".into(),
                ..Default::default()
            },
        );
        assert_eq!(inv.count_item("Iron Ore"), 35);
        assert_eq!(inv.count_item("Gold Ore"), 0);
    }

    #[test]
    fn replace_all_resets_inventory() {
        let mut inv = InventoryState::default();
        inv.set_item(
            0,
            0,
            InventorySlot {
                icon_fdid: 1,
                name: "Old".into(),
                ..Default::default()
            },
        );
        inv.replace_all(
            vec![BagInfo {
                index: 0,
                name: "New Backpack".into(),
                size: 20,
                icon_fdid: 0,
            }],
            vec![vec![InventorySlot::default(); 20]],
        );
        assert_eq!(inv.bags.len(), 1);
        assert_eq!(inv.bags[0].name, "New Backpack");
        assert_eq!(inv.total_slots(), 20);
        assert_eq!(inv.total_free_slots(), 20);
    }
}
