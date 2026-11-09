# Dungeon camera below ADT terrain

VERIFIED (2026-11-09): local engine master `7b7aea959`, Ruby Life Pools entrance, private UDP `127.0.0.1:5974`, Forever skin. The player stands at WoW `(1386.2, -169.719, 131.0005)` on a WMO floor. ADT is at `204.6584`, 73.66 yd overhead. A clear 15 yd orbit instead settles almost vertically above the player (height 17.97 yd, only 0.105 yd horizontal separation). The independent Godot physics segment to the intended orbit reports no hit.

## Decided rule

Camera collision uses the first actual collidable surface intersected by the eye-to-camera segment. Terrain, WMOs, and doodads participate equally. A high ADT height is not a solid half-space under that height. Do not select collision behavior by map ID, dungeon flag, or a guessed indoor threshold. The same ray rule applies above terrain, below terrain, and through authored holes. Keep the existing collision offset, smooth distance recovery, follow smoothing, and final eye-to-smoothed-camera sight check.

Remove the redundant ADT sample pass and height-only camera ground clamp. Native physics already contains the rendered, hole-aware ADT triangles plus WMO and doodad collision. A segment that actually crosses the terrain still collides. No substitute height clamp or missing-grid ground plane is permitted.

## Player support and reference behavior

VERIFIED: `TerrainGround::static_ground` passes the player's full 3D feet position to `shared::ground::{ground_at,collision_height}`. WMO floors are selected in a bounded vertical search; the camera instead used only ADT x/z heights. The regression test establishes WMO support below ADT before exercising the native camera follow boundary. No new player floor/group state is needed: actual segment/triangle intersection already answers which local geometry obstructs this view.

Retail's normal behind-player Ruby camera is the task's observed reference, not an independently reverse-engineered internal algorithm. VERIFIED reference implementation: local `~/Repos/solarityclient/crates/runtime/src/application/terrain_coordinator/camera.rs` combines nearest terrain/WMO/M2 segment hits; `terrain_coordinator.rs::trace_collision` traces finite hole-aware ADT surfaces. Local TrinityCore `TerrainMgr.cpp::GetStaticHeight` rejects terrain above the query's Z as a floor. These support the geometric rule without claiming identical Retail internals.

## Acceptance and proof

- RED native real-data fixture: Ruby WMO floor below ADT must retain the clear 15 yd orbit.
- Outdoor real-data control: downward orbit must retreat before actual ADT triangles.
- RED rendered private-server fixture: `godot/tests/dungeon_camera.gd` captures Forever and compares settled distance to an independent Godot physics ray.
- Preserve existing Stockade WMO-wall and stairwell camera coverage, core orbit/zoom/recovery/settling coverage.
- Final gates: debug/release cargo check, zero warnings, fmt.

VERIFIED bounded search for another affected dungeon: entrance samples in Neltharus (floor 72.60, ADT 0), Halls of Infusion (floor 20.82, ADT -190.65), and Azure Vault (floor 594.22, ADT 0) all retain a clear 15 yd orbit on the baseline. This does not qualify other positions or prove those whole dungeons unaffected. No second affected location found in these entrance samples.

Evidence: lane-local `data/diagnostics/dungeon-camera-adt-2026-11-09/` contains logs/JSON and the unaffected entrance captures. Requested screenshots are under `/syncthing/AgentShared/2026-11-09/dungeon-camera/` (`ruby-before.png`; after/outdoor pending).

## Sources

- [Native camera](../../../godot/rust/src/camera.rs) — real rays and the old height-only approximation.
- [Shared follow](../../../godot/core/src/camera_follow_data.rs) — orbit and collision-distance recovery.
- [Player ground](../../../godot/rust/src/ground.rs) — 3D support selection.
- [Camera geometry tests](../../../godot/rust/src/wmo/collision_tests.rs) — Ruby and outdoor regressions.
- [Follower combat FPS](follower-combat-fps.md#camera-under-the-adt-terrain-not-fixed) — original report.

## See Also

- [[follower-combat-fps]] — discovery and original scene.
