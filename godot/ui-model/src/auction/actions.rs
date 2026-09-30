//! Frame click actions: selection changes on [`AuctionHouseUi`], server requests on
//! [`AuctionHouseState`], and edit box texts to set.

use super::{AuctionHouseState, AuctionRequest};
use crate::auction_house_frame_component::{
    self as frame, AuctionHouseTab, AuctionsSubTab, BID_BOXES, MoneyBoxes, QUANTITY_BOX,
    SEARCH_BOX, SELL_BID_BOXES, SELL_BUYOUT_BOXES,
};
use shared::protocol::{
    AuctionDuration, AuctionSearchQuery, AuctionSortDir, AuctionSortField, BuyoutAuction,
    CancelAuction, PlaceBid,
};

use super::AuctionHouseUi;
use super::view::{self, InputTexts, ViewInputs};

/// An edit box text the frame sets (prefill, clear).
pub type InputEdit = (&'static str, String);

/// Retail `AUCTION_HOUSE_BROWSE_MAX_RESULTS` is 500; the server pages at most 50.
const SEARCH_PAGE_SIZE: u32 = 50;

fn search_query(text: &str) -> AuctionSearchQuery {
    AuctionSearchQuery {
        item_id: None,
        class_id: None,
        text: text.trim().to_string(),
        page: 0,
        page_size: SEARCH_PAGE_SIZE,
        min_level: None,
        max_level: None,
        quality: None,
        usable_only: false,
        sort_field: AuctionSortField::Name,
        sort_dir: AuctionSortDir::Asc,
        // The server searches the auctioneer's house.
        faction: 0,
    }
}

fn money_edits(boxes: MoneyBoxes, copper: Option<u64>) -> Vec<InputEdit> {
    let (gold, silver, copper) = match copper {
        Some(copper) => (
            (copper / 10_000).to_string(),
            ((copper / 100) % 100).to_string(),
            (copper % 100).to_string(),
        ),
        None => Default::default(),
    };
    vec![
        (boxes.gold, gold),
        (boxes.silver, silver),
        (boxes.copper, copper),
    ]
}

/// Applies one frame action; returns the edit box texts it sets.
pub fn dispatch(
    action: &str,
    net: &mut AuctionHouseState,
    ui: &mut AuctionHouseUi,
    texts: &InputTexts,
) -> Vec<InputEdit> {
    if let Some(tab) = action
        .strip_prefix(frame::ACTION_TAB_PREFIX)
        .and_then(AuctionHouseTab::from_token)
    {
        ui.tab = tab;
        ui.selected_auction = None;
        ui.duration_menu_open = false;
        return Vec::new();
    }
    if let Some(index) = parse(action, frame::ACTION_CATEGORY_PREFIX) {
        let index = index as usize;
        if index >= view::CATEGORIES.len() {
            return Vec::new();
        }
        ui.category = (ui.category != Some(index)).then_some(index);
        ui.browse_item = None;
        ui.row_page = 0;
        let mut query = search_query(view::text(texts, SEARCH_BOX));
        query.class_id = ui.category.map(|index| view::CATEGORIES[index].1);
        net.request(AuctionRequest::Browse(query));
        return Vec::new();
    }
    if let Some(item_id) = parse(action, frame::ACTION_BROWSE_ITEM_PREFIX) {
        ui.browse_query = net.last_query.clone();
        ui.browse_item = Some(item_id as u32);
        ui.selected_auction = None;
        let mut query = search_query("");
        query.item_id = Some(item_id as u32);
        net.request(AuctionRequest::Browse(query));
        return money_edits(BID_BOXES, None);
    }
    if let Some(auction_id) = parse(action, frame::ACTION_SELECT_AUCTION_PREFIX) {
        return select_auction(net, ui, auction_id);
    }
    if let Some(guid) = parse(action, frame::ACTION_SELL_ITEM_PREFIX) {
        return select_sell_item(net, ui, guid);
    }
    if let Some(duration) = action
        .strip_prefix(frame::ACTION_DURATION_PREFIX)
        .and_then(parse_duration)
    {
        ui.duration = duration;
        ui.duration_menu_open = false;
        return Vec::new();
    }
    dispatch_command(action, net, ui, texts)
}

fn dispatch_command(
    action: &str,
    net: &mut AuctionHouseState,
    ui: &mut AuctionHouseUi,
    texts: &InputTexts,
) -> Vec<InputEdit> {
    match action {
        frame::ACTION_CLOSE => ui.close_requested = true,
        frame::ACTION_SEARCH => {
            ui.browse_item = None;
            ui.selected_auction = None;
            let mut query = search_query(view::text(texts, SEARCH_BOX));
            query.class_id = ui.category.map(|index| view::CATEGORIES[index].1);
            net.request(AuctionRequest::Browse(query));
        }
        frame::ACTION_BACK => {
            if let Some(query) = ui.browse_query.take() {
                net.request(AuctionRequest::Browse(query));
            }
            ui.browse_item = None;
            ui.selected_auction = None;
        }
        frame::ACTION_BID => return place_bid(net, ui, texts),
        frame::ACTION_BUYOUT => ui.dialog_auction = ui.selected_auction,
        frame::ACTION_DIALOG_BUY => {
            if let Some(auction_id) = ui.dialog_auction.take() {
                net.request(AuctionRequest::Buyout(BuyoutAuction { auction_id }));
                ui.selected_auction = None;
            }
        }
        frame::ACTION_DIALOG_CANCEL => ui.dialog_auction = None,
        frame::ACTION_SELL_CLEAR => return clear_sell_item(ui),
        frame::ACTION_MAX_QUANTITY => return max_quantity(net, ui),
        frame::ACTION_BUYOUT_MODE => {
            ui.buyout_mode = !ui.buyout_mode;
            return money_edits(SELL_BID_BOXES, None);
        }
        frame::ACTION_DURATION_MENU => ui.duration_menu_open = !ui.duration_menu_open,
        frame::ACTION_POST => return post(net, ui, texts),
        frame::ACTION_CANCEL_AUCTION => {
            if let Some(auction_id) = ui.selected_auction.take() {
                net.request(AuctionRequest::Cancel(CancelAuction { auction_id }));
            }
        }
        _ => {
            if let Some(tab) = action
                .strip_prefix(frame::ACTION_AUCTIONS_TAB_PREFIX)
                .and_then(AuctionsSubTab::from_token)
            {
                ui.auctions_tab = tab;
                ui.selected_auction = None;
            }
        }
    }
    Vec::new()
}

fn parse(action: &str, prefix: &str) -> Option<u64> {
    action.strip_prefix(prefix)?.parse().ok()
}

fn parse_duration(token: &str) -> Option<AuctionDuration> {
    [
        AuctionDuration::Short,
        AuctionDuration::Medium,
        AuctionDuration::Long,
    ]
    .into_iter()
    .find(|duration| frame::duration_token(*duration) == token)
}

/// Selecting an auction prefills the bid with its next minimum bid (Retail `BidFrame`).
fn select_auction(
    net: &AuctionHouseState,
    ui: &mut AuctionHouseUi,
    auction_id: u64,
) -> Vec<InputEdit> {
    ui.selected_auction = Some(auction_id);
    let min_next = net
        .search_results
        .iter()
        .chain(&net.bid_results)
        .chain(&net.owned_results)
        .find(|listing| listing.auction_id == auction_id)
        .map(|listing| listing.min_next_bid);
    money_edits(BID_BOXES, min_next)
}

fn place_bid(
    net: &mut AuctionHouseState,
    ui: &mut AuctionHouseUi,
    texts: &InputTexts,
) -> Vec<InputEdit> {
    let Some(auction_id) = ui.selected_auction else {
        return Vec::new();
    };
    let amount = view::money_input(texts, BID_BOXES);
    net.request(AuctionRequest::Bid(PlaceBid { auction_id, amount }));
    ui.selected_auction = None;
    money_edits(BID_BOXES, None)
}

/// Putting an item in the sell slot: quantity 1, prices cleared, and a search for the
/// item's current auctions (the `ItemSellList`).
fn select_sell_item(
    net: &mut AuctionHouseState,
    ui: &mut AuctionHouseUi,
    guid: u64,
) -> Vec<InputEdit> {
    let Some(item) = net
        .inventory
        .as_ref()
        .and_then(|inventory| inventory.items.iter().find(|item| item.item_guid == guid))
    else {
        return Vec::new();
    };
    let item_id = item.item_id;
    ui.sell_item = Some(guid);
    let mut query = search_query("");
    query.item_id = Some(item_id);
    net.request(AuctionRequest::Browse(query));
    let mut edits = vec![(QUANTITY_BOX, "1".to_string())];
    edits.extend(money_edits(SELL_BUYOUT_BOXES, None));
    edits.extend(money_edits(SELL_BID_BOXES, None));
    edits
}

fn clear_sell_item(ui: &mut AuctionHouseUi) -> Vec<InputEdit> {
    ui.sell_item = None;
    let mut edits = vec![(QUANTITY_BOX, String::new())];
    edits.extend(money_edits(SELL_BUYOUT_BOXES, None));
    edits.extend(money_edits(SELL_BID_BOXES, None));
    edits
}

fn max_quantity(net: &AuctionHouseState, ui: &AuctionHouseUi) -> Vec<InputEdit> {
    let count = ui.sell_item.and_then(|guid| {
        net.inventory
            .as_ref()?
            .items
            .iter()
            .find(|item| item.item_guid == guid)
            .map(|item| item.stack_count)
    });
    count
        .map(|count| vec![(QUANTITY_BOX, count.to_string())])
        .unwrap_or_default()
}

/// `AuctionHouseSellFrameMixin:PostItem`: post, then clear the item.
fn post(
    net: &mut AuctionHouseState,
    ui: &mut AuctionHouseUi,
    texts: &InputTexts,
) -> Vec<InputEdit> {
    let request = view::sell_request(&ViewInputs {
        net,
        ui,
        texts,
        catalog: &|_| None,
        visible: true,
    });
    let Some(request) = request else {
        return Vec::new();
    };
    net.request(AuctionRequest::Create(request));
    clear_sell_item(ui)
}
