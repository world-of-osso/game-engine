use shared::item_data::ItemDefinitionSource;
use std::collections::BTreeMap;

use shared::protocol::{
    BagContents, DestroyItem, EquipItem, EquipmentSlot, EquipmentSnapshot, InventoryDelta,
    InventorySnapshot, ItemDurability, ItemLocation, ItemStack, SplitItem, SwapItem, UseItem,
};

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

/// Retail `Enum.ItemQuality` (ItemQualitiesDocumentation.lua).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ItemQuality {
    Poor,
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Artifact,
    Heirloom,
}

impl ItemQuality {
    pub fn from_id(id: u8) -> Self {
        match id {
            0 => Self::Poor,
            2 => Self::Uncommon,
            3 => Self::Rare,
            4 => Self::Epic,
            5 => Self::Legendary,
            6 => Self::Artifact,
            7 | 8 => Self::Heirloom,
            _ => Self::Common,
        }
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    /// `BAG_ITEM_QUALITY_COLORS` (ColorConstants.lua:21-30) from GlobalColor: the
    /// `WhiteIconFrame` tint of a bag slot; Poor has none.
    pub fn border_color(self) -> &'static str {
        match self {
            Self::Poor => "",
            Self::Common => "0.66,0.66,0.66,1.0",
            Self::Uncommon => "0.08,0.7,0.0,1.0",
            Self::Rare => "0.0,0.57,0.95,1.0",
            Self::Epic => "0.78,0.27,0.98,1.0",
            Self::Legendary => "1.0,0.5,0.0,1.0",
            Self::Artifact => "0.9,0.8,0.5,1.0",
            Self::Heirloom => "0.0,0.8,1.0,1.0",
        }
    }

    pub fn has_visible_border(self) -> bool {
        !self.border_color().is_empty()
    }
}

/// Contents of a single inventory slot.
#[derive(Clone, Debug, PartialEq)]
pub struct InventorySlot {
    /// Icon texture FDID (0 = empty slot).
    pub icon_fdid: u32,
    /// Stack count (0 or 1 = hide count display).
    pub count: u32,
    /// Item quality for border color.
    pub quality: ItemQuality,
    /// Item name (for tooltips).
    pub name: String,
    /// Server item instance (0 = none); what selling sends.
    pub item_guid: u64,
    pub item_id: u32,
    pub definition_source: ItemDefinitionSource,
    pub soulbound: bool,
    pub durability: Option<ItemDurability>,
}

impl Default for InventorySlot {
    fn default() -> Self {
        Self {
            icon_fdid: 0,
            count: 0,
            quality: ItemQuality::default(),
            name: String::new(),
            item_guid: 0,
            item_id: 0,
            definition_source: ItemDefinitionSource::Retail,
            soulbound: false,
            durability: None,
        }
    }
}

impl InventorySlot {
    pub fn is_empty(&self) -> bool {
        self.icon_fdid == 0
    }
}

/// A server stack shown in a bag or equipment slot. The server sends ids and
/// counts only; name and quality come from the item catalog.
pub fn stack_slot(stack: &ItemStack) -> InventorySlot {
    stack_slot_in_catalog(
        stack,
        crate::item_catalog::item_catalog_for(stack.definition_source),
    )
}

pub fn stack_slot_in_catalog(
    stack: &ItemStack,
    catalog: Option<&crate::item_catalog::ItemCatalog>,
) -> InventorySlot {
    with_catalog_item_data(
        InventorySlot {
            count: stack.count,
            item_guid: stack.item_guid,
            item_id: stack.item_id,
            definition_source: stack.definition_source,
            soulbound: stack.soulbound,
            durability: stack.durability,
            ..Default::default()
        },
        catalog,
    )
}

/// `slot` with its item's icon, quality and name. While the catalog loads, the item
/// has `INV_Misc_QuestionMark` and no name until `InventoryState::refresh_item_data`.
fn with_item_data(slot: InventorySlot) -> InventorySlot {
    let catalog = crate::item_catalog::item_catalog_for(slot.definition_source);
    with_catalog_item_data(slot, catalog)
}

pub fn with_catalog_item_data(
    slot: InventorySlot,
    catalog: Option<&crate::item_catalog::ItemCatalog>,
) -> InventorySlot {
    let entry = catalog.and_then(|catalog| catalog.get(slot.item_id));
    InventorySlot {
        icon_fdid: catalog
            .and_then(|catalog| catalog.icon_fdid(slot.item_id))
            .unwrap_or(UNKNOWN_ICON_FDID),
        quality: entry.map_or(ItemQuality::Common, |entry| {
            ItemQuality::from_id(entry.quality)
        }),
        name: entry.map(|entry| entry.name.clone()).unwrap_or_default(),
        ..slot
    }
}

