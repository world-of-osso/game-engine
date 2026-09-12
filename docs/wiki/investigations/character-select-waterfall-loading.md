# Character-Select Waterfall Loading

Verified: 2026-09-11. Split shadows, primary-backdrop filtering, and waterfall UV decoding are corrected. The final native visibility audit is running; visible render completion is not yet claimed.

Adventurer's Rest loads primary terrain `2703_31_37` and supplemental tile `2703_31_36`. The waterfall in the campsite view belongs to the primary tile: 14 waterfall/ripple placements lie about 264–350 units from the character. The neighboring tile contains another 42 placements, over 500 units away. All supplemental root, `_tex0`, and `_obj0` files exist locally.

The supplemental root contains 227 chunks with the shadow-present flag but no `HSCM` payloads. Their matching 227 shadow payloads reside in `_tex0`, as split-file data. Root-only validation previously returned `MCNK flagged with MCSH but missing HSCM sub-chunk`, aborting the whole tile. `159b2b6f` supplies companion shadows before validation, retaining all maps and strict standalone validation.

`scene_tree::spawn_warband_terrain_tile` previously swallowed that error with `.ok()`, so the supplemental loop skipped its 42 placements. `8e0495a2` reports the path and failure. This explained primary-only loading, but did not by itself restore the visible waterfall.

Native inspection after the parser fix exposed the second blocker: the primary object pass applied a 75-unit prop radius to its own waterfall backdrop. `17b77d68` reuses the existing waterfall/ripple predicate to admit the 14 authored primary-tile effects while keeping ordinary props radius-limited. The real-spawn regression changes from 62 props with no waterfall04 model to 76 placements, including that model. The neighboring tile's 42 placements remain supplemental coverage, not the only waterfall source.

Modern waterfall batches also encode their modulation coordinate in the shader-selected second UV set rather than the legacy coordinate lookup. `458868e9`, with named selector bits from `81417de0`, decodes that second UV source while preserving the authored alpha combine. Color-space, opaque, unlit, and cull probes were diagnostic controls only; none is the production fix.

`f53bba1c` keeps character-select orbit-input diagnostics at DEBUG, avoiding INFO-log noise without removing the diagnostic.

Do not discard shadow flags/data, broadly expand prop loading, or replace alpha combination as substitutes for these corrections. RED/GREEN evidence is under `verification`, `primary-verification`, and `uv-verification`; the final native visibility audit remains running.

## Evidence and sources

- `data/diagnostics/waterfall-missing-20260911/{verification,primary-verification,uv-verification}/` — RED/GREEN regression evidence.
- `data/diagnostics/waterfall-missing-20260911/split-shadow-layout.json` — root/companion chunk census.
- `src/asset/adt_format/adt/parsing.rs` — shadow validation.
- `src/asset/adt_format/adt.rs` — root parse error propagation.
- `src/rendering/terrain/terrain_objects.rs` — primary backdrop admission.
- `src/asset/m2_batch.rs` — shader-selected waterfall UV source.
- `src/scenes/char_select/scene_tree.rs` — terrain-load error reporting.
- `src/scenes/char_select/warband/mod.rs` — neighboring-tile selection.

Related: [terrain](../systems/terrain.md), [character-selection visibility](../../specs/character-selection-visibility.md).
