use bevy::prelude::*;

pub mod textures {
    // --- Trade frame chrome ---
    /// Trade player left panel (normal state).
    pub const TRADE_PLAYER_LEFT: u32 = 137059;
    /// Trade player right panel (normal state).
    pub const TRADE_PLAYER_RIGHT: u32 = 137061;
    /// Trade player left panel (accepted state).
    pub const TRADE_PLAYER_LEFT_ACCEPT: u32 = 137058;
    /// Trade target top-left (normal state).
    pub const TRADE_TARGET_TOP_LEFT: u32 = 137067;
    /// Trade target top-left (accepted state).
    pub const TRADE_TARGET_TOP_LEFT_ACCEPT: u32 = 137066;

    // --- Slot borders ---
    /// Empty item slot background.
    pub const SLOT_EMPTY: u32 = 130766;
    /// Item slot background (darker).
    pub const SLOT_BACKGROUND: u32 = 130862;
    /// Auction-style item slot frame.
    pub const SLOT_AUCTION: u32 = 365781;

    // --- Money icons ---
    /// Gold coin icon.
    pub const COIN_GOLD: u32 = 133784;
    /// Silver coin icon.
    pub const COIN_SILVER: u32 = 133785;
    /// Copper coin icon.
    pub const COIN_COPPER: u32 = 133786;

    // --- Highlight ---
    /// Check button highlight glow (accept state).
    pub const ACCEPT_HIGHLIGHT: u32 = 130724;
}

// --- Trade slot ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradeSlot {
    pub item_name: String,
    pub icon_fdid: u32,
    pub quantity: u32,
    pub item_quality: ItemQuality,
}

impl TradeSlot {
    pub fn is_empty(&self) -> bool {
        self.item_name.is_empty()
    }

    pub fn display_name(&self) -> String {
        if self.quantity > 1 {
            format!("{} x{}", self.item_name, self.quantity)
        } else {
            self.item_name.clone()
        }
    }
}

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
    pub fn border_color(self) -> &'static str {
        match self {
            Self::Poor => "0.62,0.62,0.62,1.0",
            Self::Common => "1.0,1.0,1.0,1.0",
            Self::Uncommon => "0.12,1.0,0.0,1.0",
            Self::Rare => "0.0,0.44,0.87,1.0",
            Self::Epic => "0.64,0.21,0.93,1.0",
            Self::Legendary => "1.0,0.5,0.0,1.0",
        }
    }
}

// --- Money ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Money {
    /// Total amount in copper.
    pub copper: u32,
}

impl Money {
    pub fn new(gold: u32, silver: u32, copper: u32) -> Self {
        Self {
            copper: gold * 10000 + silver * 100 + copper,
        }
    }

    pub fn gold(self) -> u32 {
        self.copper / 10000
    }

    pub fn silver(self) -> u32 {
        (self.copper % 10000) / 100
    }

    pub fn copper_rem(self) -> u32 {
        self.copper % 100
    }

    pub fn is_zero(self) -> bool {
        self.copper == 0
    }

    pub fn display(self) -> String {
        let g = self.gold();
        let s = self.silver();
        let c = self.copper_rem();
        if g > 0 {
            format!("{g}g {s}s {c}c")
        } else if s > 0 {
            format!("{s}s {c}c")
        } else {
            format!("{c}c")
        }
    }
}

// --- Accept state ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AcceptState {
    #[default]
    Pending,
    Accepted,
}

// --- Player trade panel ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradePlayerData {
    pub name: String,
    pub slots: [Option<TradeSlot>; 7],
    pub money: Money,
    pub accept: AcceptState,
}

impl TradePlayerData {
    pub fn filled_slot_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn is_accepted(&self) -> bool {
        self.accept == AcceptState::Accepted
    }
}

// --- Runtime resource ---

/// Runtime trade state, held as a Bevy Resource.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct TradeState {
    pub active: bool,
    pub player: TradePlayerData,
    pub other: TradePlayerData,
}

impl TradeState {
    pub fn both_accepted(&self) -> bool {
        self.player.is_accepted() && self.other.is_accepted()
    }

    pub fn total_items(&self) -> usize {
        self.player.filled_slot_count() + self.other.filled_slot_count()
    }

