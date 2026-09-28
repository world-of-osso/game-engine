# World map

Contract: [world-map spec](../../specs/world-map.md).

## Data flow

- `UiMapCatalog` (`src/ui_map_data.rs`) loads `UiMap`, `UiMapAssignment`, `UiMapXMapArt`, `UiMapArt`, `UiMapArtStyleLayer` and `UiMapArtTile` from `data/db2/12.1.0.69933/*.csv` once per session (Godot: on first open).
- Only `System = 0` (world) maps take part; assignments with a `WMOGroupID` or `MapID = -1` are ignored.
- World → map UV per assignment: `u = ui_min.x + (region_max.y - y) / span_y · (ui_max.x - ui_min.x)`, `v` likewise from `x`. Engine (Y-up) positions convert with `world = [x, -z, y]` (same as the server's `bevy_to_wow`).
- Child lookup inverts the parent assignment containing the UV to world `x, y`, then picks the smallest child whose whole-map assignment on that map contains it. The child rect is its region projected onto the parent assignment, clipped to that assignment's UV rect (continent art regions include ocean padding past Azeroth's map-0 rect).
- `world_map_frame_state` (`src/world_map_view_data.rs`) turns catalog + displayed map + player + quest log into `WorldMapFrameState`; `world_map_frame_screen` draws it; `apply_world_map_postsetup` sets the arrow rotation and the highlight's additive blend, which `rsx!` has no attribute for.

## Arrow rotation

Engine forward is `(sin yaw, 0, cos yaw)` → world `(sin yaw, -cos yaw)` → map `(cos yaw, -sin yaw)`. Registry rotation is counter-clockwise on screen, so the north-up arrow uses `yaw - π/2`. The Godot projection turns controls clockwise and negates it. `world_map_view_data_tests::arrow_points_where_forward_movement_goes_on_the_map` and the Godot fixture assert arrow direction against actual movement.

## Native scaled pointer conversion

At `364a29ac`, the native map layout uses the effective UI-scale logical viewport and maps physical mouse coordinates by dividing them by that scale before frame containment, canvas UV, hover, or navigation handling. The pure helper is 1/1 GREEN for a scaled center hit and outside miss (`/tmp/claude/cargo-ui-scale-map.out`). An authenticated live map click remains untested; this is not input or visual parity.

## Local CASC gaps

The synced WoW install is incomplete: many `interface/worldmap/*` tiles report "Archive location not found" (all Elwynn Forest tiles, 11/12 Stormwind City, parts of Eastern Kingdoms/Azeroth; Kalimdor and Durotar extract). The Godot host asks the resolver for every texture, keeps a per-FDID availability map, and drops unavailable tiles from the state so the projection never loads a missing file.

## Marker data availability (2026-09-28)

- Quest POIs: server `QuestLogSnapshot`/`QuestLogUpdate` → `QuestPoiSnapshot` polygons in world yards with `map_id`; pinned at the polygon centroid.
- Flight masters: `TaxiNodes.csv` (`Flags` 0x1 Alliance map, 0x2 Horde map); faction from `ChrRaces.Alliance`.
- Missing: world quests, area POIs/events, vignettes, dungeon entrances, explored overlays, party pins — see the spec's Out of scope.
