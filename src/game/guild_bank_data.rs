use bevy::prelude::*;

use crate::auction_house_data::Money;
use crate::bag_data::{InventorySlot, ItemQuality};

const SLOTS_PER_TAB: usize = 98;
const MAX_TABS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildBankPermission {
    ViewTab,
    DepositItems,
    WithdrawItems,
    DepositMoney,
    WithdrawMoney,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuildBankTabDef {
    pub name: String,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuildBankTransaction {
    pub player: String,
    pub action: String,
    pub item_name: String,
    pub amount: Option<Money>,
}

impl GuildBankTransaction {
    pub fn display(&self) -> String {
        if let Some(money) = self.amount {
            format!("{} {} {}", self.player, self.action, money.display())
        } else {
            format!("{} {} {}", self.player, self.action, self.item_name)
        }
    }
}

/// Runtime guild bank state.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct GuildBankState {
    pub tabs: Vec<GuildBankTabDef>,
    pub active_tab: usize,
    /// Slots indexed by tab, then slot index within tab.
    pub slots: Vec<Vec<InventorySlot>>,
    pub transactions: Vec<GuildBankTransaction>,
    pub guild_money: Money,
    pub permissions: Vec<GuildBankPermission>,
}

impl Default for GuildBankState {
    fn default() -> Self {
        Self {
            tabs: vec![GuildBankTabDef {
                name: "Tab 1".into(),
                icon_fdid: 0,
            }],
            active_tab: 0,
            slots: vec![vec![InventorySlot::default(); SLOTS_PER_TAB]],
            transactions: vec![],
            guild_money: Money(0),
            permissions: vec![
                GuildBankPermission::ViewTab,
                GuildBankPermission::DepositItems,
                GuildBankPermission::DepositMoney,
            ],
        }
    }
}

impl GuildBankState {
    pub fn active_tab_slots(&self) -> &[InventorySlot] {
        self.slots
            .get(self.active_tab)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn has_permission(&self, perm: GuildBankPermission) -> bool {
        self.permissions.contains(&perm)
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len().min(MAX_TABS)
    }

    pub fn can_withdraw(&self) -> bool {
        self.has_permission(GuildBankPermission::WithdrawItems)
    }

    pub fn can_deposit(&self) -> bool {
        self.has_permission(GuildBankPermission::DepositItems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_one_tab_with_98_slots() {
        let state = GuildBankState::default();
        assert_eq!(state.tab_count(), 1);
        assert_eq!(state.active_tab_slots().len(), SLOTS_PER_TAB);
    }

    #[test]
    fn permission_checks() {
        let state = GuildBankState::default();
        assert!(state.has_permission(GuildBankPermission::ViewTab));
        assert!(state.can_deposit());
        assert!(!state.can_withdraw());
    }

    #[test]
    fn transaction_display_item() {
        let tx = GuildBankTransaction {
            player: "Alice".into(),
            action: "deposited".into(),
            item_name: "Arcanite Bar".into(),
            amount: None,
        };
        assert_eq!(tx.display(), "Alice deposited Arcanite Bar");
    }

    #[test]
    fn transaction_display_money() {
        let tx = GuildBankTransaction {
            player: "Bob".into(),
            action: "withdrew".into(),
            item_name: String::new(),
            amount: Some(Money::from_gold_silver_copper(5, 0, 0)),
        };
        assert_eq!(tx.display(), "Bob withdrew 5g 0s 0c");
    }

    #[test]
    fn active_tab_out_of_bounds_returns_empty() {
        let mut state = GuildBankState::default();
        state.active_tab = 99;
        assert!(state.active_tab_slots().is_empty());
    }
}
