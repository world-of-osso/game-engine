# Player Ground

This page covers how the client and the server find the surface a player stands on: ADT terrain plus WMO collision floors, under one rule in shared-protocol `shared::ground`. The contract is in [wmo floor collision](../../specs/wmo-floor-collision.md).

## Rule

`shared::ground::ground_at(feet, terrain_y, wmos)` works in these steps:

1. It casts a vertical segment from `feet.y + STEP_UP_HEIGHT` (1.6 yd, WoWee's grounded step budget) down to `feet.y - FLOOR_SEARCH_DEPTH` (50 yd, AzerothCore `DEFAULT_HEIGHT_SEARCH`). The segment goes through every `WmoCollision` whose world XZ bounds contain the feet.
2. `select_ground` takes the highest candidate (the terrain or a WMO hit) that is at most `feet.y + STEP_UP_HEIGHT`. A WMO floor wins a tie.
3. If there is no candidate, the result is `None`: the character falls.

A face is walkable when its world normal satisfies `|n.y| >= cos(MAX_SLOPE_ANGLE)`. The test is two-sided because collision-only faces have no reliable winding.

## WMO group collision

`WmoGroupCollision::parse` reads the group file itself (MOGP header flags and bbox, MOPY, MOVI, MOVT, MOBN, MOBR). The file keeps the file's coordinates (Z up).

- **Collidable faces:** AzerothCore `vmap4_extractor/wmo.cpp` decides collision per face. A face collides when it is COLLISION, or RENDER without DETAIL, or has material 0xFF.
- **Skipped groups:** groups flagged unreachable (0x80) or antiportal (0x4000000) keep no faces (AzerothCore `WMOGroup::ShouldSkip`).
- **BSP query:** it walks MOBN nodes whose axis is `flags & 3` (0 = X, 1 = Y, 2 = Z) and whose leaf flag is 0x4. At each split it descends every child the query segment touches, then tests the MOBR leaf faces with two-sided Möller–Trumbore. A segment spanning a Z split visits both children.
- **Placement:** `WmoCollision::new(world_from_local, groups)` takes the placement affine for WMO-local Bevy space (`[x, z, -y]`, the space the renderer's group meshes use) and caches its inverse.
- **Placement conversion:** `placement_position` / `placement_rotation` convert MODF/MDDF placements. The engine renders with them and the server places collision with them.

The engine's former `wmo_format/bsp.rs` read axis 0x01 as X and 0x02 as Y, and nothing called it. It was removed.

## Client

- **WMO floors:** `wmo::load_wmo_group_with_root` parses `WmoGroupData.collision` from the same bytes as the render data. `finish_wmo_root` inserts `collision::WmoFloors` on the WMO root, with the root `Transform` (roots are top-level entities). Each tile's spawn does this, so every spawned copy of a WMO carries its own floors.
- **`WorldGround`:** it borrows `TerrainHeightmap` and every `WmoFloors`. `probe(feet)` returns one of three results:
  - `Unloaded`: the tile under the feet has no heights, so physics freezes.
  - `Unsupported`: the player falls.
  - `Supported(Ground)`.
- **Callers:** `update_grounded`, `apply_gravity_and_ground_snap`, `validate_movement_slope`, `should_end_jump` and `is_swimming` all use `WorldGround`. The slope limit applies only terrain to terrain. A grounded move snaps onto a destination at most 1.6 yd below and otherwise walks off the ledge.
- **Timing:** terrain heights register in the same system that queues the tile's WMO spawn. For at most one frame the terrain is known and the floors are not. Gravity starts from rest and a frame is capped at 250 ms, so the player falls at most about 1.2 yd in that frame, and the floor is still within step reach.

## Server

- **`GroundMap`:** a resource that holds the tiles players have stood on. Each tile loads on first `ground_at`:
  1. The tile's root/obj0 FDIDs come from the azeroth WDT MAID (FDID 775971), indexed `col * 64 + row`.
  2. The root ADT is parsed by `terrain_height::parse_adt_heights`.
  3. Each obj0 MODF entry flagged FILEDATAID (0x8) becomes a WMO placement. Entries named by path are skipped with a warning.
  4. Each WMO root's MOHD group count and GFID list give the group files, which are cached per root FDID.
- **Where files come from:** everything is read from `../game-engine/data/{terrain,models}/<fdid>.*`, the client's asset cache. A missing root ADT leaves the tile `Unloaded`, as before. Trade District tile (30, 48) loads 11 WMOs in about 23 ms.
- **Gravity:** `apply_terrain_gravity` probes the ground before integrating. `Unsupported` falls toward `-∞`. `FallTracker` restarts at the new height whenever `MovementControl.epoch` changes, and a fall lands within `GROUND_SNAP_THRESHOLD`.
- **Why the landing is tolerant:** WMO heights differ by about 1e-5 between probes, and the constant-rate fall can come to rest up to 0.1 yd above the ground. An exact `y <= ground` test left the player airborne, and the stale peak later turned a small drop into heavy fall damage.

## Live proof (2026-09-25)

The run used a private server on :5072 and a headless client (`data/diagnostics/floorcollision-20260925/`).

| Location | set-position | Client player z | Terrain there |
|---|---|---|---|
| Auction house | (-8816, 655, 100.5) | 98.018 | about 94.65 |
| Goldshire inn | (-9462.66, 16.19, 58.5) | 56.96 | 56.42 |

After the fall-tracking fix, a 40 yd downward reposition left HP unchanged at 773/1110.

## Sources

- [wowee-collision.md](../../wowee-collision.md): the WoWee floor query and step budget
- AzerothCore `src/tools/vmap4_extractor/wmo.cpp` and `Map.cpp`, in `~/Repos/azerothcore`: the MOPY rule, group skips and search depth

## See Also

- [[collision-system]]: the collision layers, including walls and doodads
- [[wmo-format]]: MOPY/MOBN/MOBR chunk layout
- [[stormwind-hilly-plaza]]: where the missing floor support was observed
- [[terrain]]: ADT heights and MODF placement
