# Movement Performance

Verified September 5, 2026 on local dev client code `4a503876`. Repeatable, collision-respecting movement now works. A short loaded-tile route showed no movement-specific FPS drop, but a separate tile crossing reproduced a **3.12-second frame-progress/IPC stall**. Startup parsing and solid canopy bounding boxes were separate blockers encountered before measurement.

## Startup and missing FPS number

The replicated-NPC spawn path synchronously called `load_m2_uncached` on the main thread. A live stack traced it through skeleton/bone-animation parsing; a 241-sample profile lost zero samples. The enabled FPS overlay initially rendered only `FPS:` because its numeric span starts empty and needs an Update diagnostic value.

After cache reuse in `484586ac`, the measured client reached connected InWorld at approximately 22 seconds rather than timing out after approximately 139 seconds. IPC and FPS updates resumed. The final code-`4a503876` [window capture](../../../data/diagnostics/movement-perf-20260905/fps-final.png) visibly contains `FPS: 44.17`, proving the missing numeric display is restored; that screenshot value is not a performance benchmark. The regression in `dfb29983` proves identical mesh vertices/indices on a second spawn after removing an isolated source-model copy. These observations establish startup recovery, not a general rendering-speed improvement. Earlier evidence and implementation details remain in [[procedural-cloud-regeneration]].

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

## Steady-route attribution

A separate 12-second, 49-Hz CPU attribution capture recorded 1,379 samples with zero lost. No FPS collected during profiling was used in the comparison. Transform parent propagation was the largest sampled symbol at **20.45% self cost**; skin extraction was **1.67%**. A later hierarchy dump contained 21,372 lines. The profile does not isolate which local, remote, or animated hierarchies cause that work, so no transform optimization is established yet.

## First recorded tile-boundary crossing

A separate two-second segment at heading `180` moved from Bevy `(-9016, 88.57, 6)` to XZ `(-9016,-8)`, crossing from tile `(32,48)` into `(31,48)`. The root file already existed as `data/terrain/777827.adt`, with March 5, 2026 file timestamps. `resolve_tile_path` selects existing coordinate-named or FDID-named files; it does not extract missing roots. The earlier description of this as an uncached/CASC-supplied tile was incorrect. OS page-cache and render-asset residency were not controlled. The client remained connected and ended with one loaded tile and no pending/failed loads.

The external log monitor observed these events relative to the movement command acknowledgement (approximately 50 ms polling resolution):

| Time | Event |
|---:|---|
| +0.94 s | Old tile unloaded; replacement background parse starts |
| +5.67 s | Background parse succeeds; heightmap registration starts |
| +5.77 s | Heightmap registration ends |
| +8.73 s | New tile spawn/memory statistics emitted |

The new tile contained 256 chunks, 650 doodads, and 8 WMOs. An IPC performance request spanning +5.745 to +8.869 seconds waited **3,124 ms**, versus a pre-crossing mean of 99 ms and maximum of 120 ms. The approximately 2.96-second registration-to-spawn-statistics gap brackets the main-thread entity/asset spawning path in `terrain_streaming::record_loaded_tile_entities`, including its bookkeeping/statistics. This is measured evidence of a boundary-associated main-loop stall; individual object creation, texture work, and deferred/render preparation costs are not yet isolated.

The delayed response still reported 36.92 FPS, illustrating why the smoothed FPS field alone does not measure the long wait. The old tile was removed approximately 7.8 seconds before replacement spawn statistics finished. With default `load_radius=0`, there was no already-loaded neighbor to cover the transition.

This was one recorded crossing, not a controlled cold/warm comparison or a survey of other maps. LOD-level swaps remain unmeasured. No streaming or canopy-collision fix was applied.

## Tile-application attribution follow-up

Two repeat captures on unchanged code `4a503876` observed approximately 350 ms between registration completion and tile-spawn statistics, versus approximately 2.96 seconds in the original run. Both used existing disk assets, but cache residency and host load were not controlled, so the difference is not an optimization result. The original application excerpt contains no CASC extraction/cache-miss/error lines.

The 499-Hz capture provides 141 samples in a conservative application-core interval. Strided `u8` maximum scans, texture conversion/compositing, and procedural water-normal work appear in those samples. The scan signature matches `fix_1bit_alpha` in `src/asset/blp.rs`, but poor stack unwinding prevents reliable enclosing-caller attribution. An event-triggered full-process stack did not catch the tile caller and is not positive attribution evidence.

