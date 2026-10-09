//! Frame view model from the network auction state, the frame's own selections and the
//! edit box texts.

use std::collections::HashMap;

use super::AuctionHouseState;
use crate::auction_house_frame_component::{
    AuctionHouseFrameState, AuctionsSubTab, AuctionsView, BID_BOXES, BrowseRow, BuyDialogView,
    ItemBuyView, ItemLine, ListingRow, MoneyBoxes, QUANTITY_BOX, SEARCH_BOX, SELL_BID_BOXES,
    SELL_BUYOUT_BOXES, SellInventoryRow, SellItemView, SellView,
};
use crate::item_catalog::ItemCatalogEntry;
use shared::protocol::{
    AuctionDuration, AuctionInventoryItem, AuctionListingSummary, AuctionTimeLeft,
};

use super::AuctionHouseUi;

pub use super::categories::CATEGORIES;

/// Text of every frame edit box, read from the registry.
pub type InputTexts = HashMap<&'static str, String>;

pub fn input_names() -> Vec<&'static str> {
    let mut names = vec![SEARCH_BOX, QUANTITY_BOX];
    for boxes in [SELL_BUYOUT_BOXES, SELL_BID_BOXES, BID_BOXES] {
        names.extend(boxes.names());
    }
    names
}

pub fn text<'a>(texts: &'a InputTexts, name: &str) -> &'a str {
    texts.get(name).map_or("", String::as_str)
}

/// Copper typed into a gold / silver / copper input; empty boxes count as zero.
pub fn money_input(texts: &InputTexts, boxes: MoneyBoxes) -> u64 {
    let part = |name| text(texts, name).trim().parse::<u64>().unwrap_or(0);
    part(boxes.gold)
        .saturating_mul(10_000)
        .saturating_add(part(boxes.silver).saturating_mul(100))
        .saturating_add(part(boxes.copper))
}

pub fn quantity_input(texts: &InputTexts) -> u32 {
    text(texts, QUANTITY_BOX).trim().parse().unwrap_or(0)
}

/// `AUCTION_TIME_LEFT1..4` band names.
pub fn time_left_label(time_left: AuctionTimeLeft) -> &'static str {
    match time_left {
        AuctionTimeLeft::Short => "Short",
        AuctionTimeLeft::Medium => "Medium",
        AuctionTimeLeft::Long => "Long",
        AuctionTimeLeft::VeryLong => "Very Long",
    }
}

/// Deposit the server takes: vendor sell price × quantity × 1 / 2 / 4 for 12 / 24 / 48 hours
/// (game-server `auction_house/operations.rs:4` `deposit_cost`).
pub fn deposit(item: &AuctionInventoryItem, quantity: u32, duration: AuctionDuration) -> u64 {
    let multiplier = match duration {
        AuctionDuration::Short => 1,
        AuctionDuration::Medium => 2,
        AuctionDuration::Long => 4,
    };
    u64::from(item.vendor_sell_price) * u64::from(quantity) * multiplier
}

/// The frame's picture of an auction's price: current bid (or the minimum bid before any).
pub fn listing_bid(listing: &AuctionListingSummary) -> u64 {
    listing.current_bid.unwrap_or(listing.min_bid)
}

pub struct ViewInputs<'a> {
    pub net: &'a AuctionHouseState,
    pub ui: &'a AuctionHouseUi,
    pub texts: &'a InputTexts,
    pub catalog: &'a dyn Fn(
        shared::item_data::ItemDefinitionSource,
        u32,
    ) -> Option<&'static ItemCatalogEntry>,
    pub visible: bool,
}

impl ViewInputs<'_> {
    fn item_line(&self, item: &AuctionInventoryItem) -> ItemLine {
        ItemLine {
            name: item.name.clone(),
            quality: item.quality,
            icon_fdid: (self.catalog)(item.definition_source, item.item_id)
                .map_or(0, |entry| entry.icon_fdid),
        }
    }

    fn money(&self) -> u64 {
        self.net
            .inventory
            .as_ref()
            .map_or(0, |inventory| u64::from(inventory.gold))
    }

    fn listing_row(&self, listing: &AuctionListingSummary) -> ListingRow {
        ListingRow {
            auction_id: listing.auction_id,
            item: self.item_line(&listing.item),
            quantity: listing.stack_count,
            bid: Some(listing_bid(listing)),
            buyout: listing.buyout_price,
            time_left: time_left_label(listing.time_left).into(),
            selected: self.ui.selected_auction == Some(listing.auction_id),
        }
    }
}

