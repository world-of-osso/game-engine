use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use shared::protocol::{
    AuctionChannel, AuctionHouseOpened, AuctionInventoryItem, AuctionInventorySnapshot,
    AuctionListingSummary, AuctionOperationResponse, AuctionSearchQuery, AuctionSearchResults,
    BidAuctionListResponse, BuyoutAuction, CancelAuction, CreateAuction, OpenAuctionHouse,
    OwnedAuctionListResponse, PlaceBid, QueryAuctionInventory, QueryAuctions, QueryBidAuctions,
    QueryOwnedAuctions,
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
    /// Failed operation messages for the UI error frame; the frame drains them.
    pub errors: Vec<String>,
    pending_actions: VecDeque<PendingAction>,
    pending_replies: VecDeque<PendingReply>,
}

/// One auction request to the server. The frame queues these with
/// [`AuctionHouseState::request`]; IPC queues them with a reply channel.
#[derive(Clone, Debug, PartialEq)]
pub enum AuctionRequest {
    Open,
    Browse(AuctionSearchQuery),
    Owned,
    Bids,
    Inventory,
    Create(CreateAuction),
    Bid(PlaceBid),
    Buyout(BuyoutAuction),
    Cancel(CancelAuction),
}

struct PendingAction {
    action: AuctionRequest,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplyKind {
    Open,
    Browse,
    Owned,
    Bids,
    Inventory,
    Operation,
}

/// Responses arrive in request order per kind; a frame request holds its slot
/// without a reply channel so IPC replies stay paired with their own requests.
struct PendingReply {
    kind: ReplyKind,
    respond: Option<mpsc::Sender<Response>>,
}

impl PendingReply {
    fn send(self, response: Response) {
        if let Some(respond) = self.respond {
            let _ = respond.send(response);
        }
    }
}

impl AuctionHouseState {
    /// Queues a frame request; its response only updates this state.
    pub fn request(&mut self, request: AuctionRequest) {
        let kind = reply_kind(&request);
        self.pending_actions
            .push_back(PendingAction { action: request });
        self.pending_replies.push_back(PendingReply {
            kind,
            respond: None,
        });
    }

    /// The frame closed: its results are stale by the next opening.
    pub fn close(&mut self) {
        let errors = std::mem::take(&mut self.errors);
        *self = Self {
            pending_actions: std::mem::take(&mut self.pending_actions),
            pending_replies: std::mem::take(&mut self.pending_replies),
            errors,
            ..Default::default()
        };
    }

    /// Everything the frame shows after an auction changed hands or was posted.
    fn refresh(&mut self) {
        self.request(AuctionRequest::Inventory);
        self.request(AuctionRequest::Owned);
        self.request(AuctionRequest::Bids);
        if let Some(query) = self.last_query.clone() {
            self.request(AuctionRequest::Browse(query));
        }
    }
}

fn reply_kind(request: &AuctionRequest) -> ReplyKind {
    match request {
        AuctionRequest::Open => ReplyKind::Open,
        AuctionRequest::Browse(_) => ReplyKind::Browse,
        AuctionRequest::Owned => ReplyKind::Owned,
        AuctionRequest::Bids => ReplyKind::Bids,
        AuctionRequest::Inventory => ReplyKind::Inventory,
        AuctionRequest::Create(_)
        | AuctionRequest::Bid(_)
        | AuctionRequest::Buyout(_)
        | AuctionRequest::Cancel(_) => ReplyKind::Operation,
    }
}

pub struct AuctionHousePlugin;

impl Plugin for AuctionHousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AuctionHouseState>();
        use crate::network_events::{register_message_handler, register_outgoing_handler};
        register_outgoing_handler(app, send_pending_actions, |world| {
            !world
                .resource::<AuctionHouseState>()
                .pending_actions
                .is_empty()
        });
        register_message_handler::<AuctionHouseOpened, _>(app, receive_opened, |_| true);
        register_message_handler::<AuctionSearchResults, _>(app, receive_search_results, |_| true);
        register_message_handler::<OwnedAuctionListResponse, _>(app, receive_owned_results, |_| {
            true
        });
        register_message_handler::<BidAuctionListResponse, _>(app, receive_bid_results, |_| true);
        register_message_handler::<AuctionInventorySnapshot, _>(
            app,
            receive_inventory_snapshot,
            |_| true,
        );
        register_message_handler::<AuctionOperationResponse, _>(
            app,
            receive_operation_response,
            |_| true,
        );
    }
}

