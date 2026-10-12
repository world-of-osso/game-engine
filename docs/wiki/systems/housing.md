# Native player housing

Build-qualified static browsing and authoritative account storage are separate. Requirements and remaining ranks live in the [housing contract](../../specs/housing.md); server authority/grant boundaries live in the [server contract](../../../../game-server/docs/specs/housing-catalog.md).

## Runtime

`HousingHud` retains the qualified catalog model, optional native registry canvas and an independent `HousingAuthority`. Opening the dashboard sends a reliable housing query; it does not enter a map or claim property. `NetworkBridge` relays HousingReply to the account dispatcher, then the native HUD. Confirmed storage counts reach catalog preview/status through the shared screen model. Unknown ownership remains unknown; a qualified empty storage snapshot means zero. Leaving InWorld clears authority independently of the cached static catalog.

The server binds requests to authenticated links, resolves property-owned map copies, validates cornerstone proximity and commits plot payment with ownership. Neighborhood admission persists independently of process-local instance IDs so saved-map login can survive restart. The bounded bookcase path uses ordinary bag UseItem, verifies the pinned vendor/item/collect-effect chain, consumes an owned Retail item and publishes storage; normal vendor costs are not bypassed.

## Proof boundary

Verified: 2026-10-11. Private UDP5870 native both-skin proof follows actual Pawdo pointer interaction, menu40669's wares choice, paid MerchantFrame purchase and normal backpack right-click. Both accounts pay500000 copper, consume item245259 and show exactly one decor1693 after real reauthentication. Gold/position are explicit private fixture setup; no item/decor is granted. Runtime account_state confirms Modern/Forever. Eight inspected1280px before/purchase/collected/relogged PNGs are in `/syncthing/AgentShared/2026-10-11/housing/housing-core-{modern,forever}-{before,vendor-purchase,collected,relogged}.png`. Receipts/logs live in `data/diagnostics/housing-core-2026-10-11/collection/{modern2,forever}/`.

Post-merge focused proof: server10, importer4, native libs8, both-skin registry integration1 and full protocol1730 pass; server/native checks have no warnings. Model clearing/unknown versus known-zero, real UDP storage delivery, competing buyers, durable ownership, conserved layouts, read-only visitor snapshots and wire/arrival/database-reopen/login are covered. Native placement/entry/editor/visitor scenes remain separate gates.

No verified first-house exterior/interior grant exists. House entry is explicitly refused; no substitute model, room, floor, grant or ownership is fabricated. Basic/expert editor, visitor scene convergence and later housing ranks remain open.

## Required Pawdo asset receipts

Display137392's real model535052 initially could not render because extracted-only appearance textures lacked receipts. Authenticated existing bytes against the actual local Retail build config/root before no-clobber publication: texture577566, eighteen authored appearance/material textures and the Pandaren-male hat2266768/SFID2267296 chain. No model/texture was substituted or unqualified path allowed at runtime. Additive `model-asset-index-housing-core-pawdo-*` fragments and evidence in `data/diagnostics/housing-core-2026-10-11/pawdo-assets/` preserve the verified identity. The isolated test cache linked already-published receipt fragments while keeping its housing catalog private. See [asset ownership](../../specs/product-isolated-model-assets.md).

## Authored cornerstone discovery

Local GameObjects DB2 FDID841620, layout44BD453C, has 31,736 readable rows at the pinned Retail build. Plot417's cornerstone GO527268 is present there despite the missing TDB template: OwnerID2735, TypeID48, display110660. Authenticated GameObjectDisplayInfo resolves display110660 to model6935310. This is source availability, not a spawned/native interaction claim. Receipt and local bytes: `data/diagnostics/housing-core-2026-10-11/`.

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