pub fn build_view(inputs: &ViewInputs) -> AuctionHouseFrameState {
    AuctionHouseFrameState {
        visible: inputs.visible,
        tab: inputs.ui.tab,
        money: inputs.money(),
        search_empty: text(inputs.texts, SEARCH_BOX).is_empty(),
        categories: super::categories::rows(&inputs.ui.category_path),
        browse: browse_rows(inputs),
        browse_empty_text: browse_empty_text(inputs),
        item_buy: item_buy(inputs),
        dialog: dialog(inputs),
        sell: sell_view(inputs),
        auctions: auctions_view(inputs),
    }
}

/// Server-global item totals, already ordered and paged over distinct items.
fn browse_rows(inputs: &ViewInputs) -> Vec<BrowseRow> {
    inputs
        .net
        .browse_results
        .iter()
        .map(|item| BrowseRow {
            item_id: item.item_id,
            item: ItemLine {
                name: item.name.clone(),
                quality: item.quality,
                icon_fdid: (inputs.catalog)(item.definition_source, item.item_id)
                    .map_or(0, |entry| entry.icon_fdid),
            },
            price: item.lowest_unit_price,
            available: item.total_quantity,
        })
        .collect()
}

/// `BROWSE_NO_RESULTS` once a search came back empty.
fn browse_empty_text(inputs: &ViewInputs) -> Option<String> {
    let searched = inputs.net.last_query.is_some();
    (searched && inputs.net.browse_results.is_empty()).then(|| "No items found".into())
}

fn item_buy(inputs: &ViewInputs) -> Option<ItemBuyView> {
    let item_id = inputs.ui.browse_item?;
    let listings: Vec<&AuctionListingSummary> = inputs
        .net
        .search_results
        .iter()
        .filter(|listing| listing.item.item_id == item_id)
        .collect();
    let item = listings
        .first()
        .map(|listing| inputs.item_line(&listing.item))?;
    let selected = selected_listing(inputs, &inputs.net.search_results);
    Some(ItemBuyView {
        item,
        rows: listings
            .iter()
            .map(|listing| inputs.listing_row(listing))
            .collect(),
        can_bid: can_bid(inputs, selected),
        can_buyout: can_buyout(inputs, selected),
    })
}

fn selected_listing<'a>(
    inputs: &ViewInputs,
    listings: &'a [AuctionListingSummary],
) -> Option<&'a AuctionListingSummary> {
    let id = inputs.ui.selected_auction?;
    listings.iter().find(|listing| listing.auction_id == id)
}

/// A bid at least the next minimum that the player can pay.
fn can_bid(inputs: &ViewInputs, listing: Option<&AuctionListingSummary>) -> bool {
    let amount = money_input(inputs.texts, BID_BOXES);
    listing.is_some_and(|listing| {
        amount >= listing.min_next_bid && amount > 0 && amount <= inputs.money()
    })
}

fn can_buyout(inputs: &ViewInputs, listing: Option<&AuctionListingSummary>) -> bool {
    listing
        .and_then(|listing| listing.buyout_price)
        .is_some_and(|buyout| buyout <= inputs.money())
}

fn dialog(inputs: &ViewInputs) -> Option<BuyDialogView> {
    let id = inputs.ui.dialog_auction?;
    let listing = inputs
        .net
        .search_results
        .iter()
        .chain(&inputs.net.bid_results)
        .find(|listing| listing.auction_id == id)?;
    Some(BuyDialogView {
        // `AUCTION_HOUSE_DIALOG_ITEM_FORMAT` "%s  x%s".
        item_text: format!("{}  x{}", listing.item.name, listing.stack_count),
        price: listing.buyout_price?,
    })
}

