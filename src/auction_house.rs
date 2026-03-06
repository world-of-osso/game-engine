use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuctionChannel, AuctionHouseOpened, AuctionInventoryItem, AuctionInventorySnapshot,
    AuctionListingSummary, AuctionMailboxSnapshot, AuctionOperationResponse, AuctionSearchQuery,
    AuctionSearchResults, BidAuctionListResponse, BuyoutAuction, CancelAuction, ClaimAuctionMail,
    CreateAuction, OpenAuctionHouse, OwnedAuctionListResponse, PlaceBid, QueryAuctionInventory,
    QueryAuctionMailbox, QueryAuctions, QueryBidAuctions, QueryOwnedAuctions,
};

use crate::ipc::{Request, Response};

#[derive(Resource, Default)]
pub struct AuctionHouseState {
    pub is_open: bool,
    pub last_error: Option<String>,
    pub last_message: Option<String>,
    pub last_query: Option<AuctionSearchQuery>,
    pub search_total: u32,
    pub search_results: Vec<AuctionListingSummary>,
    pub owned_results: Vec<AuctionListingSummary>,
    pub bid_results: Vec<AuctionListingSummary>,
    pub inventory: Option<AuctionInventorySnapshot>,
    pub mailbox: Vec<shared::protocol::AuctionMailEntry>,
    pending_actions: VecDeque<PendingAction>,
    pending_replies: VecDeque<PendingReply>,
}

#[derive(Clone)]
enum Action {
    Open,
    Browse(AuctionSearchQuery),
    Owned,
    Bids,
    Inventory,
    Mailbox,
    Create(CreateAuction),
    Bid(PlaceBid),
    Buyout(BuyoutAuction),
    Cancel(CancelAuction),
    Claim(ClaimAuctionMail),
}

struct PendingAction {
    action: Action,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplyKind {
    Open,
    Browse,
    Owned,
    Bids,
    Inventory,
    Mailbox,
    Operation,
}

struct PendingReply {
    kind: ReplyKind,
    respond: mpsc::Sender<Response>,
}

pub struct AuctionHousePlugin;

impl Plugin for AuctionHousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AuctionHouseState>();
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_opened);
        app.add_systems(Update, receive_search_results);
        app.add_systems(Update, receive_owned_results);
        app.add_systems(Update, receive_bid_results);
        app.add_systems(Update, receive_inventory_snapshot);
        app.add_systems(Update, receive_mailbox_snapshot);
        app.add_systems(Update, receive_operation_response);
    }
}

pub fn queue_ipc_request(
    state: &mut AuctionHouseState,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::AuctionStatus => {
            let _ = respond.send(Response::Text(format_status(state)));
            true
        }
        Request::AuctionOpen => push(state, Action::Open, ReplyKind::Open, respond),
        Request::AuctionBrowse { query } => push(
            state,
            Action::Browse(query.clone()),
            ReplyKind::Browse,
            respond,
        ),
        Request::AuctionOwned => push(state, Action::Owned, ReplyKind::Owned, respond),
        Request::AuctionBids => push(state, Action::Bids, ReplyKind::Bids, respond),
        Request::AuctionInventory => push(state, Action::Inventory, ReplyKind::Inventory, respond),
        Request::AuctionMailbox => push(state, Action::Mailbox, ReplyKind::Mailbox, respond),
        Request::AuctionCreate { create } => push(
            state,
            Action::Create(create.clone()),
            ReplyKind::Operation,
            respond,
        ),
        Request::AuctionBid { bid } => push(
            state,
            Action::Bid(bid.clone()),
            ReplyKind::Operation,
            respond,
        ),
        Request::AuctionBuyout { buyout } => push(
            state,
            Action::Buyout(buyout.clone()),
            ReplyKind::Operation,
            respond,
        ),
        Request::AuctionCancel { cancel } => push(
            state,
            Action::Cancel(cancel.clone()),
            ReplyKind::Operation,
            respond,
        ),
        Request::AuctionClaimMail { claim } => push(
            state,
            Action::Claim(claim.clone()),
            ReplyKind::Operation,
            respond,
        ),
        _ => false,
    }
}

fn push(
    state: &mut AuctionHouseState,
    action: Action,
    kind: ReplyKind,
    respond: mpsc::Sender<Response>,
) -> bool {
    state.pending_actions.push_back(PendingAction { action });
    state
        .pending_replies
        .push_back(PendingReply { kind, respond });
    true
}

