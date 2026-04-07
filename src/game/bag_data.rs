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
}

/// A bank bag slot that may or may not be purchased.
#[derive(Clone, Debug, PartialEq)]
pub struct BankBagSlot {
    pub purchased: bool,
    pub bag_name: String,
    pub bag_size: usize,
    pub icon_fdid: u32,
}

impl Default for BankBagSlot {
    fn default() -> Self {
        Self {
            purchased: false,
            bag_name: String::new(),
            bag_size: 0,
            icon_fdid: 0,
        }
    }
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
}
