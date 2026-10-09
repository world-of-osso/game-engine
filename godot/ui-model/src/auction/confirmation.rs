//! Retail AH/Shared/Blizzard_AuctionHouseFrame.lua:26-56,844-865.
use super::{AuctionRequest, AuctionSession, view};
use crate::auction_house_frame_component::BID_BOXES;
use crate::popup::{PopupId, PopupOutcome, PopupResult, PopupSpec, PopupStack};
use shared::protocol::{BuyoutAuction, PlaceBid};

pub(super) struct AuctionConfirmation {
    key: &'static str,
    request: AuctionRequest,
    amount: u64,
    popup: Option<PopupId>,
}

/// GlobalStrings.csv 20567/13066; separate MoneyFrame, not commodity item/quantity text.
pub fn confirmation_spec(key: &str) -> PopupSpec {
    PopupSpec {
        key: key.into(),
        text: match key {
            "BID_AUCTION" => "Bid on auction for:",
            "BUYOUT_AUCTION" => "Buyout auction for:",
            _ => panic!("Unknown auction confirmation: {key}"),
        }
        .into(),
        accept_label: "Accept".into(),
        cancel_label: Some("Cancel".into()),
        timeout: None,
        confirm_text: None,
    }
}

impl AuctionSession {
    pub(super) fn stage_confirmation(&mut self, action: &str, texts: &view::InputTexts) {
        if self.confirmation.is_some() {
            return;
        }
        let Some(auction_id) = self.ui.selected_auction else {
            return;
        };
        let (key, amount, request) = if action == "auction_bid" {
            let amount = view::money_input(texts, BID_BOXES);
            (
                "BID_AUCTION",
                amount,
                AuctionRequest::Bid(PlaceBid { auction_id, amount }),
            )
        } else {
            let selected = self
                .net
                .search_results
                .iter()
                .chain(&self.net.bid_results)
                .find(|listing| listing.auction_id == auction_id);
            let Some(amount) = selected.and_then(|listing| listing.buyout_price) else {
                return;
            };
            (
                "BUYOUT_AUCTION",
                amount,
                AuctionRequest::Buyout(BuyoutAuction { auction_id }),
            )
        };
        self.confirmation = Some(AuctionConfirmation {
            key,
            request,
            amount,
            popup: None,
        });
    }

    /// Publish into the existing global stack; hide stale AH popups after close/reset.
    pub fn sync_popup(&mut self, stack: &mut PopupStack) {
        for key in ["BID_AUCTION", "BUYOUT_AUCTION"] {
            if self
                .confirmation
                .as_ref()
                .is_none_or(|pending| pending.key != key)
            {
                stack.hide(key);
            }
        }
        if let Some(pending) = &mut self.confirmation
            && pending.popup.is_none()
        {
            pending.popup =
                Some(stack.push_money_alert(confirmation_spec(pending.key), pending.amount));
        }
    }

    /// Consume a matching result once; the captured auction ID and price do not follow selection edits.
    pub fn popup_results(&mut self, results: &[PopupResult]) {
        let Some(pending) = &self.confirmation else {
            return;
        };
        let Some(result) = results
            .iter()
            .find(|result| Some(result.id) == pending.popup && result.key == pending.key)
        else {
            return;
        };
        let pending = self.confirmation.take().unwrap();
        if result.outcome == PopupOutcome::Accepted && self.net.is_open {
            self.net.request(pending.request);
            self.ui.selected_auction = None;
        }
    }
}