pub fn queue_ipc_request(
    state: &mut AuctionHouseState,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    if handle_auction_status_request(state, request, &respond) {
        return true;
    }
    let Some(action) = auction_ipc_action(request) else {
        return false;
    };
    state.pending_replies.push_back(PendingReply {
        kind: reply_kind(&action),
        respond: Some(respond),
    });
    state.pending_actions.push_back(PendingAction { action });
    true
}

fn handle_auction_status_request(
    state: &AuctionHouseState,
    request: &Request,
    respond: &mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::AuctionStatus => {
            let _ = respond.send(Response::Text(format_status(state)));
            true
        }
        _ => false,
    }
}

fn auction_ipc_action(request: &Request) -> Option<AuctionRequest> {
    use AuctionRequest as A;
    Some(match request {
        Request::AuctionOpen => A::Open,
        Request::AuctionBrowse { query } => A::Browse(query.clone()),
        Request::AuctionOwned => A::Owned,
        Request::AuctionBids => A::Bids,
        Request::AuctionInventory => A::Inventory,
        Request::AuctionCreate { create } => A::Create(create.clone()),
        Request::AuctionBid { bid } => A::Bid(bid.clone()),
        Request::AuctionBuyout { buyout } => A::Buyout(buyout.clone()),
        Request::AuctionCancel { cancel } => A::Cancel(cancel.clone()),
        _ => return None,
    })
}

#[derive(SystemParam)]
struct AuctionSenders<'w, 's> {
    open_senders: MessageSenders<'w, 's, OpenAuctionHouse>,
    browse_senders: MessageSenders<'w, 's, QueryAuctions>,
    owned_senders: MessageSenders<'w, 's, QueryOwnedAuctions>,
    bids_senders: MessageSenders<'w, 's, QueryBidAuctions>,
    inventory_senders: MessageSenders<'w, 's, QueryAuctionInventory>,
    create_senders: MessageSenders<'w, 's, CreateAuction>,
    bid_senders: MessageSenders<'w, 's, PlaceBid>,
    buyout_senders: MessageSenders<'w, 's, BuyoutAuction>,
    cancel_senders: MessageSenders<'w, 's, CancelAuction>,
}

fn send_pending_actions(mut state: ResMut<AuctionHouseState>, mut senders: AuctionSenders) {
    while let Some(pending) = state.pending_actions.pop_front() {
        let kind = reply_kind(&pending.action);
        let sent = match pending.action {
            AuctionRequest::Open => send_all(&mut senders.open_senders, OpenAuctionHouse),
            AuctionRequest::Browse(query) => {
                send_all(&mut senders.browse_senders, QueryAuctions { query })
            }
            AuctionRequest::Owned => send_all(&mut senders.owned_senders, QueryOwnedAuctions),
            AuctionRequest::Bids => send_all(&mut senders.bids_senders, QueryBidAuctions),
            AuctionRequest::Inventory => {
                send_all(&mut senders.inventory_senders, QueryAuctionInventory)
            }
            AuctionRequest::Create(req) => send_all(&mut senders.create_senders, req),
            AuctionRequest::Bid(req) => send_all(&mut senders.bid_senders, req),
            AuctionRequest::Buyout(req) => send_all(&mut senders.buyout_senders, req),
            AuctionRequest::Cancel(req) => send_all(&mut senders.cancel_senders, req),
        };
        if !sent {
            state.last_error = Some(NOT_CONNECTED.into());
            state.errors.push(NOT_CONNECTED.into());
            if let Some(reply) = pop_reply(&mut state, kind) {
                reply.send(Response::Error(NOT_CONNECTED.into()));
            }
        }
    }
}

const NOT_CONNECTED: &str = "auction house is unavailable: not connected";

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
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
    mut receivers: MessageReceivers<AuctionHouseOpened>,
    mut state: ResMut<AuctionHouseState>,
) {
    for receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            apply_opened_response(&mut state, response);
        }
    }
}

fn apply_opened_response(state: &mut AuctionHouseState, response: AuctionHouseOpened) {
    let opening = response.success && !state.is_open;
    state.is_open = response.success;
    state.last_error = response.error.clone();
    if opening {
        state.refresh();
    }
    let Some(reply) = pop_reply(state, ReplyKind::Open) else {
        return;
    };
    reply.send(opened_response_to_ipc(response.success, response.error));
}

fn opened_response_to_ipc(success: bool, error: Option<String>) -> Response {
    let message = if success {
        "auction house opened".to_string()
    } else {
        error.unwrap_or_else(|| "failed to open auction house".into())
    };
    if success {
        Response::Text(message)
    } else {
        Response::Error(message)
    }
}

