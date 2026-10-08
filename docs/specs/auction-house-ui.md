# Auction house frame

> Root `src/` paths and `cargo test --bin game-engine` selectors below name files and tests deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Economy row of the [in-game UI plan](../plans/2026-09-23-ingame-ui.md): Retail `AuctionHouseFrame`
opened from an auctioneer, on the existing auction protocol (`shared-protocol`
`protocol/gameplay_messages.rs`) and the server auction house (`game-server`
`auction_house/`). Reference: `~/.cache/wow-ui-sim/blizzard-ui/Blizzard_AuctionHouseUI`
(paths below are relative to it). Code: `src/ui/screens/auction_house_frame_*.rs` (layout),
`src/scenes/auction_house_frame/` (state, actions, window), `src/auction_house.rs` (network).

## What it must do

### Opening and closing

- [x] Right-clicking an auctioneer sends `InteractNpc`; `InteractionOpened` with `Role(AuctionHouse)` (directly or from a gossip option, which closes the greeting) sends `OpenAuctionHouse`.
- [x] `AuctionHouseOpened { success }` opens the frame as a Wide window (`UIPanelWindows` `area = "doublewide"`, Shared/Blizzard_AuctionHouseFrame.lua:8) and loads money (`QueryAuctionInventory`), owned auctions and bids. No search runs on open.
- [x] The close button, Escape or another Wide window close the frame and send `CloseInteraction`; `InteractionClosed` from the server (walked away) closes it without one.
- [x] A failed operation shows its server message in the UI error frame.

### Frame (Shared/Blizzard_AuctionHouseFrame.xml)

- [x] Root `AuctionHouseFrame`, 800×538 (:5), `ButtonFrameTemplate` metal chrome (`quest_art::window_chrome`), movable by its title.
- [x] Both skins bind the interacting auctioneer's masked portrait (`SetPortraitToUnit("npc")`, Shared/Blizzard_AuctionHouseFrame.lua:392), including the auction greeting. Backgrounds exclude the portrait mask.
- [x] Title per tab: Browse Auctions / Post Auctions / Auctions (`UpdateTitle`, Shared/Blizzard_AuctionHouseFrame.lua:640).
- [x] Bottom tabs Buy, Sell, Auctions (`PanelTabButtonTemplate` art, Buy at BOTTOMLEFT (20,-28), each next -15 over, width text + 40 min 70, Shared/Blizzard_AuctionHouseTab.lua:2-11); selected tab text white.
- [x] Money: `MoneyFrameInset` and 158×19 `ThinGoldEdgeTemplate` border at BOTTOMLEFT (5,6), money right-aligned 6 in (:11-40). Money displays follow `MoneyDisplayFrameMixin`: silver and copper always, gold only when non-zero, thousands separators, AH coin atlases.

### Buy

- [x] Search bar at TOPRIGHT (-12,-29): search box 241×22 with "Search" instructions, Search button 132×22 (Shared/Blizzard_AuctionHouseSearchBar.xml). Enter in the box searches.
- [x] Category list 168×438 at (4,73) on `auctionhouse-background-categories`, `AuctionCategoryButtonTemplate` 132×21 buttons (Mainline/Blizzard_AuctionHouseCategoriesList.xml); the 14 top-level Retail categories (Mainline/Blizzard_AuctionData.lua) filter results by `Item.ClassID`; clicking the selected one clears it.
- [x] Browse list (172,74) 623×437 on `auctionhouse-background-index`, columns Price 168 / Name / Available 60 (`GetBrowseListLayout`, Shared/Blizzard_AuctionHouseTableBuilder.lua:1033): one row per item with its lowest per-item price (buyout, else bid) and total quantity; name in the item's quality colour with its icon (`Item.IconFileDataID`); "No items found" after an empty search.
- [x] Clicking an item opens the item buy frame (Shared/Blizzard_AuctionHouseItemBuyFrame.xml): Back, the 622×86 item display, every auction of the item with Current Bid / Buyout Price / Available / time-left band (`GetItemBuyListLayout`, :1094).
- [x] Selecting an auction prefills the bid with its next minimum bid; Bid sends `PlaceBid` when the amount is at least that and affordable.
- [x] Buyout opens the buy dialog ("Name  xN", price, Buy Now / Cancel, Shared/Blizzard_AuctionHouseBuyDialog.xml); Buy Now sends `BuyoutAuction`.