Gated diagnostics added in `d4eaf8cf` and `7d9e7370` report BLP read/decode/convert/alpha durations (`blp_perf`) and tile/object stage durations (`tile_spawn_perf`) when `WOO_PERF_MOVEMENT` is present. They preserve the existing pixel and spawn operations. A subsequent crossing without a sampling profiler produced:

| Application work | Measured time |
|---|---:|
| 650 doodad spawn paths | 159.696 ms |
| 8 WMO spawn paths | 132.490 ms |
| Terrain water | 22.125 ms |
| Other terrain/setup, computed remainder | 2.154 ms |
| **Total `spawn_parsed_tile`** | **316.465 ms** |

The spawn-path measurements include synchronous asset preparation and recording Bevy commands, not subsequent deferred command execution or GPU preparation. Work runs in the main-world Update schedule, which may execute on a Compute Task Pool worker rather than the OS main thread.

Inside those stages, **156 CPU BLP loads across 147 paths** spent 17.559 ms reading files, 3.519 ms stripping/parsing BLP containers, 30.523 ms converting/decompressing to RGBA, and **110.015 ms normalizing alpha**. The log field `decode_us` means container parsing; `convert_us` includes `blp_to_image` conversion/decompression. These totals are nested inside the table above and must not be added to it.

Alpha normalization accounts for approximately 35% of this repeat's tile-application time: 77.803 ms inside doodad spawning and 32.212 ms inside WMO spawning. `fix_1bit_alpha` computes the maximum alpha over the entire RGBA image before deciding whether zero/one-valued alpha needs adjustment. It scans once per CPU-decoded load, not on every cache hit or every terrain batch. This directly measured cost corroborates the earlier strided-byte profile signature.

The repeat ended connected with the destination loaded. Three separate requests for missing `(30,47)`, `(30,48)`, and `(30,49)` roots failed; they are not a failure to load `(31,48)`. The approximately 2.6-second excess in the original application interval remains unattributed: the repeat timings do not retroactively explain it. No alpha, texture-cache, water-map, or spawning optimization was implemented.

## File-residency hypothesis test

A controlled follow-up on unchanged diagnostic code `7d9e7370` compared a normal crossing with a crossing after `POSIX_FADV_DONTNEED` hints on the 147 BLP paths previously loaded during application. `fincore` verified reported residency before and after; no files were deleted, no global cache drop or kernel setting was used, and file sizes/mtimes remained unchanged. The files total 7,775,564 logical bytes; resident counts are page-rounded.

| Case | Resident bytes immediately before/after hints | Tile application | BLP file reads | BLP alpha normalization |
|---|---:|---:|---:|---:|
| Control, no hints | 2,416,640 → 2,416,640 | 317.072 ms | 18.531 ms | 107.595 ms |
| Target BLP files evicted | 7,897,088 → 0 | 313.629 ms | 23.303 ms | 105.588 ms |

Both cases recorded 156 BLP loads. The control was only partially resident, not a fully warm baseline. This experiment rejects lack of residency in these target BLP files **as a sufficient explanation under the tested conditions**; it does not rule out other file/code pages, allocator state, or host contention during the original run.

Per-thread sampling brackets were approximately 411 ms, wider than application itself. The busiest Compute Task Pool thread accrued approximately 330/340 ms CPU time in the control/evicted cases, with zero observed major faults. These are bracketing observations, not exact exclusive stage CPU times. Scheduler statistics were disabled (`sched_schedstats=0`) and remained disabled, so stored runqueue counters must not be treated as active wait-time evidence.

The original raw log also recorded only three movement-system frames between the pre-stall report and the report after tile spawning, across approximately 3.2 seconds. That supports a real client-update stall rather than merely slow CLI startup. The original and repeat logs report the same eight WMO roots/group counts, 1,990 WMO collision meshes, and 520 doodad colliders; overall NPC/asset counts nevertheless varied.

The extra requests for `(30,47)`, `(30,48)`, and `(30,49)` are expected server 3×3-neighborhood requests when its player crosses into row 31, not proof that the intended tile was wrong. Source inspection additionally found render-frame-rate-dependent input emission and server movement applied once per received packet; no protocol or reconciliation fix was made. The original approximately 3.12-second stall remains open in `PLAN.md`.

## Sustained low-FPS checkpoint

A separate active investigation concerns the user's report of approximately 10 FPS after the client had remained open for more than 20 minutes. Initial settled full-scene IPC samples from PID `3323344` were 30–37 FPS, then 39–45 FPS; every sample reported `focused=false` despite an attempted Niri focus request. They therefore do **not** reproduce or disprove the reported focused 10-FPS condition.

