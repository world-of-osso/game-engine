use bevy::prelude::*;

/// Texture FDIDs for the auction house frame.
pub mod textures {
    // Frame chrome
    /// Auction frame top border.
    pub const FRAME_TOP: u32 = 130681;
    /// Auction frame top-left corner.
    pub const FRAME_TOP_LEFT: u32 = 130682;
    /// Auction frame top-right corner.
    pub const FRAME_TOP_RIGHT: u32 = 130683;
    /// Auction frame bottom border.
    pub const FRAME_BOTTOM: u32 = 130678;
    // Browse tab chrome
    /// Browse tab top border.
    pub const BROWSE_TOP: u32 = 130693;
    /// Browse tab bottom border.
    pub const BROWSE_BOTTOM: u32 = 130690;
    // Item slot / icons
    /// Buyout icon.
    pub const BUYOUT_ICON: u32 = 130677;
    // Money denomination icons
    /// Gold coin icon.
    pub const GOLD_ICON: u32 = 237618;
    /// Silver coin icon.
    pub const SILVER_ICON: u32 = 237620;
    /// Copper coin icon.
    pub const COPPER_ICON: u32 = 237617;
    /// Combined money icons sheet.
    pub const MONEY_ICONS: u32 = 136496;
}

/// A money amount stored as total copper (1g = 100s = 10000c).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Money(pub u64);

impl Money {
    pub fn from_gold_silver_copper(gold: u64, silver: u64, copper: u64) -> Self {
        Self(gold * 10_000 + silver * 100 + copper)
    }

    pub fn gold(self) -> u64 {
        self.0 / 10_000
    }

    pub fn silver(self) -> u64 {
        (self.0 % 10_000) / 100
    }

    pub fn copper(self) -> u64 {
        self.0 % 100
    }

    /// Format as "Xg Ys Zc", omitting zero leading denominations.
    pub fn display(&self) -> String {
        let g = self.gold();
        let s = self.silver();
        let c = self.copper();
        if g > 0 {
            format!("{g}g {s}s {c}c")
        } else if s > 0 {
            format!("{s}s {c}c")
        } else {
            format!("{c}c")
        }
    }

