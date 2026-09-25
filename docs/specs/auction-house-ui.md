# Auction house frame

Economy row of the [in-game UI plan](../plans/2026-09-23-ingame-ui.md): Retail `AuctionHouseFrame`
opened from an auctioneer, on the existing auction protocol (`shared-protocol`
`protocol/gameplay_messages.rs`) and the server auction house (`game-server`
`auction_house/`). Reference: `~/.cache/wow-ui-sim/blizzard-ui/Blizzard_AuctionHouseUI`
(paths below are relative to it). Code: `src/ui/screens/auction_house_frame_*.rs` (layout),
`src/scenes/auction_house_frame/` (state, actions, window), `src/auction_house.rs` (network).

## What it must do

### Opening and closing

- [ ] Right-clicking an auctioneer sends `InteractNpc`; `InteractionOpened` with `Role(AuctionHouse)` (directly or from a gossip option, which closes the greeting) sends `OpenAuctionHouse`.
- [ ] `AuctionHouseOpened { success }` opens the frame as a Wide window (`UIPanelWindows` `area = "doublewide"`, Shared/Blizzard_AuctionHouseFrame.lua:8) and loads money (`QueryAuctionInventory`), owned auctions and bids. No search runs on open.
- [ ] The close button, Escape or another Wide window close the frame and send `CloseInteraction`; `InteractionClosed` from the server (walked away) closes it without one.
- [ ] A failed operation shows its server message in the UI error frame.

### Frame (Shared/Blizzard_AuctionHouseFrame.xml)

- [ ] Root `AuctionHouseFrame`, 800×538 (:5), `ButtonFrameTemplate` metal chrome (`quest_art::window_chrome`), movable by its title.
- [ ] Title per tab: Browse Auctions / Post Auctions / Auctions (`UpdateTitle`, Shared/Blizzard_AuctionHouseFrame.lua:640).
- [ ] Bottom tabs Buy, Sell, Auctions (`PanelTabButtonTemplate` art, Buy at BOTTOMLEFT (20,-28), each next -15 over, width text + 40 min 70, Shared/Blizzard_AuctionHouseTab.lua:2-11); selected tab text white.
- [ ] Money: `MoneyFrameInset` and 158×19 `ThinGoldEdgeTemplate` border at BOTTOMLEFT (5,6), money right-aligned 6 in (:11-40). Money displays follow `MoneyDisplayFrameMixin`: silver and copper always, gold only when non-zero, thousands separators, AH coin atlases.

### Buy

- [ ] Search bar at TOPRIGHT (-12,-29): search box 241×22 with "Search" instructions, Search button 132×22 (Shared/Blizzard_AuctionHouseSearchBar.xml). Enter in the box searches.
- [ ] Category list 168×438 at (4,73) on `auctionhouse-background-categories`, `AuctionCategoryButtonTemplate` 132×21 buttons (Mainline/Blizzard_AuctionHouseCategoriesList.xml); the 14 top-level Retail categories (Mainline/Blizzard_AuctionData.lua) filter results by `Item.ClassID`; clicking the selected one clears it.
- [ ] Browse list (172,74) 623×437 on `auctionhouse-background-index`, columns Price 168 / Name / Available 60 (`GetBrowseListLayout`, Shared/Blizzard_AuctionHouseTableBuilder.lua:1033): one row per item with its lowest per-item price (buyout, else bid) and total quantity; name in the item's quality colour with its icon (`Item.IconFileDataID`); "No items found" after an empty search.
- [ ] Clicking an item opens the item buy frame (Shared/Blizzard_AuctionHouseItemBuyFrame.xml): Back, the 622×86 item display, every auction of the item with Current Bid / Buyout Price / Available / time-left band (`GetItemBuyListLayout`, :1094).
- [ ] Selecting an auction prefills the bid with its next minimum bid; Bid sends `PlaceBid` when the amount is at least that and affordable.
- [ ] Buyout opens the buy dialog ("Name  xN", price, Buy Now / Cancel, Shared/Blizzard_AuctionHouseBuyDialog.xml); Buy Now sends `BuyoutAuction`.

### Sell (Shared/Blizzard_AuctionHouseSellFrame.xml, …ItemSellFrame.xml)

- [ ] Item sell frame (4,69) 363×442 on `auctionhouse-background-sell-left` with the Create Auction tab, item display, Quantity (+ Max), Buyout Price, Bid Price (only with Buyout Mode unchecked), Duration 12 / 24 / 48 Hours (default 24), Deposit, Total Price, Create Auction, Buyout Mode check (default checked).
- [ ] Prices are per item; posting sends `CreateAuction` for the stack with bid and buyout × quantity (buyout mode: bid = buyout); Create Auction is enabled only with an item, 1..stack quantity, a price, buyout ≥ bid and money for the deposit. Posting clears the item.
- [ ] Deposit shows the server's deposit: vendor price × quantity × 1 / 2 / 4.
- [ ] Right-hand list (368,69) 427×442: without an item, the sellable items from `QueryAuctionInventory` (click to put one in the sell slot); with an item, that item's current auctions (a search by its name). Clicking the item display clears the item.

### Auctions (Mainline/Blizzard_AuctionHouseAuctionsFrame.xml)

- [ ] Auctions / Bids top tabs, summary list with the All Auctions / All Bids line, list (172,72) 623×439: Item / Bid Price / Buyout Price / time left (`GetAllAuctionsLayout` :969, `GetBidsListLayout` :983).
- [ ] Auctions: Cancel Auction (158×22 at BOTTOMRIGHT (-3,-22)) sends `CancelAuction` for the selected auction; disabled when it has bids (the server refuses).
- [ ] Bids: bid and buyout controls as in the item buy frame.
- [ ] Any successful operation re-queries money, owned auctions, bids and the last search.

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
