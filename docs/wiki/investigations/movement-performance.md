# Movement Performance

Verified September 5, 2026 on local dev client code `4a503876`. Repeatable, collision-respecting movement now works. A short loaded-tile route showed no movement-specific FPS drop, but a separate tile crossing reproduced a **3.12-second frame-progress/IPC stall**. Startup parsing and solid canopy bounding boxes were separate blockers encountered before measurement.

Current investigation: [foreground firmware-clamp evidence](#foreground-firmware-clamp-evidence) now explains a captured class of FPS collapses; the thermal-policy/cooling cause and original tile hitch remain unresolved.

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

Artifacts: `settled-low-fps/water-sky-case/{identity,performance,summary}.json`, `previous-position.txt`, `position.txt`, `terrain.txt`, `tree.txt`, and `no-water-skybox.webp`. The earlier object-present freeze suggested substantial material-update cost, but the reduced-scene repeat below did not reproduce a sustained gain.

### Conflicting freeze result in the reduced scene

A follow-up on unchanged code `544d32a5` retained every exclusion and froze the same 256 terrain materials at a logged **60.018 seconds**. PID `3889239` kept XZ `(-8866.80,329.22)`, asset counts, and the same visible terrain view. All samples reported focused. Eight before samples averaged **47.53 FPS**. The first after sample reached 80.95 FPS, then seven fell to **21.32–27.43 FPS**; the full after mean was **31.81 FPS**. This contradicts a general claim that freezing material updates resolves the current slowdown.

Across the sampled counter brackets, process CPU time rose from 27.49 seconds over 14.234 wall seconds to 37.49 over 14.617; process-associated GPU-engine busy fraction rose from 44.05% to 54.12%. Host CPU-pressure snapshots stayed low. These counters do not identify the changed work or establish clock, power, scheduling, shader, or render-pipeline causality. CPU/GPU frequency and per-thread stacks during the slowdown were not captured.

Normal updates were restored in PID `3896680` with all user-requested exclusions retained. Six later focused samples, 205–215 seconds after its launch, were **48.20–54.57 FPS**. Age alone is therefore not established as the trigger either. Preserve both freeze outcomes; the bottleneck remains unresolved. Next evidence needed is a CPU/render-state capture during the slow frozen interval, not another claim of improvement from the first fast sample.

Artifacts: `settled-low-fps/reduced-terrain-freeze-case/{before,after,freeze-event,result,requests,restored-performance}.json`, the paired screenshots, and `restored/identity.json`.

### Flat, untextured terrain

Diagnostic `cd557051` adds `--no-terrain-textures`. Streamed chunks use one flat, lit, double-sided material rather than the custom multilayer terrain material; image registration is skipped while geometry/culling metadata remain. Background texture parsing/decoding still occurs. This bypasses both terrain texture rendering and custom material-update/preparation work, so it is not a texture-bandwidth-only experiment.

PID `4060136`, SHA256 `8c32116bebb1f4556b264690cc3d2a42bc7949ed0d57462eba56e040fd40493f`, retained all prior exclusions. Runtime showed 261 meshes, 16 images, two standard materials, and zero custom terrain/water/effect materials. Five initial focused samples averaged **116.00 FPS** (112.89–123.43), followed by five at **15.66–20.15 FPS**, mean **17.85**. The user's foreground screenshots show both 16.39 and 121.20 FPS. Removing textures/custom terrain materials did not eliminate the slow periods.

The first CPU profile was taken after recovery (48–85 FPS, unfocused), not during the original low interval. A later automatic watcher captured three sub-30-FPS samples before recording another profile; after that profile, FPS was still 18.54. Leaf symbols include ECS executor/task queues/locks, but incomplete symbolization/ancestry prevents precise subsystem ownership. A focus-only comparison did not reproduce 20 FPS: the final focused group was 53.91–60.19 FPS. Bevy's default runner uses different focused/unfocused update strategies, but background-only throttling does not explain the user's foreground drops.

Artifacts: `settled-low-fps/no-textures-case/` contains identity, samples, snapshots, user screenshots, `slow-state/`, `focus-comparison-2/`, and `auto-low-capture/`. Source verification passed fmt/check, the two targeted tests, and build; no optimization was applied.

### Terrain-rendering-off control

Commit `1ce425ea`, clarified by `9d4e5d53`, adds `--no-terrain-meshes`: the streamed tile root, background parsing, height/streaming lifecycle, and height queries remain, but ground chunk meshes and their terrain materials/images are not created. It is therefore a settled rendering-workload control, not an empty-app or all-terrain-CPU measurement. The first reference/terrain-off pair ran on `9d4e5d53`; it was interrupted by the monitor-layout change and its reference had only 15 samples, so it is not a comparative result.

A later **120-second focused** terrain-off observation recorded 60 samples, mean **98.25 FPS** and **311.67%** process CPU. Twenty fast samples were 123.08–169.99 FPS with CPU limits 2,273–4,311 MHz and GPU limits 1,158–2,476 MHz. Its two 19.63–26.72 FPS lows instead had both enforced limits at 600 MHz. Removing terrain ground meshes therefore did not eliminate intermittent collapse, while that captured low pair is consistent with the separately established firmware-clamp regime. It does **not** measure a CPU reduction, establish terrain-mesh cost, or compare directly with the earlier closed-lid/display-layout runs.

Artifacts: `settled-low-fps/no-meshes-case/{focused-summary,summary,result,identity,reference-identity}.json` and `terrain-off.webp`.

### Frame-time graph isolation

Commit `4e55f6ff` adds `--no-frame-time-graph`, which keeps numeric FPS visible but hides the graph and skips its per-frame shader-buffer writes; plugin/material/hidden-node registration remains. A fresh 30-sample-per-side comparison used the same `4e55f6ff` executable, saved view, 1280×989 window geometry, all focused samples, and lid open. Graph-on mean was **180.47 FPS** with **333.60%** process CPU; graph-off was **179.87 FPS** with **338.89%** process CPU. GPU activity means were 71.03% and 70.10%, respectively. This shows no material graph-cost improvement in this reference scene; it does not establish zero overhead in every workload.

Artifacts: `settled-low-fps/no-graph-case/{summary,result,identity,reference-identity}.json`, paired position files, and screenshots. The new lid-open/display-layout setup is not comparable to older closed-lid measurements.

### Indirect-parameter upload isolation

The 13-focused-sample baseline at `ba6b756a` used **314.38%** process CPU: named `Compute Task Pool` workers contributed **234.14%** and the four `game-engine` threads **80.08%**. The scene had zero terrain, water, and M2-effect material assets, so their writers were not selected as the next CPU boundary.

Commit `c3ad0ccf` adds `--freeze-indirect-parameters-after <SECONDS>`. At a `Time<Real>` deadline, its controller removes exactly one Bevy Render-schedule system, `write_indirect_parameters_buffers`, with `ScheduleCleanupPolicy::RemoveSystemsOnly`. It leaves allocated buffers, every other render callback, camera configuration, and scene exclusions in place. The removal deadline includes initial reference sampling; it is not an FPS-stabilization delay. An earlier attempted implicit-type-set condition was rejected by Bevy and abandoned; the retained implementation uses typed schedule removal instead.

The behavioral RED showed the target still ran three times where two were expected; GREEN verifies removal of only the target after the deadline and once-only enforcement. Runtime, build, and independent verification are pending. This must run only in a stationary scene: frozen indirect metadata can affect downstream rendering, so any result measures the render-schedule boundary, not proof that upload work was unnecessary or the cause of CPU load.

Artifacts: `settled-low-fps/cpu-system-isolation/baseline/` and `settled-low-fps/indirect-parameter-isolation/{corrected-red-behavior,green}.log`.

### Directional-shadow isolation

Commit `ba6b756a` adds opt-in `--no-directional-shadows` for the InWorld world-environment light. Startup inserts a diagnostic resource only when requested; `spawn_world_environment` reads it while creating the directional light and sets only `DirectionalLight.shadow_maps_enabled` to false. It logs the applied override. The light entity, overcast-day illuminance, transform, ambient lighting, cascade configuration, 4096-pixel `DirectionalLightShadowMap` resource, camera effects, and general directional-light calculations remain. Standalone and other scene setup paths are unchanged.

Focused RED/GREEN coverage is saved in `settled-low-fps/no-directional-shadows-case/{red,green}.log` (**3/3** GREEN); the `ba6b756a` game-engine build exited 0. A planned four-phase ON/OFF capture was deliberately interrupted after its first 60-sample shadows-on phase and 27 shadows-off samples when the user observed a sudden **170–200 FPS to about 40 FPS** foreground drop. Only eight of the 60 shadows-on rows were focused. It is not a completed, focus-matched A/B and establishes no shadow benefit.

The retained shadow-off client (PID `2004379`, hash `5b5e1619e03ed4396ff3c8b5dca08813c42c4cd38bb38c5ce136fc93f74de921`) used normal MSAA and the graph with the terrain-off exclusions. An immediate 12-second focused capture measured **43.92–58.24 FPS**, with both enforced CPU and GPU limits at **600 MHz in all 12 rows**, mean GPU activity **43.5%**, and **309.04%** process CPU. Raw core/GFX thermal-residency counters each advanced by 11,006; their units remain unspecified. A CPU profile captured only after recovery (**192.33–202.57 FPS**) is not slow-state attribution. Its largest identified self-costs include contended mutex locking (4.16%) and Bevy's multithreaded `Context::tick_executor` (2.51%); incomplete caller stacks prevent assigning these to a game subsystem or explaining the clamped interval. A later bounded 12-second watch observed 198.84–235.33 FPS and never triggered a clamped-state profile. See `reported-drop/analysis-2026-09-06.md` and `reported-drop/trigger-watch/result.json`.

The user clarified that FPS settles in under 15 seconds. Future relaunch comparisons use a 10-second settling delay, distinct from a measurement window; this later drop is not startup settling. The detached capture was cancelled to preserve the live client. Its absent advertised detach log and lack of later CLI/runner output do not establish an application stall.

Independent verification passed fmt/check and changed-Rust readability at `ba6b756a`, reused the 3/3 GREEN/build proof, confirmed the startup override log, and recomputed the 12-row drop (mean 52.82 FPS). The existing `binrw v0.15.1` future-incompatibility notice remains.

Artifacts: `settled-low-fps/no-directional-shadows-case/{repeat,reported-drop,red.log,green.log,build.log,audit-2026-09-06.md}`. This remains a shadow-map-work control, not evidence of an FPS benefit, a broader lighting exclusion, or a physical-cooling cause.

### MSAA-only isolation

Commit `56258e5f` adds `--no-msaa`, changing configured 4× MSAA to single-sample rendering without persisting an option change. Its source test covers the intended camera policy: configured MSAA keeps SSAO disabled while depth/normal prepasses and common effects remain; independently configured TAA retains TAA and SSAO. Verifier 115 confirmed `cargo fmt --check` and `cargo check --locked --bin game-engine --bin game-engine-cli` exited 0 at this revision; `binrw v0.15.1` emitted the pre-existing future-incompatibility notice. The verifier did not independently read live camera components.

The fresh pair restored the frame-time graph in both commands and otherwise retained the terrain-rendering-off exclusions. Both sides used the same `56258e5f` binary hash, saved view/window geometry, 30 focused samples, and lid-open layout. MSAA-on averaged **106.14 FPS** / **317.76%** process CPU; MSAA-off averaged **180.68 FPS** / **337.33%** process CPU. This is **not causal MSAA attribution**: the MSAA-on interval included 600 MHz CPU and GPU limits (CPU 600–4,573 MHz; GPU 600–2,609 MHz), while MSAA-off ran under a different limiting regime (CPU 3,057–3,699 MHz; GPU 632–1,398 MHz). CPU time also increased rather than decreased. The pair neither proves an MSAA improvement nor a CPU-cost reduction; it preserves the next controlled candidate and separates it from the known clamp evidence.

Artifacts: `settled-low-fps/no-msaa-case/{summary,result,identity,reference-identity,msaa-on,msaa-off}.json`, paired position files, screenshots, and `settled-low-fps/{no-msaa-red,no-msaa-green,no-msaa-build,verifier-56258e5f-fmt,verifier-56258e5f-check}.log`.

### Repeated MSAA comparison

A four-phase ON/OFF/ON/OFF repeat under `settled-low-fps/no-msaa-repeat/` strengthens the throughput evidence but remains bounded. All runs used the same saved position and camera FOV, options hash, workspace, and 1280×1198 logical window. Parsed output configurations matched even though one raw JSON key-order hash differed. The lid was **closed** throughout, so this series is not directly comparable with the earlier lid-open, 1280×989 MSAA pair. The first MSAA-on phase had only 29 focused samples and is excluded from the causal comparison. The fully focused phases recorded **196.19 FPS** MSAA-off, **173.45 FPS** MSAA-on, and **204.75 FPS** MSAA-off (60 samples each).

The adjacent focused ON/OFF phases had effectively equal process CPU use (**339.29%** versus **339.55%**) and similar mean GPU clock (**1,947.13** versus **1,920.32 MHz**). MSAA-off was **31.29 FPS (+18.0%)** faster, with mean frame time **5.818→4.951 ms (-14.9%)**, and had lower GPU activity (**74.07%** versus **65.65%**). This establishes a repeated throughput difference for the diagnostic selector. The subsequent [glyph-corruption investigation](#msaa-off-fps-overlay-glyph-corruption) found that the selector also separates the world/UI intermediate targets by leaving UI MSAA enabled. The comparison therefore cannot isolate pure MSAA sampling cost. It does **not** show a CPU reduction, fix intermittent collapse, or support an exact causal/attributable percentage.

Independent saved-data audit confirmed the phase boundary and calculations. Six joint 500 MHz CPU/GPU-limit bins overlap for phases 3/4, but only **26** minimum-count rows overlap; actual graphics-clock deltas within those bins span **-431.6 to +279.0 MHz**. They are distributional cohorts rather than matched controls, so the consistent direction warrants a future controlled repeat but does not remove frequency/workload confounding.

Both commands restored the numeric FPS/graph flags. Screenshot output is visually limited: phase 3 MSAA-on has clean numeric FPS but no visible graph bars, while phases 2 and 4 MSAA-off have overdrawn numeric glyphs. Do not infer graph state or a mode-specific cause from those images.

Artifacts: `settled-low-fps/no-msaa-repeat/{summary,conditions-summary,joint-limit-bins}.json`, `audit-2026-09-06.md`, raw phase samples, positions, camera/output captures, and screenshots.

### Corrected MSAA comparison

After commit `256bd37d` aligned the world and composited UI camera sample counts, a fresh four-phase ON/OFF/ON/OFF run recorded the corrected configuration. All **240** one-second samples were focused, with the same saved position/camera, 1280×1198 window geometry, parsed display configuration, options hash, and closed-lid state. The MSAA-on/off phase means were **133.88 / 166.28 / 113.85 / 184.68 FPS**. Process CPU utilization stayed close at **325.96% / 327.67% / 326.29% / 326.12%**; that is not evidence that CPU work per rendered frame did not change, because rendered frame counts were not measured.

This repaired series removes the known stale-UI-target path and its corrupted FPS glyphs, but not hardware-state confounding. Both on phases included CPU and GPU 600 MHz limits. The off phases had CPU minimum limits of 1,555 and 1,670 MHz, although their GPU limit samples also reached 600 MHz. Three joint 500 MHz CPU/GPU-limit bins overlap, with only **29** minimum-count rows; off is higher in all three, but actual graphics-clock distributions differ. Independent saved-data audit passed artifact integrity, matching conditions, clean overlay screenshots, and repeated MSAA-off direction. The data supports that direction under the corrected visual configuration, not an exact MSAA-only causal percentage, CPU reduction, or collapse fix.

Artifacts: `settled-low-fps/corrected-msaa-repeat/{summary,conditions-summary,joint-limit-bins}.json`, `audit.md`, and raw four-phase captures. The earlier `no-msaa-repeat` measurements remain non-pure because they used mismatched world/UI sample counts.

### MSAA-off FPS-overlay glyph corruption

The MSAA repeat exposed a separate diagnostic defect: with `--no-msaa`, the changing numeric portion of the Bevy FPS overlay accumulates into solid glyph blocks. The static `FPS:` prefix remains clean. A compositor-native Niri capture and the engine IPC screenshot of live PID `1553387` both reproduce it, while frame-time graph bars are visible and update. The defect is therefore in the displayed render surface, not WebP encoding or IPC screenshot copy. Saved-image review finds the same correlation in both MSAA-off repeat phases and the earlier MSAA-off pair; known MSAA-on and graph-only captures have clean digits. This is an observed runtime correlation, not yet proof that the MSAA selector is its direct cause.

The source boundary is concrete. The original selector synchronized only `Camera3d`, changing the WoW world camera from `Msaa::Sample4` to `Msaa::Off`; it deliberately preserved SSAO policy and prepasses. The `ui-toolkit` UI camera renders after it with `order: 1` and `ClearColorConfig::None`. Bevy registers `Msaa` as a required component of every `Camera`, whose default is `Sample4`; the UI camera therefore remained 4× MSAA when the world camera was single-sample. Bevy keys intermediate main textures by render target, usage, format, and `Msaa`; the two cameras consequently used different intermediate targets. The UI camera's non-clearing pass could preserve its own target rather than the world camera's new single-sample target.

Commit `256bd37d` repairs that mismatch. `sync_ui_camera_msaa`, chained after 3D graphics synchronization, copies the actual active 3D camera sample count to the composited UI camera. It changes no UI clear/order behavior and adds no UI post-processing; it preserves the actual 3D sampling selected before Lighting, configured TAA/None modes, and `--no-msaa` restoration. RED commit `514259b9` demonstrated the former UI `Sample4` versus expected `Off`; the focused GREEN suite passed **11/11** and the game-engine build exited 0 (both retain the existing `binrw v0.15.1` future-incompatibility notice).

A fixed `--no-msaa` relaunch at `256bd37d` used the same diagnostic flags, view, and geometry. Three compositor-native Niri captures and three IPC captures, ten seconds apart, show readable changing numeric FPS values (**223.70**, **231.71**, **231.85**) without accumulated glyph blocks. Independent verification passed `cargo fmt --check`, `cargo check --locked --bin game-engine --bin game-engine-cli`, and the changed-Rust readability audit; it reused the 11/11 GREEN and build proof and independently inspected all six captures and the `Sample4 -> Off` synchronization log. This confirms the user-visible regression is repaired for that runtime path. It does not provide live component/pass readback or convert prior mixed-camera timing comparisons into pure MSAA sampling measurements; the later corrected MSAA repeat supplies the relevant aligned-camera measurement. Proof: `fps-overdraw/verifier-256bd37d-audit.md` and `verifier-{fmt,check}.log`.

Artifacts: `settled-low-fps/fps-overdraw/{msaa-off-niri.png,msaa-off-ipc.webp,audit.md,red.log,green.log,build.log}`, `fps-overdraw/fixed/{niri-0,niri-1,niri-2}.png`, `fps-overdraw/fixed/{ipc-0,ipc-1,ipc-2}.webp`, `no-msaa-case/msaa-off.webp`, and `no-msaa-repeat/{2-msaa-off,4-msaa-off}/view.webp`.

### Foreground firmware-clamp evidence

The client is using the **AMD Radeon 890M through Vulkan/RADV**, not software rasterization. Its DRM graphics-engine counter advanced 1.798 seconds over 2.001 wall seconds in a dedicated read, with duplicate file descriptors counted only once. The user's 296.7% CPU observation represents about three logical cores of CPU time; CPU scheduling/game systems/render preparation still occur alongside GPU work.

Hardware-clock snapshots differed substantially between faster and slow captures: average GPU clock **2,244.5 MHz versus 840.5 MHz**, and reported average power **27.72 W versus 11.75 W**. Those differences alone were correlation, so the firmware telemetry was checked directly.

The read-only decoder in `read_gpu_metrics.py` matches upstream AMDGPU `gpu_metrics_v3_0`: 264 bytes, format 3/content 0, verified native alignment and field offsets. `current_core_maxfreq` and `current_gfx_maxfreq` are firmware-enforced frequency limits in MHz, not idle-clock observations. STAPM limit fields were `65535` and were not treated as valid limits. Thermal-residency fields remain raw ASIC-dependent counters; no units or percentages were inferred.

A **same-process, foreground** watch then captured the transition: earlier samples around 52–60 FPS had GPU limits roughly 2.1–2.3 GHz; the final five samples were **13.89–17.21 FPS with both CPU and GPU limits at 600 MHz**. GPU activity remained 100%, and thermal-throttle counters increased through that transition. This identifies firmware frequency clamping with thermal-throttle indications as a cause of this captured FPS collapse. It does not identify the cooling/platform-policy cause, explain every engine cost, or retroactively attribute the original 3.12-second hitch.

Separate read-only platform checks reported AC connected, performance platform/EPP settings, lap mode 0, and enabled automatic fans. Fan readings varied between observations (ThinkPad sensors reported 7,075 RPM earlier; a later fan-status read reported 5,272 RPM). No power limits, fan settings, kernel settings, or thermal protections were changed. Profiling/build subprocesses had exited when checked.

Evidence: `no-textures-case/{gpu-execution-proof,clock-comparison,gpu-metrics-decoded,gpu-metrics-watch,platform-readings,fan-readings}.json`, `gpu-metrics-initial.bin`, and `focused-firmware-watch/{samples,result}.json`. Upstream layout/mapping provenance is recorded in `read_gpu_metrics.py`; the latter watch is the foreground-matched evidence, unlike the earlier unfocused telemetry.

## Sources

- [Measurement artifacts](../../../data/diagnostics/movement-perf-20260905/) — loaded-route samples/profile/tree, `tile-attribution/stage-timings/{application-excerpt.log,blp-summary.json,result.json}`, and `tile-attribution/file-residency/{summary.json,control/,target-blp-evicted/}` for measured subcosts and residency controls.
- [Route calculations](../../../data/diagnostics/movement-perf-20260905/computed-doodad-boxes.json) and [candidate selection](../../../data/diagnostics/movement-perf-20260905/find_clear_route.py) — cached assets only; raw placement-Y caveat above.
- [Movement/collision](../../../src/rendering/camera/camera.rs), [collision math](../../../src/collision.rs), [doodad spawning](../../../src/rendering/terrain/terrain_objects.rs), [BLP loading](../../../src/asset/blp.rs), and [tile stage timers](../../../src/rendering/terrain/terrain_spawn_perf.rs) — actual control, loading, and measurement boundaries.
- [Boundary samples](../../../data/diagnostics/movement-perf-20260905/boundary-samples.json), [event timeline](../../../data/diagnostics/movement-perf-20260905/boundary-events.json), and [streaming implementation](../../../src/rendering/terrain/terrain_streaming.rs) — first-crossing evidence and application boundary.
- [Movement spec](../../specs/scripted-movement.md), [InWorld scene-isolation spec](../../specs/inworld-scene-isolation.md), and [startup investigation](procedural-cloud-regeneration.md) — control contracts, selector scope, and earlier proof.
- [Settled isolation artifacts](../../../data/diagnostics/movement-perf-20260905/settled-low-fps/) — terrain-rendering-off, graph, original/corrected MSAA, directional-shadow, reported-drop, and independent-audit artifacts.

## See Also

- [[scripted-movement]] — automated route controls
- [[terrain]] — streaming and placement
- [[rendering-pipeline]] — broader rendering work
