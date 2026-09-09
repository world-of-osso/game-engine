# Wiki Log

## [2026-09-09] systems | Persist independent graphics effect configuration

Updated [[rendering-pipeline]] and [graphics effect configuration](../specs/graphics-effects.md) for `7f86f086` / `24d97a50`. `options_settings.ron` now persists particle-effects intent, depth of field, bloom, AA mode, and SSAO defaults; invalid SSAO plus MSAA fails explicitly. Persistence and camera-component proofs are recorded separately. Particle runtime suppression remains pending.

## [2026-09-09] systems | Gate no-UI observer creation

Updated [[ui-system]] for `711ade3c`: the shared UI gate hides the clock and prevents health-bar/player-nameplate/NPC-nameplate observers from creating UI assets/entities under `--no-ui`.

## [2026-09-09] investigations | Fix empty mesh uploads

Updated [[movement-performance]] for `ebcee198`: skip GPU uploads for meshes that received no allocation. Real GPU empty/populated lifecycle regression passes; native `4ea941c3` InWorld capture has zero allocator errors. Patch provenance lives in [vendor/README.md](../../vendor/README.md#empty-mesh-uploads).

## [2026-09-09] investigations | Record temporary GPU-culling probe

Updated [[movement-performance]] for `802b5014`. `WOO_PERF_GPU_CULLING_AFTER_SECS` is a one-shot diagnostic for a native attribution snapshot, not a supported interface or production setting; it has no CPU/visual result and must be removed after measurement.

## [2026-09-09] systems | Keep FPS visible in no-UI diagnostic

Updated [[ui-system]] and [InWorld scene isolation](../specs/inworld-scene-isolation.md) for `c299491b`. Opt-in `--no-ui` forces numeric FPS visibility despite saved HUD/menu writes, keeps the frame-time graph hidden, and continues to disable toolkit and world-space HUD visuals.

## [2026-09-09] systems | Add full UI-off diagnostic

Updated [[ui-system]] and [InWorld scene isolation](../specs/inworld-scene-isolation.md) for `a60cbc38918ec27c730a2166fb8312223501fb6b`. Opt-in `--no-ui` disables toolkit processing, rendering, text, and world-space HUD visuals; UI plugins/resources and 3D rendering remain. Defaults and existing pre-`Ui` diagnostics are unchanged.

## [2026-09-09] investigations | Record temporary application opt1 CPU evidence

Updated [[compile-latency]] and [[movement-performance]]. A temporary application-only opt1 build retained dynamic dependencies and runtime policy; one matched 20-second InWorld pair observed **253.179033% → 227.135068%** CPU. The reverse pair has clock confounding, initial 1m53s build is not edit-latency evidence, and default configuration/adoption remain pending.

## [2026-09-09] systems | Keep camera collision independent of view culling

Updated [[rendering-pipeline]] and [[collision-system]] for `7d1d8a86`. Camera collision raycasts now honor hierarchy visibility without excluding walls merely because collision places them behind the camera frustum. The real transform/visibility/frustum ordering regression RED recovers through the wall; GREEN retains collision and still permits recovery when the wall's parent is hidden. Original-video camera-motion pixel equivalence remains unproven.

## [2026-09-09] systems | Fix foliage-card depth coverage

Updated [[rendering-pipeline]] for `59018936` and `f39cf99b`. Single-texture M2 blend mode 1 now uses the authored alpha mask rather than alpha-to-coverage. Under 4× MSAA with depth/normal prepasses, the GPU regression changed an alpha-zero foreground sample from black (`[0, 0, 0, 255]`) to its green background (`[0, 254, 0, 255]`), retaining its opaque red sample. Camera-motion flicker from the reported world recording remains unproven.

## [2026-09-09] systems | Avoid unchanged nameplate visibility writes

Updated [[ui-system]] for `6edcdc91`. Nameplate and quest-indicator visibility still synchronizes from HUD toggles every `Update`; matching `Visibility` components are no longer marked changed. Focused tests cover unchanged components and HUD show/hide updates. No performance benefit is claimed.

## [2026-09-09] systems | Avoid unchanged UI quad component writes

Updated [[ui-system]] for `ui-toolkit` `0f5d81c` through `3eb9aa7`: ordinary and backdrop UI quads retain per-frame synchronization, but identical computed `Transform` and `Sprite` values are no longer reinserted. Real geometry, color, texture, clipping, and default-value changes still synchronize; `render_dirty` and rendering cadence are unchanged. CPU improvement remains unproven.

## [2026-09-08] systems | Replace visual doodad solidity with authored M2 triangles

Updated [[terrain]], [[collision-system]], [[scripted-movement]], and its spec for `003e2399` / `9b7e61f2`. Doodad AABBs are broadphase only; authored triangles decide solid hits, absent authored geometry has no solidity fallback, and visual bounds remain independently available to zone interactions. Scoped evidence is 45 targeted passing tests. Native displacement, rendering, final integration, and CPU conclusions remain pending.


## [2026-09-08] systems | Initialize InWorld IBL independently of skybox visuals

Updated [[skybox]] and [InWorld scene isolation](../specs/inworld-scene-isolation.md) with missing camera environment-light initialization, override/stage guards, and registered-system fixtures. Ambient brightness, exposure, shadows, fog, and authored shader combines remain unchanged; native brightness verification is separate.

## [2026-09-08] investigations | Record current CPU attribution limits

Updated [[movement-performance]] with the post-fix1,642-sample capture: diffuse leaf costs, incomplete caller stacks, no demonstrated dominant next fix. Preserved unresolved CPU objective; no speculative renderer or profiler changes.

## [2026-09-08] investigations | Record final-source shared-palette CPU non-result

Updated [[movement-performance]] with the paired final-source (`6e0c4fde`) stationary InWorld captures: unshared **255.935040%** versus shared **264.428382%** process CPU over 20 seconds. Sampled clocks (**2242.9** versus **2149.9 MHz**) and remote populations (unshared **119→120**, shared **119→119**) differ. The evidence demonstrates neither a full-client CPU gain nor a causal regression; CPU objective remains open.

## [2026-09-08] investigations | Separate billboard and terrain behavior proof from CPU gains

Updated [[animation]] and [[movement-performance]]: billboard raw/final double-write regression and staged final pose fix; terrain-time filtering withdrawn after higher native CPU. Authorized terrain application-throughput proxy rose without proven lower per-update cost. CPU objective remains open.

## [2026-09-08] systems | Record constant-curve folding proof

Updated [[animation]] and [[movement-performance]] with constant-track eligibility, catalog construction characterization, and 87-test verification at `50454f89`. Native clocks differed; no additional attributable CPU gain claimed.

## [2026-09-08] systems | Add NPC animation LOD and measure it against the propagation spin

Updated [[animation]] and added [npc-animation-lod](../specs/npc-animation-lod.md). Replicated NPC models now sample Bevy clips every frame within 30 yd, every other frame at 30–60 yd, and not at all beyond 60 yd or off screen. Recorded the A/B in [[movement-performance]]: non-spin CPU fell ~28% but the vendored `propagation_worker` spin absorbed most of it, so total CPU moved only 397% → 378%.

## [2026-09-08] investigations | Record transform invalidation CPU observations

Updated [[movement-performance]] and [event-driven application updates](../specs/event-driven-application-updates.md). `4cfe7bfb` removes a RED-proven constant-pose transform invalidation. Retained post-fix CPU observations are lower than pre-fix observations, but remotes and clocks differ and the repeat was not controlled. CPU goal remains open.

## [2026-09-08] investigations | Resume CPU goal with current baseline

Updated [[movement-performance]] and [event-driven application updates](../specs/event-driven-application-updates.md). Current `206f844f` InWorld captures are **306.86%** and **304.85%** one-core CPU over comparable 20-second intervals; Compute Task Pool workers account for **264.02%** and networking **3.60%** in the first capture. `586b619a` reproduced three transform-change notifications for a constant animated pose; `4cfe7bfb` eliminates that invalidation (**1/1** targeted GREEN). CPU improvement is still unmeasured, so the active CPU goal remains unresolved.

## [2026-09-07] systems | Consolidate execution and animation proof

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md); [[networking]] already records the native reconnect proof. `206f844f` makes deferred M2 binding retirement atomic: strict teardown RED 2 failures → GREEN 3/3, binding 11/11, and offline lifecycle 1/1 with fmt/check. Native `--screen m2debug` rendered `126487.m2`, reported it displayed, retained a screenshot, and recorded 25 changing bone positions. `--screenshot-regression` bypasses the custom animation plugin, so it is excluded. Native forced disconnect and ordinary reconnect are complete behavior proof. Pixel/GPU equivalence and performance were not measured and are not implied.