The user reported the character disabled, but a scene dump’s `is_displayed=false` is a raycast/display heuristic, not proof that rendering or processing was disabled. The inspected PID had no `--inworld-stage` argument, so its exact character-disable mechanism was not established.

A temporary `--inworld-stage terrain` run was used only as a coarse user-requested isolation: it reported 56.56–76.23 FPS. This result is confounded because that existing stage disables more than NPCs, nameplates, and game UI: it also excludes lighting and particles. The cumulative `Terrain` stage includes `Character`; it does not itself disable local-character attachment. It is not evidence assigning the improvement to any one subsystem and does not test the exact selector below.

Commit `3d364dc8` adds the temporary `--inworld-stage no-npcs-ui` selector for that exact isolation. It retains local-character policy, terrain, skybox, lighting, particles, camera effects, networking, and the standalone FPS overlay, while excluding remote visual attachment and game UI processing/rendering. Consequently, nameplate observers and their Update systems are gated off; remote replicated state remains present. Behavioral tests cover selector/gate semantics and absence of an NPC nameplate. The first direct-helper launch, PID `3510050`, exited between Pyrun invocations; it is not the measurement client. Native Niri client PID `3517219` (binary SHA256 `55708195e5b81a51043a1f986ed11a4afaa7181e423ec9a98b09490500444a04`) remained connected with zero pending terrain and an empty game-UI tree. Nine of ten subsequent focused samples were **9.66–12.14 FPS**, with the tenth at 17.87 FPS. Removing NPC visuals, nameplates, and game UI therefore did not eliminate the sustained symptom; it does not prove those systems have zero cost. Artifacts: `settled-low-fps/{native-no-npcs-ui-live.json,settled-no-npcs-ui-performance.json}`.

A subsequent CPU leaf profile had approximately 3,000 readable samples and zero reported loss. Grouped self-symbol percentages were approximately 2.29% transform-related and 11.33% material/prepare/bind-related, with 19.77% unknown offsets. These are symbol-group observations, not caller ancestry or confirmed costs during the 10-FPS interval; the capture followed the FPS samples, and its header metadata is malformed. Later process-associated AMDGPU fdinfo counters measured 12.188 GPU-engine seconds and 38.74 CPU seconds over 18.434 wall seconds, but that window ran at 27.69–43.38 FPS, not 10. Both file descriptors share one DRM client ID, so only one was counted.

The next untested candidate is terrain-material modification churn. Both `update_terrain_animation_time` and `sync_terrain_environment_map` iterate mutable terrain assets; Bevy 0.19 queues `AssetEvent::Modified` for every yielded asset even when the environment handle comparison makes no assignment. With 256 terrain materials, stopping only the time writer would leave the second source of modification events. A settled-scene freeze of both systems can isolate this candidate without removing terrain rendering. Commit `4ab3deb6` adds opt-in `--freeze-terrain-materials-after <SECONDS>`: normal startup updates continue until a `Time<Real>` deadline, then both systems are gated off together and log the cutoff with elapsed time, deadline, and material count. It retains terrain material assets and their last time/environment values; terrain rendering, textures, lighting, particles, and other material systems remain active. This intentionally freezes animated terrain UVs and future environment-map synchronization, so it applies only after a settled scene has loaded. Focused tests (3/3) and the build passed; runtime comparison is pending. No production optimization is established.

### Controlled terrain-material freeze

On diagnostic code `4ab3deb6`, the main-thread-controlled run kept the same client, view, terrain objects, lighting, particles, and `no-npcs-ui` exclusions. At a logged real-time deadline of **60.006 seconds**, both terrain-material writers stopped for **256 materials**, retaining their current values and handles. Eight focused samples before the cutoff averaged **19.87 FPS / 50.67 ms**; eight after averaged **67.80 FPS / 16.17 ms**, including a 30.88-FPS low outlier. Screenshots retain the same visible geometry. This is evidence of substantial cost from the terrain-material update path in that scene, not proof that it explains every reported 10-FPS episode. Normal updates were restored afterward; no production optimization was applied.

Artifacts: `settled-low-fps/terrain-freeze-case/{before,after,freeze-event,result,requests}.json`, `before.webp`, and `after.webp`. An earlier worker-run 35-second capture had only two pre-cutoff samples and is not substituted for this main-controlled comparison.

### Terrain without world objects

The user's next requested run retained normal terrain-material updates and added `--no-terrain-objects` to `--inworld-stage no-npcs-ui`. Code `d289a685` disables streamed `_obj*` companion loading, M2/WMO preloading and spawning, and later object-LOD loading; it does not merely hide objects after loading them. Terrain meshes/textures, water, lighting, and particles retain their existing paths.

