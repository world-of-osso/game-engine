# Client cache import sources

Verified: 2026-10-10. Retail cache imports use `db2/12.1.0.69933`, not unversioned CSV copies. The [contract](../../specs/cache-import-sources.md) owns requirements.

## Sources and receipts

Creature displays combine pinned Retail tables with the existing Forever `1.60.1.70205` overlay. Customization and character-texture importers have separate explicit Forever entrypoints; selecting Forever is not a missing-Retail fallback. Runtime race/model requirements and creation backgrounds select pinned Retail tables while retaining product-local Forever catalogs.

Freshness compares source paths relative to the data root and source mtimes. Build-qualified keys invalidate old root-source receipts even when timestamps match. Customization additionally records `ChrRaceXChrModel` and `ChrRaces`, which affect imported hair geosets.

Each tool accepts an optional `DATA_DIR`: `cargo run -p game-engine-tools --bin creature_display_cache_import -- <diagnostic-data-root>`. Output remains `<DATA_DIR>/cache/`; no canonical cache needs to be touched for regeneration or comparison.

## Export boundary

Missing customization/texture tables were exported from local Retail CASC build `12.1.0.69933`, build key `dcfc90fffd79ba00406ae46f5f657592`, using exact WDC5 layouts in the tracked exporter. New files were added with exclusive creation. The pre-existing three-column `ChrModel.csv` required customization layout/scale/camera columns; the lead explicitly authorized same-build expansion, preserving `ChrModel.csv.pre-cache-import-sources-20261010` and both hashes. Engine player-model and server LOS readers select columns by header name, so the added columns preserve their behavior. Diagnostics and source/layout/hash receipts live under `data/diagnostics/cache-import-sources-2026-10-10/`.

The outfit importer retains its entire existing seven-table root source set as a **WORKAROUND**: `CharStartOutfit` FDID 1237591 is absent from the local Retail root. Retirement condition: obtain an authentic pinned `CharStartOutfit` export, then migrate all seven inputs together. The other missing outfit tables were exported but are not partially mixed into this importer. `ItemDisplayInfoMaterialRes` export drops 177 records whose TACT keys remain unavailable.

Retail `NameGen` layout `3CA45193` is not supported by the existing Forever-only `584300FA` exporter. Its root runtime reader remains unchanged; no legacy CSV was copied into the pinned export. Other root-only runtime tables without pinned counterparts (area/light/warband) likewise remain unchanged.

## Sources

- [Importer contract](../../specs/cache-import-sources.md)
- [Exporter](../../../scripts/export_db2_csv.py)
- [Creature importer](../../../godot/core/src/game/creatures/creature_display_cache.rs)
- [Customization importer](../../../godot/core/src/rendering/character/customization_cache.rs)
- [Texture importer](../../../godot/core/src/rendering/character/char_texture_cache.rs)
- [Outfit workaround](../../../godot/core/src/game/outfit_catalog_db.rs)

## See Also

- [Forever catalogs](forever-data.md)
- [Offline asset closure](offline-asset-closure.md)