## [2026-09-07] systems | Reconcile network execution proof boundaries

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md). Worker-permit lifecycle/cadence (**3/3**), active cooldown (**3/3**), and native forced-disconnect → Login are complete proof. Remaining execution proof is native ordinary reconnect, real offline model spawn/despawn/reload, native visual equivalence, and controlled CPU/performance measurement. Historical pending statements remain historical.

## [2026-09-07] systems | Fix interrupted M2 crossfade continuity

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `aaec3864`, `a495893f`, and `5de8e843`. Bevy retains the last blended raw pose in evaluator commit before pivot correction and billboard rotation; an interruption blends that snapshot to the new sequence. The focused Bevy regression proves zero-elapsed and repeated-interruption continuity for translation, rotation, scale, and a nonzero pivot. The historic `a6a5d917` jump remains recorded. Pixel equivalence, GPU deformation, real offline-scene lifecycle, and performance remain open.

## [2026-09-07] systems | Prove native forced-disconnect lifecycle

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `1cce4171`. Authenticated client771292 matched the server netcode connection before an admin kick produced real `Disconnected`, InWorld → Login, visible `LoginRoot`, and zero links/replicas. Hidden semantic scene entries remain; this is not full cleanup or visual proof.

## [2026-09-07] systems | Record worker forced-disconnect handling

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `4d3c7ed6`. After recording `ForcedDisconnect`, the auth receiver requests the worker's actual Netcode client disconnect; its actual-worker RED/GREEN preserves the notice through the published disconnect lifecycle.