Client PID `3700518`, SHA256 `1e6cc3a2394eb908965049f94d511411565ff83a960c1abaaf0ce53f114c7327`, remained connected at XZ `(-9016,-9.75)` with one loaded tile, no pending loads, an empty game-UI tree, **zero WMO collision meshes / zero doodad colliders**, and **256 terrain materials**. Mesh assets fell from the preceding object-loaded snapshot's 3,458 to 362; standard materials fell from 3,091 to 1. User observed approximately 60 FPS. Ten IPC samples ranged **18.99–67.82 FPS**, mean **54.41 FPS**; four reported focused and six unfocused, so this is not a fully focus-controlled benchmark.

Screenshots show a dark, close-up/sloped terrain view rather than a representative outdoor vista; this limits generalization. The terrain-only screenshot removes the object foliage seen in the preceding capture. Object-related work is a remaining cost candidate, but this run does not distinguish drawing, material preparation, animation, transforms, or other object processing. Neither this result nor the separate material-freeze result resolves the original tile hitch.

Artifacts: `settled-low-fps/no-objects-case/{identity,performance}.json`, `terrain.txt`, `ui.txt`, `tree.txt`, `client/stderr.log`, and `terrain-only.webp`. Behavioral tests cover actual file-backed terrain/placement parsing, empty disabled preloads, default loading, CLI selection, and suppressed object-LOD reload decisions.

### Water and skybox visual removal

The next requested diagnostic (`544d32a5`) adds `--no-terrain-water --no-skybox` to the existing exclusions. ADT water data is omitted before surface spawning and water-height registration; authored skybox visual systems are gated, and sky-dome entities are removed after spawning commands finish. Lighting, fog/environment lighting, game time, camera effects, and normal terrain-material updates remain unchanged. Shared cloud/sky resources and empty material plugins remain initialized; this is still not a minimal terrain renderer.

PID `3831399`, SHA256 `0651ee1a516fce0d77c77efda512f2dcb37f79970c0a2c35443cce3e44a034c1`, recorded **zero water materials**, 256 terrain materials, 261 mesh assets, no object placement loads, an empty game-UI tree, and no pending terrain. Ten samples were **39.54–54.31 FPS**, mean **46.65 FPS / 21.59 ms**, all reporting unfocused. The pre-restart client reported XZ `(-8923.71,151.58)` and the new client `(-8866.80,329.22)`; view/position equality was not preserved. Do not treat this as a controlled before/after speed result or claim water/skybox cost is zero.

Artifacts: `settled-low-fps/water-sky-case/{identity,performance,summary}.json`, `previous-position.txt`, `position.txt`, `terrain.txt`, `tree.txt`, and `no-water-skybox.webp`. The earlier same-process terrain-material freeze remains the stronger measured lead; its effect in this further-reduced scene is not yet tested.

## Sources

- [Measurement artifacts](../../../data/diagnostics/movement-perf-20260905/) — loaded-route samples/profile/tree, `tile-attribution/stage-timings/{application-excerpt.log,blp-summary.json,result.json}`, and `tile-attribution/file-residency/{summary.json,control/,target-blp-evicted/}` for measured subcosts and residency controls.
- [Route calculations](../../../data/diagnostics/movement-perf-20260905/computed-doodad-boxes.json) and [candidate selection](../../../data/diagnostics/movement-perf-20260905/find_clear_route.py) — cached assets only; raw placement-Y caveat above.
- [Movement/collision](../../../src/rendering/camera/camera.rs), [collision math](../../../src/collision.rs), [doodad spawning](../../../src/rendering/terrain/terrain_objects.rs), [BLP loading](../../../src/asset/blp.rs), and [tile stage timers](../../../src/rendering/terrain/terrain_spawn_perf.rs) — actual control, loading, and measurement boundaries.
- [Boundary samples](../../../data/diagnostics/movement-perf-20260905/boundary-samples.json), [event timeline](../../../data/diagnostics/movement-perf-20260905/boundary-events.json), and [streaming implementation](../../../src/rendering/terrain/terrain_streaming.rs) — first-crossing evidence and application boundary.
- [Movement spec](../../specs/scripted-movement.md), [InWorld scene-isolation spec](../../specs/inworld-scene-isolation.md), and [startup investigation](procedural-cloud-regeneration.md) — control contracts, selector scope, and earlier proof.

## See Also

- [[scripted-movement]] — automated route controls
- [[terrain]] — streaming and placement
- [[rendering-pipeline]] — broader rendering work
