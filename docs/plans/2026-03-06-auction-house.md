# Auction House Implementation Plan

**Goal:** Add a WoW-style auction house feature spanning world interaction, client UI, network protocol, and server-authoritative auction logic.

**Scope:** This plan covers the first usable loop:
- interact with an auctioneer NPC,
- access the same feature set from the command line,
- open a browse/create/bids UI,
- search listings,
- place bids and buyouts,
- create and cancel auctions,
- receive results through mailbox-style settlement messages.

**Non-goals for v1:**
- cross-faction market splits,
- commodity stack aggregation matching retail WoW,
- neutral AH cut differences,
- anti-bot protections beyond basic rate limiting,
- full item tooltip/stat sourcing.

## Why This Shape

Auction house logic cannot live only in `game-engine`. The client can render UI and send intents, but listing ownership, gold movement, bid resolution, expiry, and item delivery must be server-authoritative. The practical split is:

- `game-engine`: world interaction, UI state, request/response protocol, local presentation.
- `game-engine-cli` + IPC: automation and debug access to browse, create, bid, buyout, cancel, and inspect session state without driving the UI manually.
- `game-server`: auction persistence, validation, search indexes, settlement, mailbox delivery.
- `shared`: replicated data types and request/response messages.

## Player Flow

### Browse

1. Player targets or interacts with an auctioneer NPC.
2. Client sends `OpenAuctionHouse` request.
3. Server validates range, NPC flags, and faction access.
4. Client opens Auction House frame with default browse results.
5. Player filters by name, class, rarity, level, usable-only, and sort order.
6. Client sends search query and paginates through results.

### Bid / Buyout

1. Player selects a listing.
2. UI shows owner, stack count, current bid, minimum next bid, buyout, time left.
3. Player places a bid or buyout.
4. Server validates gold, listing state, and bid increment rules.
5. Server updates auction state and returns an operation result.

### Sell

1. Player drags an item from inventory into the sell slot.
2. UI computes deposit preview from vendor value, stack size, duration, and auction rules.
3. Player enters stack size, starting bid, buyout, and duration.
4. Client sends `CreateAuction`.
5. Server locks/removes the item, reserves deposit, creates the listing, and returns success.

### Cancel / Settlement

1. Seller can cancel only if there is no active bidder, or pays the correct penalty if you later support that rule variant.
2. When an auction expires or is bought out, server resolves winner, fees, and returned items.
3. Results are delivered through mailbox records rather than trying to mutate inventory while the user may be offline.

## Phase 1: Shared Protocol and Domain Model

**Goal:** Define stable auction concepts so client and server agree on shapes before UI or storage work starts.

### 1a. Shared data types

**Target:** `../game-server/crates/shared/src/`

Add:
- `AuctionId`
- `AuctionHouseId` or `AuctioneerNpcId`
- `AuctionDuration`
- `AuctionTimeLeft`
- `AuctionSortField`
- `AuctionSortDir`
- `AuctionFactionScope`
- `AuctionOwnerSummary`
- `AuctionListingSummary`
- `AuctionListingDetails`
- `AuctionSearchQuery`
- `AuctionSellItemRef`
- `AuctionOperationResult`

Rules to encode:
- listing IDs are opaque server-generated IDs,
- money uses copper integers,
- durations are explicit enums (`Short`, `Medium`, `Long`),
- browse payloads contain only UI-needed summary fields,
- item instances are referenced by inventory bag/slot or item GUID, not copied blindly.

### 1b. Network messages

Add request/response messages:
- `OpenAuctionHouse`
- `AuctionHouseOpened`
- `QueryAuctions`
- `AuctionSearchResults`
- `PlaceBid`
- `BuyoutAuction`
- `CreateAuction`
- `CancelAuction`
- `QueryOwnedAuctions`
- `QueryBidAuctions`
- `AuctionOperationResponse`

Optional follow-up events:
- `AuctionOutbidNotice`
- `AuctionSoldNotice`
- `AuctionWonNotice`

### 1c. Client/server registration

Wire the new messages into the existing Lightyear protocol plugin with reliable ordered channels. Auction traffic is transactional and should not use the movement/input channel.