fn sell_item<'a>(inputs: &'a ViewInputs) -> Option<&'a AuctionInventoryItem> {
    let guid = inputs.ui.sell_item?;
    inputs
        .net
        .inventory
        .as_ref()?
        .items
        .iter()
        .find(|item| item.item_guid == guid)
}

/// The create request the sell frame would post, when its inputs are valid.
pub fn sell_request(inputs: &ViewInputs) -> Option<shared::protocol::CreateAuction> {
    let item = sell_item(inputs)?;
    let quantity = quantity_input(inputs.texts);
    if quantity == 0 || quantity > item.stack_count {
        return None;
    }
    let buyout_unit = money_input(inputs.texts, SELL_BUYOUT_BOXES);
    let bid_unit = if inputs.ui.buyout_mode {
        buyout_unit
    } else {
        money_input(inputs.texts, SELL_BID_BOXES)
    };
    if bid_unit == 0 || (buyout_unit != 0 && buyout_unit < bid_unit) {
        return None;
    }
    if deposit(item, quantity, inputs.ui.duration) > inputs.money() {
        return None;
    }
    let total = |unit: u64| unit.checked_mul(u64::from(quantity));
    Some(shared::protocol::CreateAuction {
        item_guid: item.item_guid,
        stack_count: quantity,
        min_bid: total(bid_unit)?,
        buyout_price: if buyout_unit == 0 {
            None
        } else {
            Some(total(buyout_unit)?)
        },
        duration: inputs.ui.duration,
    })
}

fn sell_view(inputs: &ViewInputs) -> SellView {
    let item = sell_item(inputs);
    let quantity = quantity_input(inputs.texts);
    let buyout = money_input(inputs.texts, SELL_BUYOUT_BOXES);
    let bid = money_input(inputs.texts, SELL_BID_BOXES);
    let unit = if buyout > 0 || inputs.ui.buyout_mode {
        buyout
    } else {
        bid
    };
    let inventory = inputs
        .net
        .inventory
        .as_ref()
        .map(|inventory| inventory.items.as_slice())
        .unwrap_or_default();
    SellView {
        item: item.map(|item| SellItemView {
            item: inputs.item_line(item),
            count: item.stack_count,
        }),
        inventory: inventory
            .iter()
            .map(|item| SellInventoryRow {
                item_guid: item.item_guid,
                item: inputs.item_line(item),
                count: item.stack_count,
                selected: inputs.ui.sell_item == Some(item.item_guid),
            })
            .collect(),
        listings: item
            .map(|item| {
                inputs
                    .net
                    .search_results
                    .iter()
                    .filter(|listing| listing.item.item_id == item.item_id)
                    .map(|listing| inputs.listing_row(listing))
                    .collect()
            })
            .unwrap_or_default(),
        buyout_mode: inputs.ui.buyout_mode,
        duration: inputs.ui.duration,
        duration_menu_open: inputs.ui.duration_menu_open,
        deposit: item.map_or(0, |item| deposit(item, quantity, inputs.ui.duration)),
        total: unit.saturating_mul(u64::from(quantity)),
        can_post: sell_request(inputs).is_some(),
    }
}

fn auctions_view(inputs: &ViewInputs) -> AuctionsView {
    let listings = match inputs.ui.auctions_tab {
        AuctionsSubTab::Auctions => &inputs.net.owned_results,
        AuctionsSubTab::Bids => &inputs.net.bid_results,
    };
    let selected = selected_listing(inputs, listings);
    AuctionsView {
        tab: inputs.ui.auctions_tab,
        rows: listings
            .iter()
            .map(|listing| inputs.listing_row(listing))
            .collect(),
        // The server refuses to cancel an auction that has bids.
        can_cancel: inputs.ui.auctions_tab == AuctionsSubTab::Auctions
            && selected.is_some_and(|listing| listing.current_bid.is_none()),
        can_bid: can_bid(inputs, selected),
        can_buyout: can_buyout(inputs, selected),
    }
}
