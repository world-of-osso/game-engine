# Wiki Log

## [2026-09-06] clarification | Bound CPU-profile coverage claims

Clarified [[movement-performance]] and JSON metadata: observed CPU is bracketed by each thread's first/last captured selected-span clocks, not exact five-second CPU. Residuals apply only inside those intervals; unobserved threads and boundary time are absent. The crossing counter records late exits, not spans already active at capture start. Accounting arithmetic is unchanged.

## [2026-09-06] measurement | Trace upload exclusions with thread CPU clocks

Updated [[movement-performance]] with three `5f3fb679` feature-build captures: both upload writers active, indirect removed, then batched also removed. Exact callback spans disappear, but worker residual remains 5.828 CPU-seconds with both absent. Differing frame counts and instrumentation prevent a normal-build optimization claim. Recovered native flat leaf samples using `perf script -G` and the matching saved binary; full ancestry remains unresolved. Default build restored; profiling clients stopped.

## [2026-09-06] measurement | Record first named-span thread-CPU capture

Updated [[movement-performance]] from feature-build `36d1d994` PID `3611636`. Within the five-second capture, first-to-last selected-span observations bracketed **18.122 s** thread CPU and **11.061 s** summed self CPU while overlapping focused telemetry measured **358.55%** process CPU and **161.64 FPS**. The **7.061 s** residual includes uninstrumented worker-task work and profiler overhead; it is not native CPU ownership. Nine late exits were counted, none on Compute Task Pool workers; spans already active at capture start are excluded without being counted. `5f3fb679` subsequently shares span labels and adds concurrent-same-span and blocked-sleep tests, so future captures have lower label-allocation overhead and are distinct. No root cause, normal-build comparison, or optimization claim follows.

## [2026-09-06] diagnostic | Add named-span thread-CPU profiler

Documented `36d1d994` in [[movement-performance]] and the InWorld isolation spec. The diagnostic-only `cpu-system-profile` feature installs its tracing layer only when `WOO_CPU_PROFILE_OUTPUT` is set. It captures selected named system/schedule/executor spans from seconds 10–15 and exports aggregate per-thread CPU-clock JSON after a one-second drain at approximately second 16. It reports inclusive/self CPU, first-to-last selected-observation thread CPU, and late-exit counts; see [[movement-performance]] for boundary and coverage limits. `bevy/trace` enables optional tracing-related transitive dependencies only in this feature build. First runtime results are recorded in the later capture entry; no cause or optimization claim follows.

## [2026-09-06] measurement | Isolate pipelined-rendering CPU contribution

Updated [[movement-performance]] with sequential upload removals, the `09e77aa1` same-extraction schedule-registry repair, and the pipelining comparison. Upload removals did not eliminate the bulk CPU load. Omitting only `PipelinedRenderingPlugin` reduced process CPU **320.72→208.15%** while FPS fell **194.86→146.81**; render frames and readable FPS remained. Earlier upload exclusions were restored individually, leaving ordinary uploads active. This is a measured CPU/throughput trade-off, not a default optimization or resolution of firmware clamps. Final source/data audit passed formatting, locked checks for both binaries, readability, and reused 5/5 upload plus 2/2 pipeline tests. A later all-uploads-active observation measured219.23%CPU/142.54FPS with recoveredCPUlimits, so the lower-CPU diagnostic mode does not require frozen upload data. Details and limitations remain in [[movement-performance]].

## [2026-09-06] diagnostic | Record GPU-cluster preparation isolation

Updated [[movement-performance]] and the InWorld isolation spec for `7d8baeb9`. A roughly five-second instrumented trace reported about **128 ms** across roughly **552** `prepare_clusters_for_gpu_clustering` calls; spans are overlapping traced wall time, not exclusive CPU cost or callback attribution. The new stationary-only cutoff resolves the exact private callback to one implicit system set and asserts removal of exactly one system. Same-process PID `3445983` logged removal at **60.006 s**. `before/` and `before-second/` were both pre-removal (**328.89% / 193.37 FPS**, **336.47% / 214.99 FPS**). Post-removal `after/` (**326.38% / 137.43 FPS**) and `after-later/` (**316.04% / 54.87 FPS**) reached 600 MHz CPU/GPU limits. This is not a comparable-FPS result and establishes no CPU reduction or causality. Artifacts: `settled-low-fps/cpu-system-isolation/gpu-clusters/`.

## [2026-09-06] measurement | Remove indirect-parameter uploads

Updated [[movement-performance]] from the first actual CPU-system removal. Same-process PID `2428304` at `c3ad0ccf` removed only Bevy Render's `write_indirect_parameters_buffers` after 20 seconds while preserving position/view and all 13 focused samples per side. Process CPU **increased** from **321.63%** to **330.97%**; this target did not reduce CPU usage. FPS changed **134.47→197.57**, but CPU/GPU limit ranges differed, so no pure FPS conclusion follows. Before/after screenshots preserve the empty view and readable overlay. The stationary-scene control can affect downstream render behavior; it is diagnostic, not an optimization. The next distinct target, `write_batched_instance_buffers<MeshPipeline>`, remains pending separate proof.

## [2026-09-06] diagnostic | Add timed indirect-parameter upload isolation

Updated [[movement-performance]] for `c3ad0ccf`. The next CPU baseline recorded **314.38%** process CPU across 13 focused samples: named Compute Task Pool workers accounted for **234.14%**, and game-engine threads **80.08%**; terrain, water, and M2-effect materials were all absent. Opt-in `--freeze-indirect-parameters-after <SECONDS>` removes only Bevy Render's typed `write_indirect_parameters_buffers` system at a `Time<Real>` deadline using `RemoveSystemsOnly`; allocated buffers and other render callbacks remain. The rejected implicit-type-set approach was abandoned. Corrected behavioral RED proved the target still ran 3 rather than 2 times; GREEN, build, runtime, and independent verification remain pending. Stationary-scene only: frozen indirect metadata can affect downstream render work, so no CPU-drop or efficiency claim is made.

## [2026-09-06] measurement | Preserve directional-shadow FPS collapse

