use bevy::prelude::*;

use crate::auction_house_data::Money;

pub mod textures {
    /// Frame chrome bottom-left.
    pub const FRAME_BOTTOM_LEFT: u32 = 136420;
    /// Repair icons sheet.
    pub const REPAIR_ICONS: u32 = 136424;
    /// Repair ability icon.
    pub const REPAIR_ABILITY: u32 = 132281;
    /// Buyback icon.
    pub const BUYBACK_ICON: u32 = 136417;
    /// Item slot border (shared).
    pub const SLOT_BORDER: u32 = 130862;
    /// Gold coin (shared).
    pub const GOLD_ICON: u32 = 237618;
    /// Silver coin (shared).
    pub const SILVER_ICON: u32 = 237620;
    /// Copper coin (shared).
    pub const COPPER_ICON: u32 = 237617;
}

#[derive(Clone, Debug, PartialEq)]
pub struct MerchantItemDef {
    pub item_id: u32,
    pub name: String,
    pub icon_fdid: u32,
    pub buy_price: Money,
    pub sell_price: Money,
    pub max_stack: u32,
}

/// Runtime merchant state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct MerchantState {
    pub inventory: Vec<MerchantItemDef>,
    pub player_money: Money,
    pub repair_cost: Money,
    pub page: usize,
    pub items_per_page: usize,
    /// The server-side entity ID of the vendor NPC (None = window closed).
    pub npc_entity_id: Option<u64>,
}

impl MerchantState {
    pub fn is_open(&self) -> bool {
        self.npc_entity_id.is_some()
    }

    /// Open the merchant window for a specific NPC vendor.
    pub fn open(
        &mut self,
        npc_entity_id: u64,
        inventory: Vec<MerchantItemDef>,
        player_money: Money,
        repair_cost: Money,
    ) {
        self.npc_entity_id = Some(npc_entity_id);
        self.inventory = inventory;
        self.player_money = player_money;
        self.repair_cost = repair_cost;
        self.page = 0;
    }

    /// Close the merchant window and clear all state.
    pub fn close(&mut self) {
        self.npc_entity_id = None;
        self.inventory.clear();
        self.player_money = Money::default();
        self.repair_cost = Money::default();
        self.page = 0;
    }

    pub fn page_count(&self) -> usize {
        if self.items_per_page == 0 {
            return 1;
        }
        self.inventory.len().div_ceil(self.items_per_page).max(1)
    }

    pub fn current_page_items(&self) -> &[MerchantItemDef] {
        if self.items_per_page == 0 {
            return &self.inventory;
        }
        let start = self.page * self.items_per_page;
        let end = (start + self.items_per_page).min(self.inventory.len());
        if start >= self.inventory.len() {
            return &[];
        }
        &self.inventory[start..end]
    }

    pub fn can_afford(&self, item: &MerchantItemDef) -> bool {
        self.player_money.0 >= item.buy_price.0
    }

    pub fn can_repair(&self) -> bool {
        self.repair_cost.0 > 0 && self.player_money.0 >= self.repair_cost.0
    }

    /// Total cost to buy `quantity` of an item.
    pub fn buy_cost(item: &MerchantItemDef, quantity: u32) -> Money {
        Money(item.buy_price.0 * quantity as u64)
    }

    /// Whether the player can afford `quantity` of an item.
    pub fn can_afford_quantity(&self, item: &MerchantItemDef, quantity: u32) -> bool {
        self.player_money.0 >= Self::buy_cost(item, quantity).0
    }

    /// Navigate to next page (clamped).
    pub fn next_page(&mut self) {
        let max = self.page_count().saturating_sub(1);
        self.page = (self.page + 1).min(max);
    }

    /// Navigate to previous page (clamped).
    pub fn prev_page(&mut self) {
        self.page = self.page.saturating_sub(1);
    }
}

// --- Client → server intents ---

/// A pending merchant transaction to send to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MerchantIntent {
    /// Buy an item from the vendor.
    Buy { item_id: u32, quantity: u32 },
    /// Sell an item from the player's inventory.
    Sell { bag: u8, slot: u8 },
    /// Buy back a recently sold item.
    Buyback { slot: u8 },
    /// Repair all equipped items.
    RepairAll,
    /// Repair a single item.
    RepairSingle { bag: u8, slot: u8 },
}

/// Queue of merchant intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct MerchantIntentQueue {
    pub pending: Vec<MerchantIntent>,
}

