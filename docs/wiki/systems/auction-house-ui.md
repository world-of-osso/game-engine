# Auction house UI

Requirements: [auction house UI spec](../../specs/auction-house-ui.md). Server side: game-server `auction_house/`, `npc_interaction.rs`.

## Data flow

- Right-click (or IPC `quest interact --npc`) writes `NpcInteractionRequest::Interact`; the server answers `InteractionOpened`. The quest dialog handler (`src/game/networking/quests.rs`) owns the single `InteractionOpened` inbox, so for any role other than quest giver it closes the gossip greeting and writes `NpcFrameEvent::Opened { npc, role }`; `InteractionClosed` also writes `NpcFrameEvent::Closed`.
- `scenes/auction_house_frame::open_on_auctioneer` turns `Opened { role: AuctionHouse }` into `AuctionRequest::Open`. The server only answers auction requests while that interaction is open (`ActiveInteraction.auction_house`, the auctioneer's Alliance/Horde/neutral house).
- `game_engine::auction_house::AuctionHouseState` is the one network model for the frame and IPC. Frame requests (`AuctionHouseState::request`) take a reply slot without a channel so IPC replies stay paired per response kind. Opening and every successful operation re-query inventory (money + sellable items), owned auctions, bids and the last search.
- `AuctionHouseUi` holds the frame's own state (tab, category, selected item/auction, sell item, buyout mode, duration, dialog). Edit box text lives in the frame registry (edit boxes are built without `text`, so rebuilds keep what was typed); `view::build_view` reads it each frame.
- `sync_auction_window` keeps `WindowId::AuctionHouse` (Wide) and the house together: opened house → window; window closed by the manager (close button, Escape, another Wide window) → `CloseInteraction`; server close → window closes.

## Screens

`ui/screens/auction_house_frame_component*.rs`: root + tabs + money (`…component.rs`), Buy (`…_buy.rs`), Sell (`…_sell.rs`), Auctions (`…_auctions.rs`), art crops (`auction_house_frame_art.rs`, `UiTextureAtlasMember.csv` members on sheets 1495 / 1499 / 2134 / 948-950 / 3172). Chrome is `quest_art::window_chrome`. Positions are absolute results of the Retail anchors, cited per function.

## Gotchas

- Item icons and categories come from `data/db2/12.1.0.69933/Item.csv` (`item_catalog`); `item_icons` (ItemModifiedAppearance) has no rows for trade goods.
- The bid inputs are shared by the item buy frame and the Bids tab: only the visible mode builds them, or two frames would share a name.
- Category filtering is client-side over the fetched page (≤ 50 results).
- JS automation `ui.dumpUiTree()` exits the app after dumping; live runs end without it and read the tree over IPC (`data/diagnostics/auction-ui-20260924/run.sh`).
- Toolkit buttons draw a default skin unless `button_default_skin: false`; list rows, categories, tabs and the item display turn it off.
- Children share their parent's frame level, so a child added on a later rebuild draws over earlier siblings: row selection and stripes are the row's siblings, drawn before it.
