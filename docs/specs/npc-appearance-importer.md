# NPC authored appearance importer

Local-only Python stdlib importer in `scripts/import_npc_appearance.py`. Produces authored NPC data for later integration, not renderer behavior. [Usage and decoding](../wiki/formats/db2-format.md#npc-authored-appearance-import).

## What it must do

- [x] Decode the three supported WDC5 layouts, inline/noninline IDs, integer storage, relationships, and copy records; reject unsupported layouts/storage and missing encrypted payloads.
- [x] Join Extra customization choices to display IDs; retain display-specific geoset overrides independently of Extra. Displays without Extra receive no appearance or choice row.
- [x] Select HD or SD baked material from the actual model-cache FDID's listfile path; resolve materials from local TextureFileData or explicit outfit SQLite. Missing/ambiguous mappings fail, never guess.
- [x] Write deterministic `appearances(display_id PRIMARY KEY, race, sex, class, baked_texture_fdid)`, `choices(display_id, choice_id)`, and `geosets(display_id, geoset_index, geoset_value)` tables. Composite primary keys prohibit duplicate choices/geoset indices.
- [x] Require an explicit new output path, refuse replacement, and report failures without producing an incomplete database.

## How it works

- [DB2 importer usage and supported storage](../wiki/formats/db2-format.md#npc-authored-appearance-import)

## Implementation inventory

- `scripts/import_npc_appearance.py` — bounded reader, joins, SQLite writer, CLI.
- `scripts/test_import_npc_appearance.py` — synthetic WDC5 and local-file import behavior.

## Tests asserting this spec

`python3 -m unittest discover -s scripts -p test_import_npc_appearance.py`

## Known gaps (current cycle)

- [ ] Main owns production-cache promotion and runtime acceptance of this proposed data contract.

## Out of scope

Renderer/Rust changes, native/runtime execution, network extraction, general-purpose DB2 decoding, and automatic replacement of production caches.
