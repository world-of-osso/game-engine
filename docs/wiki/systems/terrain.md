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

## Collision (Reference: WoWee)

Collision is not yet implemented in game-engine but WoWee's architecture provides the reference for when it is built:

- **Terrain**: bilinear heightmap interpolation for floor height
- **WMO**: vertical ray cast through collision triangles; horizontal cylinder wall sweep (radius 0.45–0.5, height 2.0, max step 1.0)
- **M2**: AABB-based with per-category scaling (tree trunks, narrow props, small solid props); optional collision mesh via Möller–Trumbore ray test
- **Priority**: WMO floor beats terrain when inside; M2 platforms use 5-point footprint sampling
- Camera collision: WMO/M2 raycast + terrain floor clamp

See [wowee-collision.md](../wowee-collision.md) for full algorithm reference.

## Known Issues

- **Mountain ridge topology**: even with the correct tile loaded, Adventurer's Rest mountain chunks show slab-like silhouettes. Highest vertices cluster on south/east chunk edges (camera-facing side), and height discontinuities exist along some shared edges between adjacent ridge chunks. Suspected chunk-local ADT mesh reconstruction issue in `src/asset/adt.rs`.
- **Terrain normals**: stored MCNR normals in the mountain area are inconsistent with geometric face normals; normal decoding may still be wrong.

## Sources

- [adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md) — tile ordering bug, mountain silhouette issues
- [world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — placement rotation formula derivation
- [wowee-collision.md](../wowee-collision.md) — WoWee collision architecture reference
- AGENTS.md — ADT split files section

## See Also

- [[rendering-pipeline]] — terrain shader, ADT rendering
- [[asset-pipeline]] — CASC extraction for ADT and companion files
- [[character-rendering]] — character models spawned from ADT doodad placement