## [2026-09-07] systems | Bound interrupted Bevy crossfade proof

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `a6a5d917`. The test preserves legacy two-pose controller timing (`x0→4→16→20`) but does not prove full-pose/outgoing-weight continuity: a 40% A→B blend interrupted by B→C at 30% begins B→C with B weight 70%. Attachment and skinning proof remain open.

## [2026-09-07] systems | Record native replicated-equipment preservation

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `eaeaf9ed`. Native `bevy-animation/native-fixed/{before,head,cleared}.json` proves server-driven Head 1128 set/clear retained `humanmale_hd.m2` and restored the exact baseline appearance data. This is exported lifecycle-state evidence, not pixel-level appearance or animation-equivalence proof. At this point forced-disconnect worker handling was pending; CPU/FPS claims remain open.

## [2026-09-07] investigation | Record Bevy dynamic-link compile measurement

Added [[compile-latency]] for the corrected `dev = ["bevy/dynamic_linking"]` wiring in `8fca26b9` and the command documentation in `50991d70`. Same-literal real-edit samples fell from 21.173815 s default to 6.956593 s, then 4.566569 s after the dynamic cache warmed. Deleted-cache warmups and concurrent compile activity are excluded; the requested under-three-second edit-build target remains unmet. Repository history does not support a Windows rationale for the prior unwired direct `bevy_dylib` dependency.

## [2026-09-07] systems | Replace M2 pose loop with Bevy playback

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `6ed7129b`, `780baa1f`, `efe0cb26`, `8056741d`, and `12cced65`. M2 sequence policy, timing, and crossfade state remain in `M2AnimPlayer`; paused Bevy graph clips seek to those times. Bevy now evaluates/blends supported sequence-local raw TRS through custom curves, then commits existing `BonePivot` correction. The old per-model bone-application loop is removed. Graph bindings use existing joints and two nodes per sequence for independent outgoing/current times. Bevy evaluates in `PostUpdate` before transform propagation; this is not a separate 60 Hz animation worker. Focused integrated evidence is 67/67; native equivalence and CPU/FPS claims remain open.

## [2026-09-07] systems | Record literal clean-frame application exits

Updated [[networking]], [[sound]], [[ui-system]], and [event-driven application updates](../specs/event-driven-application-updates.md) for `dc6183f8`, `550b637a`, `9a6b6679`, `1c1d7998`, `ae222f0e`, `e9b81652`, and `fc99128b`. Clean render frames no longer run reconnect/reset lifecycle, sound maintenance, active cooldown advancement, local mount/tag/alive synchronization, addon watcher handling, or the combined spellbook/UI path. Rendering, interpolation, animation, and active presentation remain render-frame-driven. CPU/FPS improvement remains unclaimed pending controlled measurement and user observation.

