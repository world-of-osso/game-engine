# Terrain Tile Ordering

The Adventurer's Rest background mountain was missing from char-select because the scene loader was loading the wrong ADT tile as the primary terrain. The mountain exists exclusively on `2703_31_37.adt`; when `2703_31_36.adt` was loaded in its place, no rendering or camera work could recover the missing silhouette.

## Finding

`spawn_warband_terrain()` called `ensure_warband_terrain_tiles(scene).into_iter().next()` to get the primary tile. `ensure_warband_terrain_tiles()` sorted the tile list alphabetically. For Adventurer's Rest the sort reordered `[primary=(31,37), supplemental=(31,36)]` into `(31,36), (31,37)`, so the supplemental tile was loaded as primary and the primary tile was never loaded at all. Live render logs confirmed only `2703_31_36.adt` appeared in terrain load output.

## Root Cause

`ensure_warband_terrain_tiles()` in `src/warband_scene.rs` destroyed primary-first ordering by sorting. Tile coordinate sort order happened to invert primary/supplemental for this scene.

## Resolution

- `src/char_select_scene_tree.rs` now loads the primary tile via `ensure_warband_terrain(scene)` instead of indexing the full list.
- `src/warband_scene.rs` now preserves primary-first ordering and appends only distinct supplemental tiles.
- Regression test asserts Adventurer's Rest loads `data/terrain/2703_31_37.adt` first while still including `data/terrain/2703_31_36.adt` as supplemental.

**Note:** Secondary terrain artifacts (hard slab silhouette, chunk-edge height discontinuities) remain as a separate issue in ADT mesh reconstruction (`src/asset/adt.rs`). The tile ordering fix is a prerequisite, not a complete visual fix.

## Sources

- [adventurers-rest-mountain-brief.md](../../adventurers-rest-mountain-brief.md) — root cause analysis and fix details

## See Also

- [[object-rotation-transforms]] — separate investigation triggered by the same Adventurer's Rest scene
