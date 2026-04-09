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
    /// Post a new auction listing.
    Post {
        item_bag: u8,
        item_slot: u8,
        stack_count: u32,
        min_bid: Money,
        buyout: Money,
        duration: AuctionDuration,
    },
    /// Place a bid on an existing auction.
    Bid { auction_id: u64, amount: Money },
    /// Instantly buy out an auction at the listed buyout price.
    Buyout { auction_id: u64 },
    /// Cancel one of the player's own active auctions.
    Cancel { auction_id: u64 },
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

    pub fn post(
        &mut self,
        item_bag: u8,
        item_slot: u8,
        stack_count: u32,
        min_bid: Money,
        buyout: Money,
        duration: AuctionDuration,
    ) {
        self.pending.push(AuctionIntent::Post {
            item_bag,
            item_slot,
            stack_count,
            min_bid,
            buyout,
            duration,
        });
    }

    pub fn bid(&mut self, auction_id: u64, amount: Money) {
        self.pending.push(AuctionIntent::Bid { auction_id, amount });
    }

    pub fn buyout(&mut self, auction_id: u64) {
        self.pending.push(AuctionIntent::Buyout { auction_id });
    }

    pub fn cancel(&mut self, auction_id: u64) {
        self.pending.push(AuctionIntent::Cancel { auction_id });
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
#[path = "auction_house_data_tests/mod.rs"]
mod tests;
