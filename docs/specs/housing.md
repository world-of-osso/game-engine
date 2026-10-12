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

HouseInfo distinguishes unconfirmed ownership from a confirmed account-owned plot count without claiming a granted exterior/interior. Catalog preview and status show confirmed storage counts in both skins. Normal bag use of vendor252312's item245259 collects decor1693 only through the verified spell1238340/effect324 source chain. Server consumption, quantity conservation and persisted storage—not client catalog membership—decide the displayed count. House placement/editor/visitor and complete acquisition parity remain separate acceptance gates.

Live acceptance: private UDP5870, two fb_* accounts, both skins, inspected PNG captures. Static importer/loader tests do not prove gameplay or UI. `godot/tests/housing_collection_live.gd` exercises real Pawdo pointer interaction, paid vendor purchase, normal bag right-click collection, exact qualified storage count and account reauthentication; fixture money/position setup remains explicit and grants no item/decor.

## Local catalog promotion (lead-owned canonical writes)

After landing the server importer, run it with the audit evidence and pinned build/key as described in game-server `docs/specs/housing-catalog.md`, using `--db /home/osso/Projects/world-of-osso/game-engine/data/cache/housing.sqlite` for the client cache. Agents test with isolated output only.

## References

Local `Blizzard_HousingDashboard/Blizzard_HousingDashboard.xml` defines 814×544 portrait shell, HouseInfo/Catalog/Collection side tabs. `Blizzard_HousingDashboardCatalog.xml` defines categories, search/filter, scrolling catalog and preview. `Blizzard_HousingDashboardCatalog.lua` searches unowned as well as stored entries and previews catalog selections. Audit: `/home/osso/.worktrees/scope-housing.md`; raw local evidence `data/diagnostics/housing-audit-2026-10-11/`.
