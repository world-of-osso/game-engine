pub(super) use super::*;

mod filter_state;
mod intents;
mod values;

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