## [2026-09-07] systems | Remove confirmed idle application frame work

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `1a8c6d58`, `cd47743e`, `c4d936b1`, `12cb981e`, `dc6183f8`, `550b637a`, and `9a6b6679`. Dedicated worker transport remains 60 Hz with negotiated 20 Hz simulation. Main receive/apply/send and reconnect/reset run on `NetworkTick`; equipment remains mutation-driven. UI sync/pointer, automation, addon application, local-player/mount synchronization, sound maintenance, and active cooldown progression now use change, request, relevance, playback, or active-cooldown triggers rather than their prior idle per-frame work. Rendering and remote interpolation remain frame-driven. Controlled CPU/FPS savings are unproved.

## [2026-09-07] systems | Record worker restart and native bridge evidence

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `cf517c34`. `worker-restart-tests.log` records 3/3 actual worker shutdown cases: old worker join, old sender closure, stale queue cleanup, and a second UDP handshake without a main-app update. `migrated-ui-reconnect-fixtures.log` records 11/11, resolving the previous fixture rerun. `worker-native-build.log` succeeded; `worker-native-inworld/` reached InWorld with mirrored player/NPC entities and routed Who result `Theron`, one result. The dark scene/white UI remains pre-existing invalid visual smoke; no clean-render, complete replication/equipment, or CPU claim follows.

## [2026-09-07] systems | Record independent UDP proof and explicit wire identity mappings

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `52508d1d`, `a5f6eab0`, `23ebb85b`, and `2d8b3c0f`. `independent-udp-handshake.log` records 1/1 real UDP handshake after `54411453`, without a main-app update. Target, emote, combat, duel, inspect, and current/default spell entity fields now use explicit main/server identity conversion; numeric spell selectors remain server IDs. Scoped proof now includes worker character-create transport responses 3/3, worker auth 23/23, and wire identity 19/19. `e9e7e034` repairs the binary fixture behind the prior 60/61 result, but its targeted rerun remains pending; no integrated lifecycle, native appearance, or CPU claim follows.

## [2026-09-07] systems | Record dedicated connection-owned network world boundary

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `aa6fda57`, `6eb52d96`, and `54411453`. Main now starts one separate 60 Hz network ECS world per connection; that world owns Lightyear transport, protocol, replication, and typed receive buffers while preserving the 20 Hz simulation. Main holds only connection proxy markers, typed application inboxes, and a server-to-render entity mirror. This is not completion: focused runtime evidence is 19/20 after a UDP replication run exposed Bevy B0002 from querying resources as `EntityRef`; `54411453` excludes those resources but awaits a fresh integrated run. Wire entity-bit boundary conversions, binary fixtures, reconnect, native appearance, CPU, and renderer limits remain open.

## [2026-09-07] systems | Record unintegrated owned-inbox network runtime foundations

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `d4742d87`, `80476abe`, `daa1a7b3`, `cec56837`, and `01cfcade`. Application handlers now use worker-backed `MessageSenders`/`MessageReceivers`; `network_events` dispatches application-owned `Inbox<M>` batches and no longer parks/restores main-world Lightyear receivers. `runtime-tests-transport.log` records worker/module 13/13, including encoded Lightyear loopback while the main app is unupdated; dispatcher RED then GREEN is recorded in `owned-inbox-red-behavior.log` and `owned-inbox-green.log` (8/8). The main binary still does not start the worker or transfer client, transport, replication, lifecycle, or reconnect ownership, so no runnable independent-network or CPU claim follows. Existing native and CPU limits remain unchanged.

## [2026-09-07] investigation | Record event-driven native follow-up

Updated [event-driven application updates](../specs/event-driven-application-updates.md) and [[movement-performance]]. Empty-stage login reached InWorld; its ten-second unfocused sample was 265.779% one-core CPU and 503.746 application updates/s. The earlier 293.279%/422.366 Empty sample used different clocks, so no causal CPU reduction is claimed. A full-world client logged in and completed Who/friends replies, but its visual smoke is invalid from repeated slab-allocator errors and a white/dark screenshot. The same error predates this work in `connected-warm2/client.log:149`; no equipment-event causality is established. Test clients stopped. Render-independent network-world execution remains open.

## [2026-09-07] systems | Record deferred application inbox dispatch

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `2a8abacb`, `71d80355`, migrated API handlers, event-driven equipment, and character-create response ownership. Link/transport/message maintenance remains per-frame because transport timers consume frame delta. When no logical 60 Hz application tick is due, typed inboxes move out before `Last` clears them and return in `First`; handlers run only at logical ticks on the main thread. Collection/death and character-create responses now have one network consumer. Focused proof: network 8/8, APIs 55/55, equipment 13 plus 2 appearance tests, IPC FIFO 1/1, character-create 3/3. Connected lifecycle, independent transport/thread execution, native appearance delivery, and CPU improvement remain unproven.

