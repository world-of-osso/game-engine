# Player Ground

Client and server resolve ADT terrain, WMO floors and authored M2 doodad floors with shared step/search rules. Contracts: [WMO floor collision](../../specs/wmo-floor-collision.md), [M2 floor collision](../../specs/m2-floor-collision.md).

## Rule

`shared::ground::ground_at(feet, terrain_y, wmos)` works in these steps:

1. It casts a vertical segment from `feet.y + STEP_UP_HEIGHT` (1.6 yd, WoWee's grounded step budget) down to `feet.y - FLOOR_SEARCH_DEPTH` (50 yd, AzerothCore `DEFAULT_HEIGHT_SEARCH`). The segment goes through every `WmoCollision` whose world XZ bounds contain the feet.
2. `select_ground` takes the highest candidate (the terrain or a WMO hit) that is at most `feet.y + STEP_UP_HEIGHT`. A WMO floor wins a tie.
3. If there is no candidate, the result is `None`: the character falls.

A face is walkable when its world normal satisfies `|n.y| >= cos(MAX_SLOPE_ANGLE)`. The test is two-sided because collision-only faces have no reliable winding.

## Native grounded steps — 2026-10-15

`TerrainGround::validate_move` admits the destination floor at the current feet height before wall collision. The ordinary low wall ray retains its existing corridor. Only an actually wall-blocked grounded snapped rise attempts a lifted ray; unobstructed ramps and airborne proposals retain the original admission. When a wall clamp is retained, its final support column is sampled and its covered/slope rules run there rather than retaining an unreached landing's height. Rejecting a high/steep original destination cannot cancel an otherwise reachable wall approach, and a clamped column under an out-of-reach floor remains blocked.

Stockade WMO108631/group15/FDID108647 face3153 is a vertical lip whose top is0.6557564yd above captured feet `(108.2204,-26.26199,3.454892)`. The old feet+0.6 wall ray hit it0.04996yd away, less than the0.05 stop margin, before face3041's reachable upper floor was sampled. The shared1.6 step budget and wall constants are unchanged.

A raised ray alone can miss a thin wall up to0.6yd above the landing. A wall-blocked step attempt therefore checks the interval between the admitted feet and the existing low wall ray against live WMO/GO triangles. Ordinary unobstructed rises do not run that interval check: Jasperlode's authored floor detail extends0.025yd above a sampled support column without reaching the existing ray, and must not become a new movement obstruction. Nothing above the original0.6-yd ray enters collision admission. Godot's broad-phase box selects meshes only; exact triangle/vertical-plane clipping decides obstruction. Destination support is reconstructed from the authored floor plane so ray-interpolation rounding cannot turn its adjoining lip edge into a wall. Clipping preserves coordinates shared by both edge vertices and sets the clipped axis to the mathematical boundary exactly; no geometric epsilon or collision radius is introduced.

**Named gap — full-body/ceiling clearance:** the client has no full default-height body controller. A proposed2.03128-yd body/head check introduced extra restrictions above the existing wall ray; the lead explicitly excluded it from this change. Existing Stockade/Jasperlode routes remain required regressions. Their ascent trace separately exposed a walkable support face misclassified as a wall after float projection; destination-support faces are now excluded from wall admission by their existing walkable-floor semantics, not a tolerance. A future full-body contract must establish source dimensions and authored corridor compatibility separately.

Server `networking_movement::land_height` already samples the destination ground from current Y before applying its land envelope; it has no earlier client-style wall ray. No server change accompanies this fix. Regression fixtures: `godot/rust/src/ground_step_tests.rs`, full-WMO `wmo/collision_tests.rs::ramp_lip_full_stockade_walks_from_captured_feet_onto_landing`, native `godot/tests/ramp_lip_walk.gd`.

### Bounded native proof

Verified2026-10-15: exact-face RED stops at captured feet; final focused Rust gates pass10 ramp-lip/guard cases and4 existing Stockade/Jasperlode routes (including2FPS) on movement revision`505b5efb8`. Normal native extension build`3dcb5dadf`, fixture`1e41990bc`, pinned Godot4.7.2 and private UDP5921 peer server`3a982777`/shared`62865c2` produced the native proof.

Normal `--screen inworld --char Ramplip` startup enters Stockade at its entrance. The existing online admin command `set-position Ramplip 108.2204 -3.454892 -26.26199` then establishes the captured pose server-authoritatively in that private instance; afterward the fixture only sends native turn/W input. The level80 test character avoids making dungeon combat completion a prerequisite; it still takes trash damage, remains alive, and reports server speed7 at both measured endpoints.

Before movement, the actual Godot WMO-layer ray at feet+0.6 hits `(108.2684,-25.66199,3.441165)` at0.04996484yd, inside the unchanged0.05 wall margin. Held W then advances from `(108.2204,-26.26199,3.454892)` to `(108.8964,-25.60623,3.27322)` on the upper floor; client/server positions agree at both ends and the movement epoch stays unchanged during the walk. No client position writes or server source changes occur.

Inspected1280×720 PNGs: `/syncthing/AgentShared/2026-10-15/ramp-lip/{before,after}.png`. Durable receipts and full logs: `/home/osso/.worktrees/logs/movement-ramp-lip-success-{native-receipt.json,runtime-receipt.json,native-client.log,server.log}`. The runtime receipt records the authoritative setup command and actual extension/server SHA-256s; the compiled runtime uses the supplied62865c2 peer even though the unrelated shared checkout later advances. Owned client/server processes are stopped after capture. This proves the captured direct lip traversal, not whole-dungeon completion or full-body clearance.

## WMO group collision

`WmoGroupCollision::parse` reads the group file itself (MOGP header flags and bbox, MOPY, MOVI, MOVT, MOBN, MOBR). The file keeps the file's coordinates (Z up).

- **Collidable faces:** AzerothCore `vmap4_extractor/wmo.cpp` decides collision per face. A face collides when it is COLLISION, or RENDER without DETAIL, or has material 0xFF.
- **Skipped groups:** groups flagged unreachable (0x80) or antiportal (0x4000000) keep no faces (AzerothCore `WMOGroup::ShouldSkip`).
- **BSP query:** it walks MOBN nodes whose axis is `flags & 3` (0 = X, 1 = Y, 2 = Z) and whose leaf flag is 0x4. At each split it descends every child the query segment touches, then tests the MOBR leaf faces with two-sided Möller–Trumbore. A segment spanning a Z split visits both children.
- **Placement:** `WmoCollision::new(world_from_local, groups)` takes the placement affine for WMO-local Bevy space (`[x, z, -y]`, the space the renderer's group meshes use) and caches its inverse.
- **Placement conversion:** `placement_position` / `placement_rotation` convert MODF/MDDF placements. The engine renders with them and the server places collision with them.

The engine's former `wmo_format/bsp.rs` read axis 0x01 as X and 0x02 as Y, and nothing called it. It was removed.

## Native M2 floors — verified 2026-10-12

`los-bake` writes a companion under `los/<map>/m2/*.los`; `--m2-only` updates only that companion. The bake reads MD20 indices/vertices/face normals at0xD8/0xE0/0xE8 and places MDDF and active WMO doodad sets using their composed position/rotation/scale. Only authored normals facing upward within the shared slope limit admit a floor. Geometry outside a global WMO's group bounds still bakes; empty hulls never become render-mesh floors.

Server `GroundMap::ground_at` loads/caches these BVH tiles, querying one neighbour ring for overhangs; highest admitted height competes with existing terrain/WMO support. Native `NativeTerrainAssets` reads the same companion on its worker before a tile is published. `TerrainGround::sample` combines its heights with existing WMO/terrain candidates, tagged `Surface::M2` so terrain-only slope policy is not incorrectly applied. Ground remains available before model nodes spawn and independent of render culling. Corrupt companion files report errors, not render-geometry substitutions. Visible LOS and navigation obstacle classification are unchanged.

Real tests cover EotS model193193/MDDF995848 and995856, Northshire stonepyre198581/10325, pre-render standing/walking, signed authored normals, and a global-WMO container with a real hull beyond its group bounds. Private Modern/Forever UDP5893 geometry proof: EotS Alliance feet1247.701/full292HP after20s; stonepyre walk≈1.05yd, feet86.94 rather than terrain80.96. PNGs: `/syncthing/AgentShared/2026-10-12/m2-floor-collision/`.

**Proof boundary:** lead approved private copied map566 classification plus identical diagnostic-only hosting override to enter directly. Neither override is committed or changes canonical data. This is floor geometry, not battleground lifecycle; bg-coverage owns real matched entry. Quick same-account relog exposed existing `mount_journal.rs:188` stale-entity panic; private server restart between skins isolates that unrelated defect. Detailed command/revision ledger and tick measurements: `/home/osso/.worktrees/handoff-m2-floor-collision.md`.

## Historical Bevy client

- **WMO floors:** `wmo::load_wmo_group_with_root` parses `WmoGroupData.collision` from the same bytes as the render data. `finish_wmo_root` inserts `collision::WmoFloors` on the WMO root, with the root `Transform` (roots are top-level entities). Each tile's spawn does this, so every spawned copy of a WMO carries its own floors.
- **`WorldGround`:** it borrows `TerrainHeightmap` and every `WmoFloors`. `probe(feet)` returns one of three results:
  - `Unloaded`: the tile under the feet has no heights, so physics freezes.
  - `Unsupported`: the player falls.
  - `Supported(Ground)`.
- **Callers:** `update_grounded`, `apply_gravity_and_ground_snap`, `validate_movement_slope`, `should_end_jump` and `is_swimming` all use `WorldGround`. Godot swimming: [[swimming]]. The slope limit applies only terrain to terrain. A grounded move snaps onto a destination at most 1.6 yd below and otherwise walks off the ledge.
- **Timing:** terrain heights register in the same system that queues the tile's WMO spawn. For at most one frame the terrain is known and the floors are not. Gravity starts from rest and a frame is capped at 250 ms, so the player falls at most about 1.2 yd in that frame, and the floor is still within step reach.

## Server ADT/WMO ground

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

- [M2 floor contract](../../specs/m2-floor-collision.md), `godot/rust/src/ground.rs`, `terrain/assets.rs` — native ground and worker loading
- `game-server/crates/los/src/bake/{m2,mod}.rs`, `crates/server/src/ground.rs`; `shared-protocol/src/los/{tile,map}.rs` — authored-normal bake and height lookup
- Local solarityclient `crates/systems/src/collision/m2_model.rs:171–202` — composed MODD/MDDF vertices and authored face normals for movement; `crates/runtime/src/application/terrain_coordinator/camera.rs:41–55` — retained M2 camera rays
- [wowee-collision.md](../../wowee-collision.md): the WoWee floor query and step budget
- AzerothCore `src/tools/vmap4_extractor/wmo.cpp` and `Map.cpp`, in `~/Repos/azerothcore`: the MOPY rule, group skips and search depth

## See Also

- [[collision-system]]: the collision layers, including walls and doodads
- [[wmo-format]]: MOPY/MOBN/MOBR chunk layout
- [[stormwind-hilly-plaza]]: where the missing floor support was observed
- [[terrain]]: ADT heights and MODF placement
