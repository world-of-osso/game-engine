# Movement Performance

Verified September 5, 2026 on local dev client code `4a503876`. A repeatable, collision-respecting route now works. No movement-specific frame-rate drop was demonstrated on this short loaded-tile route. Startup parsing and solid canopy bounding boxes were separate blockers encountered before measurement.

## Startup and missing FPS number

The replicated-NPC spawn path synchronously called `load_m2_uncached` on the main thread. A live stack traced it through skeleton/bone-animation parsing; a 241-sample profile lost zero samples. The enabled FPS overlay initially rendered only `FPS:` because its numeric span starts empty and needs an Update diagnostic value.

After cache reuse in `484586ac`, the measured client reached connected InWorld at approximately 22 seconds rather than timing out after approximately 139 seconds. IPC and FPS updates resumed. The regression in `dfb29983` proves identical mesh vertices/indices on a second spawn after removing an isolated source-model copy. These observations establish startup recovery, not a general rendering-speed improvement. Earlier evidence and implementation details remain in [[procedural-cloud-regeneration]].

## Why the first routes could not move

At Bevy XZ `(-8949,132)`, both the pathfinder and timed movement failed to advance. The `WOO_PERF_MOVEMENT` distance counters added in `4a503876` isolated the boundary: a three-second segment proposed approximately 21 yards; WMO collision retained all of it; doodad collision reduced it to zero.

Local MDDF records use their `0x40` FileDataID flag, so missing MMDX/MMID string tables do not prevent model resolution. Reconstructing the current placement rotations and model bounds produced **858** colliders, matching the runtime count. Two containing placements use FDID `189929`, `world/azeroth/elwynn/passivedoodads/trees/elwynntreecanopy03.m2`. One calculated box spans X `-9092.43..-8921.62` and Z `87.62..257.96`. The placement-height clamp can raise its Y bounds; the initial calculations use raw placement Y.

`build_doodad_collider` treats sufficiently large render-model bounding boxes as solid colliders. `ray_aabb_intersect` returns zero distance when the movement ray starts inside one. Thus canopy bounds can trap a character despite no ground-level obstruction. Collision behavior was **not changed** for this investigation.

## Reproducible route

[[scripted-movement]] documents the control contract. The measured clear route is entirely inside tile `(32,48)`:

- Bevy X stays `-8916`; Z alternates between `172.67` and `186.67`.
- Use two-second forward segments at headings `0` and `180`, with automatic expiry between segments.
- Local bounds calculations exclude all doodad XZ boxes along a 20-yard corridor; sampled terrain slope is below five degrees. Actual runtime movement independently confirmed the route.

Each segment advanced **14 yards**, then remained stationary. Ten segments produced 140 yards of horizontal travel; the probe measured 140.082 yards including terrain-height changes. The administrative starting-position reset occurred before the measurement window; no teleport was counted as movement.

## Unprofiled comparison

Same binary and client, focused throughout; one loaded tile, zero pending tiles. Remote entities changed from 121 to 120, so the workload was close but not perfectly identical. The route includes short gaps between timed segments.

| Phase | FPS samples | Mean FPS | Mean frame time | Process CPU, one core = 100% |
|---|---:|---:|---:|---:|
| Idle before | 8 | 26.58 | 37.84 ms | 270.69% |
| Moving route | 20 | 26.97 | 38.02 ms | 276.68% |
| Idle after | 8 | 29.39 | 34.27 ms | 276.27% |

The instrumented portion of `player_movement` averaged 177 microseconds idle and 359 microseconds during the route. Collision averaged 214 microseconds per moving frame. These are **not whole-frame or camera-follow timings**. Frame-rate variation and the later faster idle result do not support a causal movement-specific drop.

## Attribution and remaining limits

A separate 12-second, 49-Hz CPU attribution capture recorded 1,379 samples with zero lost. No FPS collected during profiling was used in the comparison. Transform parent propagation was the largest sampled symbol at **20.45% self cost**; skin extraction was **1.67%**. A later hierarchy dump contained 21,372 lines. The profile does not isolate which local, remote, or animated hierarchies cause that work, so no transform optimization is established yet.

Tile-boundary streaming and LOD transitions were **not measured**. The route stayed within one loaded tile; adjacent root ADTs were absent from the local terrain directory during the investigation. Source shows main-thread tile application and synchronous LOD swaps, but these remain unmeasured risks rather than demonstrated causes.

## Sources

- [Measurement artifacts](../../../data/diagnostics/movement-perf-20260905/) — `clear-route-performance-samples.json`, `clear-route-performance-phases.json`, `clear-route-probe-summary.json`, `clear-route.perf-report.txt`, `clear-route-displacement.json`, and `clear-route-tree.txt`.
- [Route calculations](../../../data/diagnostics/movement-perf-20260905/computed-doodad-boxes.json) and [candidate selection](../../../data/diagnostics/movement-perf-20260905/find_clear_route.py) — cached assets only; raw placement-Y caveat above.
- [Movement/collision](../../../src/rendering/camera/camera.rs), [collision math](../../../src/collision.rs), and [doodad collider construction](../../../src/rendering/terrain/terrain_objects.rs) — actual path and clamp boundaries.
- [Movement spec](../../specs/scripted-movement.md) and [startup investigation](procedural-cloud-regeneration.md) — contracts and earlier proof.

## See Also

- [[scripted-movement]] — automated route controls
- [[terrain]] — streaming and placement
- [[rendering-pipeline]] — broader rendering work
