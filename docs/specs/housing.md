# Player housing — Retail Mainline

Retail Mainline defines controls/layout/behavior; Forever only changes art. No catalog membership implies owned decor and no StartingQuantity grant is inferred. Plot Cost retains its authored value; Retail cornerstone MoneyFrame/GetMoney defines the copper display/payment contract. No first-house exterior/interior grant is invented.

## Static catalog boundary

The client reads `data/cache/housing.sqlite` produced by game-server `scripts/import_housing.py`. Require Retail 12.1.0.69933 / dcfc90fffd79ba00406ae46f5f657592; read-only loading must reject other builds or malformed records. Preserve authored names, item/model/thumbnail IDs and category joins. Record missing/encrypted data separately. Missing cache must report an error, not fabricate entries.

## Required feature goal (completion remains open)

1. Retail Housing Dashboard/catalog: micro-menu entry, search/category filters, preview, explicit unknown ownership/source availability; identical elements in Modern and Forever.
2. Authored neighborhood entry, cornerstone plot purchase, exterior/interior transitions and persisted account ownership. Purchase price/currency semantics require an authentic source.
3. Owned decor storage, basic/expert place/move/rotate/scale/remove inside and outside, durable layouts and visitor replication. StartingQuantity alone is not a grant contract.

Further ranks: authentic vendor/quest/achievement/crafting acquisition; visiting/social permissions; charter/create-neighborhood/house finder/bulletin board; room/exterior/dye/blueprint customization; sourced initiatives/favor/levels/upgrades/market cart. Missing reward/grant/progression data stays a gap, never a synthetic substitute.

## Authoritative storage boundary

Opening the dashboard requests server state without entering a neighborhood or granting property. Registered HousingReply traffic must reach the native account dispatcher and preserve ownership, stored quantities and refusals. Before a matching-build authoritative snapshot, quantity is unknown; after one, an absent stored entry is a known zero. Unavailable/wrong-build replies cannot become fabricated empty ownership. Leaving InWorld clears account housing state.

HouseInfo distinguishes unconfirmed ownership from a confirmed account-owned plot count without claiming a granted exterior/interior. Catalog preview and status show confirmed storage counts in both skins. Normal bag use of vendor252312's item245259 collects decor1693 only through the verified spell1238340/effect324 source chain. Server consumption, quantity conservation and persisted storage—not client catalog membership—decide the displayed count. Home Interior map admission and persisted visited identity are separate from unverified room/exterior component grants; authentic terrain must never be claimed as a granted assembled house. House placement/editor/visitor and complete acquisition parity remain separate acceptance gates.

Live acceptance: private UDP5870, two fb_* accounts, both skins, inspected PNG captures. Static importer/loader tests do not prove gameplay or UI. `godot/tests/housing_collection_live.gd` exercises real Pawdo pointer interaction, paid vendor purchase, normal bag right-click collection, exact qualified storage count and account reauthentication; fixture money/position setup remains explicit and grants no item/decor.

## Native neighborhood and cornerstone controls

The dashboard's Find a Home action opens a954×490 House Finder shell with the two authored map locations and authoritative plot counts. Its Visit Neighborhood action requests real housing admission; browsing never ports or grants ownership. The dedicated System2 map uses authored UiMapAssignment relations and tile art; missing relations/art remain explicitly unavailable. Ordinary world cornerstone interaction sends UseGameObject; only its authenticated HousingReply inspection opens the340×320 purchase controls. Display the actual plot index and copper cost, confirm the irreversible purchase, disable repeat submission while pending and show ownership only after the server's paid claim. Both skins share controls/layout/behavior. Default house component grants remain unavailable.

## Dye backend versus native availability

