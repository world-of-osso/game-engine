# Client cache import sources

Client cache importers consume the authenticated active Retail DB2 export at `data/db2/active`. The publication alias points to a build-version directory with `receipt.json`; no unqualified/root CSV fallback exists.

## What it must do

- Creature displays, customization, character textures and outfit links read Retail inputs only from the active export. Missing sources fail even when root CSVs or usable caches remain.
- Source receipts identify the resolved build-relative directory, invalidating cached results after an active alias swap even when CSV sizes and coarse mtimes match.
- Spell cache keys read the actual source build receipt; visual cache filenames include that build and format version.
- Forever inputs stay product-local. Existing Retail collision precedence and the Skyborne overlay remain unchanged.
- Importers accept a diagnostic data root and never require canonical cache mutation to verify a new export.
- Outfit links v5 uses six active gear tables. Retired `CharStartOutfit` starter-cache APIs and the discarded native warm-up are removed; equipped items retain their owned-item/resource path. Server starting items remain `CharacterLoadout` Purpose9 plus TDB adjustments.
- `SpellCastingRequirements` and `AreaGroupMember` exports preserve source region/facing fields and parent-linked membership, count encrypted omissions and reject layout drift before output.

## Proof

- `godot/core/tests/cache_import_sources.rs`: actual active-alias patch ordering, explicit root-only rejection, concrete item25/display1542/model148132 without legacy starter inputs.
- `godot/core/src/outfit_data_tests.rs`: owned item/model/material behavior and concurrent first import.
- `scripts/tests/test_db2_install_export.py`: exact new layout, stable CSV symbols and narrow pallet-array decoding.
- `scripts/tests/test_toy_cast_limit_exports.py`: genuine source cast/group snapshots and layout refusal.
- `godot/network/examples/db2_cache_import.rs`: private regeneration through the same importer implementations used by tools.

## Related

- [Installed-build DB2 contract](db2-follow-install.md)
- [Cache source implementation](../wiki/systems/cache-import-sources.md)
- [Local CASC extraction](../casc-extraction.md)

Canonical publication and world.db content changes belong to the lead.
