# World loading screen (Godot client)

The loading screen shown on world entry and map transfers hides only once the world under the player can be drawn: terrain, the player's model, and the authored objects of the tile the player stands on. Readiness is one path: `lib.rs` `update_loading_readiness` → `loading::evaluate_native_loading`, which adds the object stage to the shared terrain rules (`src/game/state/loading_readiness.rs`).

## What it must do

- [x] The shared rules come first: the local player is placed and their model attached or failed (`Initializing character...`), the map's WDT is read (`Waiting for terrain...`), and the center tile's terrain is attached (`Loading terrain...`; an unbuildable tile shows `Terrain failed to load`).
- [x] Then the center tile's objects: every ADT doodad and WMO placed on it, and the MODD doodads of each of those WMOs once it spawns, is attached with its camera collision body or has failed. The center tile's WMO collision groups must be built too. The bar shows `Loading objects done/total...` from 86 to 99%.
- [x] A placement that fails (missing asset, unreadable model) is reported (`godot_error!`, `world_objects.failures`) and counts as done, so a missing asset cannot hold the loading screen.
- [x] The center tile's placements spawn ahead of the neighbouring tiles' (`TerrainObjects::prioritize_tile`), and a spawned WMO's MODD doodads spawn right after it, ahead of the rest of the queue.
- [x] A WMO-only map (a dungeon) has no tiles; it finishes with its global WMO, as before.
- [x] Neighbouring tiles' objects keep streaming in after the loading screen hides. The camera collides with each as soon as its body is added (see [WMO floor collision](wmo-floor-collision.md)).
- [ ] The retail client's loading-screen criteria are unknown: no source found (worldentry). This gate is the user's requirement, not a retail reproduction.

## Implementation inventory

- `godot/rust/src/loading.rs`: `TileObjects`, `NativeLoading`, `evaluate_native_loading`.
- `godot/rust/src/terrain/objects.rs`: `TileProgress`, `tile_progress`, `prioritize_tile`, MODD doodads queued at the front.
- `godot/rust/src/wmo/collision.rs`: `tile_pending`.
- `godot/rust/src/lib.rs`: `update_loading_readiness`.

## Tests asserting this spec

- `godot/rust/src/loading.rs` `center_tile_objects_hold_loading_until_attached_or_failed` (and the terrain-stage tests).
- `godot/tests/world_entry_camera.gd` (live): at Northshire Abbey the first in-world frame has the Northshire oak (MDDF 10452) with its `M2Collision` body and the abbey WMO 10286 with its doodads.

## Measured

Northshire Abbey (azeroth_32_48, 986 ADT placements plus 268-355 MODD doodads of its WMOs), headless client on a private server, on a shared, loaded host: loading took 2.8-5.8 s (median 4.4 s, 6 runs) before the object gate and 5.5-19.2 s (median 8.4 s, 7 runs) after. Before the tile prioritisation it took 14-50 s, with the center tile waiting behind about 6,000 neighbouring placements.