    /// Short format — gold only if > 0, e.g. "50g" or "25s".
    pub fn display_short(&self) -> String {
        let g = self.gold();
        let s = self.silver();
        let c = self.copper();
        if g > 0 && s == 0 && c == 0 {
            format!("{g}g")
        } else if g > 0 {
            format!("{g}g {s}s")
        } else if s > 0 {
            format!("{s}s")
        } else {
            format!("{c}c")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuctionDuration {
    Short,
    Medium,
    Long,
}

impl AuctionDuration {
    pub fn label(self) -> &'static str {
        match self {
            Self::Short => "12 Hours",
            Self::Medium => "24 Hours",
            Self::Long => "48 Hours",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuctionSearchResult {
    pub item_name: String,
    pub item_level: u32,
    pub time_left: AuctionDuration,
    pub seller: String,
    pub bid_amount: Money,
    pub buyout_amount: Money,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MyAuctionListing {
    pub item_name: String,
    pub time_left: AuctionDuration,
    pub bid_amount: Money,
    pub buyout_amount: Money,
    pub sold: bool,
}

impl MyAuctionListing {
    pub fn status_label(&self) -> &'static str {
        if self.sold { "Sold" } else { "Active" }
    }
}

impl AuctionSearchResult {
    /// Whether the player can place a bid on this listing.
    /// Bid must be > 0 and >= current bid, and player must have enough money.
    pub fn can_bid(&self, amount: Money, player_money: Money) -> bool {
        amount.0 > 0 && amount.0 >= self.bid_amount.0 && player_money.0 >= amount.0
    }

    /// Whether the player can buyout this listing.
    pub fn can_buyout(&self, player_money: Money) -> bool {
        self.buyout_amount.0 > 0 && player_money.0 >= self.buyout_amount.0
    }
}

impl AuctionDuration {
    /// Duration in hours.
    pub fn hours(self) -> u32 {
        match self {
            Self::Short => 12,
            Self::Medium => 24,
            Self::Long => 48,
        }
    }
}

// --- Search filters ---

/// Column to sort auction search results by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AuctionSortField {
    #[default]
    Name,
    Level,
    TimeLeft,
    Seller,
    CurrentBid,
    Buyout,
}

/// Client-side search filter for auction queries.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AuctionSearchFilter {
    pub text: String,
    pub min_level: Option<u32>,
    pub max_level: Option<u32>,
    /// Item quality filter (WoW quality ID: 0=Poor..5=Legendary).
    pub quality: Option<u8>,
    pub usable_only: bool,
    pub sort: AuctionSortField,
    pub sort_ascending: bool,
    pub page: u32,
}

impl AuctionSearchFilter {
    /// Reset all filters to defaults, keeping sort preferences.
    pub fn reset(&mut self) {
        self.text.clear();
        self.min_level = None;
        self.max_level = None;
        self.quality = None;
        self.usable_only = false;
        self.page = 0;
    }

    /// Set the page, clamping to 0 if the filter changed.
    pub fn set_page(&mut self, page: u32) {
        self.page = page;
    }

    /// Toggle sort direction or switch to a new sort field.
    pub fn set_sort(&mut self, field: AuctionSortField) {
        if self.sort == field {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort = field;
            self.sort_ascending = true;
        }
        self.page = 0;
    }
}

// --- Runtime state ---

/// Runtime state for the auction house.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct AuctionHouseState {
    pub search_results: Vec<AuctionSearchResult>,
    pub my_listings: Vec<MyAuctionListing>,
    pub player_money: Money,
    /// Total results available on the server (for pagination).
    pub total_results: u32,
    /// Whether the auction house window is open.
    pub is_open: bool,
    /// Active search filter.
    pub filter: AuctionSearchFilter,
}

impl AuctionHouseState {
    /// Open the auction house, resetting state.
    pub fn open(&mut self) {
        self.is_open = true;
        self.search_results.clear();
        self.my_listings.clear();
        self.total_results = 0;
        self.filter.reset();
    }

    /// Close the auction house and clear all state.
    pub fn close(&mut self) {
        self.is_open = false;
        self.search_results.clear();
        self.my_listings.clear();
        self.total_results = 0;
    }

    /// Update search results from a server response.
    pub fn update_results(&mut self, results: Vec<AuctionSearchResult>, total: u32) {
        self.search_results = results;
        self.total_results = total;
    }

    /// Update the player's own auction listings.
    pub fn update_listings(&mut self, listings: Vec<MyAuctionListing>) {
        self.my_listings = listings;
    }

    /// Total pages based on results per page.
    pub fn page_count(&self, per_page: u32) -> u32 {
        if per_page == 0 {
            return 1;
        }
        self.total_results.div_ceil(per_page).max(1)
    }
}

// --- Client → server intents ---

/// A pending auction house action to send to the server.
#[derive(Clone, Debug, PartialEq)]
pub enum AuctionIntent {
    /// Execute a search with the given filter.
    Search { filter: AuctionSearchFilter },
    /// Query the player's own active listings.
    QueryOwned,
    /// Query auctions the player has bid on.
    QueryBids,
    /// Open the auction house UI.
    Open,
    /// Close the auction house UI.
    Close,
}

/// Queue of auction intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct AuctionIntentQueue {
    pub pending: Vec<AuctionIntent>,
}

impl AuctionIntentQueue {
    pub fn search(&mut self, filter: AuctionSearchFilter) {
        self.pending.push(AuctionIntent::Search { filter });
    }

    pub fn query_owned(&mut self) {
        self.pending.push(AuctionIntent::QueryOwned);
    }

    pub fn query_bids(&mut self) {
        self.pending.push(AuctionIntent::QueryBids);
    }

    pub fn open(&mut self) {
        self.pending.push(AuctionIntent::Open);
    }

    pub fn close(&mut self) {
        self.pending.push(AuctionIntent::Close);
    }

    pub fn drain(&mut self) -> Vec<AuctionIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_from_gold_silver_copper() {
        let m = Money::from_gold_silver_copper(5, 30, 42);
        assert_eq!(m.gold(), 5);
        assert_eq!(m.silver(), 30);
        assert_eq!(m.copper(), 42);
        assert_eq!(m.0, 53042);
    }

    #[test]
    fn money_display_full() {
        let m = Money::from_gold_silver_copper(12, 5, 80);
        assert_eq!(m.display(), "12g 5s 80c");
    }

    #[test]
    fn money_display_no_gold() {
        let m = Money::from_gold_silver_copper(0, 25, 0);
        assert_eq!(m.display(), "25s 0c");
    }

    #[test]
    fn money_display_copper_only() {
        let m = Money(42);
        assert_eq!(m.display(), "42c");
    }

    #[test]
    fn money_display_short_round_gold() {
        let m = Money::from_gold_silver_copper(50, 0, 0);
        assert_eq!(m.display_short(), "50g");
    }

    #[test]
    fn money_display_short_with_silver() {
        let m = Money::from_gold_silver_copper(3, 20, 0);
        assert_eq!(m.display_short(), "3g 20s");
    }

    #[test]
    fn money_display_short_silver_only() {
        let m = Money::from_gold_silver_copper(0, 80, 50);
        assert_eq!(m.display_short(), "80s");
    }

    #[test]
    fn auction_duration_labels() {
        assert_eq!(AuctionDuration::Short.label(), "12 Hours");
        assert_eq!(AuctionDuration::Medium.label(), "24 Hours");
        assert_eq!(AuctionDuration::Long.label(), "48 Hours");
    }