## [2026-09-07] docs | Record event-driven network groundwork and IPC queue guard

Updated [[networking]], [event-driven application updates](../specs/event-driven-application-updates.md), and `index.md` for `1aec1091`, `fc82c5b7`, and `db7e6e7b`. `NetworkTick` is a 60 Hz logical schedule on the main ECS thread, not an independent OS networking thread; it leaves the negotiated 20 Hz simulation unchanged. The central dispatcher now routes auth and profession work over existing typed FIFO inboxes, and IPC skips dispatch parameter acquisition with no pending command. Broad application/API migration, integrated delivery/reconnect and appearance proof, idle-work measurement, and any CPU improvement remain open.

## [2026-09-07] docs | Retire forced strict-Empty pacing claims

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], [[networking]], [[ui-system]], [[replicated-unit-noops]], [[empty-window-baseline]], and `index.md`. `281d291a` removes `4fb2e5c9`'s forced 100 ms / 10 FPS Empty limiter: only the configured global frame-rate limit applies. Historical near-10-FPS readings, including 11.199% at approximately 9.994 updates/s, are capped data—not an uncapped baseline or a 97% CPU gain. `35e15d27` is build-backed successful startup after omitting Empty PBR/light/debug/material paths and target visuals; Empty is startup-only. Uncapped Green is pending.

## [2026-09-07] docs | Correct strict-Empty LightPlugin boundary

Updated [[rendering-pipeline]], [[empty-window-baseline]], [[movement-performance]], [[inworld-scene-isolation]], and `index.md` for `be9a1ff7`. The August 11, 2026 LightPlugin-absent Empty claim remains historical: native RED showed PBR requires `PointLightShadowMap` and scattering-medium assets even with no light entities. Current documentation records LightPlugin as a required provider and retains only the gizmo disables. Native GREEN is still running; no pass is claimed until `update-schedule-isolation/strict-empty-fixed/` exists and the main session confirms it.

## [2026-09-07] diagnostic | Record coarse idle-CPU isolation and repaired caller stacks

Updated [[empty-window-baseline]] and [[movement-performance]] with destructive reduced-scene schedule/render-set cutoffs. `Update`, `PostUpdate`, and the combined main schedules are material contributors, while `Prepare`'s apparent CPU drop is invalid because it halves update cadence; `Specialize` is negative and `Queue` is pending. A literal Empty-stage launch failed before measurement from missing `PointLightShadowMap` and scattering-medium initialization. Recovered Deep-DWARF ancestry now covers all 2,138 archived reduced-game samples in `flamegraphs/reduced-game-dwarf.svg`; the prior raw flamegraph's corrupt caller frames remain historical only. Bevy 0.19 scheduling/material hazards and upstream #24448 are candidates or external context, not local causal findings.

## [2026-09-07] retirement | Remove unproven task-submission batching

Retired `079f0cbf`'s temporary Bevy task-submission patch and restored stock `bevy_ecs`/`bevy_tasks` sources and the root lockfile. Four same-binary, 10-second blank-renderer samples kept CPU high: baseline 207.679% and 215.581% one core; batched 220.981% and 203.752%. Nearby update rates and clock/host conditions varied, so the lower batched CPU/update estimates do not establish a causal gain. No deployment or retirement rebuild occurred. The [spec](../specs/task-submission-batching.md) is historical. The active question is why disabled subsystem registrations, including sky-material paths, still execute and cost CPU; this result does not make their callback volume normal or explain bulk CPU.

## [2026-09-07] experiment | Batch independent Bevy system-task registration

Documented the user-approved, reversible `bc827e0f`/`2a818246`/`9b68de16`/`876fa5db` task-submission experiment. Unchanged Bevy 0.19.0 `bevy_ecs` and `bevy_tasks` are locally patched without package-version or root-feature changes. `Scope::spawn_many` retains each future's existing panic/result handling while bulk-registering independent futures. The ECS executor buffers at most 32 ready Send-system indexes only after its existing access and condition checks; it retains per-system completion, and leaves non-Send/exclusive execution unchanged. `BEVY_ECS_BATCH_TASK_SUBMISSIONS=1` enables batching; unset/`0` retain baseline and invalid values fail explicitly, with an executor-local override. This changes registration calls, not system count or body parallelism, and targets active-task registration locking rather than all task/memory/render cost. Scoped-task tests pass 3/3; ECS behavioral tests and bounded engine CPU/update/throughput comparison remain pending. No build, runtime, focus, capture, or profiler test occurred for this documentation update.

