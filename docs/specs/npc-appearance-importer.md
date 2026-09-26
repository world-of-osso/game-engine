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
- [x] Eastern Kingdoms (verified: 2026-09-25): `cache/npc_appearance.sqlite` covers 3,021 displays (2,245 profiles), every first-model display the server spawns on map 0 plus the Northshire fixtures; all earlier rows unchanged. Previous caches kept as `cache/npc_appearance-ek-20260924.sqlite` (2,999) and `cache/npc_appearance-northshire-20260909.sqlite`; import report in `diagnostics/npc-appearance-ek-20260925/`.
- [x] Wrath-only displays (verified: 2026-09-25): game-server `content_creature_template_model` replaces templates whose displays are absent from Retail `CreatureDisplayInfo.csv` with TrinityCore TDB1210 Retail models (e.g. 6808 Giant Moss Creeper → 520, 11254 Daggerspine Raider → 4763), so every map-0 spawn display is inside coverage. 44 templates world-wide stay unmapped (none spawned on map 0); see game-server content import wiki.
- [x] Runtime applies each profile after M2 spawn with isolated materials, declared body/hair selection, full choice IDs, and authored geoset overrides. The user-authorized September 9, 2026 capture shows clothed, differing background NPCs.
- [x] Retail spawns, maps 0 and 1 (verified: 2026-09-26): `cache/npc_appearance-retail-20260926.sqlite` covers 8,205 displays (5,882 profiles): all 7,591 first-model displays the TDB1210 Retail world.db spawns (event filter as the server), minus 1307, plus every earlier row unchanged. Not yet promoted to `cache/npc_appearance.sqlite`. Inputs are from local build 69933 CASC: enUS `TextureFileData` (root locale `0x202`; the resolver's last-block pick has no local archive), and the model cache `cache/creature_display-retail-20260926.sqlite` with 10 Retail-only displays (1452xx) that the March `CreatureDisplayInfo.csv` lacks. Report and scripts in `diagnostics/npc-appearance-retail-20260926/`.
- [x] Display 1307 (Pygmy Surf Crawler, template 3106) is absent from Retail `CreatureDisplayInfo`; game-server 340178b remaps 3106 to Retail display 1938 (covered). Current maps 0/1 spawn set (verified: 2026-09-26): 7,590 first-model displays, 0 outside coverage (`diagnostics/npc-appearance-retail-20260926/spawned-display-ids-current.txt`).
- [x] Every covered maps 0/1 profile prepares without error (verified: 2026-09-26, `sweep_all_spawned_profiles`: 5,432 profiles, 2,158 ordinary, 0 failures after the Dracthyr hair fix; `sweep-run3.log`, `sweep-run4-failures-rerun.log`). See [runtime spec](npc-appearance.md).
- [x] The Stockade, map 34 (verified: 2026-09-26): `cache/npc_appearance-retail-stockade-20260926.sqlite` covers 8,226 displays (5,898 profiles): every row of `npc_appearance-retail-20260926.sqlite` unchanged plus the 21 uncovered displays of every model (all `Idx`) of the 143 Retail Stockade spawns in game-server `world-stockade-20260926.db` (34 displays; 16 new profiles). Same inputs as the maps 0/1 cache (reproduced byte-identical tables first); `creature_display.sqlite` already holds all 34. Offline sweep of the 34: ok 20, ordinary 14, failures 0. Not yet promoted to `cache/npc_appearance.sqlite`. Report, id lists and sweep log in `diagnostics/stockadecontent-20260926/`.
- [ ] Spawns outside maps 0, 1 and 34 are not imported; the client panics on them when such a map is entered.
- [x] Full-catalog blocker resolved: material `444164` (display `85531`) resolves to texture `1984904` from the local enUS `TextureFileData`.

## Out of scope

Renderer changes, native/runtime execution, network extraction, general-purpose DB2 decoding, and automatic replacement of production caches.