    #[test]
    fn my_listing_status_label() {
        let active = MyAuctionListing {
            item_name: "Sword".into(),
            time_left: AuctionDuration::Long,
            bid_amount: Money(100),
            buyout_amount: Money(200),
            sold: false,
        };
        assert_eq!(active.status_label(), "Active");

        let sold = MyAuctionListing {
            sold: true,
            ..active
        };
        assert_eq!(sold.status_label(), "Sold");
    }

    #[test]
    fn default_state_is_empty() {
        let state = AuctionHouseState::default();
        assert!(state.search_results.is_empty());
        assert!(state.my_listings.is_empty());
        assert_eq!(state.player_money, Money(0));
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FRAME_TOP, 0);
        assert_ne!(textures::FRAME_BOTTOM, 0);
        assert_ne!(textures::BROWSE_TOP, 0);
        assert_ne!(textures::BUYOUT_ICON, 0);
        assert_ne!(textures::GOLD_ICON, 0);
        assert_ne!(textures::SILVER_ICON, 0);
        assert_ne!(textures::COPPER_ICON, 0);
    }

    // --- Bid validation tests ---

    fn sample_listing() -> AuctionSearchResult {
        AuctionSearchResult {
            item_name: "Thunderfury".into(),
            item_level: 80,
            time_left: AuctionDuration::Long,
            seller: "Vendor".into(),
            bid_amount: Money::from_gold_silver_copper(10, 0, 0),
            buyout_amount: Money::from_gold_silver_copper(50, 0, 0),
        }
    }

    #[test]
    fn can_bid_valid() {
        let listing = sample_listing();
        let bid = Money::from_gold_silver_copper(10, 0, 0);
        let wallet = Money::from_gold_silver_copper(100, 0, 0);
        assert!(listing.can_bid(bid, wallet));
    }

    #[test]
    fn can_bid_above_current() {
        let listing = sample_listing();
        let bid = Money::from_gold_silver_copper(15, 0, 0);
        let wallet = Money::from_gold_silver_copper(100, 0, 0);
        assert!(listing.can_bid(bid, wallet));
    }

    #[test]
    fn cannot_bid_below_current() {
        let listing = sample_listing();
        let bid = Money::from_gold_silver_copper(5, 0, 0);
        let wallet = Money::from_gold_silver_copper(100, 0, 0);
        assert!(!listing.can_bid(bid, wallet));
    }

    #[test]
    fn cannot_bid_zero() {
        let listing = sample_listing();
        let wallet = Money::from_gold_silver_copper(100, 0, 0);
        assert!(!listing.can_bid(Money(0), wallet));
    }

    #[test]
    fn cannot_bid_insufficient_funds() {
        let listing = sample_listing();
        let bid = Money::from_gold_silver_copper(10, 0, 0);
        let wallet = Money::from_gold_silver_copper(5, 0, 0);
        assert!(!listing.can_bid(bid, wallet));
    }

    #[test]
    fn can_buyout_with_funds() {
        let listing = sample_listing();
        let wallet = Money::from_gold_silver_copper(50, 0, 0);
        assert!(listing.can_buyout(wallet));
    }

    #[test]
    fn cannot_buyout_insufficient_funds() {
        let listing = sample_listing();
        let wallet = Money::from_gold_silver_copper(25, 0, 0);
        assert!(!listing.can_buyout(wallet));
    }

    #[test]
    fn cannot_buyout_zero_buyout_price() {
        let mut listing = sample_listing();
        listing.buyout_amount = Money(0);
        let wallet = Money::from_gold_silver_copper(100, 0, 0);
        assert!(!listing.can_buyout(wallet));
    }

    // --- Listing expiry tests ---

    #[test]
    fn duration_hours() {
        assert_eq!(AuctionDuration::Short.hours(), 12);
        assert_eq!(AuctionDuration::Medium.hours(), 24);
        assert_eq!(AuctionDuration::Long.hours(), 48);
    }

    // --- Money edge cases ---

    #[test]
    fn money_zero_display() {
        assert_eq!(Money(0).display(), "0c");
    }

    #[test]
    fn money_display_short_copper_only() {
        assert_eq!(Money(5).display_short(), "5c");
    }

    #[test]
    fn money_round_trip() {
        let m = Money::from_gold_silver_copper(123, 45, 67);
        assert_eq!(m.gold(), 123);
        assert_eq!(m.silver(), 45);
        assert_eq!(m.copper(), 67);
        assert_eq!(m.0, 1_234_567);
    }

    // --- AuctionSearchFilter ---