#[allow(clippy::too_many_arguments)]
fn send_pending_actions(
    mut state: ResMut<AuctionHouseState>,
    mut open_senders: Query<&mut MessageSender<OpenAuctionHouse>>,
    mut browse_senders: Query<&mut MessageSender<QueryAuctions>>,
    mut owned_senders: Query<&mut MessageSender<QueryOwnedAuctions>>,
    mut bids_senders: Query<&mut MessageSender<QueryBidAuctions>>,
    mut inventory_senders: Query<&mut MessageSender<QueryAuctionInventory>>,
    mut mailbox_senders: Query<&mut MessageSender<QueryAuctionMailbox>>,
    mut create_senders: Query<&mut MessageSender<CreateAuction>>,
    mut bid_senders: Query<&mut MessageSender<PlaceBid>>,
    mut buyout_senders: Query<&mut MessageSender<BuyoutAuction>>,
    mut cancel_senders: Query<&mut MessageSender<CancelAuction>>,
    mut claim_senders: Query<&mut MessageSender<ClaimAuctionMail>>,
) {
    while let Some(pending) = state.pending_actions.pop_front() {
        let sent = match pending.action {
            Action::Open => send_all(&mut open_senders, OpenAuctionHouse),
            Action::Browse(query) => send_all(&mut browse_senders, QueryAuctions { query }),
            Action::Owned => send_all(&mut owned_senders, QueryOwnedAuctions),
            Action::Bids => send_all(&mut bids_senders, QueryBidAuctions),
            Action::Inventory => send_all(&mut inventory_senders, QueryAuctionInventory),
            Action::Mailbox => send_all(&mut mailbox_senders, QueryAuctionMailbox),
            Action::Create(req) => send_all(&mut create_senders, req),
            Action::Bid(req) => send_all(&mut bid_senders, req),
            Action::Buyout(req) => send_all(&mut buyout_senders, req),
            Action::Cancel(req) => send_all(&mut cancel_senders, req),
            Action::Claim(req) => send_all(&mut claim_senders, req),
        };
        if !sent {
            state.last_error = Some("auction house is unavailable: not connected".into());
            if let Some(reply) = state.pending_replies.pop_front() {
                let _ = reply.respond.send(Response::Error(
                    "auction house is unavailable: not connected".into(),
                ));
            }
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<AuctionChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_opened(
    mut receivers: Query<&mut MessageReceiver<AuctionHouseOpened>>,
    mut state: ResMut<AuctionHouseState>,
) {
    for mut receiver in &mut receivers {
        for response in receiver.receive() {
            state.is_open = response.success;
            state.last_error = response.error.clone();
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Open) {
                let message = if response.success {
                    "auction house opened".to_string()
                } else {
                    response
                        .error
                        .unwrap_or_else(|| "failed to open auction house".into())
                };
                let out = if state.is_open {
                    Response::Text(message)
                } else {
                    Response::Error(message)
                };
                let _ = reply.respond.send(out);
            }
        }
    }
}

fn receive_search_results(
    mut receivers: Query<&mut MessageReceiver<AuctionSearchResults>>,
    mut state: ResMut<AuctionHouseState>,
) {
    for mut receiver in &mut receivers {
        for response in receiver.receive() {
            state.last_query = Some(response.query.clone());
            state.search_total = response.total_results;
            state.search_results = response.results;
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Browse) {
                let _ = reply
                    .respond
                    .send(Response::Text(format_search_results(&state)));
            }
        }
    }
}

fn receive_owned_results(
    mut receivers: Query<&mut MessageReceiver<OwnedAuctionListResponse>>,
    mut state: ResMut<AuctionHouseState>,
) {
    for mut receiver in &mut receivers {
        for response in receiver.receive() {
            state.owned_results = response.listings;
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Owned) {
                let _ = reply.respond.send(Response::Text(format_listing_block(
                    "owned auctions",
                    &state.owned_results,
                )));
            }
        }
    }
}

fn receive_bid_results(
    mut receivers: Query<&mut MessageReceiver<BidAuctionListResponse>>,
    mut state: ResMut<AuctionHouseState>,
) {
    for mut receiver in &mut receivers {
        for response in receiver.receive() {
            state.bid_results = response.listings;
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Bids) {
                let _ = reply.respond.send(Response::Text(format_listing_block(
                    "bid auctions",
                    &state.bid_results,
                )));
            }
        }
    }
}

