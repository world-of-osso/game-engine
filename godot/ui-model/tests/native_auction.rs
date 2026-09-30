//! Native auction session, independent of Godot and Bevy rendering.
#[cfg(test)]
mod tests {
    use game_engine_ui_model::auction_house_frame_component::duration_label;
    use shared::protocol::AuctionDuration;
    #[test]
    fn native_auction_duration_contract() {
        assert_eq!(duration_label(AuctionDuration::Short), "1 Day");
        assert_eq!(duration_label(AuctionDuration::Medium), "1 Week");
        assert_eq!(duration_label(AuctionDuration::Long), "2 Weeks");
    }
}

use game_engine_ui_model::auction::{AuctionSession, AuctionRequest, view::InputTexts};
use shared::protocol::*;
#[test]
fn native_auction_interaction_gate_refresh_rejection_close() {
 let mut s=AuctionSession::default();
 assert!(s.click("auction_search", &InputTexts::new()).is_empty());
 assert!(s.net.requests.is_empty());
 s.open(99); assert_eq!(s.net.requests, vec![AuctionRequest::Open]); s.net.requests.clear();
 s.opened(AuctionHouseOpened{success:true,error:None});
 assert_eq!(s.net.requests,vec![AuctionRequest::Inventory,AuctionRequest::Owned,AuctionRequest::Bids]);
 s.net.requests.clear(); s.operation(AuctionOperationResponse{success:false,message:"too far".into()});
 assert_eq!(s.net.errors,vec!["too far"]);assert!(s.net.requests.is_empty());
 assert_eq!(s.close(),Some(99)); assert!(!s.net.is_open);
 s.opened(AuctionHouseOpened{success:true,error:None}); assert!(!s.net.is_open);
}
#[test]
fn native_auction_all_rows_are_reachable() {
 let mut s=AuctionSession::default(); s.open(99); s.opened(AuctionHouseOpened{success:true,error:None});
 s.net.inventory=Some(AuctionInventorySnapshot{gold:1000,items:(1..=43).map(|i| AuctionInventoryItem{item_guid:i,item_id:i as u32,name:format!("item{i}"),quality:1,required_level:1,stack_count:1,vendor_sell_price:1}).collect()});
 s.click("auction_tab:sell",&InputTexts::new());
 assert_eq!(s.state(&InputTexts::new()).sell.inventory.len(),18);
 s.click("auction_rows_next",&InputTexts::new()); assert_eq!(s.state(&InputTexts::new()).sell.inventory[0].item_guid,19);
 s.click("auction_rows_next",&InputTexts::new()); assert_eq!(s.state(&InputTexts::new()).sell.inventory.last().unwrap().item_guid,43);
}