Updated [[movement-performance]] from the interrupted `ba6b756a` directional-shadow run. The planned ON/OFF series stopped after 60 shadows-on samples and 27 shadows-off samples when the user observed foreground FPS fall from 170–200 to about 40. This is not a completed A/B and provides no shadow benefit claim. The retained shadow-off PID `2004379` immediately measured 12 focused samples at **43.92–58.24 FPS** with both enforced CPU/GPU limits at **600 MHz** in every row, **309.04%** process CPU, **43.5%** mean GPU activity, and +11,006 raw core/GFX thermal-residency counters (units unspecified). A 192.33–202.57 FPS CPU profile ran after recovery and is not slow-state attribution. User clarified FPS settles in under 15 seconds; future relaunches use 10-second settling, distinct from measurement. The cancelled detach capture's missing log is not application-stall evidence.

## [2026-09-06] diagnostic | Add directional-shadow isolation control

Updated [[movement-performance]] for `ba6b756a`. Opt-in `--no-directional-shadows` inserts a startup resource consumed only by InWorld `spawn_world_environment`, which sets the spawned directional light's `shadow_maps_enabled` false and logs the override. The light, illuminance, transform, ambient lighting, cascade configuration, 4096-pixel shadow-map resource, camera effects, and general lighting remain; standalone/other scene setup paths are unchanged. Saved focused RED/GREEN is **3/3**. No build, relaunch, measurement, or independent final verification has occurred, so no FPS benefit is claimed.

## [2026-09-06] repair | Synchronize UI and world camera MSAA

Updated [[movement-performance]] for `256bd37d`. The original `--no-msaa` path left the non-clearing UI camera at Bevy-default 4× MSAA while the world camera became single-sample, splitting their intermediate targets and accumulating dynamic FPS glyphs. `sync_ui_camera_msaa` now runs after 3D graphics synchronization and copies the actual active 3D sample count to the UI camera without changing its clear/order behavior or adding UI effects. RED `514259b9` reproduced UI `Sample4` where `Off` was required; saved GREEN passed **11/11** and the game-engine build exited 0, both with the existing `binrw v0.15.1` future-incompatibility notice. Fixed live `--no-msaa` PID `1875826` produced three readable changing FPS values in both Niri-native and IPC captures, ten seconds apart. This repairs the display regression; it does not turn earlier mixed-camera MSAA timing into pure sampling-cost evidence.

## [2026-09-06] measurement | Record repeated MSAA ON/OFF comparison

Updated [[movement-performance]] and `index.md` from `settled-low-fps/no-msaa-repeat/`. The closed-lid, 1280×1198 series used fixed position/camera/options and four 60-sample phases. Excluding the first MSAA-on phase because only 29 samples were focused, the focused sequence was MSAA-off **196.19 FPS**, MSAA-on **173.45 FPS**, and MSAA-off **204.75 FPS**. The adjacent ON/OFF phases had near-equal CPU use (**339.29%/339.55%**) and similar mean GPU clocks (**1,947.13/1,920.32 MHz**), with lower MSAA-off GPU activity (**74.07%/65.65%**). This records a repeated diagnostic-selector throughput difference, not a pure MSAA sampling cost: the later glyph investigation found the selector also changed world/UI intermediate-target composition. It does not show CPU reduction, collapse remediation, or an exact causal/attributable percentage. Independent saved-data audit confirmed the phase 3/4 +18.0% FPS and 5.818→4.951 ms direction, but only 26 minimum-count rows overlap across six joint-limit bins and actual graphics-clock deltas span -431.6 to +279.0 MHz. Screenshot rendering is visually inconsistent across modes; no graph-state inference is made. The artifact base retains its September 5 directory name; measurements are September 6. Source remains `56258e5f`.

## [2026-09-06] measurement | Record corrected UI/world MSAA repeat

Updated [[movement-performance]] from `settled-low-fps/corrected-msaa-repeat/` after `256bd37d` aligned UI and world camera sample counts. All four 60-sample phases were focused with matching saved view, 1280×1198 geometry, display configuration, options, and closed-lid state: MSAA-on/off/on/off means were **133.88 / 166.28 / 113.85 / 184.68 FPS**, while process CPU stayed **325.96% / 327.67% / 326.29% / 326.12%**. The corrected series removes the stale-UI target from the comparison, but both on phases reached CPU/GPU 600 MHz limits and three coarse joint-limit bins have only 29 minimum-count overlaps with differing actual clock distributions. Verifier 137 independently passed artifact integrity, matching conditions, fixed-overlay screenshots, and repeated MSAA-off direction. It supports that direction, not an exact MSAA percentage, CPU-work-per-frame conclusion, or collapse fix; earlier `no-msaa-repeat` remains non-pure due to its world/UI sampling mismatch.

## [2026-09-06] investigation | Record MSAA-off FPS-overlay corruption hypothesis

Updated [[movement-performance]] from compositor-native and IPC captures of live MSAA-off PID `1553387`: changing numeric FPS glyphs accumulate while `FPS:` remains clean and graph bars update. The defect is displayed-surface rendering, not screenshot encoding. Source shows `--no-msaa` changes only the world `Camera3d`; the later non-clearing UI camera remains Bevy-default 4× MSAA, yielding separately keyed intermediate targets. This is a stale UI-composition hypothesis, not a confirmed root cause or authorized fix. The minimal proposed UI-camera MSAA synchronization remains pending approval and behavioral/live proof.

## [2026-09-06] audit | Record terrain-off, frame-graph, and MSAA isolation through `56258e5f`

Updated [[movement-performance]] and `index.md` from saved `settled-low-fps` artifacts. Recorded terrain-rendering-off as a logical terrain/height-state control, its 120-second focused mean (**98.25 FPS**, **311.67%** CPU), and its two 600 MHz-limited lows; no CPU reduction or terrain-mesh attribution is claimed. Recorded the lid-open matched graph pair (**180.47** versus **179.87 FPS**; **333.60%** versus **338.89%** CPU), which shows no material graph-cost improvement. Recorded the MSAA-only pair (**106.14** versus **180.68 FPS**) as inconclusive because the samples ran under different firmware-limit regimes and MSAA-off CPU was higher (**337.33%** versus **317.76%**). Graph was restored for both MSAA commands. Verifier 115 saved `cargo fmt --check` and locked binary check exits 0 for `56258e5f`; `binrw v0.15.1` retains its existing future-incompatibility notice, and live component readback was not independently verified. The original tile hitch remains unresolved; nameplate implementation remains queued.