fn receive_inventory_snapshot(
    mut receivers: Query<&mut MessageReceiver<AuctionInventorySnapshot>>,
    mut state: ResMut<AuctionHouseState>,
) {
    for mut receiver in &mut receivers {
        for response in receiver.receive() {
            state.inventory = Some(response);
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Inventory) {
                let _ = reply.respond.send(Response::Text(format_inventory(&state)));
            }
        }
    }
}

fn receive_mailbox_snapshot(
    mut receivers: Query<&mut MessageReceiver<AuctionMailboxSnapshot>>,
    mut state: ResMut<AuctionHouseState>,
) {
    for mut receiver in &mut receivers {
        for response in receiver.receive() {
            state.mailbox = response.entries;
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Mailbox) {
                let _ = reply.respond.send(Response::Text(format_mailbox(&state)));
            }
        }
    }
}

fn receive_operation_response(
    mut receivers: Query<&mut MessageReceiver<AuctionOperationResponse>>,
    mut state: ResMut<AuctionHouseState>,
) {
    for mut receiver in &mut receivers {
        for response in receiver.receive() {
            state.last_message = Some(response.message.clone());
            if !response.success {
                state.last_error = Some(response.message.clone());
            }
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Operation) {
                let out = if response.success {
                    Response::Text(response.message)
                } else {
                    Response::Error(response.message)
                };
                let _ = reply.respond.send(out);
            }
        }
    }
}

fn pop_reply(state: &mut AuctionHouseState, kind: ReplyKind) -> Option<PendingReply> {
    let index = state
        .pending_replies
        .iter()
        .position(|reply| reply.kind == kind)?;
    state.pending_replies.remove(index)
}

fn format_status(state: &AuctionHouseState) -> String {
    format!(
        "open: {}\nsearch_total: {}\nowned: {}\nbids: {}\ninventory_loaded: {}\nmailbox_entries: {}\nlast_error: {}\nlast_message: {}",
        state.is_open,
        state.search_total,
        state.owned_results.len(),
        state.bid_results.len(),
        state.inventory.is_some(),
        state.mailbox.len(),
        state.last_error.clone().unwrap_or_else(|| "-".into()),
        state.last_message.clone().unwrap_or_else(|| "-".into()),
    )
}

fn format_search_results(state: &AuctionHouseState) -> String {
    let header = if let Some(query) = &state.last_query {
        format!(
            "search page={} size={} total={} text={}",
            query.page, query.page_size, state.search_total, query.text
        )
    } else {
        format!("search total={}", state.search_total)
    };
    format!("{header}\n{}", listing_lines(&state.search_results))
}

fn format_listing_block(title: &str, listings: &[AuctionListingSummary]) -> String {
    format!("{title}: {}\n{}", listings.len(), listing_lines(listings))
}

fn listing_lines(listings: &[AuctionListingSummary]) -> String {
    if listings.is_empty() {
        return "-".into();
    }
    listings
        .iter()
        .map(|listing| {
            format!(
                "#{id} {name} x{count} owner={owner} bid={bid} next={next} buyout={buyout}",
                id = listing.auction_id,
                name = listing.item.name,
                count = listing.stack_count,
                owner = listing.owner_name,
                bid = listing.current_bid.unwrap_or(listing.min_bid),
                next = listing.min_next_bid,
                buyout = listing
                    .buyout_price
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".into()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_inventory(state: &AuctionHouseState) -> String {
    let Some(inventory) = &state.inventory else {
        return "inventory unavailable".into();
    };
    let lines = if inventory.items.is_empty() {
        "-".into()
    } else {
        inventory
            .items
            .iter()
            .map(format_inventory_item)
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!("gold: {}\n{}", inventory.gold, lines)
}

fn format_inventory_item(item: &AuctionInventoryItem) -> String {
    format!(
        "{} {} x{} q{} lvl{} vendor={}",
        item.item_guid,
        item.name,
        item.stack_count,
        item.quality,
        item.required_level,
        item.vendor_sell_price
    )
}

fn format_mailbox(state: &AuctionHouseState) -> String {
    if state.mailbox.is_empty() {
        return "mailbox: 0\n-".into();
    }
    let lines = state
        .mailbox
        .iter()
        .map(|entry| {
            let item = entry
                .attached_item
                .as_ref()
                .map(|item| format!(" item={}x{}", item.name, item.stack_count))
                .unwrap_or_default();
            format!(
                "{} {} money={}{}",
                entry.mail_id, entry.subject, entry.attached_money, item
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("mailbox: {}\n{}", state.mailbox.len(), lines)
}
