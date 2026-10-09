//! Offline auction snapshots through the same RegistryUi projection as live windows.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::{auction::preview::preview_view, auction_house_frame_component::*};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_auction_preview(&mut self) -> GString {
        self.show_auction_preview_skin(ActiveSkin::Modern)
    }

    #[func]
    fn show_forever_auction_preview(&mut self) -> GString {
        self.show_auction_preview_skin(ActiveSkin::Forever)
    }

    /// Dedicated offline instance of the same StaticPopup screen used by the global host.
    #[func]
    fn show_auction_confirmation_preview(&mut self) -> GString {
        use game_engine_ui_model::{
            auction::preview::confirmation_spec, popup::PopupStack,
            static_popup_component::StaticPopupState,
        };
        let result = (|| {
            let view = std::env::var("GODOT_AUCTION_VIEW").map_err(|error| error.to_string())?;
            let key = match view.as_str() {
                "bid-popup" => "BID_AUCTION",
                "buyout-popup" => "BUYOUT_AUCTION",
                _ => return Err(format!("Not an auction confirmation view: {view}")),
            };
            let mut stack = PopupStack::default();
            stack.push_money_alert(confirmation_spec(key), 12_345);
            self.set_ui_scale(1.0)?;
            self.show_static_popups(StaticPopupState {
                popups: stack.visible(),
            })
        })();
        GString::from(result.err().unwrap_or_default().as_str())
    }

    fn show_auction_preview_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let view = std::env::var("GODOT_AUCTION_VIEW")
                .map_err(|error| format!("Auction preview requires GODOT_AUCTION_VIEW: {error}"))?;
            self.show_auction(preview_view(&view)?)?;
            if view == "subcategory_sorted" {
                self.set_editbox_text(SEARCH_BOX, "linen")?;
            }
            if matches!(view.as_str(), "inventory" | "sell" | "duration") {
                self.set_editbox_text(QUANTITY_BOX, "3")?;
                self.set_editbox_text(SELL_BUYOUT_BOXES.gold, "1")?;
                self.set_editbox_text(SELL_BUYOUT_BOXES.silver, "23")?;
                self.set_editbox_text(SELL_BUYOUT_BOXES.copper, "45")?;
            }
            if matches!(
                view.as_str(),
                "item" | "dialog" | "bids" | "bid-popup" | "buyout-popup"
            ) {
                self.set_editbox_text(BID_BOXES.gold, "1")?;
                self.set_editbox_text(BID_BOXES.silver, "23")?;
                self.set_editbox_text(BID_BOXES.copper, "45")?;
            }
            Ok(())
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
