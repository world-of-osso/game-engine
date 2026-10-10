# NPC authored appearance importer

Local-only Python stdlib importer in `scripts/import_npc_appearance.py`. Produces authored NPC data for later integration, not renderer behavior. [Usage and decoding](../wiki/formats/db2-format.md#npc-authored-appearance-import).

## What it must do

- [x] Decode the three supported WDC5 layouts, inline/noninline IDs, integer storage, relationships, and copy records; reject unsupported layouts/storage and missing encrypted payloads.
- [x] Join Extra customization choices to display IDs; retain display-specific geoset overrides independently of Extra. Displays without Extra receive no appearance or choice row.
- [x] Select HD or SD baked material from the actual model-cache FDID's listfile path. A selected material of `0` writes `baked_texture_fdid = 0`, meaning no authored bake; never substitute the other material. Resolve nonzero materials from local TextureFileData or explicit outfit SQLite; missing/ambiguous mappings fail, never guess.
- [x] Forever NPC dependency collection with a validated authored bake excludes only unused body-component overlays. Retain required model/model-material and customization assets; absent/unresolved bake keeps strict component requirements. Do not invent a texture mapping for an unused overlay.
- [x] Write deterministic `appearances(display_id PRIMARY KEY, race, sex, class, baked_texture_fdid)`, `choices(display_id, choice_id)`, and `geosets(display_id, geoset_index, geoset_value)` tables. Composite primary keys prohibit duplicate choices/geoset indices.
- [x] Write `display_coverage(display_id PRIMARY KEY, requires_appearance)` for every selected CSV display, including ordinary creatures with `ExtendedDisplayInfoID = 0`. Unselected displays are outside coverage; coverage is reported separately from appearance counts.
- [x] `--pet-catalog` selects every resolved companion display from the local BattlePetSpecies/Creature join. `merge_npc_appearance.py` adds uncovered displays to a new cache, preserves all existing rows, rejects required profiles without an appearance, and never automatically promotes an output.
- [x] Reader returns `None` only for covered ordinary creatures. Covered required appearances without a profile and displays outside coverage return explicit errors, never raw-model fallback.
- [x] Require an explicit new output path, refuse replacement, and report failures without producing an incomplete database.
- [ ] An explicit read-only base cache may extend validated mount coverage into a new output while preserving all prior profiles, choices, geosets and coverage; conflicting rows must fail instead of downgrading required appearances.

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
- [x] Maps 0, 1 and 34 with `DEFAULT_PHASE` 169 spawns (verified: 2026-09-26): `cache/npc_appearance-retail-phase169-20260926.sqlite` covers 8,668 displays (6,203 profiles): every row of `npc_appearance-retail-20260926.sqlite` unchanged, plus the 21 displays of every model of the 143 Retail Stockade creatures and the 442 first-model displays the continents' phase-169 spawns add (game-server `world-retail-phase169-20260926.db`). Same inputs as the maps 0/1 cache (its tables reproduced identically first); `creature_display.sqlite` already holds every one. Offline sweep of all 8,073 spawned first-model displays of maps 0, 1, 34: 5,762 ok, 2,311 ordinary, 0 failures after display 35443's eye texture 3600814 was extracted (a stale `3600814.blp.missing` marker from 2026-09-22 blocked on-demand extraction). Not yet promoted to `cache/npc_appearance.sqlite`. Id lists, report and sweep logs in `diagnostics/stockadecontent-20260926/`.
- [x] Every display the server's `ChooseDisplayId` rule can spawn on maps 0, 1 and 34 (verified: 2026-09-26): spawn `modelid`, every template model with a weight (all models when every weight is 0), and the invisible model of trigger templates. That is 9,977 displays, 1,919 of them new. `cache/npc_appearance-stockadefixes-20260926.sqlite` covers 10,587 displays (7,825 profiles): every row of `npc_appearance.sqlite` (phase-169) unchanged, plus the 1,919. Same inputs as the phase-169 cache (`npc-appearance-retail-20260926/importdb2`, `/csv`, `cache/creature_display-retail-20260926.sqlite`), which already holds every model. Offline sweep over the 9,977: 7,396 ok, 2,581 ordinary, 0 failures, with 1,698 textures extracted on demand from local CASC and no stale `.missing` marker hit. Not yet promoted. Id lists, report and sweep log in `diagnostics/stockadefixes-20260926/`.
- [ ] Spawns outside maps 0, 1 and 34 are not imported; the client panics on them when such a map is entered.
- [x] Remaining-type audit (verified: 2026-10-07): raw local-CASC Extra/options/geosets joined to every world spawn display; no omitted compositor profile can consume the remaining types. Ordinary displays correctly have no Extra profile. Type5's fourth creature variation was lost in the separate native creature-display catalog importer, now retained/re-imported with all prior fields unchanged. Types21/22's second raw profile91615 is outside the current appearance cache but neither Void Elf M2 consumes them. This is texture-consumer classification, not a full-world appearance import or sweep. [Per-type audit/proof](npc-appearance.md#remaining-type-world-audit-and-proof-2026-10-07).
- [x] Full-catalog blocker resolved: material `444164` (display `85531`) resolves to texture `1984904` from the local enUS `TextureFileData`.

## Out of scope

Renderer changes, native/runtime execution, network extraction, general-purpose DB2 decoding, and automatic replacement of production caches.