## [2026-09-05] investigation | Foreground FPS collapse matches firmware frequency clamps

Updated [[movement-performance]] with the flat-material diagnostic, persistent slow periods, verified GPU execution, and a foreground transition to 600 MHz CPU/GPU firmware limits with advancing thermal-throttle counters. Cooling/platform-policy cause and original tile hitch remain unresolved; no hardware controls or optimization changed.

## [2026-09-05] experiment | Reduced-scene material freeze does not sustain gain

Updated [[movement-performance]] with conflicting unchanged-code freeze results: 47.53 mean FPS before, one80.95 sample then21–27 after, mean31.81. Same position/assets/view; CPU and GPU busy counters increased. Normal updates restored, later48–55FPS. Earlier material-freeze gain is not a general resolution; frequency/thread/render-state attribution remains open.

## [2026-09-05] diagnostic | Water and skybox visual isolation

Updated [[movement-performance]] with opt-in streamed-water and skybox-visual exclusion, zero water assets in the new client, and 46.65 mean FPS. Recorded retained lighting/camera paths and position/focus differences that prevent a controlled speed comparison. Terrain-material updates remain the stronger prior measured lead; no optimization applied.

## [2026-09-05] measurement | Terrain-material freeze and object-free isolation

Updated [[movement-performance]] with the main-controlled 256-material freeze (19.87→67.80 mean FPS), normal-update restoration, and requested object-free terrain run (user ~60 FPS; IPC mean54.41 with mixed focus). Recorded zero object colliders, reduced asset counts, close-up-view limitations, and unresolved attribution. No production optimization or tile-hitch resolution claimed.

## [2026-09-05] diagnostic | Add timed terrain-material freeze selector

Updated [[movement-performance]] for `4ab3deb6` (`Add timed terrain material freeze diagnostic`). Optional `--freeze-terrain-materials-after <SECONDS>` keeps normal startup updates, then gates terrain animation-time and environment-map synchronization together using `Time<Real>` while retaining loaded terrain material values and rendering. This isolates `Assets::iter_mut` modification churn; it intentionally freezes animated terrain and later environment-map changes. Focused tests **3/3** and build passed; no runtime comparison or performance claim yet.

## [2026-09-05] diagnostic | Add exact NPC and UI isolation selector

Updated [[movement-performance]] for `3d364dc8` (`Add NPC and UI isolation without changing scene lighting`). `--inworld-stage no-npcs-ui` preserves terrain, lighting, particles, local-character policy, networking, and FPS overlay while excluding remote visuals plus game UI/nameplates; selector and nameplate-observer behavior have focused coverage. PID `3510050` did not survive its Pyrun launch, so no runtime FPS result is claimed.

## [2026-09-05] investigation | Separate sustained-FPS report from tile stall

Updated [[movement-performance]] with the user-reported approximately 10-FPS settled condition, the non-reproducing 30–45 FPS full-scene IPC samples, and the confounded 56–76 FPS terrain-stage isolation. The terrain selector also excludes lighting and particles, so it is not subsystem attribution. No runtime selector result, optimization, or tile-stall closure is claimed.

## [2026-09-05] experiment | Target BLP residency does not reproduce the stall

Updated [[movement-performance]] with a per-file cache-advice control, verified residency changes, and bracketing CPU/fault observations. Target BLP residency alone did not reproduce the original delay. The original stall remains open in `PLAN.md`; no optimisation, global kernel-setting change, or asset-content change was made.

## [2026-09-05] measurement | Split repeat tile application into stages

Updated [[movement-performance]] with gated timing output from `7d9e7370`: 316.465 ms total application, including 159.696 ms doodads, 132.490 ms WMOs, and 22.125 ms terrain water. Nested BLP timings record 110.015 ms alpha normalization across 156 loads. This identifies repeat CPU subcosts, not the cause of the original additional delay; no optimization was implemented.

## [2026-09-05] diagnostic | Correct cache characterization and split tile timing

Corrected [[movement-performance]]: FDID-named root `777827.adt` predated the experiment; the resolver only selects existing local roots. Earlier cold-cache/CASC-supply wording was unsupported. Repeat application intervals were approximately 350 ms, but the original 2.96-second application gap remains valid. Added gated BLP and tile-stage diagnostics (`d4eaf8cf`, `7d9e7370`) for caller-independent timing; no optimization or collision change.

## [2026-09-05] measurement | Reproduce tile-crossing stall

Updated [[movement-performance]] with a normal timed segment crossing `(32,48)` to `(31,48)`. The initial cache characterization was later corrected: the FDID-named root already existed; see the diagnostic entry and investigation. The old tile unloaded at +0.94 s, background parsing finished at +5.67 s, and spawn statistics appeared at +8.73 s; one IPC request waited 3,124 ms across the main-thread tile-application interval. This establishes a boundary-associated stall, not a fix or per-subsystem timing attribution. Client remained connected with the new tile loaded; LOD swaps remain unmeasured.

## [2026-09-05] measurement | Verify autonomous route and separate movement cost

Added [[movement-performance]] and reconciled [[scripted-movement]], its spec, and the earlier investigation. Ten timed segments completed 140 yards of travel with normal collision; matched focused samples averaged 26.58 FPS idle and 26.97 moving. A separate 1,379-sample profile identified transform parent propagation at 20.45% self cost. Documented the canopy bounding-box trap and startup cache recovery separately. Streaming/LOD boundaries remain unmeasured; no general world-performance fix claimed.

## [2026-09-05] feature | Record scripted movement IPC controls

Updated [[scripted-movement]] and `docs/specs/scripted-movement.md` for `51456222` (`Add scripted movement IPC controls`). The CLI now exposes `movement forward --seconds N [--yaw-degrees D]` and `movement stop`; IPC carries `ScriptedMovementForward { duration_secs, heading_degrees }` and `ScriptedMovementStop`. Focused proof: CLI **3 passed**, IPC **4 passed**, playback validation **3 passed**, and camera integration **6 passed**. Connected-runtime displacement and a valid moving-frame comparison remain open.

## [2026-09-05] docs | Record bounded scripted movement