impl MerchantIntentQueue {
    pub fn buy(&mut self, item_id: u32, quantity: u32) {
        self.pending.push(MerchantIntent::Buy { item_id, quantity });
    }

    pub fn sell(&mut self, bag: u8, slot: u8) {
        self.pending.push(MerchantIntent::Sell { bag, slot });
    }

    pub fn buyback(&mut self, slot: u8) {
        self.pending.push(MerchantIntent::Buyback { slot });
    }

    pub fn repair_all(&mut self) {
        self.pending.push(MerchantIntent::RepairAll);
    }

    pub fn repair_single(&mut self, bag: u8, slot: u8) {
        self.pending
            .push(MerchantIntent::RepairSingle { bag, slot });
    }

    pub fn drain(&mut self) -> Vec<MerchantIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(name: &str, price_copper: u64) -> MerchantItemDef {
        MerchantItemDef {
            item_id: 1,
            name: name.into(),
            icon_fdid: 0,
            buy_price: Money(price_copper),
            sell_price: Money(price_copper / 4),
            max_stack: 20,
        }
    }

    #[test]
    fn page_count_and_items() {
        let mut state = MerchantState {
            inventory: (0..25)
                .map(|i| make_item(&format!("Item{i}"), 100))
                .collect(),
            items_per_page: 10,
            ..Default::default()
        };
        assert_eq!(state.page_count(), 3);
        assert_eq!(state.current_page_items().len(), 10);
        state.page = 2;
        assert_eq!(state.current_page_items().len(), 5);
    }

    #[test]
    fn can_afford() {
        let state = MerchantState {
            player_money: Money(500),
            ..Default::default()
        };
        assert!(state.can_afford(&make_item("Cheap", 100)));
        assert!(!state.can_afford(&make_item("Expensive", 1000)));
    }

    #[test]
    fn can_repair() {
        let state = MerchantState {
            player_money: Money(500),
            repair_cost: Money(200),
            ..Default::default()
        };
        assert!(state.can_repair());
        let broke = MerchantState {
            player_money: Money(50),
            repair_cost: Money(200),
            ..Default::default()
        };
        assert!(!broke.can_repair());
    }