### Sell (Shared/Blizzard_AuctionHouseSellFrame.xml, …ItemSellFrame.xml)

- [x] Item sell frame (4,69) 363×442 on `auctionhouse-background-sell-left` with the Create Auction tab, item display, Quantity (+ Max), Buyout Price, Bid Price (only with Buyout Mode unchecked), Duration 1 Day / 1 Week / 2 Weeks (default 1 Week), Deposit, Total Price, Create Auction, Buyout Mode check (default checked).
- [x] Prices are per item; posting sends `CreateAuction` for the stack with bid and buyout × quantity (buyout mode: bid = buyout); Create Auction is enabled only with an item, 1..stack quantity, a price, buyout ≥ bid and money for the deposit. Posting clears the item.
- [x] Deposit shows the server's deposit: vendor price × quantity × 1 / 2 / 4.
- [x] Right-hand list (368,69) 427×442: without an item, the sellable items from `QueryAuctionInventory` (click to put one in the sell slot); with an item, that item's current auctions (a search by its name). Clicking the item display clears the item.

### Auctions (Mainline/Blizzard_AuctionHouseAuctionsFrame.xml)

- [x] Auctions / Bids top tabs, summary list with the All Auctions / All Bids line, list (172,72) 623×439: Item / Bid Price / Buyout Price / time left (`GetAllAuctionsLayout` :969, `GetBidsListLayout` :983).
- [x] Auctions: Cancel Auction (158×22 at BOTTOMRIGHT (-3,-22)) sends `CancelAuction` for the selected auction; disabled when it has bids (the server refuses).
- [x] Bids: bid and buyout controls as in the item buy frame.
- [x] Any successful operation re-queries money, owned auctions, bids and the last search.

## Tests asserting this spec

- `cargo test --lib -- auction item_catalog` — frame layout and content per tab from concrete view states, money parts, Retail positions; network state reply pairing, error collection; `Item.csv` parsing.
- `cargo test --bin game-engine -- auction_house_frame auctioneer_role` — auctioneer interaction → `OpenAuctionHouse` → window and loads, close / server close, tabs, search grouping and category filter, and bid / buyout / create / cancel reaching a loopback server peer as their protocol messages.

## Verified scope (2026-09-24, shared dev server, headless client)

Evidence: `data/diagnostics/auction-ui-20260924/` (scripts `r*.js`, `run.sh`; per run a screenshot, `dump-ui-tree` and `auction status`). Character Auctionhand next to Auctioneer Fitch (Stormwind Trade District); the interaction is sent with `game-engine-cli quest interact --npc` (the right-click request, without the mouse ray cast); all frame input is JS UI automation clicks and keys.