Added [[scripted-movement]], updated [[procedural-cloud-regeneration]], and `index.md` for `15928f4e`, `d3f525ac`, and `6c9d82a3`. Timed forward segments validate duration/heading, clip their final frame, and run through ordinary player movement/collision/networking rather than teleporting. Waypoint attempts at the current spawn did not move the player, so no valid moving-frame comparison exists; connected IPC/CLI displacement proof remains required.

## [2026-09-05] fix | Record replicated M2 cache reuse

Updated [[procedural-cloud-regeneration]] and `index.md` for `484586ac` (`Reuse parsed models for replicated spawns`) and `dfb29983` (`Test cached NPC model spawning`). Shared replicated spawn helpers now reuse the existing model cache keyed by path, skin FileDataIDs, and zero-opacity mode while preserving filtering and joint binding. A concrete model/skin/skeleton regression removes the M2 after the first spawn and proves the second independent-root spawn retains identical vertices and indices; RED `/tmp/claude/npc-cache-red-corrected.log`, GREEN `/tmp/claude/npc-cache-green.log` (**1 passed**). No startup, FPS, connection-stability, or movement-performance effect is claimed before a fresh runtime measurement.

## [2026-09-05] investigation | Record startup parser blocker before movement measurement

Updated [[procedural-cloud-regeneration]] and `index.md`. A current-source client could not reach a controlled movement workload: synchronous replicated-NPC visual spawning used `load_m2_uncached` on the main thread. A five-second profile captured 241 samples with zero lost; `read_i16` held 16.60% self cost. A live stack traced `spawn_replicated_npc` through uncached M2 skeleton/animation parsing. The FPS overlay was enabled but retained its empty numeric span (`FPS:`) while IPC timed out. This records a startup blocker only—no movement root cause, fix, or performance claim.

## [2026-08-11] docs | Record movement performance probe

Updated [[procedural-cloud-regeneration]] for `53a9f66a` (`Add gated movement performance probe`). Recorded `WOO_PERF_MOVEMENT` activation, once-per-second movement/pathing/collision timing and collider-count output, and the absence of any performance conclusion before controlled runtime measurement.

## [2026-08-11] docs | Record Bevy 0.19 strict-Empty LightPlugin boundary

Updated [[rendering-pipeline]], [[inworld-scene-isolation]], and `index.md` for `17c2bdc6` (`Preserve strict Empty gizmo isolation on Bevy 0.19`). Bevy 0.19 `LightPlugin` registers nested `LightGizmoPlugin` resources, so exact configured `InWorldSceneStage::Empty` now disables `LightPlugin` as well as `GizmoPlugin` and `GizmoRenderPlugin`; unconfigured/default, `Character+`, debug, and screenshot/default paths retain the normal LightPlugin/gizmo boundary. The in-world isolation spec and implementation inventory now record this requirement.

## [2026-08-10] docs | Record Bevy 0.19 / Lightyear 0.28 migration boundary

Updated [[networking]], [[rendering-pipeline]], `AGENTS.md`, `docs/network-integration.md`, and `docs/character-generation.md` for game-engine commit `4bc50a22` (`Upgrade engine to Bevy 0.19 and Lightyear 0.28`). Recorded the current engine boundary as Bevy 0.19, `bevy_hanabi` 0.19, Lightyear 0.28, and Rust 1.95; renamed current client receive-marker references from deprecated `Replicated` to `Remote`. Historical Bevy 0.18 bug findings and the game-server's separate dependency state remain unchanged.

## [2026-08-10] proof | Record stable rebuilt reconnect runtime

Updated [[networking]] and [[procedural-cloud-regeneration]] with post-build proof for `0d215316` at engine docs commit `6b034959`. The old strict-Empty client PID `2846177`/SHA `84f6ecc...` was terminated before fixed PID `3715288`/start ticks `192167263`/SHA `aef6f08d318a21063d698816b8202ed21f6ca70b7cf4d015a2520a5f107619c3` launched; exactly one current socket remained. Ten-second stability held the same PID/start/client ID with `InWorld`, `connected=true`, `connected_links=1`, `local_players=1`, ping `pong`, and **10.21 FPS / 97.98 ms**, `focused=false`. Scene output had 78 undisplayed NPC entries, zero camera/terrain/WMO/doodad/particle terms, and zero terrain/cache counts. Logs showed one expected initial Connecting marker, one connect/login/InWorld path, and zero InWorld disconnects/reconnect loop/panic/OOM/device-loss. Cargo-watch server PID `3123827` remained unchanged, target-matched, admin-responsive, and at server HEAD `4aca4d3` containing the Who fix. This completes the pending Who live health gate without another server restart; the real reconnect ordering is covered by the App RED/GREEN proof.

## [2026-08-10] fix | Ignore initial reconnect disconnect marker

Updated [[networking]], [[procedural-cloud-regeneration]], and `index.md` for `0d215316` (`Ignore initial reconnect disconnect marker`). Recorded root cause: Lightyear `NetcodeClient` requires initial `Disconnected { reason: None }`; while `GameState::InWorld` and `ReconnectPhase::PendingConnect`, the observer misclassified it as a real loss, queued another reset, and replaced client entity/ID about every 100 ms before handshake. The fix ignores only reasonless/no-forced-notice initial markers during `PendingConnect`; reasoned pending failures, connected disconnects, forced disconnects, auth/token/selection/world-reset behavior, and retry behavior remain. RED `/tmp/claude/game-engine/reconnect-initial-marker-red.log`; GREEN `/tmp/claude/game-engine/reconnect-initial-marker-green.log` (**14 passed**); fmt `/tmp/claude/game-engine/reconnect-initial-marker-fmt.log`; readability `/tmp/claude/game-engine/reconnect-initial-marker-readability.json` and `reconnect-initial-marker-readability-metrics/`. Post-build proof `/tmp/claude/game-engine/reconnect-fixed-runtime.json` and `/tmp/claude/game-engine/reconnect-fixed-live-current.log` was recorded at engine docs commit `6b034959`: old PID `2846177`/SHA `84f6ecc...` was terminated before fixed PID `3715288`/start ticks `192167263`/SHA `aef6f08d318a21063d698816b8202ed21f6ca70b7cf4d015a2520a5f107619c3` launched. One unchanged strict-Empty client ID remained `InWorld` with `connected_links=1`, `local_players=1`, ping `pong`, **10.21 FPS / 97.98 ms**, and `focused=false` for ten seconds; 78 NPCs were all undisplayed, camera/terrain/WMO/doodad/particle terms and terrain/cache counts were zero, and logs showed one expected initial Connecting marker, one connect/login/InWorld path, and zero disconnects/reconnect loop/panic/OOM/device-loss. Server PID `3123827` stayed cargo-watch managed, matched its target, admin ping was `pong`, and server HEAD `4aca4d3` contained the Who fix. This completes the pending Who live health gate without another server restart; reconnect ordering remains covered by the App RED/GREEN proof.