## [2026-09-07] diagnostic | Resolve libc memory operations in preserved native profile

Updated [[empty-window-baseline]] with matching-library offline attribution. libc covers 1,413,726,253 sampled cycle period (12.1680% of the original total); Arch debuginfod symbols resolve `memcpy` at 2.4163%, `memset` at 1.1201%, `_int_free_chunk` at 1.7458%, and `_int_malloc` at 1.3708%. These are sampled-period shares, not CPU percentage points or caller attribution. Allocation/copy work cannot be assigned to task lifecycle or a render callback without runtime ancestry. The 11 MB libc debug artifact is retained under diagnostics; independent verification remains pending. No build, capture, runtime, focus, or profiler test occurred.

## [2026-09-07] diagnostic | Attribute decoded continuous-renderer instructions

Updated [[empty-window-baseline]] and linked [[movement-performance]] to offline attribution from preserved matching binaries. Blank continuous rendering resolves 78.61% of sampled cycle period across 1,089 executable addresses: `bevy_ecs` 19.6184%, `concurrent-queue` 15.7290%, `async-executor` 6.7951%, and `async-task` 5.5316%. Hottest sites are queue empty probing, executor locking, completion publication, and mutex spin. This identifies task lifecycle/synchronization operations, not upstream callbacks, waste, or a safe removal. Full-scene proportions use a different sample-count denominator and a reduced `no-npcs-ui`/terrain-isolated workload, so no causal comparison follows. No profiler test, build, capture, runtime, focus, or process action occurred.

## [2026-09-07] diagnostic | Bound local profiler entry overhead

Updated [[empty-window-baseline]] from the retained single-thread CPU benchmark. Across three alternating disabled/enabled rounds of 50,000 precreated equal-work span entries, median added CPU was 2.202289 µs/entry at 154 names and 2.355397 µs/entry at 1,031 names. `93cef709` relocated the test-only benchmark without behavioral change to `src/cpu_system_profile/overhead_benchmark.rs`; independent verification passed formatting and both changed Rust files' readability audit without rerunning the benchmark. This is not a whole-app bound, subtraction, or scheduling-causation result.

## [2026-09-07] investigation | Clarify update-rate normalization

Updated [[movement-performance]] with source-backed metric provenance: game IPC/overlay FPS and blank-renderer logging both measure app-update cadence, with different smoothing. Neither proves presentation cadence. Compare exact per-capture update counts before interpreting CPU-per-update ratios.

## [2026-09-07] diagnostic | Isolate blank-renderer dirty-tree callback

Updated [[empty-window-baseline]] and [[movement-performance]] for the single-selector continuous-window extension and native before/after experiment. The removed callback and its worker spans disappear, but bulk CPU remains. Clock variation prevents an efficiency claim; the source investigation remains open.

## [2026-09-07] diagnostic | Add dirty-tree worker CPU spans

Updated [[empty-window-baseline]] for `5712ae71` and `55a7abe9`. The first behavioral test proved the existing Bevy `producer_mark_dirty`, `consumer_mark_dirty`, and `par_traversal_mark_dirty` spans were excluded; the retained selector now exports them with positive CPU. This records worker-span entries, including future polls and final drop, independently, does not charge them to `mark_dirty_trees` or count distinct tasks. Bevy creates the consumer/traversal workers before scanning changed transforms, a source candidate only. `d37278bb` enables the omitted transform tracing feature; native worker output is now recorded in the investigation, without a bulk root-cause claim.

## [2026-09-07] diagnostic | Blank renderer named-system profiler output

Updated [[empty-window-baseline]] after `29c3251d` and `23bdab77`. The pre-integration `7aa4` RED logged updates for 46 seconds with `WOO_CPU_PROFILE_OUTPUT` but exported no profile. The native GREEN exported 1,910 positive named system/thread spans. A retained ten-second, 11/11-focused pair on the same `5a36…` feature binary/default pools/1280×989 window measured 236.080% unprofiled versus 241.581% profiled process CPU and 1404.792 versus 1067.050 logged main updates/s. Clock differences and 263-second versus eight-second launch age mean it is not an isolated overhead estimate or an FPS/optimization comparison; a 0/13-focused pair is discarded. The five-second selected-span report has 12.487799 observed CPU-seconds (not whole-process CPU), 6.708256 selected self CPU-seconds, 3.055642 named-system self CPU-seconds, and 5.779543 seconds outside selected spans. `submit_pending_command_buffers` is largest at 390.551 ms/5,411 calls; no dominant callback or bulk root cause follows. Independent audit 268 confirmed the arithmetic and rejected a controlled-overhead interpretation.

