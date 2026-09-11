# Character-Select Waterfall Loading

Verified: 2026-09-11. Diagnosis only; correction pending.

Adventurer's Rest loads primary terrain `2703_31_37`; its waterfall backdrop comes from supplemental tile `2703_31_36`. The supplemental root, `_tex0`, and `_obj0` files all exist locally.

The root contains 227 chunks with the shadow-present flag but no `HSCM` payloads. Their matching 227 shadow payloads reside in `_tex0`, as split-file data. `resolve_mcnk_shadow_map` currently requires those payloads while parsing the root alone and returns `MCNK flagged with MCSH but missing HSCM sub-chunk`. `load_adt_inner` propagates that failure for the whole tile.

`scene_tree::spawn_warband_terrain_tile` converts the error to `None` with `.ok()`. The supplemental loop then skips terrain and object spawning, excluding 42 waterfall/ripple placements. This explains the primary-only runtime logs; fog and the unrelated mossy-rock WMO do not cause this omission.

The correction must respect split-file shadow ownership and report loading errors rather than silently skipping the backdrop. Do not treat absent root shadow payloads as corrupt data without considering the companion file, or discard valid shadow data as a workaround.

## Evidence and sources

- `data/diagnostics/waterfall-missing-20260911/report.md` — independent source/data validation; no runtime fix test yet.
- `data/diagnostics/waterfall-missing-20260911/split-shadow-layout.json` — root/companion chunk census.
- `src/asset/adt_format/adt/parsing.rs` — shadow validation.
- `src/asset/adt_format/adt.rs` — root parse error propagation.
- `src/scenes/char_select/scene_tree.rs` — supplemental loading and swallowed error.
- `src/scenes/char_select/warband/mod.rs` — waterfall neighbor selection.

Related: [terrain](../systems/terrain.md), [character-selection visibility](../../specs/character-selection-visibility.md).