- [x] r1: the frame opens from the auctioneer interaction; money 10g, sellable items loaded.
- [x] r2: Sell tab, Linen Cloth ×20 in the slot, quantity 5, buyout 2s: deposit 1s 30c, total 10s.
- [x] r3: Create Auction posts it (server: `#1 Linen Cloth x5 bid=1000 buyout=1000`, gold 99870); it shows on the Auctions tab.
- [x] r4: search "linen" + Enter lists Linen Cloth at 2s per item, 5 available.
- [x] r5: item buy frame lists the auction; selecting it prefills the bid with 10s.
- [x] r6: Cancel Auction: owned 0, the item comes back by mail, Cancel Auction disabled.
- [x] r7 / r8: Buyout opens the buy dialog; Buy Now reaches the server, whose "cannot buy out your own auction" shows in the error frame over the auction frame.
- [x] r9: Auctions tab after the selection/top-tab fixes (r3's screenshot predates them).
- [ ] Not live: mouse right-click on the NPC, Escape/close button, server-side close on walking away, category clicks, Bids tab (needs a second character bidding), buyout by another character.

## Not yet

- Commodities (unit-price quantity buying, `CommoditiesBuyFrame` / `CommoditiesSellFrame`): the protocol has only per-stack auctions.
- Favorites, the Filter dropdown (usable only, rarity, level range — the query supports them), sort headers, sub-categories, WoW Token, the refresh button, scrolling past the visible rows (20 browse / 13 item buy / 20 lists).
- Bag right-click or drag to the sell slot: bag slots carry no item GUID; the sell list offers `QueryAuctionInventory` items instead.
- NPC portrait (`SetPortraitToUnit("npc")`), circular item button mask, `DialogBorderDarkTemplate` (the dialog and duration menu use the static popup border), `WowStyle1DropdownTemplate` art (the duration dropdown uses `auctionhouse-ui-dropdown-*`), upside-down top tab art (the toolkit rejects reversed `tex_coords`).
- Exact time left for owned auctions (the protocol sends a band).

## Offline Retail layout audit — 2026-10-08

Scope: geometry, art, fonts and colours only. Trading state, requests, sorting and tooltip record-ID lines stay unchanged. Source root is `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`. `AH/` below means `Blizzard_AuctionHouseUI/`. Findings describe authored code at `0374279c8`; native measurements require fresh captures, not historical exit codes. The offline harness is documented in [native debug screens](native-debug-screens.md#offline-auction-layout-snapshots).

| View / element | Cached Retail value and citation | Current comparison / remaining work |
| --- | --- | --- |
| Root / chrome | 800×538, `ButtonFrameTemplate`: `AH/Shared/Blizzard_AuctionHouseFrame.xml:4-5` | Authored dimensions match; shared skin-aware metal chrome retained. |
| Browse categories | List168×438, button132×21, normal art136×32 at(-2,0): `AH/Mainline/Blizzard_AuctionHouseCategoriesList.xml:4-17,60-90` | Authored sizes/positions match. Subcategory hierarchy and scroll bar absent; implementing navigation/scroll behaviour is excluded. |
| Search control | 241×22: `AH/Shared/Blizzard_AuctionHouseSearchBar.xml:4-5`; bar618×40:49; root TOPRIGHT(-12,-29): `AH/Shared/Blizzard_AuctionHouseFrame.xml:72-76` | Authored241×22 already matches. Historical native241×25 is not a reason to change authored height. |
| Search font / placeholder | `GameFontHighlightSmall` overrides `ChatFontNormal`, instructions `GameFontDisableSmall`, explicit grey0.35: `Blizzard_SharedXML/Shared/InputBox/InputBoxTemplates.xml:177-203`; FRIZQT__10: `Blizzard_Fonts_Shared/Shared/Fonts.xml:39-45`, `FontStyles.xml:56,92-94` | Baseline generic money-input helper supplied ArialNarrow14; placeholder was Friz12/grey0.5. Corrected search alone to Friz10/grey0.35. Native baseline measured25px high in both skins; GREEN rendering pending. |
| Search magnifier | 10×10 at LEFT(1,-1), unloaded tint0.6: `InputBoxTemplates.xml:212-218`; `InputBoxTemplates.lua:175-183` | Geometry matches; authored tint is white, not empty-search grey. Focus-specific tint is not represented in the auction view state. |
| Search button / filters | Search132×22: `AH/Shared/Blizzard_AuctionHouseSearchBar.xml:39-40`; favorites/filter anchors:49-69 | Search button matches. Favorites/filter absent; their functional implementation is excluded. |
| Browse headers / rows | Headers19 high, left4/right26, rows20: `AH/Mainline/Blizzard_AuctionHouseItemList.xml:4-5,28-30,55-70`; Price168, Name fill, Available60, favorite29 in copper mode: `AH/Shared/Blizzard_AuctionHouseTableBuilder.lua:1038-1061` | Authored geometry matches. Native paging reserves bottom rows; no Retail scrolling/refresh implementation. |
| Row typography / icons | `Number14FontWhite`, icon14×14/border16×16 and name offset4: `AH/Shared/Blizzard_AuctionHouseTableBuilder.xml:21-24,125-149` | Authored ArialNarrow14, icon and quality-colour representation match. Blank icon deliberately remains blank. Hover additive art is not implemented. |
| Paging | Retail list uses `WowScrollBoxList` and `MinimalScrollBar`: `AH/Mainline/Blizzard_AuctionHouseItemList.xml:62-75` | **Open:** local Prev/Next and server-page controls are client additions. Replacing paging with scrolling changes behaviour and is excluded. |
| Item Buy header | Back110×22, display622×86: `AH/Shared/Blizzard_AuctionHouseItemBuyFrame.xml:7-30` | Authored header/back/list anchors match. |
| Circular item icon | Button54×54, icon46×46, UV0.078125..0.921875, inset2 portrait mask: `Blizzard_ItemButton/Mainline/ItemButtonTemplate.xml:23-41` | Corrected baseline50×50/full-UV square to46×46 with the Retail crop,42×42 mask inset2 and68px quality ring; removed non-Retail square empty-slot backing. Mask uses the existing native icon pipeline. GREEN rendering pending. |
| Item Buy columns / bands | Bid120, Buyout140, Available fill, info24, band140 (10px each side): `AH/Shared/Blizzard_AuctionHouseTableBuilder.lua:1094-1106`; widths: `AH/Mainline/Blizzard_AuctionData.lua:1-2` | Authored band is already140, displayed text width120. Previous blanket50px finding is not applicable to current authored Item Buy. |
| Bid inputs / buttons | Bid frame240×22: `AH/Shared/Blizzard_AuctionHouseSharedTemplates.xml:127-143`; copper-visible MoneyInput frame176 wide: `Blizzard_MoneyFrame/Shared/MoneyInputFrame.lua:2-4`; child widths70/48/48, gaps10: `Blizzard_MoneyFrame/Mainline/MoneyInputFrame.xml:72-130` | **Open:** button uses x+186, not parent MoneyInput RIGHT at176. Inputs use search-border art rather than `Common-Input-Border`; edit widths subtract14 instead of using full Retail width/text insets. Do not infer130px from BidFrame240. |
| Sell slot / quantity | ItemDisplay342×72, GiantItemButton54×54; Quantity134×33, Max75×22: `AH/Shared/Blizzard_AuctionHouseSellFrame.xml:51-69,211-216`; secondary bid height20/topPadding5: `AH/Shared/Blizzard_AuctionHouseItemSellFrame.xml:19-26` | Authored rectangles match. Native sellable-inventory substitute for bag drag/right-click is preserved. |
| Sell prices / deposit / post | LargeMoneyInput190×33: `AH/Shared/Blizzard_AuctionHouseSellFrame.xml:74-83`; duration anchor19/-2:124-130; deposit/total left-aligned18 after label:140-149; Post194×22:259-265 | Authored price/deposit/post rectangles match; calculation/validation unchanged. |
| Duration dropdown | `WowStyle1DropdownTemplate`120×25, background(-8,-7)..(+8,+9), arrow27×27 RIGHT(1,-3), text left8/top8: `Blizzard_Menu/Mainline/MenuTemplates.xml:3-28` | Corrected baseline150×26/obsolete AH crops to120×25 and active-skin `common-dropdown-textholder` / `common-dropdown-a-button`, including background/arrow/text offsets. GREEN rendering pending. |
| Sell competing listings | Bid120; Buyout fill (left pad20 in copper mode); non-equipment quantity60/right pad10. Equipment adds socket24/level50; pets owner90: `AH/Shared/Blizzard_AuctionHouseTableBuilder.lua:1112-1141` | **Open:** current Bid110/Buyout120/Available fill/time90 differs. Equipment/pet-specific metadata is not in `SellItemView`; do not invent it. |
| Owned / Bids list top | Top tabs at local(47,-1), summary begins2 up from their bottom; list TOP one down from summary: `AH/Mainline/Blizzard_AuctionHouseAuctionsFrame.xml:56-62,90-96,135-152` | **Open:** tab y43 +32 −2 gives summary y73; list should y74, not current y72. This is a2px authored anchor error. |
| Owned / Bids time | Owned exact-time column50: `AH/Shared/Blizzard_AuctionHouseTableBuilder.lua:969-977`; Bids band140:983-991 | Authored widths already match. Owned receives protocol bands, not exact time; clipping there is a data-contract gap, not evidence that Retail owned width is140. |
| Bottom tabs / wallet | Buy BOTTOMLEFT(20,-28), text+40/min70: `AH/Shared/Blizzard_AuctionHouseFrame.xml:44-67`, `Blizzard_AuctionHouseTab.lua:2-11`; wallet158×19 BOTTOMLEFT(5,6): `Blizzard_AuctionHouseFrame.xml:24-35` | Authored geometry matches. Skin-aware chrome/close/buttons retained. |
| Existing420px dialog art | `AuctionHouseBuyDialogTemplate`420 wide, item17 down, money6 below text, buttons120×22/bottom18: `AH/Shared/Blizzard_AuctionHouseBuyDialog.xml:41-90`; height100: `Blizzard_AuctionHouseBuyDialog.lua:177`; dark texture inset7: `Blizzard_SharedXML/Shared/Dialog/DialogTemplates.xml:83-96` | **Open:** authored420×100 matches this template, but flat black/static-popup art does not. This template is the Retail **commodity** dialog, not proof of stack-buyout parity. |
| Actual stack buyout / bid confirmations | `StartItemBuyout` / `StartItemBid` use `BUYOUT_AUCTION` / `BID_AUCTION` static popups, Accept/Cancel, money and alert: `AH/Shared/Blizzard_AuctionHouseFrame.lua:26-56,844-865`; commodity path separately calls BuyDialog:844-848 | **Open:** client stack buyout uses commodity-shaped dialog; bids submit directly. Adding bid confirmation or changing trading flow is explicitly outside this visual-only task. Do not silently add it or equate420px commodity art with full stack parity. |
| Forever art selection | Supplied set1 art resolves before shared set0 (project skin contract); cached DB2 audit retained at `data/diagnostics/ahlayout-2026-10-08/skin-atlas-audit.json` | Auction sheets1495/1499 and tab2134 share FDIDs in both exports. DiamondMetal dialog has set1 c60 members; a future dialog-art correction must resolve active-skin atlas names, not hardcode Modern. |

First font/dropdown/circular-icon corrections are implemented after model RED (1 passed/3 failed) and16 inspected native baseline captures. No GREEN or full-parity acceptance yet. Other open rows remain open. Proof/results and commands belong to `data/diagnostics/ahlayout-2026-10-08/proof-ledger.txt`.

## Protocol and server gaps

- `AuctionSearchQuery` now carries optional exact `item_id` and `class_id`; native searches use them. The preserved Bevy category filter remains page-local.
- Listings carry no icon, item level or class: the client reads `Item.csv`.
- No buyout-only auctions: `min_bid` is required, so buyout mode posts `min_bid = buyout`.
- No deposit query: the client repeats the server formula.
- Query rejections ("not interacting with an auctioneer") arrive as `AuctionOperationResponse`, not as the query's own response type.


## Native Godot baseline (2026-09-30)

Native implementation and proof are separate from the historical Bevy checkboxes above.
See [native data flow](../wiki/systems/auction-house-ui.md#native-godot).

### What it must do

- [x] Portable session gates actions until the house opens, loads inventory/money, owned auctions and bids, refreshes after success, surfaces server rejection messages, and discards replies after close. Changed replicated player Gold also updates displayed money and affordability while open (including other bidders' refunds); repeating an unchanged entity balance does not overwrite a newer inventory reply.
- [x] Category searches send server `class_id`; selecting a browse/sell item sends exact `item_id`, not a name substring or page-local category filter. Search previous/next keeps its filters; Back restores the browse query.
- [x] Buy browse Name / Price and item Buyout Price headers request server sorting. Clicking the current ascending field switches to descending; other clicks start ascending. Sorting retains text/category/exact-item filters and resets server/local pages and selection. Available sorting and subcategory filtering have no protocol fields. Current Bid sorting is not offered: server `MinBid` sorts the starting bid, not the current bid. No page-local or differently named substitutes.
- [x] Every fetched browse, item-auction, inventory, owned and bids row is reachable through local row paging. Native browse sends `QueryAuctionBrowse`; server pages over distinct items and supplies global eligible-item `lowest_unit_price` (ceiling integer copper) and `total_quantity` (`u64`). Display uses these values unchanged, never sums or prices flat client-page listings. Drilldown/sell-market queries remain flat `QueryAuctions` with exact `item_id`; selection and trading retain real auction IDs.
- [x] Bid prefills the next minimum; affordable bids/buyouts and owned cancellation without bids emit their requests. Sell validates stack quantity, per-item bid/buyout totals and deposit; 1 Day / 1 Week / 2 Weeks retain deposit multipliers 1 / 2 / 4. Operations awaiting a reply disable conflicting trading actions.
- [x] After saved game-cli proof, native NPC pointer entry, authored selection/editing, posting all three durations, own-buyout rejection, cancellation, buyer bid/buyout and Escape/close pass in the owned loopback fixtures. See [saved runtime evidence](../wiki/systems/auction-house-ui.md#saved-native-runtime-proof-2026-10-01).
- [x] Native large-market browse exposes local rows and disjoint 50-item server pages; category filtering resets the page and filters global results.
- [x] Native server range-close passes on the private integrated server (2026-10-07 acceptance below); prior fixture-only scope remains historical.

### Implementation inventory

- `godot/ui-model/src/auction.rs`, `auction/{actions,view}.rs` — portable session, request/reply state, validation and page-limited views.
- `godot/rust/src/auction.rs`, `account.rs`, `merchant.rs` — native registry host, NPC/gossip interaction and protocol routing.
- `godot/rust/src/ui/mod.rs`, `godot/network/src/lib.rs` — native projection/inputs and all auction reply relays.
- `src/ui/screens/auction_house_frame_component*.rs` — shared authored frame and corrected duration labels.

### Tests asserting this spec

- `godot/ui-model/tests/native_auction.rs` — trading, deposits/durations, gate/rejection/close, authoritative global price/`u64` stock, distinct-item second page, exact flat drilldown and real-ID buyout, stale-query rejection and all list-page traversal; run with `scripts/depot-build.py --test -p game-engine-ui-model --test native_auction`.
- `godot/network/src/wire_tests.rs::native_bridge_auction_operations_and_query_rejections` — owned loopback UDP: ten request types and seven reply relays, including global item browse with stock beyond `u32`, unchanged flat drilldown and query rejection.
- `godot/tests/world_auction_flow.gd` — main-owned ordered native pointer smoke. Requires `GODOT_AUCTION_CLI_PROVED=1`, `GODOT_TEST_SERVER` loopback, `GODOT_AUCTION_NPC`, `GODOT_AUCTION_MODE=seller|buyer`; startup client flags select the disposable authenticated roster character. Seller needs three sellable items and deposits; buyer requires two distinct other-player auction IDs in `GODOT_AUCTION_BID_ID` and `GODOT_AUCTION_BUYOUT_ID`. Optional `GODOT_AUCTION_ITEM_ID` defaults to 2589. No embedded credentials/server setup. Fixture mutations are confined to main's owned disposable server. Seller run 3 and buyer run completed after saved CLI proof; exact receipts are linked from the wiki.
- `godot/tests/world_auction_browse_flow.gd` — native local-row paging, disjoint 50/50 server pages and global category filtering on the CLI-proved large market.

### Known gaps (current cycle)

- [ ] Title dragging and Wide-window replacement remain runtime-unproved. Private range-close now passes; saved older trading/browse evidence remains bounded to its linked fixtures.
- [x] Global browse implemented at `5b9cb76c`; targeted Depot model 8/8, owned UDP 1/1 and shared frame 1/1 passed. Later native large-market proof covers 311 global groups, 87 category groups and disjoint 50/50 server pages; it is not a universal performance claim.

## Private live acceptance — 2026-10-07

Evidence: canonical `data/diagnostics/ahloop-2026-10-07/` (paths below), especially `proof-ledger.txt`, `server-findings.txt`, `whole-crate-summary.json` and the inspected image sheets. Local run date is CDT; server timestamps are UTC. Private copy of deployed server `26f3c5c`, UDP5340, fresh private player storage and admin socket; accounts `fb_ah1`, `fb_ah2`, `fbtest` only. No shared-server administration, merge or push.

Final Rust code **`ab65b0fa6`**: native Name/Price/Buyout header fix (`4f63e7530`, corrected by `ab65b0fa6`), live replicated money fix (`2a0f56de2`, `7b278780f`, `37f13af74`). Native build/CLI and required entire Godot/UI-model/core/network/CLI crate gate pass: **2,353 passed, 0 failed, 6 ignored**. `native_auction_sort_preserves_filters_and_resets_pages` and `native_auction_live_refund_updates_money_and_bid_affordability` pass. The former header failure and real three-account stale-refund failure are retained; subsequent native runs re-prove both fixes on the final artifact.

| Acceptance step | Full-scope result | Observed boundary and inspected captures |
| --- | --- | --- |
| 1 Lifecycle | **FAIL full scope; direct lifecycle PASS** | Real pointer opens Fitch directly. X/Escape, actual walking range-close, reopening and "You are too far away." pass. No gossip-option selection demonstrated. `green-npc-pointer-open`, `green-final-close-x`, `green-final-escape`, `forever-range-closed`, `forever-too-far` PNG/JSON; `lifecycle-closed-ipc-ui.txt`, `forever-range-error-ipc-ui.txt`. |
| 2 Buy | **FAIL full scope; supported queries/sorts PASS** | All14 categories produce paired server replies (some empty). Seeded51 distinct groups give disjoint50/1 pages and back; explicit empty text search passes. Native Name descending, Price ascending and nonempty exact-item Buyout sort pass. Two subclass queries cannot be encoded; Available and Current Bid sort are not approximated by different fields. `current-category-*`, `current-page-two/back`, `current-empty-text-search`, `green-name-sort`, `green-price-sort`, `green-item-price-sort-valid` PNG/JSON; `buy-empty-ipc-ui.txt`. |
| 3 Bid/buyout | **PASS functional; server text finding open** | Bid200 accepted, third bidder210 refunds200 copper and sends Outbid mail24. Buyer pays1000; won mail27 contains Linen×2, taken/merged78→80. Seller mail26 pays1002 (950 proceeds +52 deposit), claimed with exact balance delta. Own/below-minimum server rejections and selected unaffordable-bid UI guard pass. Final authentic other-account outbid177 restores110 copper while AH stays open: displayed/replicated money1000085→1000195, without requery/reopen. `forever-outbid-visible-body`, `forever-won-item-taken`, `green-seller-proceeds-after`, `green-authentic-refund`, negative-outcome PNG/JSON; mail/take/bags IPC receipts. Outbid body incorrectly calls200 copper "200 gold". |
| 4 Sell | **FAIL binding probe; other sell cases PASS** | Qty2, unit200, stack400; all three duration labels charge exact26/52/104 deposits and remove2 items. Zero and buyout-below-bid blocked. Server exposes/posts starter Hearthstone6948 despite staged BoP metadata (`bonding=1`): auction175. No client filtering workaround; bound rejection not established. `current-duration-*-before/after`, `current-sell-zero-after`, `current-sell-buyout-below-bid`, `current-hearthstone-after` PNG/JSON; `hearthstone-server-finding.json`, `modern-sell-final-ipc-ui.txt`. |
| 5 Owned/Bids | **PASS cancellation/list; expiry N/A** | Owned/Bids lists, existing-bid cancel disabled, unbid169 cancel removes listing with balance unchanged (deposit forfeited). Mail25 returns Linen×2, taken67→69. Admin `clock-advance86400` explicitly advances instance-lock time, not auction wall time; no auction-expiry advance exposed. `modern-owned-final`, `modern-bids-final`, `green-cancel-return-taken` PNG/JSON; return/bags IPC, `expiry-clock-request-response.txt`. |
| 6 Layout | **FAIL parity; both-skin audit complete** | Modern and Forever Buy/Sell/Owned/Bids/ItemBuy opened and inspected on final artifact. Root800×538 matches. Search control is241×25, not241×22; Bids/ItemBuy time-band column50, not cached Retail140, causing clipping; duration and buy-dialog art use older/static-popup templates rather than the cached templates. NPC portrait renders after warm-up; early placeholder is not a final defect. `inspected-final-modern-*`, `inspected-final-forever`, corresponding final-view PNG/JSON and IPC trees; `retail-layout-reference.json`, `modern-layout-measurements.json`. |

Server findings retain actual request/response data. This server emits no INFO operation-audit line for successful auction mutations; absence is documented rather than fabricated. Protocol-unrepresentable requests have request/response/log **N/A**. User-requested tooltip record-ID lines are unchanged. Overall acceptance remains **Partial**, not full Retail parity.