Server/wire supports owner-local staged dyes, explicit atomic commit, preserved dyed storage variants and versioned existing-state migration; see the [server contract](../../../game-server/docs/specs/housing-catalog.md#dye-authority-boundary-serverwire-only). This is not a native dye feature: recoloring, dye acquisition and variant controls remain unavailable. Existing native placements explicitly select the undyed stack (`variant_id=None`), never substitute an undyed actor for an owned dyed variant. No grant is inferred from680decor-slot/77color catalog membership. Currently paid1693/2330 have no dye slots. Source rule: generated HousingDecorDyeSlot allows all color categories; category is labeling only.

## HouseSettings access controls

Owner House Settings opens the Retail600×550 panel and stages House Access without changing confirmed state. Dropdown Anyone checks/disables all four choices; No One clears/disables them; Limited enables Neighbors/Guild/Friends/Party independently. Explicit Save sends only HouseAccess and closes the panel; close discards the draft. Unconfigured access stays unknown and Save is disabled until an explicit selection. Reopening/relogging reads authoritative settings. Visitors cannot open owner settings. Both skins share the same controls/layout/behavior; Forever changes art only.

Plot Access, Blueprint Export and owner assignment remain disabled/unavailable and are never altered by saving house flags. Source: local Blizzard_HousingHouseSettings.lua HouseAccessFlags, OnAccessTypeSelected, OnSaveClicked and XML three160px columns. Server derives relationships from persisted plot ownership, assigned owner's character friends/guild and current party membership; no client relationship claim. Battle.net social authority remains unavailable. Previous Anyone permissions migrate once without changing storage/layout/ownership. Live acceptance includes staged/closed drafts, saved/relogged selections and denied other-account entry in both skins.

## Catalog model preview

Catalog selection mounts the authenticated decor model in a transparent, isolated 3D viewport. Dispatch literal ModelType1 to M2 and2 to WMO root/groups through their maintained loaders; do not create an M2 filename for WMO bytes. Any required WMO batch failure makes that preview unavailable instead of hiding missing geometry. Replace the previous actor on selection, clear it when search has no results, and tear down before its UI parent disappears on tab switch, close or logout. Missing/unsupported models or required texture receipts show Retail's Preview unavailable state; never substitute a thumbnail or retain the previous actor. Browsing/drag rotation cannot change owned quantities, placements or gameplay camera. Tile thumbnails remain tile art only. Honor Retail SetPreferModelCollisionBounds(true): use authored collision bounds when present, otherwise the authored model header bounds; never include animation/particle envelopes in catalog framing.

Source: `Blizzard_HousingModelPreview.lua` ApplyCurrentVariant/ClearPreviewData and its XML ModelScene anchors (TOPLEFT30,-100; BOTTOMRIGHT-30,55). Native framing, fill lighting and drag sensitivity use the existing isolated model-scene renderer; they do not claim authenticated UiModelScene preset parity. Preserve authored WMO material/vertex-light semantics. Pixel20/MOMT23 appearance remains a named source gap: the primary WebWowViewerCpp1a8cccbe reference uses a black AO TODO. Do not replace it with invented AO colors, rewrite MOCV or force unlit materials. Dyed variants, source preset cameras and scene-control zoom remain separate capabilities.

## Verified default unlocks versus starter assembly

Generated `PlayerHousingConstantsDocumentation.lua` explicitly names RoomFlags.UnlockedByDefault4, HouseExteriorWMODataFlags.UnlockedByDefault1, HousingFixtureFlags.IsDefaultFixture1/UnlockedByDefault2, HousingThemeFlags.UnlockedByDefault1 and HousingRoomComponentTextureFlags.UnlockedByDefault1. Pinned DB2 flags therefore identify default unlock availability; the earlier blanket claim that default component availability was unknown is too broad. Unlocks do not establish instance quantities, a particular first exterior choice, a starting layout transform or room budget. Assembly/grant authority remains unavailable until those contracts are sourced; do not infer them from UnlockItemID0 or unnamed NeighborhoodPlot vectors.

## Local catalog promotion (lead-owned canonical writes)

After landing the server importer, run it with the audit evidence and pinned build/key as described in game-server `docs/specs/housing-catalog.md`, using `--db /home/osso/Projects/world-of-osso/game-engine/data/cache/housing.sqlite` for the client cache. Agents test with isolated output only.

## References

Local `Blizzard_HousingDashboard/Blizzard_HousingDashboard.xml` defines 814×544 portrait shell, HouseInfo/Catalog/Collection side tabs. `Blizzard_HousingDashboardCatalog.xml` defines categories, search/filter, scrolling catalog and preview. `Blizzard_HousingDashboardCatalog.lua` searches unowned as well as stored entries and previews catalog selections. Audit: `/home/osso/.worktrees/scope-housing.md`; raw local evidence `data/diagnostics/housing-audit-2026-10-11/`.

## Storage drawer controls

Basic Decor exposes the source collapse handle and replaces the drawer with its open handle when collapsed. Collapsing never resets search/category/filters or changes authority. Cleanup and Expert hide both handles independently of the saved collapsed preference. Account-local `ui_layout.ron` preserves collapsed/width/height while retaining other layouts and window positions; a fresh process restores it.

The source XML initial size is600×651; resize clamps width415–827 and height651–800. Bottom-right pointer capture continues outside the handle until release. Release snaps options width to the nearest102px multiple except at the clamp boundaries; options width excludes source85px left/5px right/23px ScrollBox gutter. Result tiles reflow to the new column count. No room/storage quantity/cap policy is inferred from panel dimensions. Source: HouseEditorStorageFrame Lua OnLoad/OnResizeStopped/SetCollapsed, XML and SharedUIPanelTemplates PanelResizeButtonMixin. Both skins physically verify clamp/snap, release outside the original handle, retained browsing state, unchanged authority and fresh-process collapsed/size restoration. Preferences load at UI mount, never each per-frame world-layout sync. Source0.2s drawer/0.15s button animations and cursor fidelity remain separate appearance acceptance; immediate toggles do not claim them.

## Owned storage filters

Owned storage has independent Dyeable Only and alphabetical ordering controls. Dyeable requires authentic DecorDyeSlot membership, not an item/source guess. Reset clears filtering without changing search, category or selected sort. Browsing cannot mutate confirmed quantities or layout. Collection and first-acquisition filters are absent (`SetCollectionFiltersAvailable(false)`). Date Added remains unavailable without authenticated acquisition timestamps; indoor/outdoor and tag mappings are not invented. Catalog filters and storage filters never share state.

Source: `Blizzard_HousingCatalogFilters.lua` Initialize/ResetFiltersToDefault and `Blizzard_HouseEditorStorageFrame.lua` OnStorageTabSelected. Both-skin physical acceptance exercises alphabetical actual tiles, paid nondyeable stacks excluded, hidden/remounted filter retention, reset/search composition, conservation and normal paid-copy replacement. Dropdowns outside the drawer must remain UI input, never world-edit clicks. Cross-process saved filter/CVar persistence remains separate.

## Owned storage search

Basic editor storage exposes Retail's header SearchBox and shows only authenticated positive stored stacks whose qualified decor names match the case-insensitive search. Placed-only/unowned/unconfirmed entries are not storage results. Preserve storage search separately from Dashboard Catalog search and while Cleanup/Expert hides the drawer; remount the retained input without sampling an absent field as empty. Clear input restores stored results. Search never changes ownership, quantities, selected transforms or pending server authority. Both skins share input/filter behavior. Paging/collection filters, drawer collapse/resize, Market/Blueprints, dyed variant controls and maximum-storage authority remain separate capabilities; this search slice must not claim full storage parity.

Source: `Blizzard_HouseEditorStorageFrame.lua` CreateCatalogSearcher/SetStoredOnly(true)/OnSearchTextUpdated/OnShow and XML SearchBox TOPLEFT20,-20/TOPRIGHT-160,-20/height30 in600×651 drawer.

## Owned storage categories

Storage category focus is separate from Dashboard Catalog focus. Show All and source categories/subcategories with confirmed owned entries (stored plus placed), never Featured/Rooms/unowned categories merely because they exist in DB2. Result tiles remain stored-only. Category selection clears storage text search; new nonempty search clears category/subcategory focus to All. Back restores top-level categories; All subcategories clears only subcategory focus. Hidden Cleanup/Expert retains Basic storage focus. Do not infer indoor/outdoor legality from category membership.

Use the source75px category rail,64px roots and54px subcategories. A category with more than one authored child shows Back/All; specific child buttons appear only when more than one owned child is showable, exactly as DisplaySubcategoriesUnderCategory. Header shows source category or `%s | %s` path. Source: Retail StorageFrame Initialize/OnStorageTabSelected/OnSearchTextUpdated/OnCategoryFocusChanged and HousingCatalogCategories Lua/XML filter/populate/focus/order/Back/All rules. Animated selection art remains separate native presentation coverage.

## Cleanup mode

Retail Cleanup is a distinct editor mode: hide storage, highlight the hovered placed decor, show its name at the pointer, and left-click the actual picked target to remove it into owned storage. Do not require a prior Basic selection or a second click. Empty ground clicks do nothing. A visitor may inspect the hover name but receives a read-only refusal; never change quantities/layout optimistically. Pending requests cannot submit twice. Mode exit/removal/close clears hover and restores saved materials. Existing owner-only RemoveDecor authority preserves paid dye variants and exact stored/placed quantities across relog. Both skins share the same controls/behavior; native highlight styling is presentation, not authenticated Retail shader/art parity.

Source: cached `Blizzard_HouseEditorCleanupMode.lua/.xml` OnShow hides storage, target-selected calls RemoveSelectedDecor, hover shows name/removal errors and XML provides left-click instructions; `HousingCleanupModeUIDocumentation.lua` defines hovered info and selected-target events. Unavailable exterior assembly/locks remain separate source gaps; do not infer new removal policy.

## Expert transforms

Expert uses the same owned decor and server conservation rules as Basic. Retail Expert Translate/Rotate/Scale submodes stage complete transforms; commit saves once, cancel restores the confirmed layout, rotation axis cycles X→Y→Z, and WMO scaling is refused. Both skins share the64px mode/submode controls and expert's hidden storage drawer. Initial placement uses authenticated HouseDecor InitialRotation/InitialScale, not identity defaults. Persistence migration and finite/positive constraints are in the server contract. Native bindings/rates are documented input tuning rather than certified Retail defaults:1/2/3 choose submode, arrows/PageUp/PageDown translate, Q/E increment rotation/scale, X cycles axis, Enter commits, Esc cancels. Native3D manipulator identity and Shift snapping remain source gaps; sourced unsnapped mouse controls are specified below. Outdoor bounds and default grants remain unsourced.

### Mouse precision continuation

Start/Change/Complete mouse manipulation stages an axis transform from its press-time value, never accumulates prior motion twice and never saves on release. Explicit commit remains authoritative; cancel restores saved state. Visitors, unselected targets and WMO scale are refused. Parent-approved2026-10-12 scope uses sourced unsnapped controls only: Retail transform axes/events and scale/rotate widget art. Shift snap increments and native3D handle identity remain named source gaps; do not invent increments, interpret unnamed GameParameter rows or draw substitute3D handles. Source widgets retain rotation80×20/13px arrows and scale200×19/32px thumb/80×20 percentage amount. Mouse arrows hold the same existing native incremental rate until release; the scale bar uses relative dragging, not invented per-decor extrema. Direct dragging of selected decor projects the chosen X/Y/Z axis into the viewport; X selects the next axis. These input mappings are native tuning, not certified Retail3D handle identity or slider-range parity.

## Neighborhood source boundary

Authored bulletin-board type48/PlayerInteractionType72 objects open the600×400 Retail roster, showing durable plot owners, online status and repeatable Name/Status/Plot sorting in both skins. Charter/create remain explicitly unavailable: pinned TrinityCore a352 HousingHandler/Packets implement decline-invites only; local TDB1210 carries housing NPC/object records but no authoritative charter fee/signature-count/name-length contract. Generated NeighborhoodNameMaxCharacters0 is not a valid inferred policy. No fee, signature quota or name length is fabricated.

## Finder map and real friend discovery

Use the dedicated System2 UiMap via NeighborhoodMap.MapID → UiMapAssignment, never names or a world-map fallback. Parent-approved interpretation2026-10-12: Founders2735 assignment68589→UiMap2401/art2056; Razorwind2736 assignment68590→UiMap2402/art2057. Missing/ambiguous relations leave preview explicitly unavailable. Tile art and normalized source cornerstone positions use the existing standard world-to-map projection; these map regions are NOT legal edit bounds.

Retail698×465 MapCanvas draws28px sale and20px real friend pins. Self/stranger plots are not rendered as friend pins. Server relationships come from real persisted social records; no friendship is inferred from sharing a neighborhood or visitor permission. Selecting a pin shows the source plot index/copper price and the single PlotInfo Visit control. Sale visits port to the authored plot teleport without payment/grants; friend interior visits keep existing permission/admission authority. Remove the synthetic occupied-house visit list. Basic/expert, visiting and observer proofs must use this current discovery path.
