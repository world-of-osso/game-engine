# Dungeon camera below ADT terrain

VERIFIED (2026-11-09): local engine master `7b7aea959`, Ruby Life Pools entrance, private UDP `127.0.0.1:5974`, Forever skin. The player stands at WoW `(1386.2, -169.719, 131.0005)` on a WMO floor. ADT is at `204.6584`, 73.66 yd overhead. A clear 15 yd orbit instead settles almost vertically above the player (height 17.97 yd, only 0.105 yd horizontal separation). The independent Godot physics segment to the intended orbit reports no hit.

## Decided rule

Camera collision uses the first actual collidable surface intersected by the eye-to-camera segment. Terrain, WMOs, and doodads participate equally. A high ADT height is not a solid half-space under that height. Do not select collision behavior by map ID, dungeon flag, or a guessed indoor threshold. The same ray rule applies above terrain, below terrain, and through authored holes. Keep the existing collision offset, smooth distance recovery, follow smoothing, and final eye-to-smoothed-camera sight check.

VERIFIED implementation: commits `4da81cdc3` and `43b8d3434` remove the ADT sample sweep, the height-only ground clamp and terrain plumbing into the camera follow calculation. `raycast_solid` still queries actual terrain, WMO and doodad collision. There is no replacement height fallback.

The camera now relies on native physics geometry: hole-aware ADT triangles, WMO collision and doodad collision. A segment that crosses terrain still collides. No height clamp or missing-grid ground plane remains.

## Player support and reference behavior

VERIFIED: `TerrainGround::static_ground` passes the player's full 3D feet position to `shared::ground::{ground_at,collision_height}`. WMO floors are selected in a bounded vertical search; the camera instead used only ADT x/z heights. The regression test establishes WMO support below ADT before exercising the native camera follow boundary. No new player floor/group state is needed: actual segment/triangle intersection already answers which local geometry obstructs this view.

Retail's normal behind-player Ruby camera is the task's observed reference, not an independently reverse-engineered internal algorithm. VERIFIED reference implementation: local `~/Repos/solarityclient/crates/runtime/src/application/terrain_coordinator/camera.rs` combines nearest terrain/WMO/M2 segment hits; `terrain_coordinator.rs::trace_collision` traces finite hole-aware ADT surfaces. Local TrinityCore `TerrainMgr.cpp::GetStaticHeight` rejects terrain above the query's Z as a floor. These support the geometric rule without claiming identical Retail internals.

## Acceptance and proof

- VERIFIED native Ruby fixture: WMO support at 131.0005 lies below ADT 204.6584, the 15 yd orbit ray is clear, and the camera reaches the intended pose. The camera-named native suite passes 17/17 (`camera-native-green.log`).
- VERIFIED native outdoor control: the downward orbit retreats before actual ADT triangles. Existing Stockade stairwell and indoor-WMO wall tests also pass (`camera-native-green.log`).
- VERIFIED core camera behavior: `camera_data` passes 9/9, including mesh hits, recovery, clear orbit below overhead terrain, zoom and final sight obstruction (`core-camera-green.log`).
- VERIFIED rendered Forever Ruby before/after with an independent Godot physics-ray oracle: camera distance changes from 16.169 yd to 15.000 yd; height changes from 17.969 yd to 6.943 yd. Terrain, object and collision queues are all 0. The outdoor Azeroth capture retains a 15 yd eye-to-camera distance and passes the same ray oracle. Original 1920×1080 PNGs are `/syncthing/AgentShared/2026-11-09/dungeon-camera/ruby-before.png`, `ruby-after.png`, and `outdoor-after.png`; JSON/log receipts are in `data/diagnostics/dungeon-camera-adt-2026-11-09/`.
- VERIFIED final gates: debug and release workspace `cargo check --all-targets` each report 0 warnings and 0 errors; `cargo fmt --check` passes (`check-debug.log`, `check-release.log`).

VERIFIED bounded search for another affected dungeon: entrance samples in Neltharus (floor 72.60, ADT 0), Halls of Infusion (floor 20.82, ADT -190.65), and Azure Vault (floor 594.22, ADT 0) all retain a clear 15 yd orbit on the baseline. This does not qualify other positions or prove those whole dungeons unaffected. No second affected location found in these entrance samples.

Evidence: lane-local `data/diagnostics/dungeon-camera-adt-2026-11-09/` contains logs, JSON receipts and unaffected entrance captures. All requested screenshots are under `/syncthing/AgentShared/2026-11-09/dungeon-camera/`: `ruby-before.png`, `ruby-after.png`, and `outdoor-after.png`.

## Sources

- [Native camera](../../../godot/rust/src/camera.rs) — real rays and the old height-only approximation.
- [Shared follow](../../../godot/core/src/camera_follow_data.rs) — orbit and collision-distance recovery.
- [Player ground](../../../godot/rust/src/ground.rs) — 3D support selection.
- [Camera geometry tests](../../../godot/rust/src/wmo/collision_tests.rs) — Ruby and outdoor regressions.
- [Follower combat FPS](follower-combat-fps.md#camera-under-the-adt-terrain-fixed) — original report.

## See Also

- [[follower-combat-fps]] — discovery and original scene.
