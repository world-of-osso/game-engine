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

Native NPC `InteractionOpened Role(AuctionHouse)` queues Open; gossip options send `SelectGossipOption`. An explicit close/Escape sends `CloseInteraction`; matching server closure resets the session without sending another close. Leaving InWorld frees the host. The bridge registers all seven auction reply types; account dispatch preserves their concrete types via `AuctionReply`. Failed operations (including queries rejected through `AuctionOperationResponse`) enter UIErrors text; successful operations refresh inventory/money, owned/bids and the latest query.

Two paging levels are independent: server result pages preserve filters, while 18-row local slices (11 for the item buy list) expose every fetched row. Local paging also works on unpaged inventory/owned/bids replies. `AuctionRequest::Browse` sends `QueryAuctionBrowse`; `AuctionBrowseResults.items` populates `net.browse_results`. Each row uses server-global `lowest_unit_price` and `total_quantity` unchanged; browse pages and `total_results` count distinct items. No client-page aggregation or page-scoped price label remains. Category filtering sends `class_id`; drilldown and sell-market `AuctionRequest::Listings` sends flat `QueryAuctions` with `item_id`. Flat results populate `net.search_results` and keep real auction IDs. Refresh/page changes preserve the active request kind through `query_is_browse`; mismatched query or endpoint replies are ignored. `search_revision` increments only on accepted replies so smoke waits observe a reply, not merely a changed requested page.

Native `auction_state()` retains `search` as flat listing dictionaries and adds `groups` containing item ID/name/quality/required level/lowest unit price/total quantity, plus `query_is_browse`. `search_total`, `search_page` and `search_revision` describe the active endpoint. The GDScript fixture finds browse items through `groups`, then waits for flat `search` drilldown. Shared `BrowseRow.available` is `u64`; preserved Bevy producers only widen their existing stack counts.

The shared frame now labels Short/Medium/Long as 1 Day/1 Week/2 Weeks; deposit multipliers remain 1/2/4. Trading validation disables unaffordable/under-minimum bids, unavailable buyouts, cancel with bids and invalid sell quantity/prices/deposit; outstanding operations suppress duplicate frame submissions. Server remains authoritative for economy and interaction eligibility.

### Bounded evidence

At `5b9cb76c` with shared protocol `88bc3fe`, Depot `scripts/depot-build.py --root <canonical-engine> --test -p game-engine-ui-model -p game-engine-network auction` exited 0: native model 8/8, owned UDP 1/1, shared frame 1/1 (`/tmp/native-ah-global-green-5b9cb76c.log`). Assertions cover global 17-copper price and 5,000,000,001 stock despite unrelated flat data, distinct-item second page out of 103 results, stale browse rejection, flat exact-item drilldown and real-ID buyout. Model RED was missing-API compilation failure; wire RED reached UDP and timed out awaiting the unregistered browse reply (`/tmp/native-ah-global-{model,wire}-red.log`). Later docs-only changes do not invalidate this proof; host compilation/GDScript execution are not covered. Main owns extension rebuild and game-cli-first native runtime acceptance. No local extension Cargo, engine integration, broad/check/lint or operations ran.

At `5f855404`, targeted Depot native model tests passed 8/8 (`target/native-auction-model-green-final.log`, build `xstxljj1h2`) and host compile/right-click-range proof passed 1/1 (`target/native-auction-host-green-final.log`, build `wfkdvgnm8z`). This compiles the input-first read order and read-only fixture lookup; it does not execute Godot. `97e96435` owned UDP passed 1/1 (`target/native-auction-wire-green.log`, build `w7w7zc9rff`); its source remains unchanged. Two unrelated existing `terrain/assets.rs` unused-mut warnings remain. This historical CPU proof did not execute the native fixture. Later saved runtime proof below supersedes its pending-fixture status, without establishing rendered parity or full conversion.

### Saved native runtime proof (2026-10-01)

Evidence root: `/home/osso/.cache/economy-cli-20260930/a54a0ad0`. Real CLI receipts there precede all native runs: `buyer-buyout.json`, `buyer-take-won.json`, `seller-take-proceeds.json` and `seller-take-return.json` exit 0 and confirm server trading plus inbox/Gold/inventory claims. Native run manifests record owned loopback endpoints and capture directories.

- `native-seller-run3.json` → `/tmp/pyrun-tmux/02cf3245798a4262ad7a7ff2484438e5.log`, exit 0: NPC pointer entry, authored controls, all three duration choices, own-buyout rejection, cancellation and Escape/close. The log retains wire duration values 12/24/48; displayed labels remain 1 Day/1 Week/2 Weeks.
- `native-buyer-run.json` → `/tmp/pyrun-tmux/149529b576ae499e86d1b676bdc8ee7d.log`, exit 0: real bid and buyout; `FIXTURE BID_BUYOUT money=99700` followed by buyer completion.
- `native-large-run.json` → `/tmp/pyrun-tmux/676eac7fea0d4aa9b0406265e2827b68.log`, exit 0: 311 global item groups, 87 category groups, disjoint 50/50 server pages and local-row advancement. `world_auction_browse_flow.gd` asserts a real second page, no overlap and category page reset. Main inspected `native-large-shots/` captures; this docs update inspected receipts, not screenshots.

Root fix `dc638c8e` accepts the server-selected house faction in otherwise matching query replies while retaining stale-filter rejection. `native-house-red.json` records the concrete 0-versus-1 mismatch; `native-house-green.json` records all nine native model tests passing. These are saved proofs, not newly rerun tests.

LiquidObject 42, local-CASC and UI icon errors remain. `extract-ah-icons.json` records 23 of 25 missing icons extracted locally; FDIDs 133849 and 136113 were unavailable. No universal performance, rendering, clean-resource or shutdown claim follows from fixture exit 0. Server range-close, title dragging and Wide-window replacement remain unproved. Full Godot conversion is owned elsewhere and remains open. Auction delivery receiving proof is in [[trade-and-mail]].

## Sources

- [Auction requirements](../../specs/auction-house-ui.md) — native baseline and ordered smoke contract.
- Saved root manifests/receipts and exact runtime logs listed above — bounded CLI-first native acceptance.
- `godot/tests/world_auction_flow.gd`, `godot/tests/world_auction_browse_flow.gd` — native pointer and paging assertions.
- `godot/ui-model/src/auction.rs`, `auction/{actions,view}.rs` — portable decisions and paging.
- `godot/rust/src/{auction,account,merchant}.rs`, `ui/mod.rs`, `godot/network/src/{lib,wire_tests}.rs` — native protocol/UI boundary.

## See Also

- [[merchant-frame]] — original native NPC/input/registry patterns.
- [[networking]] — owned native Lightyear bridge.
