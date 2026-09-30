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
- Preserved Bevy category filtering is page-local; native Godot uses the server query fields described below.
- JS automation `ui.dumpUiTree()` exits the app after dumping; live runs end without it and read the tree over IPC (`data/diagnostics/auction-ui-20260924/run.sh`).
- Toolkit buttons draw a default skin unless `button_default_skin: false`; list rows, categories, tabs and the item display turn it off.
- Children share their parent's frame level, so a child added on a later rebuild draws over earlier siblings: row selection and stripes are the row's siblings, drawn before it.


## Native Godot

`godot/ui-model/src/auction.rs` owns the native `AuctionSession`; its `net` field stores wire reply data and queued `AuctionRequest` enum values, and its `ui` field stores tab/item/auction selections. `auction/actions.rs` and `auction/view.rs` port the legacy decision logic without importing Bevy's renderer/plugin/IPC resources. The native host in `godot/rust/src/auction.rs` drains projected input before reading edit texts, applies actions, sends queued requests through `Account` and rebuilds only changed views. `RegistryUi::control_for_action` exposes a read-only projected control for pointer fixtures, not a synthetic trading path.

Native NPC `InteractionOpened Role(AuctionHouse)` queues Open; gossip options send `SelectGossipOption`. An explicit close/Escape sends `CloseInteraction`; matching server closure resets the session without sending another close. Leaving InWorld frees the host. The bridge registers all six auction reply types; account dispatch preserves their concrete types via `AuctionReply`. Failed operations (including queries rejected through `AuctionOperationResponse`) enter UIErrors text; successful operations refresh inventory/money, owned/bids and the latest query.

Two paging levels are independent: server result pages preserve exact-item/category filters, while 18-row local slices (11 for the item buy list) expose every fetched row. Local paging also works on unpaged inventory/owned/bids replies. Category filtering sends `class_id`; drilldown and sell-market searches send `item_id`. Responses for a different latest query are ignored. `search_revision` increments only on accepted replies so smoke waits observe a reply, not merely a changed requested page. Grouped browse quantities/prices are page-local and labelled accordingly; the listing protocol supplies no global item aggregates.

The shared frame now labels Short/Medium/Long as 1 Day/1 Week/2 Weeks; deposit multipliers remain 1/2/4. Trading validation disables unaffordable/under-minimum bids, unavailable buyouts, cancel with bids and invalid sell quantity/prices/deposit; outstanding operations suppress duplicate frame submissions. Server remains authoritative for economy and interaction eligibility.

### Bounded evidence

At `5f855404`, targeted Depot native model tests passed 8/8 (`target/native-auction-model-green-final.log`, build `xstxljj1h2`) and host compile/right-click-range proof passed 1/1 (`target/native-auction-host-green-final.log`, build `wfkdvgnm8z`). This compiles the input-first read order and read-only fixture lookup; it does not execute Godot. `97e96435` owned UDP passed 1/1 (`target/native-auction-wire-green.log`, build `w7w7zc9rff`); its source remains unchanged. Two unrelated existing `terrain/assets.rs` unused-mut warnings remain. Native GDScript fixture exists but has not run or been parse-certified; main must first prove game-cli against its integrated disposable server. No rendered parity, full conversion or economic acceptance is claimed.

## Sources

- [Auction requirements](../../specs/auction-house-ui.md) — native baseline and ordered smoke contract.
- `godot/ui-model/src/auction.rs`, `auction/{actions,view}.rs` — portable decisions and paging.
- `godot/rust/src/{auction,account,merchant}.rs`, `ui/mod.rs`, `godot/network/src/{lib,wire_tests}.rs` — native protocol/UI boundary.

## See Also

- [[merchant-frame]] — original native NPC/input/registry patterns.
- [[networking]] — owned native Lightyear bridge.
