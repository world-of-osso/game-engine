# Native player housing

Build-qualified static browsing and authoritative account storage are separate. Requirements and remaining ranks live in the [housing contract](../../specs/housing.md); server authority/grant boundaries live in the [server contract](../../../../game-server/docs/specs/housing-catalog.md).

## Runtime

`HousingHud` retains the qualified catalog model, optional native registry canvas and an independent `HousingAuthority`. Opening the dashboard sends a reliable housing query; it does not enter a map or claim property. `NetworkBridge` relays HousingReply to the account dispatcher, then the native HUD. Confirmed storage counts reach catalog preview/status through the shared screen model. Unknown ownership remains unknown; a qualified empty storage snapshot means zero. Leaving InWorld clears authority independently of the cached static catalog.

The server binds requests to authenticated links, resolves property-owned map copies, validates cornerstone proximity and commits plot payment with ownership. Neighborhood admission persists independently of process-local instance IDs so saved-map login can survive restart. The bounded bookcase path uses ordinary bag UseItem, verifies the pinned vendor/item/collect-effect chain, consumes an owned Retail item and publishes storage; normal vendor costs are not bypassed.

## Proof boundary

Verified: 2026-10-11. Private UDP5870 native both-skin proof follows actual Pawdo pointer interaction, menu40669's wares choice, paid MerchantFrame purchase and normal backpack right-click. Both accounts pay500000 copper, consume item245259 and show exactly one decor1693 after real reauthentication. Gold/position are explicit private fixture setup; no item/decor is granted. Runtime account_state confirms Modern/Forever. Eight inspected1280px before/purchase/collected/relogged PNGs are in `/syncthing/AgentShared/2026-10-11/housing/housing-core-{modern,forever}-{before,vendor-purchase,collected,relogged}.png`. Receipts/logs live in `data/diagnostics/housing-core-2026-10-11/collection/{modern2,forever}/`.

Post-merge focused proof: server10, importer4, native libs8, both-skin registry integration1 and full protocol1730 pass; server/native checks have no warnings. Model clearing/unknown versus known-zero, real UDP storage delivery, competing buyers, durable ownership, conserved layouts, read-only visitor snapshots and wire/arrival/database-reopen/login are covered. Native placement/entry/editor/visitor scenes remain separate gates.

No verified first-house exterior/interior component grant exists. Authenticated Home Interior map admission is separate: it uses NeighborhoodMap7's real entry and the extracted25-tile terrain, without claiming an assembled room/house. No substitute model, room, floor, grant or ownership is fabricated. Exterior/expert editor, visitor native convergence and later housing ranks remain open.

## Required Pawdo asset receipts

Display137392's real model535052 initially could not render because extracted-only appearance textures lacked receipts. Authenticated existing bytes against the actual local Retail build config/root before no-clobber publication: texture577566, eighteen authored appearance/material textures and the Pandaren-male hat2266768/SFID2267296 chain. No model/texture was substituted or unqualified path allowed at runtime. Additive `model-asset-index-housing-core-pawdo-*` fragments and evidence in `data/diagnostics/housing-core-2026-10-11/pawdo-assets/` preserve the verified identity. The isolated test cache linked already-published receipt fragments while keeping its housing catalog private. See [asset ownership](../../specs/product-isolated-model-assets.md).

## Authored cornerstone discovery

Local GameObjects DB2 FDID841620, layout44BD453C, has 31,736 readable rows at the pinned Retail build. Plot417's cornerstone GO527268 is present there despite the missing TDB template: OwnerID2735, TypeID48, display110660. Authenticated GameObjectDisplayInfo resolves display110660 to model6935310. Private runtime now spawns and replicates these objects from HousingAvailability on first-copy transfer/login, independently of generic startup population. Receipt and local bytes: `data/diagnostics/housing-core-2026-10-11/`.

## Neighborhood and paid plot proof

Verified: 2026-10-11. Modern Houseproof and Forever Housevisitor physically open the Finder, enter authored map2735, pick GameObjects527268/527310, confirm paid claims417/418, debit10,000,000 copper each, then reauthenticate into the same persisted neighborhood with one owned plot. Native fixtures grant no house/decor/items; money and DB2 teleport coordinates are explicit private setup. Seven1280px captures per skin: `/syncthing/AgentShared/2026-10-11/housing/housing-core-{modern,forever}-property-{finder,neighborhood-entry,cornerstone,purchase-confirm,purchased,owned,relogged}.png`. Full private logs/receipts: `/home/osso/.worktrees/housing-core-evidence/property/{modern,forever}/triangles/`.

