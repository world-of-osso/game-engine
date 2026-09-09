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

## Authored Height Grid Axes

MCVT rows advance toward negative Bevy X; columns advance toward positive Bevy Z. Mesh positions and the four-triangle CPU sampler use that same basis, with upward triangle winding. The former transposed basis put part of Northshire's `stormwindgypsywagon01.m2` (FDID198288, placement69265) inside an incorrectly reconstructed hill. The wagon's authored transform is unchanged. Incorrect border averaging was removed: it combined unrelated edge samples and altered authored heights. Asymmetric four-chunk regressions cover positions, height preservation, shared borders, winding, and center-vertex sampling. Water-layer offsets, mesh winding, and water-height queries use the same corrected grid axes; asymmetric offset tests cover mesh/query agreement. Runtime visual confirmation remains pending.

## Doodad Collision

Doodad solidity uses authored M2 collision triangles, not render/visual bounds. The M2 parser reads collision bounds, u16 triangle indices, and vertices; placements share the parsed geometry. A world-space AABB narrows candidates, then a backface-inclusive triangle ray test decides a hit through the placement affine transform. Starting inside the broadphase box is not a collision by itself.

Models with no authored collision indices create no solid doodad collider. There is no visual-bounds fallback. `DoodadVisualBounds` independently retains the visual extent for zone-transition/contact interactions, so non-solid doodads can still publish those bounds.

Terrain and WMO collision behavior is unchanged. [WoWee collision notes](../wowee-collision.md) remain reference material for broader collision design.

## Known Issues

- **Mountain ridge topology**: the earlier slab-like silhouettes and discontinuous chunk edges were observed before the height-grid axis correction above. Adventurer's Rest requires visual revalidation; no mountain-specific completion claim is made.
- **Terrain normals**: stored MCNR normals in the mountain area are inconsistent with geometric face normals; normal decoding may still be wrong.

## Sources

- [adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md) — tile ordering bug, mountain silhouette issues
- [world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — placement rotation formula derivation
- [wowee-collision.md](../wowee-collision.md) — broader collision reference
- `../../data/diagnostics/cpu-goal-resumed/doodad-authored-collision/report.md` — authored doodad collision implementation and scoped proof
- AGENTS.md — ADT split files section

## See Also

- [[rendering-pipeline]] — terrain shader, ADT rendering
- [[asset-pipeline]] — CASC extraction for ADT and companion files
- [[character-rendering]] — character models spawned from ADT doodad placement
- [[collision-system]] — broader collision design and remaining layers