    #[test]
    fn filter_default_values() {
        let filter = AuctionSearchFilter::default();
        assert!(filter.text.is_empty());
        assert!(filter.min_level.is_none());
        assert!(filter.quality.is_none());
        assert!(!filter.usable_only);
        assert_eq!(filter.sort, AuctionSortField::Name);
        assert!(!filter.sort_ascending);
        assert_eq!(filter.page, 0);
    }

    #[test]
    fn filter_reset_clears_text_and_levels() {
        let mut filter = AuctionSearchFilter {
            text: "Sword".into(),
            min_level: Some(10),
            max_level: Some(60),
            quality: Some(3),
            usable_only: true,
            sort: AuctionSortField::Buyout,
            sort_ascending: true,
            page: 5,
        };
        filter.reset();
        assert!(filter.text.is_empty());
        assert!(filter.min_level.is_none());
        assert!(filter.quality.is_none());
        assert!(!filter.usable_only);
        assert_eq!(filter.page, 0);
        // Sort preferences preserved.
        assert_eq!(filter.sort, AuctionSortField::Buyout);
        assert!(filter.sort_ascending);
    }

    #[test]
    fn filter_set_sort_toggles_direction() {
        let mut filter = AuctionSearchFilter::default();
        filter.set_sort(AuctionSortField::Buyout);
        assert_eq!(filter.sort, AuctionSortField::Buyout);
        assert!(filter.sort_ascending);

        // Same field again → toggle direction.
        filter.set_sort(AuctionSortField::Buyout);
        assert!(!filter.sort_ascending);
    }

    #[test]
    fn filter_set_sort_new_field_resets_direction() {
        let mut filter = AuctionSearchFilter {
            sort: AuctionSortField::Buyout,
            sort_ascending: false,
            page: 3,
            ..Default::default()
        };
        filter.set_sort(AuctionSortField::Level);
        assert_eq!(filter.sort, AuctionSortField::Level);
        assert!(filter.sort_ascending);
        assert_eq!(filter.page, 0); // reset on sort change
    }

    // --- AuctionHouseState lifecycle ---

    #[test]
    fn state_starts_closed() {
        let state = AuctionHouseState::default();
        assert!(!state.is_open);
    }

    #[test]
    fn state_open_and_close() {
        let mut state = AuctionHouseState::default();
        state.open();
        assert!(state.is_open);
        assert!(state.search_results.is_empty());

        state.close();
        assert!(!state.is_open);
    }

    #[test]
    fn state_update_results() {
        let mut state = AuctionHouseState::default();
        state.open();
        let results = vec![sample_listing()];
        state.update_results(results, 42);
        assert_eq!(state.search_results.len(), 1);
        assert_eq!(state.total_results, 42);
    }

    #[test]
    fn state_update_listings() {
        let mut state = AuctionHouseState::default();
        let listings = vec![MyAuctionListing {
            item_name: "Axe".into(),
            time_left: AuctionDuration::Short,
            bid_amount: Money(100),
            buyout_amount: Money(200),
            sold: false,
        }];
        state.update_listings(listings);
        assert_eq!(state.my_listings.len(), 1);
    }

    #[test]
    fn state_page_count() {
        let mut state = AuctionHouseState::default();
        state.total_results = 25;
        assert_eq!(state.page_count(10), 3);
        assert_eq!(state.page_count(25), 1);
        assert_eq!(state.page_count(0), 1);
    }

    #[test]
    fn close_clears_results() {
        let mut state = AuctionHouseState::default();
        state.open();
        state.update_results(vec![sample_listing()], 1);
        state.close();
        assert!(state.search_results.is_empty());
        assert_eq!(state.total_results, 0);
    }

    // --- AuctionIntentQueue ---

    #[test]
    fn intent_search() {
        let mut queue = AuctionIntentQueue::default();
        let filter = AuctionSearchFilter {
            text: "Sword".into(),
            ..Default::default()
        };
        queue.search(filter.clone());
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0], AuctionIntent::Search { filter });
    }

    #[test]
    fn intent_query_owned() {
        let mut queue = AuctionIntentQueue::default();
        queue.query_owned();
        let drained = queue.drain();
        assert_eq!(drained[0], AuctionIntent::QueryOwned);
    }

    #[test]
    fn intent_query_bids() {
        let mut queue = AuctionIntentQueue::default();
        queue.query_bids();
        let drained = queue.drain();
        assert_eq!(drained[0], AuctionIntent::QueryBids);
    }

    #[test]
    fn intent_open_close() {
        let mut queue = AuctionIntentQueue::default();
        queue.open();
        queue.close();
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0], AuctionIntent::Open);
        assert_eq!(drained[1], AuctionIntent::Close);
    }

    #[test]
    fn intent_drain_clears() {
        let mut queue = AuctionIntentQueue::default();
        queue.search(AuctionSearchFilter::default());
        queue.query_owned();
        assert_eq!(queue.drain().len(), 2);
        assert!(queue.pending.is_empty());
    }
}