fn receive_search_results(
    mut receivers: MessageReceivers<AuctionSearchResults>,
    mut state: ResMut<AuctionHouseState>,
) {
    for receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            state.last_query = Some(response.query.clone());
            state.search_total = response.total_results;
            state.search_results = response.results;
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Browse) {
                reply.send(Response::Text(format_search_results(&state)));
            }
        }
    }
}

fn receive_owned_results(
    mut receivers: MessageReceivers<OwnedAuctionListResponse>,
    mut state: ResMut<AuctionHouseState>,
) {
    for receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            state.owned_results = response.listings;
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Owned) {
                reply.send(Response::Text(format_listing_block(
                    "owned auctions",
                    &state.owned_results,
                )));
            }
        }
    }
}

fn receive_bid_results(
    mut receivers: MessageReceivers<BidAuctionListResponse>,
    mut state: ResMut<AuctionHouseState>,
) {
    for receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            state.bid_results = response.listings;
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Bids) {
                reply.send(Response::Text(format_listing_block(
                    "bid auctions",
                    &state.bid_results,
                )));
            }
        }
    }
}

fn receive_inventory_snapshot(
    mut receivers: MessageReceivers<AuctionInventorySnapshot>,
    mut state: ResMut<AuctionHouseState>,
) {
    for receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            state.inventory = Some(response);
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Inventory) {
                reply.send(Response::Text(format_inventory(&state)));
            }
        }
    }
}