## [2026-09-07] measurement | Continuous blank renderer is the first major CPU rise

Updated [[empty-window-baseline]], the staged service spec, and the index for source `76076850` and renderer binary `d847c8c5568a549b976972f8cdffc8e2cf8e1d99ec1cbfccb7a499bfe9cfa56d`. Twelve-second, 13/13-focused 1280×1198 samples measured native 0 ticks, Bevy core 0.16664% one core, reactive blank GPU 0.08331%, and continuous blank GPU 216.31478%. Reactive telemetry was 0.2 updates/s; continuous mean was 1212.906 updates/s (1071.767–1306.683 logged range), not presented FPS. `continuous` changes only Winit policy; renderer, camera, Mailbox, default pools, pipelining and no FPS limiter remain fixed. This locates the first major rise in continuous framework updates before project services, not a full-game root cause or an optimization. Independent runtime audit 248 passes this bounded attribution. Raw GPU metrics captured average graphics activity 0–1 reactive versus 80–86 continuous and average GPU clocks 618–682 versus 1945–2326 MHz; there is still no presented FPS, frame pacing, render-thread/per-thread CPU attribution, or hardware-counter proof. Continuous PID 1217890/window 331 remains open.

## [2026-09-07] diagnostic | Blank renderer smoke requires Bevy accessibility resource

Updated [[empty-window-baseline]], its staged service spec, and the index for `0669eac5` and `76076850`. `--service-window <core|render|continuous>` has no implicit stage: `render` is a reactive blank `Camera2d` GPU layer and `continuous` changes only the Winit update policy. The initial render smoke initialized the Vulkan adapter but failed before window creation because Winit required `AccessibilityRequested`; `AccessibilityPlugin` supplies that resource. Input and mesh remain required Bevy dependencies for Winit focus processing and render mesh extraction. No CPU, update-rate, or throughput result is claimed from the failed smoke; rebuild and visible-window proof precede measurement.

## [2026-09-07] measurement | Bevy core services have no bulk idle CPU source

Updated [[empty-window-baseline]] and its service spec with the source `217b4de8` + `6f6e10e3` focused comparison. Same binary SHA-256 `822ebfde5fa13dd13c9489a8bd519e41be91fb746954eea5a19c894b4c45fd25` and 1280×1198 native loop: core services measured **2 ticks/12 seconds** (**0.16664% of one core**), 25 threads and 13/13 focused; native measured 0 ticks, one thread and 13/13 focused. Host 5.407% versus 5.799% is not attributable to the single process. An unfocused closed-lid native capture is excluded. No renderer, continuous-frame FPS comparison, or full-game CPU conclusion follows. Source verification passed fmt/check/readability and 6/6 targeted tests; routing RED was valid, but the core module test has no demonstrated pre-implementation RED. Runtime data audit remains pending.

## [2026-09-07] documentation | Record additive Bevy-core service-window stage

Updated [[empty-window-baseline]], its specs, and the index for `217b4de8` and `6f6e10e3`. `--service-window` reuses the native softbuffer `Wait` loop, adds `MinimalPlugins` without `ScheduleRunnerPlugin`, and calls `App::update()` at `about_to_wait`. It excludes renderer, assets, game services, networking, IPC, sound, UI, timers, and continuous redraws. Native zero-work proof remains canonical on [[empty-window-baseline]]; service-stage runtime CPU measurement is pending.

## [2026-09-07] measurement | Empty window waits with no measurable idle CPU

Verified [[empty-window-baseline]]: two 12-second idle samples recorded zero CPU ticks, one sleeping thread in `do_epoll_wait`, and no game IPC/GPU resources. Each sample is below approximately 0.0833% of one core at accounting resolution. Native resizing and closing worked; the window was reopened for observation. Routing tests, formatting, checking, readability, dependency-version review, and independent native-data audit passed. This establishes the requested zero-work baseline, not a full-game optimization.

## [2026-09-07] documentation | Record empty native Wayland baseline