Cornerstone pick diagnosis: authenticated model6935310 includes a tall plot beam; its M2 header-box centre is empty space. Native triangle picking correctly rejects that ray. The physical fixture now projects real visible mesh triangle centroids and requires UnitPicker to return the exact object before right-clicking. Earlier priming accidentally passed a map argument to the XYZ-only admin set-position command; corrected coordinates and explicit revive restored that private actor after lethal fall. No picker, collision, visibility, grant or model policy was relaxed.

## Private authenticated terrain closure

Existing local-CASC walkers extracted1329 ground dependencies and closed17,227 total assets with zero missing payloads for maps2735,2736,2783. Real WDT roots210/225/25 parse. Existing LOS baker produced21,100,583/28,571,544/1,638,400 triangles respectively; output788,377,372/1,085,813,396/57,011,000 bytes. Source authentication, private publication receipts and bake metadata remain under `/home/osso/.worktrees/housing-core-evidence/{map-assets,map-closure,housing-map-bakes.json}`. This is real authored terrain, not invented floor or a component grant. Textured Founder's terrain/WMO is visible on both relog captures; Both-skin Home Interior map admission/relog/return is now proved separately below; Razorwind rendered admission is not yet proved.

Lead promotion commands live in the [server catalog contract](../../../../game-server/docs/specs/housing-catalog.md#authored-map-ground); supplemental cornerstone import uses `qualified-cornerstones`. Isolated client receipt fragment `model-asset-index-housing-core-neighborhood-maps.json` and staged manifest `housing-models-staged.json` require the maintained importer before any canonical publication. No canonical data was changed.

## Authenticated Home Interior admission and basic editor

Verified: 2026-10-11. Authored map2783 entry(-1000,-1000,0.1) has real extracted ADT support at height-2. The server persists visited house/account identity separately from instance IDs, rechecks visitor access at saved-house login and returns LeaveHouse to that plot's authored teleport. Modern and Forever native Visit→real map2783→reauthenticated visited identity→Leave→map2735 pass; inspected1280px `housing-core-{modern,forever}-home-{entry,relogged,left}.png` captures are public. Terrain uses authored debug-grid art; it is not a verified default room/exterior component grant.

Basic editor decisions require confirmed stored quantity and an editable layout. Preview/cancel never debits quantity; Place/Move/Rotate/Remove and subsequent snapshots use existing HousingCommand authority. Native placement uses the existing visible terrain/WMO surface ray. Saved nodes resolve the qualified decor ModelFileDataID and real texture receipts; no furniture model is substituted. Owner storage and visitor layout are published independently; accepted edits reach authenticated observers in that exact house copy without leaking owner storage. A real wire regression failed with zero observer replies, then passes with matching owner/visitor layouts and a read-only visitor refusal.

Modern native place/move/rotate/relog/remove is proved against genuinely purchased/redeemed bookcase1693/M2 1096761. Data/receipts remain private under `/home/osso/.worktrees/housing-core-evidence/editor/modern/`; an instrumented movement retry completed, but selected-object re-picking exposed a separate BasicDecorMode contract gap. The pure click policy now commits the active movement rather than re-selecting its own ray hit; current-source live proof is still pending. Forever editor/native visitor convergence, exterior edits, expert scale/axes and other housing ranks remain open. Both skins share the600×651 source storage drawer; the basic0.1-radian rotation step is native interaction tuning, not a certified Retail step-size claim.

## Sources

- [Housing specification](../../specs/housing.md) — controls, ownership distinction and acceptance.
- [Server housing specification](../../../../game-server/docs/specs/housing-catalog.md) — import, persistence, admission and acquisition.
- [Native housing host](../../../godot/rust/src/housing.rs) — canvas lifecycle and message projection.
- [Authority model](../../../godot/ui-model/src/housing.rs) — qualified snapshot/unknown-state handling.
- [Catalog screen](../../../godot/ui-model/src/housing_component.rs) — shared both-skin preview/status output.
- Local Retail `Blizzard_HousingDashboard`, `Blizzard_HousingCornerstone`, `Blizzard_HousingHouseFinder` and generated Housing API docs — UI contract.
- Local pinned GameObjects bytes and WoWDBDefs revision07d008f28907c00827cacadf2d7c1a40f578b6a3 — cornerstone record semantics.

## See Also

- [[instances]] — map/copy isolation and transfer handshakes.
- [[game-objects]] — authenticated display/model resolution and interaction distance.
