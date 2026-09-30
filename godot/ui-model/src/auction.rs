//! Native auction session: wire replies, actions and portable presentation.
mod actions;
pub mod view;
use crate::auction_house_frame_component::{AuctionHouseTab, AuctionsSubTab};
use shared::protocol::*;
pub struct AuctionHouseUi {
    /// Auctioneer whose interaction opened the frame (server entity bits).
    pub npc: Option<u64>,
    pub tab: AuctionHouseTab,
    pub category: Option<usize>,
    /// Item whose auctions the item buy frame lists.
    pub browse_item: Option<u32>,
    pub selected_auction: Option<u64>,
    /// Auction the buy dialog asks to buy out.
    pub dialog_auction: Option<u64>,
    pub sell_item: Option<u64>,
    pub buyout_mode: bool,
    pub duration: AuctionDuration,
    pub duration_menu_open: bool,
    pub auctions_tab: AuctionsSubTab,
    pub close_requested: bool,
    pub row_page: usize,
    pub browse_query: Option<AuctionSearchQuery>,
}

impl Default for AuctionHouseUi {
    fn default() -> Self {
        Self {
            npc: None,
            tab: AuctionHouseTab::Buy,
            category: None,
            browse_item: None,
            selected_auction: None,
            dialog_auction: None,
            sell_item: None,
            // `AuctionHouseBuyoutModeCheckButtonMixin:OnShow` checks it.
            buyout_mode: true,
            // `AuctionHouseSellFrameMixin` defaults to the middle duration.
            duration: AuctionDuration::Medium,
            duration_menu_open: false,
            auctions_tab: AuctionsSubTab::Auctions,
            close_requested: false,
            row_page: 0,
            browse_query: None,
        }
    }
}

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
#[derive(Default)]
pub struct AuctionHouseState {
    pub is_open: bool,
    pub last_query: Option<AuctionSearchQuery>,
    pub search_total: u32,
    pub search_results: Vec<AuctionListingSummary>,
    pub owned_results: Vec<AuctionListingSummary>,
    pub bid_results: Vec<AuctionListingSummary>,
    pub inventory: Option<AuctionInventorySnapshot>,
    pub errors: Vec<String>,
    pub requests: Vec<AuctionRequest>,
}
impl AuctionHouseState {
    pub fn request(&mut self, request: AuctionRequest) {
        if let AuctionRequest::Browse(query) = &request {
            self.last_query = Some(query.clone());
        }
        self.requests.push(request);
    }
}
#[derive(Default)]
pub struct AuctionSession {
    pub net: AuctionHouseState,
    pub ui: AuctionHouseUi,
}
impl AuctionSession {
    pub fn open(&mut self, npc: u64) {
        self.close();
        self.ui.npc = Some(npc);
        self.net.request(AuctionRequest::Open);
    }
    pub fn close(&mut self) -> Option<u64> {
        let npc = self.ui.npc;
        *self = Self::default();
        npc
    }
    pub fn opened(&mut self, reply: AuctionHouseOpened) {
        if self.ui.npc.is_none() {
            return;
        }
        self.net.is_open = reply.success;
        if reply.success {
            self.refresh();
        } else {
            self.net.errors.push(
                reply
                    .error
                    .unwrap_or_else(|| "Auction house refused to open".into()),
            );
        }
    }
    pub fn refresh(&mut self) {
        for request in [
            AuctionRequest::Inventory,
            AuctionRequest::Owned,
            AuctionRequest::Bids,
        ] {
            self.net.request(request);
        }
        if let Some(query) = self.net.last_query.clone() {
            self.net.request(AuctionRequest::Browse(query));
        }
    }
    pub fn search_results(&mut self, reply: AuctionSearchResults) {
        if !self.net.is_open
            || self
                .net
                .last_query
                .as_ref()
                .is_some_and(|query| *query != reply.query)
        {
            return;
        }
        self.net.last_query = Some(reply.query);
        self.net.search_total = reply.total_results;
        self.net.search_results = reply.results;
        self.ui.row_page = 0;
        self.ui.selected_auction = None;
    }
    pub fn operation(&mut self, reply: AuctionOperationResponse) {
        if !self.net.is_open {
            return;
        }
        if reply.success {
            self.refresh();
        } else {
            self.net.errors.push(reply.message);
        }
    }
    pub fn click(&mut self, action: &str, texts: &view::InputTexts) -> Vec<actions::InputEdit> {
        if !self.net.is_open {
            return Vec::new();
        }
        match action {
            "auction_rows_prev" => {
                self.ui.row_page = self.ui.row_page.saturating_sub(1);
                return Vec::new();
            }
            "auction_rows_next" => {
                if (self.ui.row_page + 1) * self.row_capacity() < self.row_count(texts) {
                    self.ui.row_page += 1;
                }
                return Vec::new();
            }
            "auction_page_prev" | "auction_page_next" => {
                if let Some(mut query) = self.net.last_query.clone() {
                    if action.ends_with("prev") {
                        query.page = query.page.saturating_sub(1);
                    } else if u64::from(query.page + 1) * u64::from(query.page_size)
                        < u64::from(self.net.search_total)
                    {
                        query.page += 1;
                    } else {
                        return Vec::new();
                    }
                    self.net.request(AuctionRequest::Browse(query));
                    self.ui.row_page = 0;
                    self.ui.selected_auction = None;
                }
                return Vec::new();
            }
            _ => {}
        }
        if action.starts_with("auction_tab:")
            || action.starts_with("auction_auctions_tab:")
            || action == "auction_back"
            || action == "auction_sell_clear"
            || action.starts_with("auction_browse_item:")
            || action.starts_with("auction_sell_item:")
            || action == "auction_search"
        {
            self.ui.row_page = 0;
        }
        actions::dispatch(action, &mut self.net, &mut self.ui, texts)
    }
    pub fn state(
        &self,
        texts: &view::InputTexts,
    ) -> crate::auction_house_frame_component::AuctionHouseFrameState {
        let mut state = self.full_state(texts);
        let offset = self.ui.row_page * self.row_capacity();
        fn page<T>(rows: &mut Vec<T>, offset: usize, capacity: usize) {
            *rows = rows.drain(..).skip(offset).take(capacity).collect();
        }
        page(&mut state.browse, offset, 18);
        if let Some(item) = &mut state.item_buy {
            page(&mut item.rows, offset, 11);
        }
        page(&mut state.sell.inventory, offset, 18);
        page(&mut state.sell.listings, offset, 18);
        page(&mut state.auctions.rows, offset, 18);
        state
    }
    fn full_state(
        &self,
        texts: &view::InputTexts,
    ) -> crate::auction_house_frame_component::AuctionHouseFrameState {
        view::build_view(&view::ViewInputs {
            net: &self.net,
            ui: &self.ui,
            texts,
            catalog: &|_| None,
            visible: self.net.is_open,
        })
    }
    pub fn row_capacity(&self) -> usize {
        if self.ui.tab == AuctionHouseTab::Buy && self.ui.browse_item.is_some() {
            11
        } else {
            18
        }
    }
    pub fn row_count(&self, texts: &view::InputTexts) -> usize {
        let state = self.full_state(texts);
        match self.ui.tab {
            AuctionHouseTab::Buy => state
                .item_buy
                .map_or(state.browse.len(), |item| item.rows.len()),
            AuctionHouseTab::Sell => {
                if self.ui.sell_item.is_some() {
                    state.sell.listings.len()
                } else {
                    state.sell.inventory.len()
                }
            }
            AuctionHouseTab::Auctions => state.auctions.rows.len(),
        }
    }
}
