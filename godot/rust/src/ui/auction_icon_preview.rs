//! Offline Browse icon regression through the production auction screen.
use game_engine_ui_model::auction::NativeAuctionView;
use game_engine_ui_model::auction_house_frame_component::{
    AuctionHouseFrameState, BrowseRow, ItemLine,
};
use game_engine_ui_model::item_catalog::ItemCatalog;
use godot::prelude::*;

use super::{RegistryUi, party_preview};

fn preview_item_row(catalog: &ItemCatalog, item_id: u32) -> BrowseRow {
    let item = catalog.get(item_id).expect("offline icon fixture item");
    BrowseRow {
        item_id,
        item: ItemLine {
            name: item.name.clone(),
            quality: item.quality,
            icon_fdid: item.icon_fdid,
        },
        price: 100,
        available: 2,
    }
}

fn load_preview_rows() -> Vec<BrowseRow> {
    let catalog = game_engine_ui_model::item_catalog::wait_for_item_catalog();
    let mut rows: Vec<_> = [
        8165, 4304, 2459, 25, 2589, 238513, 3182, 10285, 2997, 2304, 828, 8169, 14047,
    ]
    .into_iter()
    .map(|item_id| preview_item_row(catalog, item_id))
    .collect();
    rows.push(BrowseRow {
        item_id: 0,
        item: ItemLine {
            name: "Missing icon reference".into(),
            quality: 1,
            icon_fdid: 134_400,
        },
        price: 0,
        available: 1,
    });
    rows
}

fn preview_view(browse: Vec<BrowseRow>) -> NativeAuctionView {
    NativeAuctionView {
        frame: AuctionHouseFrameState {
            visible: true,
            search_empty: true,
            browse,
            ..Default::default()
        },
        row_page: 0,
        row_pages: 1,
        search_page: 0,
        search_pages: 1,
        search_paging: true,
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_auction_icons_preview(&mut self) -> GString {
        self.show_auction_icons_skin(ui_toolkit::atlas::ActiveSkin::Modern)
    }

    #[func]
    pub fn show_forever_auction_icons_preview(&mut self) -> GString {
        self.show_auction_icons_skin(ui_toolkit::atlas::ActiveSkin::Forever)
    }

    fn show_auction_icons_skin(&mut self, skin: ui_toolkit::atlas::ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            ui_toolkit::atlas::set_thread_skin(skin);
            let view = preview_view(load_preview_rows());
            self.set_ui_scale(1.0)?;
            self.show_auction(view)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