/// `INV_Misc_QuestionMark`, Retail's icon for an item without one.
const UNKNOWN_ICON_FDID: u32 = 134_400;

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

/// A bag or equipment request for the server (`InventoryChannel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InventoryRequest {
    Swap(SwapItem),
    Equip(EquipItem),
    Split(SplitItem),
    Destroy(DestroyItem),
    Use(UseItem),
}

/// Runtime inventory state for all bags and the equipped items.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryState {
    pub bags: Vec<BagInfo>,
    /// Slots indexed by `[bag_index][slot_index]`.
    pub slots: Vec<Vec<InventorySlot>>,
    /// Occupied equipment slots (`EquipmentSnapshot` and deltas).
    pub equipment: BTreeMap<EquipmentSlot, InventorySlot>,
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
            equipment: BTreeMap::new(),
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

    pub fn equipped(&self, slot: EquipmentSlot) -> Option<&InventorySlot> {
        self.equipment.get(&slot)
    }

    /// The item at a bag or equipment location; `None` when it is empty.
    pub fn item_at(&self, location: ItemLocation) -> Option<&InventorySlot> {
        match location {
            ItemLocation::Bag { bag, slot } => self.slot(usize::from(bag), usize::from(slot)),
            ItemLocation::Equipment(slot) => self.equipped(slot),
        }
        .filter(|item| !item.is_empty())
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

    /// The server's `InventorySnapshot`: bag sizes and every item. Bags of size 0
    /// (no container support on the server) are left out.
    pub fn apply_snapshot(&mut self, snapshot: &InventorySnapshot) {
        let bags: Vec<&BagContents> = snapshot.bags.iter().filter(|bag| bag.size > 0).collect();
        self.bags = bags
            .iter()
            .map(|bag| BagInfo {
                index: usize::from(bag.bag),
                name: if bag.bag == 0 {
                    "Backpack".into()
                } else {
                    format!("Bag {}", bag.bag)
                },
                size: usize::from(bag.size),
                icon_fdid: 0,
            })
            .collect();
        let slot_bags = bags
            .iter()
            .map(|bag| usize::from(bag.bag))
            .max()
            .map_or(0, |max| max + 1);
        self.slots = vec![Vec::new(); slot_bags];
        for bag in &bags {
            self.slots[usize::from(bag.bag)] =
                vec![InventorySlot::default(); usize::from(bag.size)];
            for entry in &bag.items {
                self.set_item(
                    usize::from(bag.bag),
                    usize::from(entry.slot),
                    stack_slot(&entry.item),
                );
            }
        }
    }

    /// The server's `EquipmentSnapshot`: every occupied equipment slot.
    pub fn apply_equipment_snapshot(&mut self, snapshot: &EquipmentSnapshot) {
        self.equipment = snapshot
            .items
            .iter()
            .map(|entry| (entry.slot, stack_slot(&entry.item)))
            .collect();
    }

    /// The server's `InventoryDelta`: bag and equipment locations that changed.
    pub fn apply_delta(&mut self, delta: &InventoryDelta) {
        for change in &delta.changes {
            let item = change.item.as_ref().map(stack_slot);
            match change.location {
                ItemLocation::Bag { bag, slot } => {
                    self.set_item(
                        usize::from(bag),
                        usize::from(slot),
                        item.unwrap_or_default(),
                    );
                }
                ItemLocation::Equipment(slot) => match item {
                    Some(item) => {
                        self.equipment.insert(slot, item);
                    }
                    None => {
                        self.equipment.remove(&slot);
                    }
                },
            }
        }
    }

    /// Resolve again every item's icon, quality and name: items that arrived while the
    /// item catalog loaded get their data once it is loaded (`GET_ITEM_INFO_RECEIVED`).
    pub fn refresh_item_data(&mut self) {
        let items = self.slots.iter_mut().flatten();
        for slot in items.chain(self.equipment.values_mut()) {
            if slot.item_id != 0 {
                *slot = with_item_data(std::mem::take(slot));
            }
        }
    }

    /// Replace the entire inventory (bulk sync from server).
    pub fn replace_all(&mut self, bags: Vec<BagInfo>, slots: Vec<Vec<InventorySlot>>) {
        self.bags = bags;
        self.slots = slots;
    }
}

#[cfg(test)]
#[path = "bag_data_tests/mod.rs"]
mod tests;
