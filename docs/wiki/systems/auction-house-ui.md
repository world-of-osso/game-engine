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

Two paging levels are independent: server result pages preserve filters, while 18-row local slices (11 for the item buy list) expose every fetched row. Local paging also works on unpaged inventory/owned/bids replies. `AuctionRequest::Browse` sends `QueryAuctionBrowse`; `AuctionBrowseResults.items` populates `net.browse_results`. Each row uses server-global `lowest_unit_price` and `total_quantity` unchanged; browse pages and `total_results` count distinct items. No client-page aggregation or page-scoped price label remains. Category filtering sends `class_id` plus the selected node's class/subclass/inventory alternatives; drilldown and sell-market `AuctionRequest::Listings` sends flat `QueryAuctions` with `item_id`. Flat results populate `net.search_results` and keep real auction IDs. Refresh/page changes preserve the active request kind through `query_is_browse`; mismatched query or endpoint replies are ignored. `search_revision` increments only on accepted replies so smoke waits observe a reply, not merely a changed requested page.

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


## Missing result icons (2026-10-08)

Verified baseline: `a860f7df4`, canonical `data/diagnostics/ahicons-2026-10-08/` (`five-item-trace.tsv`, `items.json`, `before-*-capture.{png,stderr.log,exit.json}`, proof ledger). Browse reads source-local `ItemCatalogEntry.icon_fdid` from `db2/12.1.0.69933/Item.csv`; `ItemSparse.csv` supplies names. This is not a failed FDID lookup or an unrequested row:

| Item | Item ID | Icon FDID | Baseline `textures/<fdid>.blp` |
| --- | --- | --- | --- |
| Worn Dragonscale | 8165 | 134319 | Absent |
| Thick Leather | 4304 | 134257 | Absent |
| Swiftness Potion | 2459 | 134875 | Absent |
| Worn Shortsword | 25 | 135274 | Present |
| Linen Cloth | 2589 | 132889 | Present |

`auction_house_frame_art::icon_texture` binds each nonzero FDID; `ui::assets::load_source` requests the BLP. Baseline native logs record each absent file, and projection caches `Art::Absent`, leaving the TextureRect unbound. Both standalone skin captures show empty bordered icon slots; the earlier live evidence described white/blank squares. Those are not claimed pixel-identical captures. The auction host does not run merchant's proactive `cache_merchant_icons` extraction.