**Deliverable:** `shared` compiles with auction protocol types and a small serialization test set.

## Phase 2: Server Auction Core

**Goal:** Build the authoritative auction subsystem without any client assumptions.

### 2a. Auction resource and components

**Target:** `../game-server/crates/server/src/`

Add an `AuctionHousePlugin` with:
- `AuctionRepository` resource for active listings,
- `AuctionIndex` resource for search/filter access,
- `PendingAuctionMail` queue for settlement output,
- server systems for create, bid, buyout, cancel, expire.

Each auction record should contain:
- auction ID,
- item instance reference or cloned immutable item snapshot,
- seller character ID,
- bidder character ID optional,
- stack size,
- bid amount,
- buyout amount optional,
- deposit paid,
- duration,
- created-at and expires-at timestamps,
- auction house/faction scope.

### 2b. Validation rules

Implement explicit validation for:
- player is interacting with an auctioneer and within range,
- item is tradable and not soulbound,
- stack count is valid,
- seller owns the item and it is not already reserved,
- starting bid and buyout are sane,
- bidder has enough gold,
- next bid meets minimum increment,
- seller cannot bid on own listing,
- listing is still active and unexpired.

### 2c. Settlement model

Use mailbox-style settlement from the start:
- expired without bid: return item by mail,
- sold: mail gold minus cut to seller,
- won: mail item to buyer,
- outbid: refund previous bidder by mail or immediate gold unlock depending on how your character economy is built.

This avoids awkward live inventory mutation and matches WoW semantics better.

### 2d. Search and pagination

Start with simple indexed filtering:
- case-insensitive name prefix/contains,
- class/subclass,
- rarity,
- level range,
- usable-only optional,
- page + page size,
- sort by unit price, total buyout, time left, name, required level.

Do not start with SQL-like freeform queries. Keep it deterministic and bounded.

**Deliverable:** server-only tests covering create, bid, buyout, expiry, cancel, and refund/sale mail generation.

## Phase 3: Client Interaction and Auctioneer Entry Point

**Goal:** Make the auction house open from the 3D world through existing targeting/network patterns.

### 3a. Auctioneer identification

**Targets:** [src/target.rs](/syncthing/Sync/Projects/wow/game-engine/src/target.rs), [src/networking.rs](/syncthing/Sync/Projects/wow/game-engine/src/networking.rs), possible NPC metadata source in `game-server`

Add an auctioneer marker in replicated NPC data or a generic NPC flags field. The client needs only enough metadata to show an interaction prompt and send an `OpenAuctionHouse` request.

### 3b. Interaction flow

When the player activates an auctioneer:
- validate that an NPC is targeted or hovered,
- send `OpenAuctionHouse`,
- on success insert an `AuctionHouseSession` resource with session metadata,
- on failure surface a small UI error message.

### 3c. Session lifetime

Close the AH session if:
- the player walks out of range,
- the target despawns,
- the server rejects continued access,
- the user closes the frame.

**Deliverable:** interacting with an auctioneer opens an empty or stubbed AH frame driven by server approval.

## Phase 3.5: Command-Line Access

**Goal:** Make auction-house operations accessible through the existing IPC + CLI path for testing, automation, and headless workflows.

### 3.5a. IPC request/response expansion

**Targets:** [src/ipc/mod.rs](/syncthing/Sync/Projects/wow/game-engine/src/ipc/mod.rs), [src/ipc/plugin.rs](/syncthing/Sync/Projects/wow/game-engine/src/ipc/plugin.rs)

Add IPC requests such as:
- `AuctionOpen`
- `AuctionClose`
- `AuctionBrowse { query, page }`
- `AuctionOwned`
- `AuctionBids`
- `AuctionCreate { item_ref, stack_count, bid, buyout, duration }`
- `AuctionBid { auction_id, amount }`
- `AuctionBuyout { auction_id }`
- `AuctionCancel { auction_id }`
- `AuctionStatus`

Add matching responses:
- `AuctionSessionOpened`
- `AuctionSearchResults`
- `AuctionOwnedResults`
- `AuctionBidResults`
- `AuctionOperationResult`
- `AuctionStatus`

