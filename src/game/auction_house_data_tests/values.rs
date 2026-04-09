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

#[test]
fn duration_hours() {
    assert_eq!(AuctionDuration::Short.hours(), 12);
    assert_eq!(AuctionDuration::Medium.hours(), 24);
    assert_eq!(AuctionDuration::Long.hours(), 48);
}

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
