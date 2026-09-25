# Auction house frame

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

- [x] Item sell frame (4,69) 363×442 on `auctionhouse-background-sell-left` with the Create Auction tab, item display, Quantity (+ Max), Buyout Price, Bid Price (only with Buyout Mode unchecked), Duration 12 / 24 / 48 Hours (default 24), Deposit, Total Price, Create Auction, Buyout Mode check (default checked).
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

## Protocol and server gaps

- `AuctionSearchQuery` has no category / item class; category filtering is client-side over the fetched page (≤ 50 results, `MAX_PAGE_SIZE`).
- Listings carry no icon, item level or class: the client reads `Item.csv`.
- No buyout-only auctions: `min_bid` is required, so buyout mode posts `min_bid = buyout`.
- No deposit query: the client repeats the server formula.
- Query rejections ("not interacting with an auctioneer") arrive as `AuctionOperationResponse`, not as the query's own response type.
