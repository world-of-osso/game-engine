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

## Catalog model preview

Catalog selection mounts the authenticated decor model in a transparent, isolated 3D viewport. Dispatch literal ModelType1 to M2 and2 to WMO root/groups through their maintained loaders; do not create an M2 filename for WMO bytes. Any required WMO batch failure makes that preview unavailable instead of hiding missing geometry. Replace the previous actor on selection, clear it when search has no results, and tear down before its UI parent disappears on tab switch, close or logout. Missing/unsupported models or required texture receipts show Retail's Preview unavailable state; never substitute a thumbnail or retain the previous actor. Browsing/drag rotation cannot change owned quantities, placements or gameplay camera. Tile thumbnails remain tile art only. Honor Retail SetPreferModelCollisionBounds(true): use authored collision bounds when present, otherwise the authored model header bounds; never include animation/particle envelopes in catalog framing.

Source: `Blizzard_HousingModelPreview.lua` ApplyCurrentVariant/ClearPreviewData and its XML ModelScene anchors (TOPLEFT30,-100; BOTTOMRIGHT-30,55). Native framing, fill lighting and drag sensitivity use the existing isolated model-scene renderer; they do not claim authenticated UiModelScene preset parity. Preserve authored WMO material/vertex-light semantics. Pixel20/MOMT23 appearance remains a named source gap: the primary WebWowViewerCpp1a8cccbe reference uses a black AO TODO. Do not replace it with invented AO colors, rewrite MOCV or force unlit materials. Dyed variants, source preset cameras and scene-control zoom remain separate capabilities.

## Verified default unlocks versus starter assembly

Generated `PlayerHousingConstantsDocumentation.lua` explicitly names RoomFlags.UnlockedByDefault4, HouseExteriorWMODataFlags.UnlockedByDefault1, HousingFixtureFlags.IsDefaultFixture1/UnlockedByDefault2, HousingThemeFlags.UnlockedByDefault1 and HousingRoomComponentTextureFlags.UnlockedByDefault1. Pinned DB2 flags therefore identify default unlock availability; the earlier blanket claim that default component availability was unknown is too broad. Unlocks do not establish instance quantities, a particular first exterior choice, a starting layout transform or room budget. Assembly/grant authority remains unavailable until those contracts are sourced; do not infer them from UnlockItemID0 or unnamed NeighborhoodPlot vectors.

## Local catalog promotion (lead-owned canonical writes)

After landing the server importer, run it with the audit evidence and pinned build/key as described in game-server `docs/specs/housing-catalog.md`, using `--db /home/osso/Projects/world-of-osso/game-engine/data/cache/housing.sqlite` for the client cache. Agents test with isolated output only.

## References

Local `Blizzard_HousingDashboard/Blizzard_HousingDashboard.xml` defines 814×544 portrait shell, HouseInfo/Catalog/Collection side tabs. `Blizzard_HousingDashboardCatalog.xml` defines categories, search/filter, scrolling catalog and preview. `Blizzard_HousingDashboardCatalog.lua` searches unowned as well as stored entries and previews catalog selections. Audit: `/home/osso/.worktrees/scope-housing.md`; raw local evidence `data/diagnostics/housing-audit-2026-10-11/`.

## Expert transforms

Expert uses the same owned decor and server conservation rules as Basic. Retail Expert Translate/Rotate/Scale submodes stage complete transforms; commit saves once, cancel restores the confirmed layout, rotation axis cycles X→Y→Z, and WMO scaling is refused. Both skins share the64px mode/submode controls and expert's hidden storage drawer. Initial placement uses authenticated HouseDecor InitialRotation/InitialScale, not identity defaults. Persistence migration and finite/positive constraints are in the server contract. Native bindings/rates are documented input tuning rather than certified Retail defaults:1/2/3 choose submode, arrows/PageUp/PageDown translate, Q/E increment rotation/scale, X cycles axis, Enter commits, Esc cancels. Native3D manipulator identity and Shift snapping remain source gaps; sourced unsnapped mouse controls are specified below. Outdoor bounds and default grants remain unsourced.

### Mouse precision continuation

Start/Change/Complete mouse manipulation stages an axis transform from its press-time value, never accumulates prior motion twice and never saves on release. Explicit commit remains authoritative; cancel restores saved state. Visitors, unselected targets and WMO scale are refused. Parent-approved2026-10-12 scope uses sourced unsnapped controls only: Retail transform axes/events and scale/rotate widget art. Shift snap increments and native3D handle identity remain named source gaps; do not invent increments, interpret unnamed GameParameter rows or draw substitute3D handles. Source widgets retain rotation80×20/13px arrows and scale200×19/32px thumb/80×20 percentage amount. Mouse arrows hold the same existing native incremental rate until release; the scale bar uses relative dragging, not invented per-decor extrema. Direct dragging of selected decor projects the chosen X/Y/Z axis into the viewport; X selects the next axis. These input mappings are native tuning, not certified Retail3D handle identity or slider-range parity.

## Neighborhood source boundary

Authored bulletin-board type48/PlayerInteractionType72 objects open the600×400 Retail roster, showing durable plot owners, online status and repeatable Name/Status/Plot sorting in both skins. Charter/create remain explicitly unavailable: pinned TrinityCore a352 HousingHandler/Packets implement decline-invites only; local TDB1210 carries housing NPC/object records but no authoritative charter fee/signature-count/name-length contract. Generated NeighborhoodNameMaxCharacters0 is not a valid inferred policy. No fee, signature quota or name length is fabricated.

## Finder map and real friend discovery

Use the dedicated System2 UiMap via NeighborhoodMap.MapID → UiMapAssignment, never names or a world-map fallback. Parent-approved interpretation2026-10-12: Founders2735 assignment68589→UiMap2401/art2056; Razorwind2736 assignment68590→UiMap2402/art2057. Missing/ambiguous relations leave preview explicitly unavailable. Tile art and normalized source cornerstone positions use the existing standard world-to-map projection; these map regions are NOT legal edit bounds.

Retail698×465 MapCanvas draws28px sale and20px real friend pins. Self/stranger plots are not rendered as friend pins. Server relationships come from real persisted social records; no friendship is inferred from sharing a neighborhood or visitor permission. Selecting a pin shows the source plot index/copper price and the single PlotInfo Visit control. Sale visits port to the authored plot teleport without payment/grants; friend interior visits keep existing permission/admission authority. Remove the synthetic occupied-house visit list. Basic/expert, visiting and observer proofs must use this current discovery path.
