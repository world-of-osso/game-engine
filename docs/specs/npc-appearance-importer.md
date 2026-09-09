# NPC authored appearance importer

Local-only Python stdlib importer in `scripts/import_npc_appearance.py`. Produces authored NPC data for later integration, not renderer behavior. [Usage and decoding](../wiki/formats/db2-format.md#npc-authored-appearance-import).

## What it must do

- [x] Decode the three supported WDC5 layouts, inline/noninline IDs, integer storage, relationships, and copy records; reject unsupported layouts/storage and missing encrypted payloads.
- [x] Join Extra customization choices to display IDs; retain display-specific geoset overrides independently of Extra. Displays without Extra receive no appearance or choice row.
- [x] Select HD or SD baked material from the actual model-cache FDID's listfile path. A selected material of `0` writes `baked_texture_fdid = 0`, meaning no authored bake; never substitute the other material. Resolve nonzero materials from local TextureFileData or explicit outfit SQLite; missing/ambiguous mappings fail, never guess.
- [x] Write deterministic `appearances(display_id PRIMARY KEY, race, sex, class, baked_texture_fdid)`, `choices(display_id, choice_id)`, and `geosets(display_id, geoset_index, geoset_value)` tables. Composite primary keys prohibit duplicate choices/geoset indices.
- [x] Write `display_coverage(display_id PRIMARY KEY, requires_appearance)` for every selected CSV display, including ordinary creatures with `ExtendedDisplayInfoID = 0`. Unselected displays are outside coverage; coverage is reported separately from appearance counts.
- [x] Reader returns `None` only for covered ordinary creatures. Covered required appearances without a profile and displays outside coverage return explicit errors, never raw-model fallback.
- [x] Require an explicit new output path, refuse replacement, and report failures without producing an incomplete database.

## How it works

- [DB2 importer usage and supported storage](../wiki/formats/db2-format.md#npc-authored-appearance-import)

## Implementation inventory

- `scripts/import_npc_appearance.py` — bounded reader, joins, SQLite writer, CLI.
- `scripts/test_import_npc_appearance.py` — synthetic WDC5 and local-file import behavior.
- `game_engine::creature_display::npc_appearance` — public shared-library reader API; runtime consumers use this module rather than a duplicate binary-local reader.

## Tests asserting this spec

`python3 -m unittest discover -s scripts -p test_import_npc_appearance.py`

## Verified scope

- [x] Northshire's current local server population has 63/63 source-present required profiles, plus two fixture profiles; 103 display coverage rows prevent a missing required profile from becoming a raw-model fallback.
- [x] Runtime applies each profile after M2 spawn with isolated materials, declared body/hair selection, full choice IDs, and authored geoset overrides. The user-authorized September 9, 2026 capture shows clothed, differing background NPCs.
- [ ] Full-catalog import remains blocked by unavailable local `TextureFileData` metadata for nonzero material `444164` (display `85531`). The importer remains strict; Northshire coverage does not imply universal catalog support.

## Out of scope

Renderer changes, native/runtime execution, network extraction, general-purpose DB2 decoding, and automatic replacement of production caches.
