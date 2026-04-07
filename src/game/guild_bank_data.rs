use bevy::prelude::*;

use crate::auction_house_data::Money;
use crate::bag_data::InventorySlot;

/// Texture FDIDs for the guild bank frame.
pub mod textures {
    /// Frame chrome left panel.
    pub const FRAME_LEFT: u32 = 132070;
    /// Frame chrome right panel.
    pub const FRAME_RIGHT: u32 = 132072;
    /// Slot grid background.
    pub const SLOTS_BG: u32 = 132073;
    /// Tab button texture.
    pub const TAB_BUTTON: u32 = 132074;
    /// Vault background.
    pub const VAULT_BG: u32 = 590068;
    /// Item slot border (shared).
    pub const SLOT_BORDER: u32 = 130862;
}

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

    /// Sort transactions so newest (last added) appear first.
    pub fn sort_transactions_newest_first(&mut self) {
        self.transactions.reverse();
    }

    /// Switch to a tab by index (clamped to valid range).
    pub fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
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

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FRAME_LEFT, 0);
        assert_ne!(textures::FRAME_RIGHT, 0);
        assert_ne!(textures::SLOTS_BG, 0);
        assert_ne!(textures::TAB_BUTTON, 0);
        assert_ne!(textures::VAULT_BG, 0);
        assert_ne!(textures::SLOT_BORDER, 0);
    }

    // --- Tab permission checks ---

    #[test]
    fn full_permissions_allow_everything() {
        let state = GuildBankState {
            permissions: vec![
                GuildBankPermission::ViewTab,
                GuildBankPermission::DepositItems,
                GuildBankPermission::WithdrawItems,
                GuildBankPermission::DepositMoney,
                GuildBankPermission::WithdrawMoney,
            ],
            ..Default::default()
        };
        assert!(state.has_permission(GuildBankPermission::ViewTab));
        assert!(state.can_deposit());
        assert!(state.can_withdraw());
        assert!(state.has_permission(GuildBankPermission::DepositMoney));
        assert!(state.has_permission(GuildBankPermission::WithdrawMoney));
    }

    #[test]
    fn empty_permissions_deny_everything() {
        let state = GuildBankState {
            permissions: vec![],
            ..Default::default()
        };
        assert!(!state.has_permission(GuildBankPermission::ViewTab));
        assert!(!state.can_deposit());
        assert!(!state.can_withdraw());
    }

    #[test]
    fn view_only_permissions() {
        let state = GuildBankState {
            permissions: vec![GuildBankPermission::ViewTab],
            ..Default::default()
        };
        assert!(state.has_permission(GuildBankPermission::ViewTab));
        assert!(!state.can_deposit());
        assert!(!state.can_withdraw());
    }

    // --- Transaction log ordering ---

    #[test]
    fn transaction_log_newest_first() {
        let mut state = GuildBankState::default();
        state.transactions = vec![
            GuildBankTransaction {
                player: "Alice".into(),
                action: "deposited".into(),
                item_name: "Ore".into(),
                amount: None,
            },
            GuildBankTransaction {
                player: "Bob".into(),
                action: "withdrew".into(),
                item_name: "Gem".into(),
                amount: None,
            },
            GuildBankTransaction {
                player: "Charlie".into(),
                action: "deposited".into(),
                item_name: "Bar".into(),
                amount: None,
            },
        ];
        state.sort_transactions_newest_first();
        assert_eq!(state.transactions[0].player, "Charlie");
        assert_eq!(state.transactions[1].player, "Bob");
        assert_eq!(state.transactions[2].player, "Alice");
    }

    // --- Tab switching ---

    #[test]
    fn switch_tab_changes_active_slots() {
        let mut state = GuildBankState::default();
        state.tabs.push(GuildBankTabDef {
            name: "Tab 2".into(),
            icon_fdid: 0,
        });
        state
            .slots
            .push(vec![InventorySlot::default(); SLOTS_PER_TAB]);
        // Place item in tab 0
        state.slots[0][0] = InventorySlot {
            icon_fdid: 100,
            name: "Ore".into(),
            ..Default::default()
        };
        // Place different item in tab 1
        state.slots[1][0] = InventorySlot {
            icon_fdid: 200,
            name: "Gem".into(),
            ..Default::default()
        };
        assert_eq!(state.active_tab_slots()[0].name, "Ore");
        state.switch_tab(1);
        assert_eq!(state.active_tab_slots()[0].name, "Gem");
    }

    #[test]
    fn switch_tab_out_of_bounds_stays() {
        let mut state = GuildBankState::default();
        state.switch_tab(99);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn tab_count_capped_at_max() {
        let mut state = GuildBankState::default();
        for i in 1..12 {
            state.tabs.push(GuildBankTabDef {
                name: format!("Tab {}", i + 1),
                icon_fdid: 0,
            });
        }
        assert_eq!(state.tabs.len(), 12);
        assert_eq!(state.tab_count(), MAX_TABS);
    }
}