Chronological record of wiki operations.

## [2026-08-10] fix | Accept strict Empty gizmo A/B result

Updated [[rendering-pipeline]], [[procedural-cloud-regeneration]], and `index.md` for `1503cd1c` (`Disable gizmos in strict Empty`). Explicit fixed `InWorldSceneStage::Empty` disables both Bevy `GizmoPlugin` and `GizmoRenderPlugin`; unconfigured/default, `Character+`, debug, and screenshot/default paths retain gizmos. Baseline PID `2655273` measured **8/12** passing windows, **9.97% mean**, **10.90% maximum**, and a 60-second profile with **2,153 samples** where `GizmoBuffer<LightGizmoConfigGroup>::queue` contributed **1.51% sampled CPU**. Candidate PID `2846177`, SHA `84f6ecc9590979a2fba498b8206549d33ebff7fdc27878a16331acccc19811da`, measured **10/12**, **9.47% mean**, **10.60% maximum**; its 60-second profile collected **1,936 samples**, **0 lost**, and no `Gizmo` symbol. The targeted path disappeared and passive mean improved **0.50 percentage points**, so the commit is retained, but the final every-window `<=10.0%` gate remains open. Readiness proved one client/server, IPC/admin ping, approximately **10.09 FPS**, zero displayed NPCs/cameras/terrain, no panic/device-loss/OOM, and no Character advancement. One startup missing-despawn warning was recorded without blocking readiness.

## [2026-08-10] fix | Record idle Who runtime change-tick correction

Updated [[networking]], [[procedural-cloud-regeneration]], and `index.md` for `2c265ffa` (`Avoid dirtying idle Who runtime`). `send_pending_queries` previously called `pop_front()` on an empty queue every `Update`, advancing `WhoRuntimeState` change ticks without a query, send, reply, or snapshot transition. A read-only empty guard now preserves FIFO query sending, unavailable replies, inbound receive handling, reset cleanup, and later-stage behavior. RED evidence: `/tmp/claude/game-engine-perf/who-idle-change-tick-red.log`; GREEN evidence: `/tmp/claude/game-engine-perf/who-idle-change-tick-green.log`; `cargo fmt` passed, and Rust-readability evidence is under `/tmp/claude/game-engine-perf/who-idle-change-tick-readability/`. No CPU-savings or Character-readiness claim is made before runtime measurement.

## [2026-08-10] fix | Record audio backend Empty boundary

Updated [[sound]], [[procedural-cloud-regeneration]], and `index.md` for `463e9e47` (`Disable audio backend without sound flag`). No-sound mode now omits Bevy `AudioPlugin`, while `--sound` retains Bevy audio plus project `SoundPlugin` exactly. Outside `src/sound/`, optional `AudioSink` status and `SoundSettings` do not require `AudioPlugin`. Recorded RED/GREEN, formatting, and readability artifacts.

Post-fix PID `2468254` remained focused, connected, and visually Empty. It had no audio backend threads or PipeWire/CPAL/ALSA profile symbols. Twelve passive windows measured **9.61% mean**, **9.55% median**, and **10.60% maximum** CPU; **8/12** met `<=10.0%`. Audio removal materially lowered the mean, but the strict all-window gate and Character advancement remain blocked.

## [2026-08-10] fix | Record network reset due-gate boundary

Updated [[procedural-cloud-regeneration]], [[networking]], and `index.md` for `ce0ce2d0` (`Gate network reset flush until due`). The due predicate skips no-pending/not-due frames while preserving earliest-target selection, one-frame deferral, exactly-once reset, and existing reset content. Recorded RED `/tmp/claude/game-engine-perf/network-reset-due-gate-red.log`, GREEN `network-reset-due-gate-green.log`, formatting, and readability artifacts.

Post-fix PID `2402583` remained focused, `InWorld`, connected with one link/player, and visually Empty. Its steady profile contained neither the flush wrapper nor due predicate. Twelve passive windows measured **12.05% mean**, **11.75% median**, and **14.00% maximum** CPU; **0/12** met `<=10.0%`. The due gate is verified, but the performance hypothesis is rejected and Character remains blocked.

## [2026-08-10] fix | Record final Empty M2 asset-store boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], and `index.md` for the `0a1a1bfb` → `49304144` → `e7f98704` correction. Exact Empty now keeps lightweight `Assets<M2EffectMaterial>` for active consumers while omitting Bevy `EntitiesNeedingSpecialization` and material/render schedules; Character+, unconfigured, and debug runs retain the full plugin. PID `2339740` scene-setup and PID `2365242` `sync_equipment` panics remain failure evidence; their stale sockets were removed only after identity-safe verification. Corrective RED, GREEN, formatting, and readability artifacts use the `empty-m2-asset-specialization-*` prefix.

Final PID `2390217` matched `e7f98704`, stayed `InWorld` and connected with one link/player, retained FPS text, and had zero terrain, `Camera3d`, or displayed NPCs. Its profile contained no Hanabi, `M2EffectMaterial`, or M2 specialization symbol. After a 30-second warm-up, twelve passive windows measured **10.24% mean**, **10.20% median**, and **10.60% maximum** CPU; only **2/12** met `<=10.0%`. The M2 boundary is verified, but the overall Empty CPU and Character gates remain open.

