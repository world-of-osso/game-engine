# Adventurer's Rest Mountain Brief

## Summary

The missing/wrong Adventurer's Rest background mountain is not a separate asset. It is present in the ADT elevation data for the main campsite tile, and the authored camera is pointed toward it. The concrete root cause found in the char-select scene loader is that the client was loading the supplemental west tile first and never loading the actual primary campsite tile. For Adventurer's Rest, that meant `2703_31_36.adt` rendered while `2703_31_37.adt`, which contains the mountain ridge, was omitted.

## What Was Verified

- The reference background mountain is terrain, not a dedicated WMO/M2 backdrop.
- The authored Adventurer's Rest camera is in [`data/WarbandScene.csv`](../data/WarbandScene.csv):
  - position `(-2982.99, 468.06, 455.52)`
  - look-at `(-2985.54, 456.02, 454.40)`
- With the current WoW-to-Bevy mapping in [`src/asset/m2.rs`](../src/asset/m2.rs), the camera faces roughly `-12°` yaw from +Z.
- This is not consistent with a gross 90°/180° rotation error.
- The mountain/ridge the camera is looking toward is in [`data/terrain/2703_31_37.adt`](../data/terrain/2703_31_37.adt), not the west neighbor tile.
- The most relevant mountain chunks are:
  - `(11, 9)`
  - `(10, 10)`
  - `(10, 11)`
  - nearby support chunks `(9, 9)`, `(9, 10)`, `(9, 11)`
- Example mountain peaks from parsed elevation data:
  - chunk `(11, 9)` peak near `(-3000.0, 513.2, -166.7)`
  - chunk `(10, 10)` peak near `(-3033.3, 496.5, -166.7)`
  - chunk `(10, 11)` peak near `(-3062.5, 511.1, -166.7)`
- MCNK raw positions in the ADT match the derived chunk origins almost exactly. This rules out chunk placement/origin math as the primary bug.
- `spawn_warband_terrain()` in [`src/char_select_scene_tree.rs`](../src/char_select_scene_tree.rs) was calling `ensure_warband_terrain_tiles(scene).into_iter().next()`.
- `ensure_warband_terrain_tiles()` in [`src/warband_scene.rs`](../src/warband_scene.rs) sorted the tile list.
- For Adventurer's Rest, sorting reordered `[primary=(31, 37), supplemental=(31, 36)]` into `(31, 36), (31, 37)`.
- That caused the root terrain load to pick `2703_31_36.adt`, and the later supplemental path loaded `2703_31_36.adt` again.
- Live render logs confirmed this failure mode: only `data/terrain/2703_31_36.adt` appeared in the terrain load output during char select, never `2703_31_37.adt`.

## What Was Ruled Out

- Not a missing mountain asset.
- Not a simple wrong-facing camera bug.
- Not a chunk-origin mismatch from MCNK position parsing.
- Not fixed by forcing flat upward terrain normals. That changed shading inputs but did not materially fix the silhouette.
- Not explained by the single nearby campsite WMO.
- Not explained by widening char-select WMO spawning to include the farther `10xt_exterior_rock*` / `redwoodtree` WMOs on the main tile.
- Not explained by the companion `_lod.adt` meshes that were extracted and rendered experimentally.
- Not just a terrain texturing/material problem. Rendering the terrain through the fallback terrain material still produced the same slab-like background silhouette.

## Important Observations

- The main campsite tile and the supplemental waterfall tile were both present locally, but the scene assembly logic selected the wrong one as the primary terrain.
- The mountain exists on `2703_31_37.adt`; if that tile is absent from the render, no amount of WMO or shader work can recover the missing silhouette.
- The mountain chunks form a ridge in the correct direction, but the rendered result still appears as hard slab-like silhouettes instead of the expected mountain profile.
- Several of the relevant ridge peaks sit on chunk-edge vertices. That is suspicious and points toward a terrain mesh/topology issue rather than a camera issue.
- In the most visible ridge chunks, the highest vertices cluster on the south/east chunk edges, i.e. the side facing the camera. That pattern is suspicious for a chunk-internal layout/topology problem rather than a missing distant asset.
- Stored MCNR normals in the mountain area are inconsistent with the geometric face normals in some cases, so terrain normal decoding may still be wrong, but it does not appear to be the only problem.
- Border continuity between adjacent mountain chunks shows large height differences along some shared edges, for example:
  - between `(10,10)` and `(10,11)`
  - between `(11,9)` and `(11,10)`
  - between `(10,11)` and `(11,11)`
- Those discontinuities strongly suggest the terrain reconstruction is still wrong in this ridge area even though chunk origins are correct.
- When the root ADT terrain is hidden in a render test, the black slab background disappears. That isolates the visible artifact to the normal root-terrain render path itself.

## Root Cause And Fix

- [`src/char_select_scene_tree.rs`](../src/char_select_scene_tree.rs) now loads the primary tile with `ensure_warband_terrain(scene)` instead of taking the first element from the full tile list.
- [`src/warband_scene.rs`](../src/warband_scene.rs) now preserves primary-first ordering in `ensure_warband_terrain_tiles()` and appends only distinct supplemental tiles.
- A regression test was added to assert that Adventurer's Rest loads `data/terrain/2703_31_37.adt` first while still including `data/terrain/2703_31_36.adt` as supplemental.

## Verification Status

- Library-only builds still work, but full app verification is currently blocked by unrelated local compile errors in `src/equipment_appearance.rs`, `src/char_create_scene.rs`, and [`src/char_select_scene/mod.rs`](../src/char_select_scene/mod.rs).
- Because of those unrelated errors, a fresh post-fix screenshot from the current tree could not be produced in this investigation pass.
- The root-cause evidence is still strong: the mountain tile was provably not being loaded before this fix.

## Remaining Risk

If Adventurer's Rest still shows residual terrain ugliness after the tree is buildable again, the next suspect remains chunk-local ADT mesh reconstruction in [`src/asset/adt.rs`](../src/asset/adt.rs). But that is secondary to the concrete scene-loader bug above: without loading `2703_31_37.adt`, the real mountain could never appear.

## Primary Code Paths

- [`src/asset/adt.rs`](../src/asset/adt.rs)
- [`src/char_select_scene_tree.rs`](../src/char_select_scene_tree.rs)
- [`src/terrain.rs`](../src/terrain.rs)
- [`assets/shaders/terrain.wgsl`](../assets/shaders/terrain.wgsl)
- [`src/warband_scene.rs`](../src/warband_scene.rs)
- [`src/char_select_scene/mod.rs`](../src/char_select_scene/mod.rs)
