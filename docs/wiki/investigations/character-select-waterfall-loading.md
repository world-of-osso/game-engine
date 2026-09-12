# Character-Select Waterfall Loading

Verified: 2026-09-11. Parsing and primary-backdrop filtering corrected; final native visibility proof pending.

Adventurer's Rest loads primary terrain `2703_31_37` and supplemental tile `2703_31_36`. The waterfall in the campsite view belongs to the primary tile: 14 waterfall/ripple placements lie about 264–350 units from the character. The neighboring tile contains another 42 placements, over 500 units away. All supplemental root, `_tex0`, and `_obj0` files exist locally.

The supplemental root contains 227 chunks with the shadow-present flag but no `HSCM` payloads. Their matching 227 shadow payloads reside in `_tex0`, as split-file data. Root-only validation previously returned `MCNK flagged with MCSH but missing HSCM sub-chunk`, aborting the whole tile. `159b2b6f` supplies companion shadows before validation, retaining all maps and strict standalone validation.

`scene_tree::spawn_warband_terrain_tile` previously swallowed that error with `.ok()`, so the supplemental loop skipped its 42 placements. `8e0495a2` reports the path and failure. This explained primary-only loading, but did not by itself restore the visible waterfall.

Native inspection after the parser fix exposed the second blocker: the primary object pass applied a 75-unit prop radius to its own waterfall backdrop. `17b77d68` reuses the existing waterfall/ripple predicate to admit those authored background effects while keeping ordinary props radius-limited. The real-spawn regression changes from 62 props with no waterfall04 model to 76 placements, including that model. Camera, fog, and skybox ordering are unchanged.

Do not discard shadow flags/data or broadly expand prop loading as substitutes for these corrections. Evidence includes `primary-red`, `primary-green`, `all-waterfall-placements.json`, and the supplemental 42-placement/256-mesh/227-shadow regression under the diagnostic directory.

## Evidence and sources

- `data/diagnostics/waterfall-missing-20260911/report.md` — independent source/data validation; no runtime fix test yet.
- `data/diagnostics/waterfall-missing-20260911/split-shadow-layout.json` — root/companion chunk census.
- `src/asset/adt_format/adt/parsing.rs` — shadow validation.
- `src/asset/adt_format/adt.rs` — root parse error propagation.
- `src/scenes/char_select/scene_tree.rs` — supplemental loading and swallowed error.
- `src/scenes/char_select/warband/mod.rs` — waterfall neighbor selection.

Related: [terrain](../systems/terrain.md), [character-selection visibility](../../specs/character-selection-visibility.md).
