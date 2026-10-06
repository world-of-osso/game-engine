//! Native NPC auction host. Only the portable session decides trading actions.
use game_engine_session::SessionScreen;
use game_engine_ui_model::auction::{
    AuctionSession,
    view::{self, InputTexts},
};
use godot::prelude::*;
use shared::protocol::GossipMenu;
use ui_toolkit::frame::WidgetData;

use crate::{
    GameClient,
    frame_error::{FrameError, SessionError},
    ui::RegistryUi,
};

#[derive(Default)]
pub(crate) struct Auction {
    pub(crate) session: AuctionSession,
    pub(crate) ui: Option<Gd<RegistryUi>>,
    texts: InputTexts,
    gossip: Option<(u64, GossipMenu)>,
}
impl Auction {
    pub(crate) fn portrait_unit(&self) -> Option<u64> {
        self.gossip
            .as_ref()
            .map(|(npc, _)| *npc)
            .or(self.session.ui.npc)
    }

    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
    fn reset(&mut self) {
        self.session.close();
        self.texts.clear();
        self.gossip = None;
        self.free_ui();
    }
    fn read_inputs(&mut self) {
        let Some(ui) = self.ui.as_ref() else {
            return;
        };
        let bound = ui.bind();
        let Some(registry) = bound.registry() else {
            return;
        };
        for name in view::input_names() {
            if let Some(WidgetData::EditBox(edit)) = registry
                .get_by_name(name)
                .and_then(|id| registry.get(id))
                .and_then(|frame| frame.widget_data.as_ref())
            {
                self.texts.insert(name, edit.text.clone());
            }
        }
    }
}
#[cfg(test)]
mod portrait_tests {
    use super::Auction;
    use shared::protocol::GossipMenu;

    #[test]
    fn npcportraits_auction_uses_greeting_or_open_auctioneer() {
        let mut auction = Auction::default();
        assert_eq!(auction.portrait_unit(), None);
        auction.gossip = Some((
            42,
            GossipMenu {
                text: "Welcome".into(),
                options: Vec::new(),
            },
        ));
        assert_eq!(auction.portrait_unit(), Some(42));
        auction.gossip = None;
        auction.session.open(73);
        assert_eq!(auction.portrait_unit(), Some(73));
        auction.session.close();
        assert_eq!(auction.portrait_unit(), None);
    }
}

