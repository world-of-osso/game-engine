//! Client vendor state: the server's `VendorInventory` / `BuybackList` for the open
//! vendor NPC, the Retail MerchantFrame tab and page, and the player actions the
//! frame asks the network layer to send (`MerchantRequest`).

use shared::protocol::{BuybackItem, ItemLocation, VendorInventory, VendorItem};

/// Retail `MERCHANT_ITEMS_PER_PAGE` (MerchantFrame.lua:1).
pub const MERCHANT_ITEMS_PER_PAGE: usize = 10;
/// Retail `BUYBACK_ITEMS_PER_PAGE` (MerchantFrame.lua:2).
pub const BUYBACK_ITEMS_PER_PAGE: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MerchantTab {
    #[default]
    Merchant,
    Buyback,
}

/// The open vendor frame (`npc` = server entity bits; `None` = closed).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MerchantState {
    pub npc: Option<u64>,
    pub vendor_name: String,
    pub can_repair: bool,
    /// Guild money the player may spend on repairs here; `Some` is Retail
    /// `CanGuildBankRepair()`, the amount `GetGuildBankWithdrawMoney` capped by the bank.
    pub guild_repair_money: Option<u64>,
    pub items: Vec<VendorItem>,
    pub buyback: Vec<BuybackItem>,
    pub tab: MerchantTab,
    /// Zero-based page of the merchant tab.
    pub page: usize,
}

impl MerchantState {
    pub fn is_open(&self) -> bool {
        self.npc.is_some()
    }

    /// A vendor list from the server: opens the frame on the merchant tab, or
    /// refreshes the open frame of the same vendor (limited stock changed).
    pub fn apply_inventory(&mut self, inventory: VendorInventory, vendor_name: String) {
        if self.npc != Some(inventory.npc) {
            *self = Self {
                npc: Some(inventory.npc),
                vendor_name,
                ..Self::default()
            };
        }
        self.can_repair = inventory.can_repair;
        self.guild_repair_money = inventory.guild_repair_money;
        self.items = inventory.items;
        self.page = self.page.min(self.page_count() - 1);
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    /// Merchant-tab pages (`math.ceil(numMerchantItems / MERCHANT_ITEMS_PER_PAGE)`), at least 1.
    pub fn page_count(&self) -> usize {
        self.items.len().div_ceil(MERCHANT_ITEMS_PER_PAGE).max(1)
    }

    pub fn page_items(&self) -> &[VendorItem] {
        let start = (self.page * MERCHANT_ITEMS_PER_PAGE).min(self.items.len());
        let end = (start + MERCHANT_ITEMS_PER_PAGE).min(self.items.len());
        &self.items[start..end]
    }

    pub fn next_page(&mut self) {
        self.page = (self.page + 1).min(self.page_count() - 1);
    }

    pub fn prev_page(&mut self) {
        self.page = self.page.saturating_sub(1);
    }

    /// Retail tab clicks keep the merchant page (`MerchantFrame.page`).
    pub fn set_tab(&mut self, tab: MerchantTab) {
        self.tab = tab;
    }

    /// The most recent sale, shown in the merchant tab's `MerchantBuyBackItem`.
    pub fn last_buyback(&self) -> Option<&BuybackItem> {
        self.buyback.last()
    }

    /// Name, quality, stack count and stock of the item in cell `index` of the
    /// shown tab.
    /// Item id, name, quality, stack count and stock of a cell.
    pub fn cell_item(&self, index: usize) -> Option<(u32, &str, u8, u32, Option<u32>)> {
        match self.tab {
            MerchantTab::Merchant => self.page_items().get(index).map(|item| {
                (
                    item.item_id,
                    item.name.as_str(),
                    item.quality,
                    item.stack_count,
                    item.num_available,
                )
            }),
            MerchantTab::Buyback => self.buyback.get(index).map(|item| {
                (
                    item.item_id,
                    item.name.as_str(),
                    item.quality,
                    item.count,
                    None,
                )
            }),
        }
    }
}

/// Retail `ITEM_QUALITY_COLORS`.
pub fn quality_color(quality: u8) -> &'static str {
    match quality {
        0 => "0.62,0.62,0.62,1.0",
        2 => "0.12,1.0,0.0,1.0",
        3 => "0.0,0.44,0.87,1.0",
        4 => "0.64,0.21,0.93,1.0",
        5 => "1.0,0.5,0.0,1.0",
        6 | 7 => "0.9,0.8,0.5,1.0",
        _ => "1.0,1.0,1.0,1.0",
    }
}

/// A MerchantFrame action for the server, sent to the open vendor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MerchantRequest {
    /// `count` purchases of the vendor slot, into `destination` when a merchant
    /// cursor was dropped on a bag slot.
    Buy {
        slot: u32,
        item_id: u32,
        count: u32,
        destination: Option<ItemLocation>,
    },
    /// `C_MerchantFrame.SellAllJunkItems`.
    SellAllJunk,
    /// Sell a bag stack (`count` 0 = all of it).
    Sell {
        item_guid: u64,
        count: u32,
    },
    Buyback {
        slot: u8,
    },
    /// Repair one item, or all when `item_guid` is `None`.
    Repair {
        item_guid: Option<u64>,
    },
    /// `RepairAllItems(true)`: repair all from the guild bank.
    GuildRepairAll,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(slot: u32) -> VendorItem {
        VendorItem {
            slot,
            item_id: 2320 + slot,
            name: format!("Item {slot}"),
            quality: 1,
            price: 10,
            stack_count: 1,
            max_stack: 1,
            num_available: None,
            usable: true,
            max_durability: None,
        }
    }

    fn inventory(npc: u64, count: u32) -> VendorInventory {
        VendorInventory {
            npc,
            can_repair: false,
            guild_repair_money: None,
            items: (0..count).map(item).collect(),
        }
    }

    #[test]
    fn nineteen_items_make_two_pages_of_ten_and_nine() {
        let mut state = MerchantState::default();
        state.apply_inventory(inventory(66, 19), "Tharynn Bouden".into());

        assert_eq!(state.page_count(), 2);
        assert_eq!(state.page_items().len(), 10);
        state.next_page();
        state.next_page();
        assert_eq!(state.page, 1);
        assert_eq!(state.page_items()[0].name, "Item 10");
        assert_eq!(state.page_items().len(), 9);
        state.prev_page();
        state.prev_page();
        assert_eq!(state.page, 0);
    }

    #[test]
    fn a_stock_refresh_keeps_the_page_and_a_new_vendor_resets_it() {
        let mut state = MerchantState::default();
        state.apply_inventory(inventory(66, 19), "Tharynn Bouden".into());
        state.next_page();
        state.set_tab(MerchantTab::Buyback);

        state.apply_inventory(inventory(66, 18), String::new());
        assert_eq!((state.page, state.tab), (1, MerchantTab::Buyback));
        assert_eq!(state.vendor_name, "Tharynn Bouden");

        state.apply_inventory(inventory(1213, 8), "Godric Rothgar".into());
        assert_eq!((state.page, state.tab), (0, MerchantTab::Merchant));
        assert_eq!(state.vendor_name, "Godric Rothgar");
    }
}
