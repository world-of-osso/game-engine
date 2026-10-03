# WMO Floor Collision

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

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
- [x] Godot client: the camera collides with WMO walls. Every collidable face of every non-antiportal, reachable WMO group (the same `WmoGroupCollision` faces as the floors, placed with the same transform) is a `ConcavePolygonShape3D` on physics layer 2 (`wmo::collision::WMO_LAYER`), under `WmoCollision`, never under a render node, so portal or distance culling does not remove it. Camera rays query terrain (layer 1), WMO walls and doodad collision (below). The smoothed camera position is ray-checked from the eye (`camera_follow_data::keep_in_sight`, Bevy `c55ae4c5`). Shapes build for the global WMO and each parsed tile's MODF WMOs, group by group within 2 ms a frame, nearest group bounds first; one shape per group asset is shared by its placements. Inside the Stockade the camera is pulled in at every yaw instead of framing the dungeon from outside (`godot/tests/world_wmo_camera_collision.gd`; `godot/rust/src/wmo/collision_tests.rs` on real `sw_magicdistrict` stairwell and `stormwindjail` faces).
- [x] Godot client: the camera collides with M2 doodads, as the original client, which traces terrain, then WMO, then M2 collision (solarityclient `terrain_coordinator/camera.rs:41-55`). Only a model's dedicated MD20 collision triangles (0xD8/0xE0) obstruct, never its render mesh (`collision/m2_model.rs:39-41`); a model without them never does. One two-sided `ConcavePolygonShape3D` per model is shared by its placements, in a `StaticBody3D` named `M2Collision` on physics layer 4 (`terrain::doodad_collision::DOODAD_LAYER`, mask value 8) under each ADT and WMO doodad's render node, so it takes the placement transform and is skipped while the doodad is hidden (distance fade, portal cull). Camera rays query the layer every frame, so a doodad that streams in after the camera is placed pulls it in on the next drawn frame (`godot/tests/world_entry_camera.gd`: a slab added across the 15 yd orbit puts the camera at 7.60 yd on the next frame). Doodads do not stop the player. Measured October 1, 2026 at Northshire Abbey with the 3x3 tiles' 6,487 placements streamed in (`ENTRY_CAMERA_MEASURE=1`, two runs, host load average 25-60): 6,497 bodies share 535 shapes (93,675 face vertices, 1.1 MiB); freeing every body released 35-43 MiB of Godot static memory; a 15 yd camera ray cost 5.9-14.1 us with the bodies and 2.5-3.2 us without, and the physics step 0.40-1.24 ms against 0.54-0.67 ms. Frame time (50-230 ms) varied more between consecutive samples than between with and without bodies, so the host load hides any frame-time cost. Resident memory changed by hundreds of MiB in either direction for the same reason.
- [x] Godot client: WMO walls stop the player, as the original `clamp_movement_against_wmo_meshes`: before the slope and step rules, one horizontal ray 0.6 yd above the feet along the move against the WMO wall bodies (layer 2 only, drawn or not) stops the move 0.05 yd short of the first hit, keeping the proposed height; no slide (`player_physics_data::clamp_movement_to_walls`, `TerrainGround::validate_move`). The server adopts the client-reported x/z within its movement bank (game-server `a1fec9d`), so this also keeps the server character inside. On the Stockade stairs an 8 yd move across the stairwell stops at 6.22 yd, and a run down and back up the stairs still reaches both ends (`godot/rust/src/wmo/collision_tests.rs`); live, 1.5 s of held W across the stairwell walked 10.50 yd through the wall on client and server before, 6.22 yd on both after (`godot/tests/world_wmo_wall_walk.gd`), and `stockade_walk.gd` still walks down the stairwell into the Stockade.
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
- `godot/rust/src/terrain/assets.rs`: `NativeTerrainTile.wmo_floors` (by MODF unique id), read by the terrain worker; `godot/rust/src/ground.rs`: `TerrainGround`.
- `godot/rust/src/wmo/collision.rs`: `WmoCollisionBodies`, the WMO wall physics bodies, and `wall_hit`, the player's wall ray; `godot/rust/src/camera.rs`: `follow_pose`, the camera's terrain + WMO ray; `godot/rust/src/ground.rs`: `TerrainGround::validate_move` clamps against the walls.
- `src/rendering/camera/camera.rs`: player movement, jump landing and swimming on `WorldGround`.
- `src/rendering/terrain/terrain_heightmap.rs`: `has_tile_at`.
- game-server `crates/server/src/ground.rs`: `GroundMap`, the lazy per-tile terrain and WMO loader.
- game-server `crates/server/src/networking.rs`: `apply_terrain_gravity` / `clamp_to_ground` and `FallTracker`.

## Tests asserting this spec

- shared-protocol `src/ground_tests.rs`
- `godot/rust/src/wmo/collision_tests.rs`, `godot/core/tests/camera_data.rs`, `godot/core/tests/player_physics_data.rs`, `godot/tests/world_wmo_camera_collision.gd`, `godot/tests/world_wmo_wall_walk.gd`, `godot/tests/world_entry_camera.gd`
- `src/rendering/terrain/terrain_objects_wmo_tests/floor_collision.rs`
- game-server `crates/server/src/ground_tests.rs`
- game-server `crates/server/src/networking_tests/physics.rs` (step reach, noisy-floor landing)

## Known gaps (current cycle)

- [ ] Walking up WMO stairs and ramps (the Goldshire inn stairs) has not been observed live. Ramps are covered only by a synthetic 30° test.
- [ ] A zone-transition Loading screen after a `set-position` more than one tile away never ends. `check_loading_complete` waits for `AdtManager.initial_tile`, which has been unloaded. This bug predates this feature.

## Out of scope

- Sliding along WMO walls. Bevy blocks with the render-mesh ray; Godot with the same ray against the WMO collision faces. Neither slides.
- Floors on M2 doodads (bridges, ships, platforms).
- Swimming in WMO liquid (MLIQ).
- Server-side extraction from CASC. The server reads only files the client has already cached. A tile the client never cached has no ground on the server.
- Waypoint pathing over WMO floors. `pathing.rs` plans over terrain only.
