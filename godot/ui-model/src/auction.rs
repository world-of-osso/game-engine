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
    Listings(AuctionSearchQuery),
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
    pub query_is_browse: bool,
    pub browse_results: Vec<AuctionBrowseItem>,
    pub search_total: u32,
    pub search_revision: u64,
    pub search_results: Vec<AuctionListingSummary>,
    pub owned_results: Vec<AuctionListingSummary>,
    pub bid_results: Vec<AuctionListingSummary>,
    pub inventory: Option<AuctionInventorySnapshot>,
    pub errors: Vec<String>,
    pub operation_pending: bool,
    pub requests: Vec<AuctionRequest>,
}
impl AuctionHouseState {
    pub fn request(&mut self, request: AuctionRequest) {
        match &request {
            AuctionRequest::Browse(query) => {
                self.last_query = Some(query.clone());
                self.query_is_browse = true;
            }
            AuctionRequest::Listings(query) => {
                self.last_query = Some(query.clone());
                self.query_is_browse = false;
                self.search_results.clear();
            }
            _ => {}
        }
        if matches!(
            request,
            AuctionRequest::Create(_)
                | AuctionRequest::Bid(_)
                | AuctionRequest::Buyout(_)
                | AuctionRequest::Cancel(_)
        ) {
            self.operation_pending = true;
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
            self.request_query_page(query);
        }
    }
    fn request_query_page(&mut self, query: AuctionSearchQuery) {
        let request = if self.net.query_is_browse {
            AuctionRequest::Browse(query)
        } else {
            AuctionRequest::Listings(query)
        };
        self.net.request(request);
    }
    pub fn browse_results(&mut self, reply: AuctionBrowseResults) {
        if !self.net.query_is_browse || !self.accept_query(&reply.query) {
            return;
        }
        self.net.search_revision += 1;
        self.net.search_total = reply.total_results;
        self.net.browse_results = reply.items;
        self.ui.row_page = 0;
        self.ui.selected_auction = None;
    }
    fn accept_query(&self, query: &AuctionSearchQuery) -> bool {
        let Some(expected) = self.net.last_query.as_ref().filter(|_| self.net.is_open) else {
            return false;
        };
        // The authoritative auctioneer session selects the house, not the client.
        let mut effective = expected.clone();
        effective.faction = query.faction;
        effective == *query
    }
    pub fn search_results(&mut self, reply: AuctionSearchResults) {
        if self.net.query_is_browse || !self.accept_query(&reply.query) {
            return;
        }
        self.net.search_revision += 1;
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
        self.net.operation_pending = false;
        if reply.success {
            self.ui.selected_auction = None;
            self.refresh();
        } else {
            self.net.errors.push(reply.message);
        }
    }
    pub fn click(&mut self, action: &str, texts: &view::InputTexts) -> Vec<actions::InputEdit> {
        if !self.net.is_open {
            return Vec::new();
        }
        let state = self.full_state(texts);
        let operation = matches!(
            action,
            "auction_bid"
                | "auction_buyout"
                | "auction_dialog_buy"
                | "auction_post"
                | "auction_cancel"
        );
        if operation && self.net.operation_pending {
            return Vec::new();
        }
        let allowed = match action {
            "auction_bid" => {
                state.item_buy.as_ref().is_some_and(|item| item.can_bid)
                    || (self.ui.tab == AuctionHouseTab::Auctions && state.auctions.can_bid)
            }
            "auction_buyout" => {
                state.item_buy.as_ref().is_some_and(|item| item.can_buyout)
                    || (self.ui.tab == AuctionHouseTab::Auctions && state.auctions.can_buyout)
            }
            "auction_cancel" => state.auctions.can_cancel,
            "auction_post" => state.sell.can_post,
            "auction_dialog_buy" => state
                .dialog
                .as_ref()
                .is_some_and(|dialog| dialog.price <= state.money),
            _ => true,
        };
        if !allowed {
            return Vec::new();
        }
        match action {
            "auction_rows_prev" => {
                self.ui.row_page = self.ui.row_page.saturating_sub(1);
                self.ui.selected_auction = None;
                return Vec::new();
            }
            "auction_rows_next" => {
                if (self.ui.row_page + 1) * self.row_capacity() < self.row_count(texts) {
                    self.ui.row_page += 1;
                    self.ui.selected_auction = None;
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
                    self.request_query_page(query);
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
        if self.net.operation_pending {
            state.sell.can_post = false;
            state.auctions.can_bid = false;
            state.auctions.can_buyout = false;
            state.auctions.can_cancel = false;
            if let Some(item) = &mut state.item_buy {
                item.can_bid = false;
                item.can_buyout = false;
            }
        }
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
            catalog: &crate::item_catalog::item_catalog_entry,
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

pub enum AuctionReply {
    Opened(AuctionHouseOpened),
    Browse(AuctionBrowseResults),
    Search(AuctionSearchResults),
    Inventory(AuctionInventorySnapshot),
    Owned(OwnedAuctionListResponse),
    Bids(BidAuctionListResponse),
    Operation(AuctionOperationResponse),
}
impl AuctionSession {
    pub fn receive(&mut self, reply: AuctionReply) {
        match reply {
            AuctionReply::Opened(reply) => self.opened(reply),
            AuctionReply::Browse(reply) => self.browse_results(reply),
            AuctionReply::Search(reply) => self.search_results(reply),
            AuctionReply::Operation(reply) => self.operation(reply),
            AuctionReply::Inventory(reply) if self.net.is_open => self.net.inventory = Some(reply),
            AuctionReply::Owned(reply) if self.net.is_open => {
                self.net.owned_results = reply.listings
            }
            AuctionReply::Bids(reply) if self.net.is_open => self.net.bid_results = reply.listings,
            _ => {}
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeAuctionView {
    pub frame: crate::auction_house_frame_component::AuctionHouseFrameState,
    pub row_page: usize,
    pub row_pages: usize,
    pub search_page: u32,
    pub search_pages: u32,
    pub search_paging: bool,
}
impl AuctionSession {
    pub fn native_view(&self, texts: &view::InputTexts) -> NativeAuctionView {
        let query = self.net.last_query.as_ref();
        NativeAuctionView {
            frame: self.state(texts),
            row_page: self.ui.row_page,
            row_pages: self.row_count(texts).div_ceil(self.row_capacity()).max(1),
            search_page: query.map_or(0, |q| q.page),
            search_pages: query.map_or(1, |q| {
                self.net.search_total.div_ceil(q.page_size.max(1)).max(1)
            }),
            search_paging: self.ui.tab == AuctionHouseTab::Buy
                || (self.ui.tab == AuctionHouseTab::Sell && self.ui.sell_item.is_some()),
        }
    }
}
pub fn native_auction_screen(
    ctx: &ui_toolkit::screen::SharedContext,
) -> ui_toolkit::widget_def::Element {
    use ui_toolkit::rsx;
    let state = ctx.get::<NativeAuctionView>().expect("NativeAuctionView");
    let rows = format!("Rows {}/{}", state.row_page + 1, state.row_pages);
    let pages = format!("Results {}/{}", state.search_page + 1, state.search_pages);
    let hide = !state.frame.visible;
    let sell = state.frame.tab == AuctionHouseTab::Sell;
    let left = if sell { 372.0 } else { 176.0 };
    let top = if sell { 460.0 } else { 482.0 };
    let search_left = if sell { 0.0 } else { 240.0 };
    let search_top = if sell { 24.0 } else { 0.0 };
    let hide_search = !state.search_paging;
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state.frame.clone());
    let content = crate::auction_house_frame_component::auction_house_frame_screen(&shared);
    rsx! {
        r#frame { name:"NativeAuctionRoot", width:800.0,height:570.0,hidden:hide,strata:ui_toolkit::strata::FrameStrata::High,pos_type:"absolute",left:16.0,top:104.0,
            {content}
            r#frame { name:"AuctionPaging",width:610.0,height:26.0,pos_type:"absolute",left:left,top:top,
                button {name:"AuctionRowsPrev",width:48.0,height:22.0,text:"Prev",onclick:"auction_rows_prev",enabled:{state.row_page>0},pos_type:"absolute",left:0.0,top:0.0,}
                fontstring {name:"AuctionRowsLabel",width:120.0,height:22.0,text:{rows.as_str()},pos_type:"absolute",left:50.0,top:0.0,}
                button {name:"AuctionRowsNext",width:48.0,height:22.0,text:"Next",onclick:"auction_rows_next",enabled:{state.row_page+1<state.row_pages},pos_type:"absolute",left:170.0,top:0.0,}
                r#frame {name:"AuctionResultPaging",width:360.0,height:22.0,hidden:hide_search,pos_type:"absolute",left:search_left,top:search_top,
                    button {name:"AuctionPagePrev",width:64.0,height:22.0,text:"Prev page",onclick:"auction_page_prev",enabled:{state.search_page>0},pos_type:"absolute",left:0.0,top:0.0,}
                    fontstring {name:"AuctionPageLabel",width:140.0,height:22.0,text:{pages.as_str()},pos_type:"absolute",left:70.0,top:0.0,}
                    button {name:"AuctionPageNext",width:64.0,height:22.0,text:"Next page",onclick:"auction_page_next",enabled:{state.search_page+1<state.search_pages},pos_type:"absolute",left:215.0,top:0.0,}
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuctionGossipView {
    pub text: String,
    pub options: Vec<GossipMenuOption>,
}
pub fn auction_gossip_screen(
    ctx: &ui_toolkit::screen::SharedContext,
) -> ui_toolkit::widget_def::Element {
    use ui_toolkit::rsx;
    struct DynName(String);
    let view = ctx.get::<AuctionGossipView>().expect("AuctionGossipView");
    let mut options = Vec::new();
    for (index, option) in view.options.iter().enumerate() {
        let name = format!("AuctionGossipOption{}", option.option_id);
        let action = format!("auction_gossip:{}", option.option_id);
        let top = 100.0 + index as f32 * 30.0;
        options.extend(rsx!{button { name:{DynName(name)},width:280.0,height:26.0,text:{option.text.as_str()},onclick:{action.as_str()},pos_type:"absolute",left:20.0,top:top,}});
    }
    let height = 140.0 + view.options.len() as f32 * 30.0;
    let chrome = crate::quest_art::window_chrome(
        "AuctionGossip",
        (320.0, height),
        "Greeting",
        "auction_gossip_close",
    );
    rsx! {r#frame {name:"AuctionGossip",width:320.0,height:height,strata:ui_toolkit::strata::FrameStrata::High,pos_type:"absolute",left:16.0,top:104.0,
        {chrome}
        fontstring {name:"AuctionGossipText",width:280.0,height:64.0,text:{view.text.as_str()},pos_type:"absolute",left:20.0,top:32.0,}
        {options}
    }}
}
