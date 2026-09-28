# WMO Floor Collision

Players stand on WMO floors (building interiors, paving, ramps), not only on ADT terrain. The client (prediction) and the server (authority) apply the same rule, `shared::ground` in shared-protocol. The client code is in `src/collision.rs`. How it works: [player ground](../wiki/systems/player-ground.md).

## What it must do

### Ground rule (`shared::ground`)
- [x] The ground is the highest candidate among the terrain height and the walkable WMO collision faces under the feet that is at most `STEP_UP_HEIGHT` (1.6 yd) above the feet. A floor higher than that is a ceiling or a roof.
- [x] A WMO floor wins a tie with the terrain it paves over.
- [x] Nothing within reach means the character is unsupported and falls. Terrain far above the feet does not lift them.
- [x] Collidable faces follow the AzerothCore vmap extractor rule: MOPY COLLISION (0x08), or RENDER (0x20) without DETAIL (0x04), or material 0xFF. Render-only detail faces and unflagged faces do not support.
- [x] Faces are reached through the group BSP (MOBN/MOBR, axis = `flags & 3`).
- [x] Unreachable (MOGP 0x80) and antiportal (MOGP 0x4000000) groups have no floors.
- [x] A face steeper than `MAX_SLOPE_ANGLE` (50°) does not support.
- [x] WMO floors follow the MODF placement (`placement_position` / `placement_rotation`), which is the same conversion the client renders with.

### Client
- [x] Inside the Stormwind auction house the ground is its floor (98.06), not the terrain 3.4 yd below it.
- [x] A player set down just above the auction house floor lands on the floor.
- [x] Vertical physics is frozen only while the terrain tile under the feet is not loaded. In an ADT hole the player can fall onto a WMO floor.
- [x] Godot client: the floors of every `_obj0` MODF WMO of a parsed tile are ground candidates from the moment the terrain worker parses the tile, as on the server (`GroundMap`), not once the in-world object queue (thousands of placements at 8 ms a frame, minutes) spawns the WMO's node. At the Stockade entrance (`sw_magicdistrict` 321999) the player stands on the room floor (97.63) and the doorway (88.0) and runs down the stairwell, not on the terrain 11.4 yd below (`godot/rust/src/ground.rs` `stockade_entrance_ground_is_the_placed_wmo_floor_and_stairs`; live: `godot/tests/stockade_walk.gd`).
- [ ] The slope limit applies only between two terrain samples. A move onto a WMO floor is judged by the face normal, and a grounded move walks off a ledge more than 1.6 yd high instead of snapping down.
- [ ] Deep terrain water under a WMO floor (a bridge) does not make the player swim.

### Server
- [x] Tiles load on first use from the client's asset cache. The WDT MAID gives the root/obj0 FDIDs, MODF places WMOs by FDID and GFID names the groups.
- [x] The auction house ground is 98.06 on the server too.
- [x] A `set-position` just above the auction house floor lands on it under server gravity.
- [x] A reposition (a new `MovementControl` epoch: set-position, graveyard, resurrect, taxi landing) is not a fall and deals no fall damage.
- [x] A fall has landed once the player is within `GROUND_SNAP_THRESHOLD` of the ground, so resting a hair above a WMO floor deals the fall's damage and clears the fall.
- [x] A player falling below `Map::GetMinHeight` dies (TrinityCore fall to void): -500 in the Stockade and on tiles without MFBO, else the root ADT's MFBO minimum plane (game-server `ground::grid_min_height`).

## How it works

- [player ground](../wiki/systems/player-ground.md)
- [collision system](../wiki/design/collision-system.md)
- [wowee-collision](../wowee-collision.md): the reference floor rules

## Implementation inventory

- shared-protocol `src/ground.rs`: the ground rule (`ground_at`, `select_ground`), `WmoCollision` (placed WMO) and the MODF placement conversion.
- shared-protocol `src/ground/wmo.rs`: `WmoGroupCollision`, which parses a group file (MOGP, MOPY, MOVI, MOVT, MOBN, MOBR) and runs the BSP segment query.
- `src/asset/wmo.rs`: `WmoGroupData.collision`, parsed with the group.
- `src/rendering/terrain/terrain_objects_wmo.rs`: `finish_wmo_root` puts `WmoFloors` on each spawned WMO root.
- `src/collision.rs`: `WmoFloors`, `WorldGround` / `GroundProbe`, grounding, gravity and movement validation.
- `godot/rust/src/terrain/assets.rs`: `NativeTerrainTile.wmo_floors`, read by the terrain worker; `godot/rust/src/ground.rs`: `TerrainGround`.
- `src/rendering/camera/camera.rs`: player movement, jump landing and swimming on `WorldGround`.
- `src/rendering/terrain/terrain_heightmap.rs`: `has_tile_at`.
- game-server `crates/server/src/ground.rs`: `GroundMap`, the lazy per-tile terrain and WMO loader.
- game-server `crates/server/src/networking.rs`: `apply_terrain_gravity` / `clamp_to_ground` and `FallTracker`.

## Tests asserting this spec

- shared-protocol `src/ground_tests.rs`
- `src/rendering/terrain/terrain_objects_wmo_tests/floor_collision.rs`
- game-server `crates/server/src/ground_tests.rs`
- game-server `crates/server/src/networking_tests/physics.rs` (step reach, noisy-floor landing)

## Known gaps (current cycle)

- [ ] Walking up WMO stairs and ramps (the Goldshire inn stairs) has not been observed live. Ramps are covered only by a synthetic 30° test.
- [ ] A zone-transition Loading screen after a `set-position` more than one tile away never ends. `check_loading_complete` waits for `AdtManager.initial_tile`, which has been unloaded. This bug predates this feature.

## Out of scope

- Wall collision and sliding against WMO collision faces. Horizontal blocking still uses the render-mesh ray.
- Floors on M2 doodads (bridges, ships, platforms).
- Swimming in WMO liquid (MLIQ).
- Server-side extraction from CASC. The server reads only files the client has already cached. A tile the client never cached has no ground on the server.
- Waypoint pathing over WMO floors. `pathing.rs` plans over terrain only.