The IPC layer should remain a thin bridge. It should dispatch into the same in-engine auction systems used by the UI so the command line is not a second implementation.

### 3.5b. CLI commands

**Target:** [src/bin/game-engine-cli.rs](/syncthing/Sync/Projects/wow/game-engine/src/bin/game-engine-cli.rs)

Add subcommands like:
- `auction open`
- `auction browse --text linen --page 0 --min-level 10 --quality uncommon`
- `auction owned`
- `auction bids`
- `auction create --item <guid|bag:slot> --stack 5 --bid 12000 --buyout 18000 --duration medium`
- `auction bid --id <auction_id> --amount 15000`
- `auction buyout --id <auction_id>`
- `auction cancel --id <auction_id>`
- `auction status`

Output format should start human-readable. If you later want scripting support, add `--json` rather than making the default output machine-shaped.

### 3.5c. Session semantics

Two reasonable models exist:
- strict WoW mode: CLI operations require an active in-world AH session opened near an auctioneer,
- debug mode: CLI can bypass proximity checks behind an explicit dev-only flag.

Default to strict mode. If you add a bypass, make it explicit and clearly non-production, for example `auction open --debug-anywhere`.

### 3.5d. Why this matters

CLI access gives you:
- regression coverage without clicking through the UI,
- easier server/client protocol debugging,
- faster iteration while inventory and widget systems are still moving,
- future GM/admin tooling if you decide to expose read-only inspection commands server-side.

**Deliverable:** a running engine instance can be queried and driven via `game-engine-cli auction ...`, backed by the same auction-house systems as the UI.

## Phase 4: Client UI Frame

**Goal:** Implement the classic WoW auction-house workflow inside the engine's UI system.

### 4a. Plugin and state

**New file:** `src/auction_house.rs`

Add:
- `AuctionHousePlugin`
- `AuctionHouseState` resource
- `AuctionBrowseResults`
- `OwnedAuctionsState`
- `BidAuctionsState`
- `AuctionUiError`

Register the plugin from [src/main.rs](/syncthing/Sync/Projects/wow/game-engine/src/main.rs).

### 4b. Frame layout

Build one root frame with three tabs:
- `Browse`
- `Bids`
- `Auctions`

Browse tab:
- search box,
- filter controls,
- sortable results table,
- details pane,
- bid/buyout actions.

Bids tab:
- auctions the player is currently highest bidder on,
- auctions where the player was outbid,
- quick rebid or buyout.

Auctions tab:
- seller's active listings,
- create-auction pane,
- inventory item drop target,
- duration selector,
- stack count,
- starting bid,
- buyout,
- deposit and cut preview.

### 4c. UI primitives

Implement with the existing custom UI system, not a parallel ad hoc overlay. If the current widget set is not sufficient, the missing pieces are likely:
- table/list rows with selection state,
- tab buttons,
- money input/display widget,
- item slot widget with icon and count,
- scrollable results list,
- tooltip hook for item details.

### 4d. Incremental rendering approach

Do not try to build the full Blizzard frame in one step. Build:
1. bare root frame,
2. browse list,
3. row selection + details,
4. bid/buyout actions,
5. sell tab item slot and form,
6. owned/bids tabs.

**Deliverable:** functional AH UI using server-backed data, even if art/styling is still placeholder.

## Phase 5: Inventory and Item Integration

**Goal:** Connect auctions to actual item instances rather than fake rows.

### 5a. Inventory source of truth

If `game-engine` does not yet have a client-side inventory model, add a minimal resource that mirrors server inventory summaries for UI purposes. The sell tab only needs:
- item GUID,
- icon/path or FDID,
- display name,
- stack count,
- binding/tradability flags,
- vendor sell price,
- class/subclass,
- quality,
- required level.

### 5b. Sell slot behavior

Support:
- drag item into sell slot,
- split stack for partial listing,
- clear slot,
- refuse non-auctionable items,
- update deposit preview when duration or stack changes.

### 5c. Tooltip and icon loading

Icons can start with placeholder texture IDs if full inventory art is not ready. The key requirement is that item identity survives round trips cleanly.

**Deliverable:** player can post a real inventory item as an auction and see it removed/reserved correctly.