## [2026-08-10] fix | Record Particle/Hanabi Empty render boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], `docs/particle-system.md`, and `index.md` for `beead231` (`Register particles only at Particles stage`). Recorded that exact Empty through Lighting no longer registers ParticlePlugin/Hanabi, while Particles, Ui, and unconfigured normal runs retain it. The pre-fix PID `2176863` profile sampled `bevy_hanabi::render::VfxSimulateNode::run` at 2.18% self CPU despite stage-gated emitter systems. Recorded RED `/tmp/claude/game-engine-perf/empty-particle-plugin-red.log`, GREEN `empty-particle-plugin-green.log`, and formatting/readability artifacts. Rebuilt PID `2297374` stayed connected with one link/player, FPS text, and zero world content; its post-fix profile contained no Hanabi symbol. Three passive samples measured 12.50%, 12.70%, and 9.90% of one core, so the `<=10%` result is not stable and no causal CPU reduction is claimed. Temporary 10 FPS pacing and the blocked Character gate remain.

## [2026-08-10] fix | Record strict Empty FPS graph boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], [[ui-system]], and `index.md` for `8cac2b03` (`Disable Empty FPS frame-time graph`) and follow-up `cc5780a8` (`Preserve Empty FPS graph disablement`). Corrected the startup-only failure: runtime option writers restored the graph, visibly producing the solid red block in `empty-fps-graph-2246158.webp`. `cc5780a8` makes every writer stage-aware. Rebuilt PID `2283621` kept FPS text with no graph (`empty-fps-graph-options-2283621.webp`), stayed connected with one link/player and zero world content, reported 9.95 FPS / 100.45 ms, and measured 9.80% of one core over 10 seconds. This meets the numerical threshold under temporary 10 FPS pacing; it is not accepted as the final Empty design, and Character remains blocked.

## [2026-08-10] fix | Record Character-stage camera/input guard

Updated [[procedural-cloud-regeneration]], [[replicated-unit-noops]], [[rendering-pipeline]], and `index.md` for `c446d81c` (`Gate camera updates at Character stage`). Recorded that strict Empty skips `sync_camera_options`, `camera_input`, `cursor_grab`, `player_movement`, and `camera_follow`; Character and later stages retain them. Recorded Empty-only collision/pathing collection and raycast setup, RED/GREEN/fmt/readability evidence, and protected `camera.rs` instrumentation preservation through partial staging. Rebuilt PID `2176863` remained connected at 10.01 FPS / 99.89 ms with zero world camera/terrain/displayed NPCs and measured 11.10% of one core. The prior paced result was 11.20%, but remote entities changed from 70 to 75, so no measurable improvement is accepted. The 10 FPS pacing remains temporary and Character remains blocked.

## [2026-08-10] fix | Record strict Empty diagnostic pacing

Updated [[procedural-cloud-regeneration]], [[networking]], [[rendering-pipeline]], and `index.md` for `4fb2e5c9` (`Pace strict Empty stage at 10 FPS`). Recorded the exact gate: only `GameState::InWorld` with exact `InWorldSceneStage::Empty` uses the existing limiter's 100 ms interval; Character/later stages and other states retain persisted/global frame limiting and PresentMode, while FPS overlay, networking, and IPC remain registered. Framed pacing as diagnostic frame-cadence control, not render/application-work removal. Recorded RED `/tmp/claude/game-engine-perf/empty-frame-interval-red.log`, valid GREEN `/tmp/claude/game-engine-perf/empty-frame-interval-green-2.log`, module GREEN `/tmp/claude/game-engine-perf/empty-frame-client-options-green-2.log`, formatting/readability evidence, and runtime measurements: pre-pacing PID `2093844` at 490.62 FPS / 369.05% one-core CPU; paced PID `2130439` at 9.98 FPS / 100.23 ms and 11.20% one-core CPU, connected with one link/player and zero world camera/terrain/displayed NPCs. The `<=10%` gate failed. Alessio chose to keep 10 FPS temporarily for investigation, not as the final fix; Character remains blocked.

## [2026-08-10] fix | Record demand-driven IPC status refresh

Updated [[procedural-cloud-regeneration]] and [[networking]] for `abf68fd9` (`Refresh IPC status snapshots on demand`). Recorded the `Receive → RefreshStatus → Dispatch` ordering, explicit request-to-snapshot dependency matrix, FIFO command dispatch with coalesced refresh flags, no-command idle behavior, and removal of duplicate map synchronization. Recorded RED evidence in `/tmp/claude/game-engine-perf/status-demand-red.log` and `status-refresh-matrix-red.log`, GREEN evidence in the corresponding `*-green-2.log` files, formatting checks with empty stderr, and readability artifacts under `status-demand-readability/`. No CPU improvement or `<=10%` Empty-stage claim is made until a rebuilt live measurement. Protected `src/rendering/camera/camera.rs` and all source/tests/PLAN/Cargo files were left untouched.

## [2026-08-09] fix | Record event-driven NPC visibility

Updated [[networking]], [[replicated-unit-noops]], and `index.md` for `e745d35e` (`Make NPC visibility event driven`). Recorded the prior per-`Update` scan history (`1a1a8179`, extended by `6ffa6ce0`, with `3c77d346` guarding writes only), then the new trigger boundaries: added/changed `Npc` entity-scoped updates, semantic `LocalAliveState` changes for `DeadOnly` policies, dawn/dusk phase changes for scheduled policies, and one full reconciliation on state/NPC-stage activation. Recorded focused RED/GREEN behavioral proof without claiming measured CPU improvement.

## [2026-08-09] fix | Record Npcs-gated remote interpolation

Updated [[networking]], [[replicated-unit-noops]], [[procedural-cloud-regeneration]], and [[rendering-pipeline]] for `538e8329` (`Gate remote interpolation at Npcs`). Connection/auth, replication receive, and replicated target synchronization remain active before `Npcs`; `interpolate_remote_entities` now runs only from cumulative `Npcs`, so `Empty`, `Character`, `Skybox`, and `Terrain` no longer mutate remote visual `Transform`. Recorded the RED failure and GREEN pass in `/tmp/claude/game-engine-perf/strict-empty-interpolation-red.log` and `strict-empty-interpolation-green.log`. The live replacement (`538e83290769c70a6980ec0903db74bf2981c0fb`, PID `2960624`, start ticks `182917558`, socket `/tmp/game-engine-2960624.sock`) compared cautiously with the pre-gate client (`96e1308a31940ed5c03046b574ca0b52fe15d8e2`, PID `2592665`, start ticks `182782909`, socket `/tmp/game-engine-2592665.sock`): both had no world camera, remote counts `133` versus `134`, aggregate CPU `325.49% → 288.12%`, compute CPU `229.17% → 197.62%`, and client gfx `6.93% → 5.60%`. FPS/frame direction is not acceptance evidence because runqueue and live conditions drifted. The subsequent strict-Empty eu-stack capture contains no interpolation stack, but about `2.9` cores remain, so the root-cause loop continues.

