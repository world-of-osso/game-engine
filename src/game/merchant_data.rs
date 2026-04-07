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
}

impl MerchantState {
    pub fn page_count(&self) -> usize {
        if self.items_per_page == 0 {
            return 1;
        }
        ((self.inventory.len() + self.items_per_page - 1) / self.items_per_page).max(1)
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
}