## Phase 6: Mailbox and Economy Closure

**Goal:** Finish the loop so auction outcomes resolve into player-visible results.

### 6a. Mail records

Ensure the server mailbox model can represent:
- returned item,
- won item,
- outbid gold refund,
- seller proceeds after auction cut.

### 6b. Client mailbox visibility

If mailbox UI already exists in the server/client roadmap, integrate with it. If not, at minimum expose an unread-mail indicator or debug command so auction results are observable during development.

### 6c. Fees and rules

Implement:
- deposit formula,
- house cut percentage,
- expiration sweep cadence,
- time-left buckets (`Short`, `Medium`, `Long`, `VeryLong` if needed for display).

Keep the formulas data-driven so neutral/faction auction houses can diverge later without rewrites.

**Deliverable:** completed auctions pay out correctly and expired auctions return items.

## Phase 7: Polish and Scale

### 7a. Anti-footgun limits

Add:
- server-side rate limiting on browse queries,
- max page size,
- debounced client search,
- optimistic UI disabled for monetary actions,
- structured error messages for insufficient funds / invalid listing / stale listing.

### 7b. Better presentation

Add:
- money formatting in gold/silver/copper,
- color-coded item quality,
- time-left labels matching WoW feel,
- tooltips for deposit and cut,
- disabled-state reasons on buttons.

### 7c. Persistence and cleanup

Verify auctions survive restart and that expiry continues correctly after downtime.

## Suggested File Layout

### game-engine

- New: `src/auction_house.rs`
- Modify: [src/main.rs](/syncthing/Sync/Projects/wow/game-engine/src/main.rs)
- Modify: [src/networking.rs](/syncthing/Sync/Projects/wow/game-engine/src/networking.rs)
- Modify: [src/target.rs](/syncthing/Sync/Projects/wow/game-engine/src/target.rs)
- Modify: [src/ipc/mod.rs](/syncthing/Sync/Projects/wow/game-engine/src/ipc/mod.rs)
- Modify: [src/ipc/plugin.rs](/syncthing/Sync/Projects/wow/game-engine/src/ipc/plugin.rs)
- Modify: [src/bin/game-engine-cli.rs](/syncthing/Sync/Projects/wow/game-engine/src/bin/game-engine-cli.rs)
- Optional UI additions under [src/ui/](/syncthing/Sync/Projects/wow/game-engine/src/ui)

### shared

- New or extend: `../game-server/crates/shared/src/protocol.rs`
- New or extend: `../game-server/crates/shared/src/components.rs`
- Possibly new: `../game-server/crates/shared/src/auction.rs`

### game-server

- New: `../game-server/crates/server/src/auction_house.rs`
- Modify: `../game-server/crates/server/src/networking.rs`
- Modify: mailbox/inventory/economy modules as needed

## TDD Slice Order

Implement in this order to keep the risk down:

1. Shared auction types + serialization tests.
2. Server create-auction validation tests.
3. Server bid/buyout/expiry tests.
4. Client open-session flow with a stubbed `AuctionHouseOpened`.
5. IPC + CLI browse/status commands against stubbed in-engine data.
6. Client browse results UI with hardcoded local data.
7. Replace local data with real server messages.
8. Inventory sell-slot integration.
9. Mail settlement visibility.

This sequence gets server correctness first, then UI integration, then economy closure.

## Risks

- The client currently appears to lack a mature inventory/economy UI model, so sell flow may expose broader missing systems.
- Mailbox support may exist only partially in `game-server`; settlement will stall if that layer is absent.
- Search/filter payloads can get large fast; pagination and compact summaries should be enforced from day one.
- If NPC interaction metadata is weak, auctioneer discovery may require a small replicated NPC-flags refactor first.

## Minimal v1 Definition

Ship v1 when all of this works:
- player can open the AH from an auctioneer NPC,
- the same AH session can be inspected and driven from `game-engine-cli`,
- browse paginated listings,
- place a bid,
- buy out a listing,
- post one real inventory item,
- cancel own listing with valid rules,
- expire/sold outcomes produce mailbox settlement,
- state persists across server restart.

Anything beyond that is polish, not the core feature.