    /// Player accepts the trade.
    pub fn player_accept(&mut self) {
        self.player.accept = AcceptState::Accepted;
    }

    /// Reset both accept states (e.g. when trade contents change).
    pub fn reset_accepts(&mut self) {
        self.player.accept = AcceptState::Pending;
        self.other.accept = AcceptState::Pending;
    }

    /// Validate that the player's offered money does not exceed their wallet.
    pub fn validate_player_money(&self, wallet: u32) -> bool {
        self.player.money.copper <= wallet
    }

    /// Place an item in the player's next empty slot. Returns the slot index or None if full.
    pub fn add_player_item(&mut self, item: TradeSlot) -> Option<usize> {
        let idx = self.player.slots.iter().position(|s| s.is_none())?;
        self.player.slots[idx] = Some(item);
        self.reset_accepts();
        Some(idx)
    }

    /// Remove an item from a player slot by index.
    pub fn remove_player_item(&mut self, index: usize) {
        if index < self.player.slots.len() {
            self.player.slots[index] = None;
            self.reset_accepts();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(name: &str, qty: u32) -> TradeSlot {
        TradeSlot {
            item_name: name.into(),
            icon_fdid: 1,
            quantity: qty,
            item_quality: ItemQuality::Common,
        }
    }

    // --- TradeSlot ---

    #[test]
    fn slot_empty() {
        assert!(TradeSlot::default().is_empty());
        assert!(!slot("Ore", 1).is_empty());
    }

    #[test]
    fn slot_display_name() {
        assert_eq!(slot("Sword", 1).display_name(), "Sword");
        assert_eq!(slot("Ore", 20).display_name(), "Ore x20");
    }

    // --- ItemQuality ---

    #[test]
    fn quality_border_colors_non_empty() {
        for q in [
            ItemQuality::Poor,
            ItemQuality::Common,
            ItemQuality::Uncommon,
            ItemQuality::Rare,
            ItemQuality::Epic,
            ItemQuality::Legendary,
        ] {
            assert!(!q.border_color().is_empty());
        }
    }

    // --- Money ---

    #[test]
    fn money_new_and_parts() {
        let m = Money::new(15, 30, 50);
        assert_eq!(m.copper, 153050);
        assert_eq!(m.gold(), 15);
        assert_eq!(m.silver(), 30);
        assert_eq!(m.copper_rem(), 50);
    }

    #[test]
    fn money_display() {
        assert_eq!(Money::new(15, 0, 0).display(), "15g 0s 0c");
        assert_eq!(Money::new(0, 3, 50).display(), "3s 50c");
        assert_eq!(Money::new(0, 0, 42).display(), "42c");
    }

    #[test]
    fn money_zero() {
        assert!(Money::default().is_zero());
        assert!(!Money::new(0, 0, 1).is_zero());
    }

    // --- TradePlayerData ---

    #[test]
    fn player_filled_slot_count() {
        let mut p = TradePlayerData::default();
        assert_eq!(p.filled_slot_count(), 0);
        p.slots[0] = Some(slot("A", 1));
        p.slots[3] = Some(slot("B", 5));
        assert_eq!(p.filled_slot_count(), 2);
    }

    #[test]
    fn player_is_accepted() {
        let mut p = TradePlayerData::default();
        assert!(!p.is_accepted());
        p.accept = AcceptState::Accepted;
        assert!(p.is_accepted());
    }

    // --- TradeState ---

    #[test]
    fn both_accepted() {
        let mut state = TradeState {
            active: true,
            ..Default::default()
        };
        assert!(!state.both_accepted());
        state.player.accept = AcceptState::Accepted;
        assert!(!state.both_accepted());
        state.other.accept = AcceptState::Accepted;
        assert!(state.both_accepted());
    }

    #[test]
    fn total_items() {
        let mut state = TradeState::default();
        state.player.slots[0] = Some(slot("A", 1));
        state.other.slots[2] = Some(slot("B", 1));
        state.other.slots[4] = Some(slot("C", 1));
        assert_eq!(state.total_items(), 3);
    }

    // --- Accept state transitions ---

    #[test]
    fn player_accept_sets_state() {
        let mut state = TradeState {
            active: true,
            ..Default::default()
        };
        state.player_accept();
        assert!(state.player.is_accepted());
        assert!(!state.other.is_accepted());
    }

    #[test]
    fn reset_accepts_clears_both() {
        let mut state = TradeState {
            active: true,
            ..Default::default()
        };
        state.player.accept = AcceptState::Accepted;
        state.other.accept = AcceptState::Accepted;
        state.reset_accepts();
        assert!(!state.player.is_accepted());
        assert!(!state.other.is_accepted());
    }

    // --- Money validation ---

    #[test]
    fn validate_money_sufficient() {
        let mut state = TradeState::default();
        state.player.money = Money::new(5, 0, 0);
        assert!(state.validate_player_money(100_000)); // 10g wallet
    }

    #[test]
    fn validate_money_exact() {
        let mut state = TradeState::default();
        state.player.money = Money::new(10, 0, 0);
        assert!(state.validate_player_money(100_000)); // exactly 10g
    }

    #[test]
    fn validate_money_insufficient() {
        let mut state = TradeState::default();
        state.player.money = Money::new(10, 0, 0);
        assert!(!state.validate_player_money(50_000)); // only 5g
    }

    #[test]
    fn validate_money_zero_offered() {
        let state = TradeState::default();
        assert!(state.validate_player_money(0));
    }

    // --- Slot mirroring (panels independent) ---

    #[test]
    fn player_and_other_slots_independent() {
        let mut state = TradeState {
            active: true,
            ..Default::default()
        };
        state.player.slots[0] = Some(slot("Ore", 20));
        state.other.slots[0] = Some(slot("Gem", 5));
        assert_eq!(state.player.slots[0].as_ref().unwrap().item_name, "Ore");
        assert_eq!(state.other.slots[0].as_ref().unwrap().item_name, "Gem");
        assert_eq!(state.total_items(), 2);
    }

    #[test]
    fn add_player_item_fills_next_empty() {
        let mut state = TradeState::default();
        let idx0 = state.add_player_item(slot("A", 1));
        assert_eq!(idx0, Some(0));
        let idx1 = state.add_player_item(slot("B", 1));
        assert_eq!(idx1, Some(1));
        assert_eq!(state.player.filled_slot_count(), 2);
    }

    #[test]
    fn add_player_item_full_returns_none() {
        let mut state = TradeState::default();
        for i in 0..7 {
            state.player.slots[i] = Some(slot(&format!("Item{i}"), 1));
        }
        assert_eq!(state.add_player_item(slot("Extra", 1)), None);
    }

    #[test]
    fn add_item_resets_accepts() {
        let mut state = TradeState::default();
        state.player.accept = AcceptState::Accepted;
        state.other.accept = AcceptState::Accepted;
        state.add_player_item(slot("New", 1));
        assert!(!state.player.is_accepted());
        assert!(!state.other.is_accepted());
    }

    #[test]
    fn remove_item_resets_accepts() {
        let mut state = TradeState::default();
        state.player.slots[0] = Some(slot("A", 1));
        state.player.accept = AcceptState::Accepted;
        state.remove_player_item(0);
        assert!(state.player.slots[0].is_none());
        assert!(!state.player.is_accepted());
    }

    #[test]
    fn remove_item_out_of_bounds_no_panic() {
        let mut state = TradeState::default();
        state.remove_player_item(99); // should not panic
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        // Trade frame chrome
        assert_ne!(textures::TRADE_PLAYER_LEFT, 0);
        assert_ne!(textures::TRADE_PLAYER_RIGHT, 0);
        assert_ne!(textures::TRADE_PLAYER_LEFT_ACCEPT, 0);
        assert_ne!(textures::TRADE_TARGET_TOP_LEFT, 0);
        assert_ne!(textures::TRADE_TARGET_TOP_LEFT_ACCEPT, 0);
        // Slot borders
        assert_ne!(textures::SLOT_EMPTY, 0);
        assert_ne!(textures::SLOT_BACKGROUND, 0);
        assert_ne!(textures::SLOT_AUCTION, 0);
        // Money icons
        assert_ne!(textures::COIN_GOLD, 0);
        assert_ne!(textures::COIN_SILVER, 0);
        assert_ne!(textures::COIN_COPPER, 0);
        // Highlight
        assert_ne!(textures::ACCEPT_HIGHLIGHT, 0);
    }
}