    #[test]
    fn empty_inventory_one_page() {
        let state = MerchantState {
            items_per_page: 10,
            ..Default::default()
        };
        assert_eq!(state.page_count(), 1);
        assert!(state.current_page_items().is_empty());
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FRAME_BOTTOM_LEFT, 0);
        assert_ne!(textures::REPAIR_ICONS, 0);
        assert_ne!(textures::REPAIR_ABILITY, 0);
        assert_ne!(textures::BUYBACK_ICON, 0);
        assert_ne!(textures::SLOT_BORDER, 0);
        assert_ne!(textures::GOLD_ICON, 0);
    }

    // --- Inventory paging ---

    #[test]
    fn page_beyond_range_returns_empty() {
        let state = MerchantState {
            inventory: vec![make_item("A", 100)],
            items_per_page: 10,
            page: 5,
            ..Default::default()
        };
        assert!(state.current_page_items().is_empty());
    }

    #[test]
    fn exact_page_boundary() {
        let state = MerchantState {
            inventory: (0..20).map(|i| make_item(&format!("I{i}"), 100)).collect(),
            items_per_page: 10,
            page: 0,
            ..Default::default()
        };
        assert_eq!(state.page_count(), 2);
        assert_eq!(state.current_page_items().len(), 10);
    }

    #[test]
    fn next_page_navigation() {
        let mut state = MerchantState {
            inventory: (0..25).map(|i| make_item(&format!("I{i}"), 100)).collect(),
            items_per_page: 10,
            page: 0,
            ..Default::default()
        };
        state.next_page();
        assert_eq!(state.page, 1);
        state.next_page();
        assert_eq!(state.page, 2);
        state.next_page(); // clamped at last page
        assert_eq!(state.page, 2);
    }

    #[test]
    fn prev_page_navigation() {
        let mut state = MerchantState {
            inventory: (0..25).map(|i| make_item(&format!("I{i}"), 100)).collect(),
            items_per_page: 10,
            page: 2,
            ..Default::default()
        };
        state.prev_page();
        assert_eq!(state.page, 1);
        state.prev_page();
        assert_eq!(state.page, 0);
        state.prev_page(); // clamped at 0
        assert_eq!(state.page, 0);
    }

    // --- Price calculation ---

    #[test]
    fn buy_cost_single() {
        let item = make_item("Arrow", 10);
        assert_eq!(MerchantState::buy_cost(&item, 1), Money(10));
    }

    #[test]
    fn buy_cost_stack() {
        let item = make_item("Arrow", 10);
        assert_eq!(MerchantState::buy_cost(&item, 20), Money(200));
    }

    #[test]
    fn can_afford_quantity() {
        let state = MerchantState {
            player_money: Money(500),
            ..Default::default()
        };
        let item = make_item("Arrow", 10);
        assert!(state.can_afford_quantity(&item, 20)); // 200 <= 500
        assert!(state.can_afford_quantity(&item, 50)); // 500 <= 500
        assert!(!state.can_afford_quantity(&item, 51)); // 510 > 500
    }

    #[test]
    fn sell_price_is_quarter_of_buy() {
        let item = make_item("Sword", 10000);
        assert_eq!(item.sell_price, Money(2500));
    }

    #[test]
    fn repair_zero_cost_cannot_repair() {
        let state = MerchantState {
            player_money: Money(10000),
            repair_cost: Money(0),
            ..Default::default()
        };
        assert!(!state.can_repair());
    }

    #[test]
    fn items_per_page_zero_shows_all() {
        let state = MerchantState {
            inventory: (0..5).map(|i| make_item(&format!("I{i}"), 100)).collect(),
            items_per_page: 0,
            ..Default::default()
        };
        assert_eq!(state.page_count(), 1);
        assert_eq!(state.current_page_items().len(), 5);
    }

    // --- Open / close lifecycle ---

    #[test]
    fn state_starts_closed() {
        let state = MerchantState::default();
        assert!(!state.is_open());
        assert!(state.npc_entity_id.is_none());
    }

    #[test]
    fn open_and_close() {
        let mut state = MerchantState {
            items_per_page: 10,
            ..Default::default()
        };
        let items = vec![make_item("Sword", 5000), make_item("Shield", 3000)];
        state.open(42, items, Money(10000), Money(500));

        assert!(state.is_open());
        assert_eq!(state.npc_entity_id, Some(42));
        assert_eq!(state.inventory.len(), 2);
        assert_eq!(state.player_money, Money(10000));
        assert_eq!(state.repair_cost, Money(500));
        assert_eq!(state.page, 0);

        state.close();
        assert!(!state.is_open());
        assert!(state.inventory.is_empty());
        assert_eq!(state.player_money, Money(0));
    }

    #[test]
    fn open_resets_page() {
        let mut state = MerchantState {
            page: 3,
            items_per_page: 10,
            ..Default::default()
        };
        state.open(1, vec![], Money(0), Money(0));
        assert_eq!(state.page, 0);
    }

    // --- MerchantIntentQueue ---

    #[test]
    fn intent_buy() {
        let mut queue = MerchantIntentQueue::default();
        queue.buy(100, 5);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(
            drained[0],
            MerchantIntent::Buy {
                item_id: 100,
                quantity: 5
            }
        );
    }

    #[test]
    fn intent_sell() {
        let mut queue = MerchantIntentQueue::default();
        queue.sell(0, 3);
        let drained = queue.drain();
        assert_eq!(drained[0], MerchantIntent::Sell { bag: 0, slot: 3 });
    }

    #[test]
    fn intent_buyback() {
        let mut queue = MerchantIntentQueue::default();
        queue.buyback(2);
        let drained = queue.drain();
        assert_eq!(drained[0], MerchantIntent::Buyback { slot: 2 });
    }

    #[test]
    fn intent_repair_all() {
        let mut queue = MerchantIntentQueue::default();
        queue.repair_all();
        let drained = queue.drain();
        assert_eq!(drained[0], MerchantIntent::RepairAll);
    }

    #[test]
    fn intent_repair_single() {
        let mut queue = MerchantIntentQueue::default();
        queue.repair_single(1, 5);
        let drained = queue.drain();
        assert_eq!(drained[0], MerchantIntent::RepairSingle { bag: 1, slot: 5 });
    }

    #[test]
    fn intent_drain_clears() {
        let mut queue = MerchantIntentQueue::default();
        queue.buy(1, 1);
        queue.sell(0, 0);
        queue.repair_all();
        assert_eq!(queue.drain().len(), 3);
        assert!(queue.pending.is_empty());
    }
}
