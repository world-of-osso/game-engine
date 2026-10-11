# Client cache import sources

Client cache importers consume the pinned Retail `12.1.0.69933` CSV export. Source selection lives in `godot/core/src/`; [source policy and export limitations](../wiki/systems/cache-import-sources.md) describe the implementation.

## What it must do

- [ ] Creature displays, customization and character textures read Retail inputs only from `data/db2/12.1.0.69933/`; missing inputs fail even when root CSVs or caches remain.
- [ ] Source receipts include build-relative paths. Changing from root inputs invalidates old caches; customization receipts also include race/model dependencies.
- [ ] Forever inputs remain product-local, with existing Retail collision precedence and Skyborne overlay behavior.
- [ ] Runtime customization race/requirement readers and creation backgrounds use pinned Retail inputs.
- [ ] Import tools accept an optional data-root argument for isolated regeneration without changing canonical caches.

## How it works

- [Cache sources](../wiki/systems/cache-import-sources.md)
- [Local CASC extraction](../casc-extraction.md)

## Implementation inventory

- `godot/core/src/game/creatures/creature_display_cache.rs` — display import and receipts.
- `godot/core/src/rendering/character/{customization_cache,char_texture_cache}.rs` — Retail and explicitly selected Forever imports.
- `godot/core/src/npc_appearance_assets.rs` — product-selected runtime catalogs.
- `godot/core/src/scenes/char_create/background_data.rs` — pinned creation background catalog.
- `godot/core/src/game/outfit_catalog_db.rs` — temporary outfit source workaround.
- `scripts/export_db2_csv.py` — exact-layout local DB2 exports.
- `tools/src/bin/*_cache_import.rs` — optional diagnostic data-root argument.

## Tests asserting this spec

- `godot/core/tests/cache_import_sources.rs`
- `godot/core/tests/unit/customization_catalog_cache_tests.rs`
- Existing cache-module and creation-background fixture tests.
- `scripts/tests/test_cache_import_exports.py`

## Known gaps (current cycle)

- [ ] Outfit importer keeps its existing seven root inputs: local Retail CASC root lacks `CharStartOutfit` FDID 1237591. Retire workaround when an authentic pinned export becomes available; never relabel the old CSV.

## Out of scope

- Canonical cache replacement and master merge belong to the lead.
- Root runtime tables without a comparable pinned export are not silently relabeled as `69933`.