## [2026-08-09] fix | Record strict Empty world-camera boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], and [[replicated-unit-noops]] for `96e1308a` (`Skip world camera at Empty stage`). `Empty` no longer spawns the world `WowCamera`/`Camera3d`; `Character` and later cumulative stages still retain one. The standalone performance panel/UI camera remains active, and the permanent `Empty`–`Npcs` post-process gate from `3b144afc` is unchanged. Commit tests cover zero world cameras in `Empty` and one world camera across `Character` re-entry. This audit changed documentation only; no live relaunch or performance claim was made.

## [2026-08-09] fix | Record permanent Empty-to-Npcs camera gate

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], and `index.md` for permanent behavior commit `3b144afc`. Recorded removal of the WowCamera TAA/SSAO/depth/normal/motion-prepass bundle plus `TemporalJitter`/`MipBias` through cumulative `Empty`–`Npcs`; `Lighting` onward and unconfigured/default stages retain graphics-option-driven behavior, including TAA restoration and configured MSAA depth/normal prepasses. Common bloom/render-scale/CAS/DoF, camera identity, tonemapping, shadow filtering, spatial audio, UI/network/IPC, and FPS overlay remain unchanged. The temporary selector was removed in `e9d3d470`. Alessio accepted the diagnostic cause. The historical PID `3367453` diagnostic client later exited at `2026-08-09T06:05:43Z` with `WindowCloseRequested` followed by `AppExit Success`; its socket is gone, with no coredump, OOM kill, crash, or agent lifecycle action. It was not permanent-build verification, and the permanent Empty replacement human gate remains pending; do not relaunch or advance it without Alessio's explicit permission.

## [2026-08-09] investigation | Confirm Empty-stage camera bundle cause

Updated [[procedural-cloud-regeneration]] and [[rendering-pipeline]] for diagnostic commit `b6468868` and live proof `/tmp/claude/game-engine-perf/empty-camera-post-process-live.json`. At capture time, PID `3367453` was the only running client on `/tmp/game-engine-3367453.sock`, connected to `InWorld` with one link, one local player, 134 remote entities after sampling, zero terrain tiles, empty game UI, and zero UI/font/panic/GPU-error evidence. The performance panel and non-targeted camera behavior remained active. Alessio judged performance improved and accepted the camera bundle as the Empty-stage cause; six post-warmup samples are supporting only. The client later exited at `2026-08-09T06:05:43Z` with `WindowCloseRequested` followed by `AppExit Success`; no coredump, OOM kill, crash, or agent lifecycle action was observed. Commit `e9d3d470` removed the temporary selector from source before permanent implementation. No permanent fix is claimed yet; do not relaunch or advance the human gate without Alessio's explicit permission.

## [2026-08-08] fix | Record pre-UI game/UI-toolkit scheduling boundary

Updated [[ui-system]], [[procedural-cloud-regeneration]], [[replicated-unit-noops]], [[networking]], and `index.md` for `game-engine` `508891a6` and `ui-toolkit` `50e4a17`. Recorded the empty-stage `UIActionBar.BLP` flood (**853,196 lines**, **75.9 MB**) and root cause: `UiRenderEnabled(false)` gated only the inner renderer while game-UI builders, sync/input work, texture-related frame processing, and observers continued. Recorded `UiProcessingEnabled` around the complete toolkit UI update chain, cumulative pre-`Ui` game-UI gates, independent FPS overlay tests, and machine-side relaunch proof from `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json`: connected `InWorld`, one link, one local player, 133 remote entities, empty toolkit UI tree and `MainActionBar` filter, zero `[UI]`/`UIActionBar.BLP` lines, no font panic/GPU OOM/device-loss/panic, and responsive ping/performance. The client was left running for Alessio at that time; human visual approval remained pending and no next stage launched. The three performance samples are not comparative evidence. Preserved existing M2 and replicated-unit NOOP facts.

## [2026-08-08] investigation/fix | Record empty-stage replicated-unit NOOPs and committed suppression

Added `investigations/replicated-unit-noops.md`; updated `systems/networking.md`, `investigations/procedural-cloud-regeneration.md`, and `index.md`. Recorded the preserved empty-stage boundary: `remote_entities=133` includes one local player (132 NPCs plus one local player), client per-frame unchanged `Transform`/`Visibility` writes, Lightyear receiver equality suppression versus server-side same-value movement/gravity serialization, real wander movement, and nearby movement-type-2 NPCs without waypoint rows. Recorded tests and fixes from `3c77d346`, `2927382`, and `ae81c65`. No runtime FPS improvement is claimed before corrected-binary relaunch.

## [2026-08-08] feature | Document World Builder diagnostic sidebar

Added `specs/world-builder.md` and `systems/world-builder.md`; updated `systems/ui-system.md`, `reference/keybindings.md`, and `index.md`. Recorded opt-in lifecycle, scene inventory, reversible render/processing overrides, bounded property editing, fixed F9 input, and measurement constraints.