impl GameClient {
    pub(crate) fn auction_interaction_closed(&mut self, npc: u64) {
        if self.auction.session.ui.npc == Some(npc)
            || self
                .auction
                .gossip
                .as_ref()
                .is_some_and(|(id, _)| *id == npc)
        {
            self.auction.reset();
        }
    }
    pub(crate) fn auction_interaction_closed_any(&mut self) {
        self.auction.reset();
    }
    pub(crate) fn show_auction_gossip(&mut self, npc: u64, menu: GossipMenu) -> Result<(), String> {
        self.auction.reset();
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("AuctionUI");
        self.base_mut().add_child(&ui);
        let result =
            ui.bind_mut()
                .show_auction_gossip(game_engine_ui_model::auction::AuctionGossipView {
                    text: menu.text.clone(),
                    options: menu.options.clone(),
                });
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.auction.gossip = Some((npc, menu));
        self.auction.ui = Some(ui);
        Ok(())
    }
    pub(super) fn update_auction(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.auction.reset();
            return Ok(());
        }
        if let Some(ui) = &mut self.auction.ui {
            let error = ui.bind_mut().sync_input();
            if !error.is_empty() {
                return Err(error.to_string().into());
            }
        }
        self.auction.read_inputs();
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.poll_auction_input()?;
        }
        if self.auction.session.ui.close_requested {
            self.close_auction()?;
        }
        for error in std::mem::take(&mut self.auction.session.net.errors) {
            self.add_world_error(&error)?;
        }
        for request in std::mem::take(&mut self.auction.session.net.requests) {
            self.account.send_auction_request(request)?;
        }
        if self.auction.session.net.is_open {
            if self.auction.gossip.take().is_some() {
                self.auction.free_ui();
            }
            let view = self.auction.session.native_view(&self.auction.texts);
            let scale = self.effective_ui_scale();
            if self.auction.ui.is_none() {
                let mut ui = RegistryUi::new_alloc();
                ui.set_name("AuctionUI");
                self.base_mut().add_child(&ui);
                let result = ui.bind_mut().show_auction(view.clone());
                if let Err(error) = result {
                    ui.free();
                    return Err(error.into());
                }
                self.auction.ui = Some(ui);
            }
            let mut ui = self.auction.ui.clone().ok_or("Auction UI missing")?;
            ui.bind_mut().set_ui_scale(scale)?;
            ui.bind_mut().set_auction(view)?;
            for (name, text) in &self.auction.texts {
                if ui
                    .bind()
                    .registry()
                    .is_some_and(|reg| reg.get_by_name(name).is_some())
                {
                    ui.bind_mut().set_editbox_text(name, text)?;
                }
            }
        } else if self.auction.gossip.is_none() {
            self.auction.free_ui();
        }
        Ok(())
    }
    fn poll_auction_input(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.auction.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == "auction_gossip_close" {
                self.close_auction()?;
                break;
            }
            if let Some(option) = action
                .strip_prefix("auction_gossip:")
                .and_then(|id| id.parse::<u32>().ok())
            {
                if let Some((npc, menu)) = &self.auction.gossip
                    && menu.options.iter().any(|item| item.option_id == option)
                {
                    self.account.send_gossip_option(*npc, option)?;
                }
                continue;
            }
            let edits = self.auction.session.click(&action, &self.auction.texts);
            for (name, text) in edits {
                self.auction.texts.insert(name, text);
            }
        }
        Ok(())
    }
    fn close_auction(&mut self) -> Result<(), SessionError> {
        let npc = self
            .auction
            .session
            .ui
            .npc
            .or_else(|| self.auction.gossip.as_ref().map(|(npc, _)| *npc));
        self.auction.reset();
        if let Some(npc) = npc {
            self.account.send_close_interaction(npc)?;
        }
        Ok(())
    }
    pub(super) fn close_auction_window(&mut self) -> Result<bool, SessionError> {
        if self.auction.session.ui.npc.is_none() && self.auction.gossip.is_none() {
            return Ok(false);
        }
        self.close_auction()?;
        Ok(true)
    }
    pub(super) fn auction_snapshot(&self) -> VarDictionary {
        let session = &self.auction.session;
        let mut result = VarDictionary::new();
        result.set("open", session.net.is_open);
        result.set("npc", session.ui.npc.unwrap_or(0) as i64);
        result.set(
            "money",
            session.net.inventory.as_ref().map_or(0, |inv| inv.gold) as i64,
        );
        result.set("search_total", session.net.search_total);
        result.set("search_revision", session.net.search_revision as i64);
        result.set("row_page", session.ui.row_page as i64);
        result.set(
            "search_page",
            session
                .net
                .last_query
                .as_ref()
                .map_or(0, |query| query.page),
        );
        fn listings(rows: &[shared::protocol::AuctionListingSummary]) -> VarArray {
            let mut result = VarArray::new();
            for row in rows {
                let mut item = VarDictionary::new();
                item.set("auction_id", row.auction_id as i64);
                item.set("item_id", row.item.item_id);
                item.set("name", row.item.name.as_str());
                item.set("quantity", row.stack_count);
                item.set("bid", row.min_next_bid as i64);
                item.set("buyout", row.buyout_price.unwrap_or(0) as i64);
                result.push(&item.to_variant());
            }
            result
        }
        let mut groups = VarArray::new();
        for row in &session.net.browse_results {
            let mut item = VarDictionary::new();
            item.set("item_id", row.item_id);
            item.set("name", row.name.as_str());
            item.set("quality", row.quality);
            item.set("required_level", row.required_level);
            item.set("lowest_unit_price", row.lowest_unit_price as i64);
            item.set("total_quantity", row.total_quantity as i64);
            groups.push(&item.to_variant());
        }
        result.set("groups", &groups);
        result.set("query_is_browse", session.net.query_is_browse);
        result.set("search", &listings(&session.net.search_results));
        result.set("owned", &listings(&session.net.owned_results));
        result.set("bids", &listings(&session.net.bid_results));
        let mut inventory = VarArray::new();
        if let Some(inv) = &session.net.inventory {
            for row in &inv.items {
                let mut item = VarDictionary::new();
                item.set("guid", row.item_guid as i64);
                item.set("item_id", row.item_id);
                item.set("name", row.name.as_str());
                item.set("quantity", row.stack_count);
                inventory.push(&item.to_variant());
            }
        }
        result.set("inventory", &inventory);
        result
    }
}
