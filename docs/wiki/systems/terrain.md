# Terrain

ADT terrain is loaded from split WoW map tiles. The engine renders heightmap meshes with texture layer compositing, spawns doodads and WMOs from placement data, and uses a custom rotation formula to convert WoW-space placement angles into Bevy-space transforms.

## ADT Split Files

Each tile is three files:
- Root `.adt` — MCNK chunks with heightmaps and normals
- `_tex0.adt` — texture layer compositing (MDID/MHID for diffuse/height FDIDs)
- `_obj0.adt` — MDDF doodad placements and MODF WMO placements

The engine loads all three. Finding companion files uses the community listfile (path-based sibling lookup).

**Critical tile ordering bug (fixed)**: `ensure_warband_terrain_tiles()` was sorting the tile list alphabetically, which reordered `[primary=(31,37), supplemental=(31,36)]` to `(31,36), (31,37)`. The scene loader took `next()` and loaded the supplemental tile as primary, never loading the mountain tile `2703_31_37.adt`. Fix: preserve primary-first ordering and append only distinct supplemental tiles. See [adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md).

## World Object Placement (MDDF/MODF)

WoW ADT stores object rotations as `[X, Y, Z]` Euler angles. These are converted to Bevy using `placement_rotation()` in `src/terrain_objects.rs`:

```
stored [X, Y, Z] → model rotation [Z, Y - 180, -X] → EulerRot::YZX
```

This was derived by visual validation against Adventurer's Rest campsite props. The key reference was Noggit3's `from_model_rotation()` (`[-Z, Y-90, X]` in YZX), which was the starting point but required further adjustment for this renderer's full transform chain. See [world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md).

**Tests**: `placement_rotation_matches_current_model_rotation_formula` and `placement_rotation_zero_matches_current_yaw_correction` in `terrain_objects.rs` lock in the current formula.

## Empty Water Layers

MH2O `WaterLayer` dimensions describe its grid bounds, not guaranteed drawable area: every quad can still be absent in its `exists` bitset. Before `d11948b1`, those layers reached `spawn_water()` as empty Bevy meshes. The local Bevy 0.19 allocator skips their zero-byte allocation but attempts their upload, producing the misleading `slab_allocator` unallocated-key error.

`build_water_mesh()` now returns `None` when no existing quad generated positions; `spawn_water()` skips that result. Layers with one or more existing quads still create the same water mesh. This prevents empty water assets rather than filtering Bevy diagnostics.

A probe at `8fd12e79` observed 47 zero-vertex meshes and 94 allocator errors (two per mesh), consistent with this path. It did not identify every asset by type, so native post-fix proof that this correction removes the runtime spam remains pending. Bevy tracked the same empty-mesh allocator defect in [issue #24874](https://github.com/bevyengine/bevy/issues/24874); [PR #24960](https://github.com/bevyengine/bevy/pull/24960) skips the empty copy and emits a warning upstream.

## Doodad Collision

Doodad solidity uses authored M2 collision triangles, not render/visual bounds. The M2 parser reads collision bounds, u16 triangle indices, and vertices; placements share the parsed geometry. A world-space AABB narrows candidates, then a backface-inclusive triangle ray test decides a hit through the placement affine transform. Starting inside the broadphase box is not a collision by itself.

Models with no authored collision indices create no solid doodad collider. There is no visual-bounds fallback. `DoodadVisualBounds` independently retains the visual extent for zone-transition/contact interactions, so non-solid doodads can still publish those bounds.

Terrain and WMO collision behavior is unchanged. [WoWee collision notes](../wowee-collision.md) remain reference material for broader collision design.

## Known Issues

- **Mountain ridge topology**: even with the correct tile loaded, Adventurer's Rest mountain chunks show slab-like silhouettes. Highest vertices cluster on south/east chunk edges (camera-facing side), and height discontinuities exist along some shared edges between adjacent ridge chunks. Suspected chunk-local ADT mesh reconstruction issue in `src/asset/adt.rs`.
- **Terrain normals**: stored MCNR normals in the mountain area are inconsistent with geometric face normals; normal decoding may still be wrong.

## Sources

- [adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md) — tile ordering bug, mountain silhouette issues
- [world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — placement rotation formula derivation
- [wowee-collision.md](../wowee-collision.md) — broader collision reference
- `../../data/diagnostics/cpu-goal-resumed/doodad-authored-collision/report.md` — authored doodad collision implementation and scoped proof
- AGENTS.md — ADT split files section
- `../../src/asset/adt.rs` and `../../src/rendering/terrain/terrain_spawn.rs` — water mesh construction/spawn boundary
- [Bevy issue #24874](https://github.com/bevyengine/bevy/issues/24874) and [PR #24960](https://github.com/bevyengine/bevy/pull/24960) — empty-mesh allocator behavior

## See Also

- [[rendering-pipeline]] — terrain shader, ADT rendering
- [[asset-pipeline]] — CASC extraction for ADT and companion files
- [[character-rendering]] — character models spawned from ADT doodad placement
- [[collision-system]] — broader collision design and remaining layers