## [2026-08-08] investigation | Record preliminary M2 UV comparison

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md`. The initial direct pair used the same binary/source/options/server/token/environment, ten readiness polls, one local player, stable recorded world/material invariants, 125-second holds, and six unprofiled samples per condition. Enabled measured **31.767 FPS / 54.177 ms**; disabled measured **36.843 FPS / 54.482 ms**. The first sample in each condition immediately followed an expensive `dump-scene` request and inherited its long diagnostic frame (**188.06 ms** enabled after **215 ms** scene latency; **210.88 ms** disabled after **266 ms** scene latency). Recorded readiness workloads also differed (**135** versus **133** remote entities). The result is therefore **preliminary/inconclusive pending a clean repeat** with a prospective performance warm-up; it supports no M2 performance conclusion or fix. An earlier startup attempt ended at a Friz parse failure; the pre-overwrite bytes were not preserved, while the current Friz/Arial bytes pass the exact Bevy parser. Selector/tests/flag were removed in `58e2f9c2`, then restored temporarily in `a7784e70` for the repeat.

## [2026-08-08] investigation | Record stabilized UI/render performance evidence

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md`. Recorded the enabled control (**12.332 FPS / 81.157 ms**, 71 remote entities, `game-engine` `00e7b3b0` / `ui-toolkit` `5ead575`) and all-text-disabled diagnostic (**37.415 FPS / 26.785 ms**, 76 remote entities, `game-engine` `6806717c` / `ui-toolkit` `43a2784`). The runs used different revisions and exact workloads; the delta strongly implicates UI-text-associated rendering with moderate confidence, not proof. Recorded the rejected/reverted equality-guard experiment (**11.373 FPS / 90.230 ms**, commits `33fa74d`/`36d4692`), engine CPU/Compute Task Pool load, adapter-wide shared GPU-busy measurement, wgpu buffer-transition/unmap attribution, low text-extraction self-cost, and unresolved downstream causality. Distinguished unprofiled CLI performance evidence from profiler attribution. Recorded transient reconnect failures later cleared by an unchanged logged launch. Shadow-only diagnostic is implemented with a GREEN behavioral test; live measurement is pending. No production fix or FPS improvement claim is made.

## [2026-08-08] fix | Record SSAO anti-aliasing compatibility

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md` for commit `cff4ad46` (`Keep SSAO compatible with anti-aliasing`). The real `WowCamera` now removes SSAO under default MSAA4x; switching to TAA restores `Msaa::Off`, `TemporalAntiAliasing`, and SSAO. The RED test reproduced SSAO with `Msaa::Sample4`; the exact GREEN compatibility test passes, removing the per-frame Bevy incompatibility error path. No runtime FPS improvement is claimed until the restarted engine is measured.

## [2026-08-08] correction | Record Mailbox presentation evidence

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md` for commit `89f58874` (`Use mailbox presentation for VSync`). Corrected prior performance evidence: screenshots and the stale 15.68 FPS overlay are visual artifacts, not baselines; five seconds without CLI requests produced zero completed SSAO extraction frames; `ping`, `status`, and `performance` waited approximately one second; main-thread stacks waited in `SubApps::update`; and the render worker blocked in Vulkan `Queue::present` through Wayland `wl_display_dispatch_queue`/`ppoll` with events every approximately 0.96–0.97 seconds. The Vulkan surface supports Mailbox and FIFO. Existing `vsyncEnabled=false` selected Mailbox and removed the stall; production now maps VSync-enabled mode to Mailbox while VSync-disabled remains `AutoNoVsync`. Fully visible unfocused Mailbox evidence: 142 frames/5.009 seconds (28.35 FPS), CLI six-sample mean 29.32 FPS / 34.37 ms, request mean 38.7 ms, CPU 226.57% of one core, process GPU gfx busy 58.01%, system GPU busy mean 60.33%, network InWorld/connected. The cloud simplex hotspot disappeared, but its earlier FPS attribution is invalidated by the presentation stall. SSAO/MSAA remains unresolved.

## [2026-08-07] update | Record final procedural-cloud post-fix evidence

Updated `investigations/procedural-cloud-regeneration.md` with provisional post-fix evidence later superseded by the presentation-stall investigation: screenshot-derived 27.98/27.89 FPS values and the single 28.14 CLI result were not valid baselines. The later correction records the Mailbox presentation fix and valid CLI evidence. The cloud simplex hotspot remains removed; SSAO/MSAA remains separate and unresolved.

## [2026-08-07] update | Document IPC performance diagnostics

Updated `AGENTS.md` and `investigations/procedural-cloud-regeneration.md` for commit `9003b421`: `game-engine-cli performance` reports `fps`, `frame_time_ms`, and `focused`; screenshot `FPS: 1.00` overlays are capture-frame artifacts, not timing evidence.

## [2026-08-07] investigation | Remove synchronous procedural cloud regeneration

Updated `systems/rendering-pipeline.md` and added `investigations/procedural-cloud-regeneration.md` for commit `b2b07e5b`: 512×1024 six-octave cloud textures regenerated synchronously every five seconds, with profiler self samples placing about 60% of sampled CPU in simplex cloud functions. Shader UV/time scrolling already animates clouds, so runtime regeneration was removed while preserving the three startup textures and visual settings. The earlier screenshot/CLI FPS values are now marked invalid because a separate presentation stall affected the measurement. IPC screenshots can show a transient 1.00 FPS overlay and are not valid FPS evidence; capture does not leave screenshot entities in the scene.

## [2026-04-30] update | Add Scenemachine M2 loading reference

Updated `reference/open-source-wow-clients.md` with Scenemachine as a C# reference for loading M2 scene/model data.

## [2026-04-09] ingest | Initial bulk ingest of 32 existing docs

Ingested all existing documentation from `docs/` into the wiki structure. Created pages across systems/, formats/, investigations/, design/, and reference/ categories.

## [2026-04-11] update | Document authored skybox black-output repro

Added `investigations/authored-skybox-black-output.md`, updated `systems/skybox.md`, and recorded the current `skyboxdebug` repro showing effectively black output for both default authored lookup and forced `LightSkyboxID 653`.

## [2026-04-21] update | Document LightParams sky-affecting flag composition

Updated `systems/skybox.md` with the implemented `LightParams::Flags` contract (`DontInheritSkybox`, `HideSun`, `HideMoon`, `HideStars`, `HideCelestialObject`, `OverrideCelestialSphere`, `HeightFogAbovePlane`) and how those flags now alter `skyboxdebug` procedural baseline/fog composition.

## [2026-04-21] update | Trace modern authored skybox shader/effect path

Updated `investigations/authored-skybox-black-output.md` with a detailed trace for `11xp_cloudsky01.m2` modern shader batches (`0x4014`, `0x8012`, `0x8016`), including stage binding, combine-mode routing, UV mode mapping, and the current WGSL combine-coverage gap for `0x8012`/`0x8016`.
## [2026-05-01] update | Document direct DB2 CASC access

Updated [[db2-format]] and [[asset-pipeline]] to record that DB2 bytes can be read directly from CASC via `AssetResolver::resolve_bytes`, with `ensure_db2_path` as a cache/debug path. Added `Frostshake/WDBx` as external verifier/export tooling rather than a runtime dependency.
