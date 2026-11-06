# Healer ground rings

Contract: [Healer ground rings spec](../specs/healer-ground-rings.md). Client only; the
server and protocol are unchanged.

## Implementation

| Layer | Path | Role |
| --- | --- | --- |
| DB2 | `godot/core/src/dispel_data.rs` | Streams `SpellEffect.csv`, keeps `Effect` 38 rows by `SpellID` and their `EffectMiscValue_0` school, and reads `ChrSpecialization.Role` for healer specs. |
| Options | `godot/core/src/client_options_data.rs` | `HealerGroundRingOptions` under `hud.healerGroundRings`: enabled, all_roles, colour-blind ramp, raid lowest N. |
| Rules | `godot/ui-model/src/healer_ground_ring.rs` | Arc span, 92%/90% fade, palette ramps, scope, lowest N, dispel filtering, chevron placement, ring ray hit and station angles. |
| Render | `godot/rust/src/healer_ground_ring.rs` | Per-member draped arc mesh, pulsing outer ring, chevron `Polygon2D` layer, click picking, offline fixture API. |
| Options UI | `godot/ui-model/src/ui/options_menu_data.rs`, `options_menu_active_sections.rs` | HUD page rows (shared by both skins). |
| Fixture | `godot/tests/healer_ground_ring.gd` | Forever captures and pixel/cost proof. |

## Decisions

- **Arc direction.** The visible arc keeps its end at 12 o'clock and its start advances
  clockwise; the gap grows clockwise from 12. Draped mesh stations sit every 7.5°, and the
  exact arc ends interpolate between two stations, so health steps stay smooth without extra
  ground samples.
- **Self excluded.** Party frames omit the local player, and a ring under the camera's own
  character reads as clutter.
- **Dispel ring at any health.** The health arc fades at 92%, but a dispellable debuff is
  actionable at 100%, so the outer ring does not fade with health.
- **Schools from data.** Healer role is `ChrSpecialization.Role` 1; schools are the union of
  `SpellEffect` 38 rows for the spells the server lists as known. No spell ids are written in
  code. `SpecializationSpells` already lists exactly one cleanse per healing spec (4987, 527,
  88423, 77130, 115450, 360823), which the tests pin against the real export.
- **Raid darkest first.** In a raid the option shows the N lowest-health members, still
  subject to the health fade threshold.

## Ground contact and cost

- Every vertex samples `TerrainObjects::visible_surface` (terrain + doodad/WMO BVHs) inside
  the same 1 yd rise window as ground spell draping, so rings climb stairs and carpets.
  `mesh_min_y`/`mesh_max_y` in the fixture state exposes the drawn span.
- A ring drapes when it first appears, once 2 s later (late streamed surfaces), and whenever
  the member moved more than 0.35 yd. A still ring costs no ground queries. Materials update
  only when colour/alpha changes; the dispel pulse is quantized to 32 steps; chevrons
  re-project at 10 Hz.
- Measured with `CAPTURE_CASE=fps` (debug build, host under load): the feature's own update
  averages 0.12-0.16 ms per frame over ~1000 frames; on/off process-time medians differ by
  -3.1 to +0.6 ms, i.e. inside the host's noise band.

## Evidence

- Commits `c8086409b` (model, DB2, options) and later renderer commit (see handoff
  `handoff-healer-ground-ring.md`).
- Unit tests: `cargo test -p game-engine-core dispel_data`,
  `-p game-engine-ui-model --lib healer_ground_ring`, options policy and views,
  `-p game-engine-godot healer_ground_ring`.
- Forever captures and diffs: `/syncthing/AgentShared/2026-11-05/healer-ground-ring/`
  (`party-rings`/`party-no-rings`, `chevron-rings`/`chevron-no-rings`,
  `stairs-rings`/`stairs-no-rings`). The party case asserts single pixels inside and outside
  the 62% and 28% arcs and the click pick targets the ring's member while the gap picks
  nothing.

## Fixture gotchas

- `preview_world_map` must be called with the capture point; the preview camera must stand
  on the same side of the Stormwind wall as the members, or the wall occludes the whole ring
  (uniform fog-like frame, zero diff).
- Party members stay on the pavement row at `z=-770`: the terrain south of the wall is clean
  but the wall blocks the low camera.
- The ring must be lifted `RING_LIFT` (0.02 yd) above the sampled surface; anchoring at the
  surface z-fights the terrain and only speckles appear.