Bags, merchant, loot, mail and bank rectangular item images share this native loader, although their metadata producers differ (base-appearance helper vs `Item.IconFileDataID`) and merchant proactively caches local-CASC icons. They have the same unbound-image outcome if a requested BLP remains unavailable. Circular bag-bar/portrait images use synchronous decoding/masking. Item tooltips render text/marks, not a separate item icon. The [shared unavailable-icon policy](asset-pipeline.md#native-unavailable-icons) owns the fix across both image paths.

The [host store check](../../casc-extraction.md#ossobuild-local-store-2026-10-08) found a usable local install. Existing canonical `target/debug/casc-local`, run through slot `scripts/agent/agent-run ahicons` with `WOW_INSTALL_PATH=/mnt/c/World of Warcraft`, extracted seven unique missing evidence FDIDs: 134319,134257,134875,7549246,136113,132913,133611. Spider's Silk/Shadow Silk share 136113; the two Void-Tempered Scales IDs share 7549246. Extraction was first staged under diagnostics for unchanged before/fallback captures. The matching `green-name-sort`, `green-price-sort` and `current-page-back` dumps contain 52 distinct items, 24 initially without local icon art. Extraction was expanded to all 22 unique absent FDIDs in those dumps, not only the named examples (`evidence-market-icons.json`). All 22 payload MD5s match active root content keys; every BLP decodes as a real 64×64 icon (`extraction-content-keys.json`, `extracted-icons.json`). Completed copies were installed without overwrites at canonical `data/textures/<fdid>.blp` (`canonical-install.json`); all 52 evidence item icon paths now exist. No CDN or committed data files.

### Native proof and remaining gate

Final code **`ff3129712`** (policy `d350298b3`, resolver-alias correction `ff3129712`); offline fixtures `cf40028ef` / `a860f7df4`, formatter ordering `81908aaac`. Native helper build/extension load pass. Evidence names below are under canonical `data/diagnostics/ahicons-2026-10-08/`:

| Boundary | Exact result |
| --- | --- |
| RED, before files/code fix | `before-modern-capture` and `before-forever-capture`, head `a860f7df4`, each exit 1 from the missing-icon pixel assertion, not a compilation failure. |
| GREEN, same missing files | `fallback-modern-capture` and `fallback-forever-capture`, head `ff3129712`, each exit 0: three missing rows equal the question-mark reference pixels, two good icons retain their own pixels. Seven distinct FDIDs each log once, including shared Spider's/Shadow Silk 136113. |
| After canonical extraction | `after-modern-capture` and `after-forever-capture`, head `ff3129712`, each exit 0 with no missing-icon diagnostics. All 13 evidence preview rows have real icons; only the deliberate reference row is a question mark. |

All six PNGs inspected: before slots are empty/dark bordered squares; fallback slots contain red question marks; extracted scale/leather/green-potion/purple-web/cloth/armor-patch art is distinct in both skins. Available sword/cloth/bag/hide/runecloth art stays intact. Captures exercise the production Browse screen without `GameClient` or a server, not live trading or full auction parity.

Required whole-crate command (`--test -p game-engine-godot -p game-engine-ui-model -p game-engine-core --no-fail-fast`) ran after the last code change on **`ff3129712`** and exited **101**, before any tests executed: `ui-model/tests/merchant.rs:196` constructs `BuybackItem` without `definition_source` (E0063). Counts are **N/A, zero executed**, not a passing subset. This fixture and `assets/appearance.rs` formatter-import ordering are unchanged from the requested base `4b74c9129`; changed Rust files pass formatting. At inspection, newer master `0374279c8` already held the merchant-fixture repair, but no merge/rebase/adjacent fixture edit was authorized or performed. `whole-crates-final.log` / `whole-crate-summary.json` retain the full gate failure. This icon fix has bounded native proof, **not whole-crate acceptance**.

## Subcategory and displayed-column sorts — 2026-10-08

`godot/ui-model/src/auction/categories.rs` stores class-filter nodes and their children. `AuctionHouseUi.category_path` is a vector of up to three selected indices, not a parallel query/message. Clicking the selected level clears that level and descendants; sibling/parent selection removes deeper indices. Rows carry slash-separated paths for click actions; selected ancestors expand children. Native category rows virtualize20 at a time using the existing registry scroll state and MinimalScrollBar. The production frame receives the original SharedContext so wheel/stepper/drag position changes rebuild it.

`Bid` sorts the existing displayed current/minimum value, separate from `MinBid`; `Quantity` sorts server-global browse totals or real listing-stack quantities. Header actions clone the active query, change primary key/direction, and reset server/local pages and selection. No client-side sorting/filtering or new message was added. Server filtering uses authoritative source-local templates before grouping/paging; ordering retains ascending AuctionID or ItemID/source ties in either direction. TrinityCore uses LIFO posting ties and raw zero pre-bid values; the project intentionally preserves its existing stable IDs and displayed minimum-before-first-bid contract, rather than calling those differences Retail parity.

### Sources and bounded evidence

- Cached Retail `Blizzard_AuctionHouseUI/Mainline/Blizzard_AuctionData.lua:15-211` (explicit trees), `Shared/Blizzard_AuctionData.lua:133-149` (OR filters propagated to parents), `Shared/Blizzard_AuctionHouseCategoriesList.lua:102-131` (selection/toggle).
- Cached Retail names the enum `AuctionHouseSortOrder`, not `AuctionHouseSortType`: `Shared/Blizzard_AuctionHouseTableBuilder.lua:1033-1061` shows browse Price/Name/Quantity, `:1094-1106` shows item Bid/Buyout/Quantity; header `:879-881` and `Shared/Blizzard_AuctionHouseFrame.lua:213-236` promote/reverse. Level/TimeRemaining sorts exist upstream but are not added to headers this change does not show.
- TrinityCore `a352b1fa`, `AuctionHouseMgr.cpp:1182-1198` (class/subclass/inventory restrictions before browse paging), `:243-273` bucket sorts and`:280-332` posting sorts. Quantity is exposed by Retail but not implemented by that TC comparator; the requested global/count ordering is explicitly implemented here. Source saved in `data/diagnostics/ahsort-2026-10-08/TrinityCore-a352b1fa-AuctionHouseMgr.cpp`.
- `/home/osso/.worktrees/logs/ahsort-protocol-roundtrip.log`:3 passed; `ahsort-protocol-layout-green.log`:5 passed. `ahsort-server-red.log`:0 passed/3 failed; subsequent filter-only `ahsort-server-sort-red.log`:1 passed/3 failed on exact ordering/ties; `ahsort-server-green.log`:4 passed. Server tests concatenate50/25 or50/29 pages and assert the complete order.
- Engine `ahsort-engine-red.log`:2 layout failures; `ahsort-engine-actions-red.log`:3 action failures. `ahsort-engine-green-2.log` at code`aeec2d9a0`:2 layout +3 action tests passed (5 total), EXIT0. Native extension build `ahsort-native-build.log` also EXIT0; extension SHA256`feada14b009f18f68966bada1c7234895eb4bfc6b1b9952b3f634ae5a40fb8fe`. See `/home/osso/.worktrees/handoff-ahsort.md` for exact commands and current state. No whole-crate suites or live runs are part of this bounded task.

Offline `subcategory_sorted` preview applies production Armor→Cloth→Chest actions and clicks Available twice, then supplies an explicit server-response fixture (Brown Linen Robe37, White Linen Robe12, Blue Linen Vest8). Class/subclass/inventory metadata was checked against local Item/ItemSparse CSVs. `capture_ui_screen.gd` with `auction_both`/`GODOT_AH_SORT_CAPTURE=1` asserts actual1920×1080 window and viewport for both skins. This is rendered fixture evidence, not a live server query; server order is proved separately above. Capture recipe and interposer copied from `spellbookshot-2026-10-08`.

Capture completed in one cage process, EXIT0 (`/home/osso/.worktrees/logs/ahsort-cage-capture.log`, detailed `data/diagnostics/ahsort-2026-10-08/capture.log`). Both query logs show Quantity/Desc with class4/subclass1 and inventory5/20 alternatives. `modern-subcategory_sorted.png` and `forever-subcategory_sorted.png` were read and inspected:1920×1080, root800×538 at(16,104); Armor/Cloth/Chest expanded and selected, indented children/scrollbar visible, names and quantities37/12/8 readable; Modern metal and Forever bronze chrome differ as expected. Two icons are question marks because FDIDs132645/132678 are absent in canonical and slot data. Local extraction failed because no local WoW `.build.info` exists (also no `/mnt/c` install); no CDN used. Capture retains the pre-existing placeholder contract and logged failures, not a claim of complete art parity. Offline portrait is empty because this fixture has no auctioneer.

Host/editor diagnostics retained: editor import EXIT0 with a popup-parent error and8 ObjectDB exit leaks; cage lacks optional XWayland/libdecor/activation/FIFO facilities, but native Wayland/Vulkan rendering and both1920×1080 assertions passed. No claim of warning-free import, universal shutdown/resource correctness, live trading, or whole-crate gates.
