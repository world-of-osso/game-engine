use bevy::prelude::*;

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

/// Runtime state for the auction house.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct AuctionHouseState {
    pub search_results: Vec<AuctionSearchResult>,
    pub my_listings: Vec<MyAuctionListing>,
    pub player_money: Money,
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
}
