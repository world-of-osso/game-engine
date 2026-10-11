# Player housing — Retail Mainline

Retail Mainline defines controls/layout/behavior; Forever only changes art. No catalog membership implies owned decor, no StartingQuantity grant is inferred, and raw plot Cost is not interpreted as a sourced currency price.

## Static catalog boundary

The client reads `data/cache/housing.sqlite` produced by game-server `scripts/import_housing.py`. Require Retail 12.1.0.69933 / dcfc90fffd79ba00406ae46f5f657592; read-only loading must reject other builds or malformed records. Preserve authored names, item/model/thumbnail IDs and category joins. Record missing/encrypted data separately. Missing cache must report an error, not fabricate entries.

## Required feature goal (not yet implemented)

1. Retail Housing Dashboard/catalog: micro-menu entry, search/category filters, preview, explicit unknown ownership/source availability; identical elements in Modern and Forever.
2. Authored neighborhood entry, cornerstone plot purchase, exterior/interior transitions and persisted account ownership. Purchase price/currency semantics require an authentic source.
3. Owned decor storage, basic place/move/rotate/remove, durable layouts and one visitor replication. StartingQuantity alone is not a grant contract.

Live acceptance: private UDP5870, two fb_* accounts, both skins, inspected PNG captures. Static importer/loader tests do not prove gameplay or UI.

## Local catalog promotion (lead-owned canonical writes)

After landing the server importer, run it with the audit evidence and pinned build/key as described in game-server `docs/specs/housing-catalog.md`, using `--db /home/osso/Projects/world-of-osso/game-engine/data/cache/housing.sqlite` for the client cache. Agents test with isolated output only.

## References

Local `Blizzard_HousingDashboard/Blizzard_HousingDashboard.xml` defines 814×544 portrait shell, HouseInfo/Catalog/Collection side tabs. `Blizzard_HousingDashboardCatalog.xml` defines categories, search/filter, scrolling catalog and preview. `Blizzard_HousingDashboardCatalog.lua` searches unowned as well as stored entries and previews catalog selections. Audit: `/home/osso/.worktrees/scope-housing.md`; raw local evidence `data/diagnostics/housing-audit-2026-10-11/`.