fn receive_operation_response(
    mut receivers: MessageReceivers<AuctionOperationResponse>,
    mut state: ResMut<AuctionHouseState>,
) {
    for receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            state.last_message = Some(response.message.clone());
            if !response.success {
                state.last_error = Some(response.message.clone());
            }
            if state.is_open {
                if response.success {
                    state.refresh();
                } else {
                    state.errors.push(response.message.clone());
                }
            }
            if let Some(reply) = pop_reply(&mut state, ReplyKind::Operation) {
                let out = if response.success {
                    Response::Text(response.message)
                } else {
                    Response::Error(response.message)
                };
                reply.send(out);
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
        "open: {}\nsearch_total: {}\nowned: {}\nbids: {}\ninventory_loaded: {}\nlast_error: {}\nlast_message: {}",
        state.is_open,
        state.search_total,
        state.owned_results.len(),
        state.bid_results.len(),
        state.inventory.is_some(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{AuctionSortDir, AuctionSortField, AuctionTimeLeft};

    #[test]
    fn queued_request_without_connection_reports_failure_once() {
        let mut app = App::new();
        app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
        app.add_plugins(AuctionHousePlugin);
        let (respond, replies) = mpsc::channel();
        queue_ipc_request(
            &mut app.world_mut().resource_mut::<AuctionHouseState>(),
            &Request::AuctionOpen,
            respond,
        );
        crate::network_events::dispatch_outgoing(app.world_mut());
        assert!(matches!(replies.try_recv().unwrap(), Response::Error(_)));
        crate::network_events::dispatch_outgoing(app.world_mut());
        assert!(replies.try_recv().is_err());
    }

    #[test]
    fn auction_status_request_returns_immediate_snapshot() {
        let (tx, rx) = mpsc::channel();
        let mut state = AuctionHouseState {
            is_open: true,
            search_total: 3,
            ..Default::default()
        };

        let handled = queue_ipc_request(&mut state, &Request::AuctionStatus, tx);

        assert!(handled);
        let Response::Text(text) = rx.recv().expect("response") else {
            panic!("expected text response");
        };
        assert!(text.contains("open: true"));
        assert!(text.contains("search_total: 3"));
        assert!(state.pending_actions.is_empty());
    }

    #[test]
    fn browse_request_enqueues_network_action_and_reply_slot() {
        let (tx, _rx) = mpsc::channel();
        let mut state = AuctionHouseState::default();
        let query = AuctionSearchQuery {
            item_id: None,
            class_id: None,
            text: "linen".into(),
            page: 1,
            page_size: 20,
            min_level: None,
            max_level: None,
            quality: Some(1),
            usable_only: false,
            sort_field: AuctionSortField::Name,
            sort_dir: AuctionSortDir::Asc,
            faction: 0,
        };

        let handled = queue_ipc_request(
            &mut state,
            &Request::AuctionBrowse {
                query: query.clone(),
            },
            tx,
        );

        assert!(handled);
        assert_eq!(state.pending_actions.len(), 1);
        assert_eq!(state.pending_replies.len(), 1);
        match &state.pending_actions[0].action {
            AuctionRequest::Browse(queued) => assert_eq!(queued, &query),
            _ => panic!("expected browse action"),
        }
    }

    fn query(text: &str) -> AuctionSearchQuery {
        AuctionSearchQuery {
            item_id: None,
            class_id: None,
            text: text.into(),
            page: 0,
            page_size: 50,
            min_level: None,
            max_level: None,
            quality: None,
            usable_only: false,
            sort_field: AuctionSortField::Name,
            sort_dir: AuctionSortDir::Asc,
            faction: 0,
        }
    }

    fn plugin_app() -> App {
        let mut app = App::new();
        app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
        app.add_plugins(AuctionHousePlugin);
        app
    }

    fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
        app.insert_resource(game_engine::network_runtime::messages::Inbox::new(messages));
        crate::network_events::dispatch_incoming(app.world_mut());
    }

    #[test]
    fn frame_request_response_leaves_ipc_reply_for_its_own_request() {
        let mut app = plugin_app();
        let (respond, replies) = mpsc::channel();
        {
            let mut state = app.world_mut().resource_mut::<AuctionHouseState>();
            state.request(AuctionRequest::Browse(query("linen")));
            queue_ipc_request(
                &mut state,
                &Request::AuctionBrowse {
                    query: query("copper"),
                },
                respond,
            );
        }

        deliver(
            &mut app,
            vec![
                AuctionSearchResults {
                    query: query("linen"),
                    total_results: 0,
                    results: vec![],
                },
                AuctionSearchResults {
                    query: query("copper"),
                    total_results: 0,
                    results: vec![],
                },
            ],
        );

        let Response::Text(text) = replies.try_recv().expect("ipc reply") else {
            panic!("expected text reply");
        };
        assert!(text.contains("text=copper"), "{text}");
        assert!(replies.try_recv().is_err());
    }

    #[test]
    fn failed_operation_while_open_reaches_the_error_frame() {
        let mut app = plugin_app();
        app.world_mut().resource_mut::<AuctionHouseState>().is_open = true;

        deliver(
            &mut app,
            vec![AuctionOperationResponse {
                success: false,
                message: "not enough gold for auction deposit".into(),
            }],
        );

        let state = app.world().resource::<AuctionHouseState>();
        assert_eq!(state.errors, ["not enough gold for auction deposit"]);
    }

    #[test]
    fn format_search_results_includes_listing_data() {
        let state = AuctionHouseState {
            last_query: Some(AuctionSearchQuery {
                item_id: None,
                class_id: None,
                text: "linen".into(),
                page: 0,
                page_size: 10,
                min_level: None,
                max_level: None,
                quality: None,
                usable_only: false,
                sort_field: AuctionSortField::Name,
                sort_dir: AuctionSortDir::Asc,
                faction: 0,
            }),
            search_total: 1,
            search_results: vec![AuctionListingSummary {
                auction_id: 42,
                item: AuctionInventoryItem {
                    item_guid: 10,
                    item_id: 2589,
                    name: "Linen Cloth".into(),
                    quality: 1,
                    required_level: 1,
                    stack_count: 5,
                    vendor_sell_price: 13,
                },
                owner_name: "Seller".into(),
                stack_count: 5,
                min_bid: 100,
                current_bid: Some(110),
                min_next_bid: 115,
                buyout_price: Some(150),
                time_left: AuctionTimeLeft::Medium,
            }],
            ..Default::default()
        };

        let text = format_search_results(&state);

        assert!(text.contains("search page=0 size=10 total=1 text=linen"));
        assert!(text.contains("#42 Linen Cloth x5 owner=Seller bid=110 next=115 buyout=150"));
    }

    #[test]
    fn opened_response_success_returns_text() {
        let response = opened_response_to_ipc(true, None);

        match response {
            Response::Text(message) => assert_eq!(message, "auction house opened"),
            _ => panic!("expected text response"),
        }
    }

    #[test]
    fn opened_response_failure_prefers_error_message() {
        let response = opened_response_to_ipc(false, Some("no auctioneer".into()));

        match response {
            Response::Error(message) => assert_eq!(message, "no auctioneer"),
            _ => panic!("expected error response"),
        }
    }

    #[test]
    fn opened_response_failure_uses_fallback_message() {
        let response = opened_response_to_ipc(false, None);

        match response {
            Response::Error(message) => assert_eq!(message, "failed to open auction house"),
            _ => panic!("expected error response"),
        }
    }
}