Added [[empty-window-baseline]] and linked it from the index. `c3568e59` routes the exclusive `--empty-window` mode before normal startup, so it creates no Bevy app, plugins, assets, networking, IPC, renderer plugins, or game task pools. Winit waits for OS events; softbuffer paints a flat background only for requested redraws and resize-triggered redraws. Linux painting is Wayland-only. `softbuffer` is the sole new dependency; existing locked versions remain unchanged. Native results are recorded in the subsequent measurement entry; the zero-work baseline cannot establish an FPS-comparable game optimization.

## [2026-09-07] measurement | Material group negative; native ownership mapped

Updated [[movement-performance]] with cumulative visibility/tree/material-preparation measurements and the explicitly approved 25-callback material group. With 11 removals retained, the group changed CPU 332.55→340.89% and FPS 231.63→240.90; no CPU benefit. Corrected native CU attribution maps 514/2,134 user-mode samples to application units, mostly generated/helper symbols rather than directly named game functions. ELF file offsets require PT_LOAD conversion; the unadjusted result is invalid. Independent audits passed. Bulk CPU cause remains unresolved; compiler/worker/FPS policy unchanged.

## [2026-09-06] measurement | Cumulative exclusions retain bulk CPU

Updated [[movement-performance]] with sender-only and cumulative results. Sender removal retained connection state but did not reduce CPU. All six retained exclusions still measured 330.30% CPU / 214.74 FPS in a focused six-second tail, then 331.80% / 234.68 FPS in a separately recreated, fully focused 12-second state. Mixed-focus and firmware-clamped phases are excluded from savings claims. Independent audits passed; diagnostic clients stopped.

## [2026-09-06] diagnostic | Add exact application-message sender isolation

Documented `dc922265` in [[movement-performance]]. `--freeze-message-send-after <SECONDS>` resolves only the uniquely named PostUpdate `MessagePlugin::send` callback and its one-member implicit set; receive, transport, rendering, and other send-group work remain active. The five tests cover exact removal, queued-message retention, unrelated work, ambiguity, defaults, and argument parsing. A worker stack proves the sender loop executes, not that it sent a message or explains CPU. Connected-idle runtime validation is still required; no CPU/FPS result or optimization claim follows.

## [2026-09-06] documentation | Canonicalize timed callback removal

Updated [[movement-performance]], the InWorld isolation spec, and `index.md` for `52e44eff`. The current interface is repeatable `--remove-system-after <main:SCHEDULE|render:SCHEDULE> <EXACT_SYSTEM_NAME> <SECONDS>`; it resolves one exact schedule/name and a one-member implicit set, removes with `RemoveSystemsOnly`, uses Extract as the safe barrier, and delays `render:ExtractSchedule` removal to Render cleanup. Added a six-row migration table. The retired `--freeze-*-after` flags have no aliases; dated entries retain their former syntax as historical evidence.

## [2026-09-06] measurement | Mesh-collection removal lacks comparable-throughput benefit

Updated [[movement-performance]] with the main-owned `78b3a63c` pair. The exact callback was removed at 30.004 seconds: all-focused windows measured **330.55% CPU / 200.17 FPS** before and **326.55% / 179.20 FPS** after, with a post-removal 600 MHz GPU limit. A later **310.70% / 103.89 FPS** window is excluded for collapsed throughput. The earlier agent-created pair corroborates but does not replace this evidence. `9271c885` behavioral proof passed 7/7; `78b3a63c` retained the established log prefix and removed test-only schedule synthesis. No CPU benefit.

## [2026-09-06] measurement | Camera-follow removal leaves bulk CPU unchanged

Updated [[movement-performance]] with the verified `a9de6b17` same-process pair: 338.80→338.39% CPU and 210.41→214.54 FPS after removal at 30.004 seconds. Focus and reported camera/player positions remained unchanged; rotation was not exposed. Five behavioral tests, formatting, locked checking, and readability passed. No material CPU reduction.

## [2026-09-06] diagnostic | Add timed camera-follow isolation

Documented `a9de6b17` in [[movement-performance]]. `--freeze-camera-follow-after <SECONDS>` removes only the registered `camera_follow` callback from `Update` at its `Last`-schedule cutoff, retaining the current camera transform while leaving camera input, player movement, graphics synchronization, rendering, normal pipelining, and unrelated systems active. Behavioral proof passed 5/5. This is stationary-only attribution infrastructure; no CPU/FPS result or optimization claim exists yet.

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
