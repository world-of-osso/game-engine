# Empty-window baseline

`--empty-window` provides a native Wayland window/event-loop baseline outside the game runtime. It measures external idle CPU separately from Bevy/game integration. The initial native baseline is verified below; it does not identify the full game's CPU cause.

## Runtime boundary

`main` checks the complete argument list before normal startup. The mode accepts only `--empty-window`; any extra argument is an error. A successful selection calls `empty_window::run()` then returns before thread-pool configuration, asset-root discovery, resource limits, simple-flag handling, CLI parsing, and `run_app`.

The mode therefore does not construct a Bevy `App`, register plugins, initialize game assets, networking, IPC, renderer plugins, or game task pools. Normal invocations do not select this branch.

## Native window behavior

`empty_window::run()` creates a winit event loop with `ControlFlow::Wait`, a softbuffer context, and one application handler. On resume it creates one window and requests its first redraw.

Painting occurs only for `RedrawRequested`. Resize and scale-factor events request another redraw. The paint path resizes the surface, fills it with a flat background, notifies the window before presentation, then presents. It requests no application timer, periodic redraw, update loop, or continuous rendering. Zero-size windows return before a surface resize or buffer acquisition. Close exits the event loop. Window, surface, context, event-loop, and presentation errors include context and return a nonzero process status through `main`.

Linux window attributes set the Wayland application name. The direct softbuffer dependency enables only `wayland` and `wayland-dlopen`; this diagnostic does not provide X11 or KMS painting. `winit` is reused as a direct dependency for its public event-loop API. `softbuffer` is the only added dependency, used for a shared-memory surface without starting a GPU renderer. Existing locked versions remain unchanged.

## Additive Bevy-core stage

`--service-window core` retains the same native event loop, `ControlFlow::Wait`, softbuffer surface, and resize/close path, then constructs an optional Bevy app before normal startup. It installs `MinimalPlugins` without `ScheduleRunnerPlugin`: task pools, frame counter, time, and core schedules. The native handler calls `App::update()` from `about_to_wait`; it adds no timer, redraw request, renderer, assets, game services, networking, IPC, sound, UI, or resource-limit setup. It logs the first update and each 64th update.

This separates Bevy core-service overhead from both native waiting and later renderer or game workload. The selector requires exactly one of `core`, `render`, or `continuous`; it has no implicit core alias and leaves ordinary startup unchanged.

## Measured additive core result

Source revisions `217b4de8` and `6f6e10e3` used binary SHA-256 `822ebfde5fa13dd13c9489a8bd519e41be91fb746954eea5a19c894b4c45fd25` and the same 1280×1198 native window loop as the native baseline. The focused 12-second core sample recorded **2 CPU ticks**: **0.16664% of one core**, across 25 threads, with all 13 samples focused. The corresponding focused native sample recorded **0 ticks**, one thread, and all 13 samples focused.

Whole-host busy CPU was 5.407% during the core sample and 5.799% during the native sample. That difference is not attributable to this single process. An earlier native sample with 0/13 focused observations is excluded because the lid was closed and no compositor window was focused; the user subsequently unlocked the desktop and the later focused samples are valid despite the lid state. There is no bulk CPU source at this core-service stage.

The source verifier passed formatting, locked checking, and readability; targeted tests passed 6/6. The routing RED was valid. The core module test did not demonstrate a pre-implementation RED, so it is not claimed as such. Runtime measurement has not received an independent data audit.

## Blank renderer stage

`--service-window render` installs a clear-only `Camera2d` renderer with normal Bevy pipelined rendering and the same default pool availability. It uses the desktop Winit policy: focused reactive updates with a five-second timeout and unfocused reactive-low-power updates with a sixty-second timeout. `--service-window continuous` retains that renderer, camera, Mailbox presentation, and pools but uses Bevy's normal game Winit policy, so focused updates become continuous. It records update rate rather than presenting it as FPS.

The renderer deliberately excludes project game services, PBR, lights, sprites, audio, and UI. Required Bevy dependencies are logging, transforms, input, accessibility, window, asset, Winit, render, image, mesh, camera, pipelined rendering, and core pipeline. Input is needed by Winit focus processing, mesh by render mesh extraction, and `AccessibilityPlugin` initializes the `AccessibilityRequested` resource required by Winit window creation.

The first `0669eac5` render smoke initialized the Vulkan adapter but panicked before window creation because `AccessibilityRequested` was absent. Commit `76076850` adds the required accessibility plugin. That failed launch has no CPU or renderer-throughput result.

## Measured renderer and continuous stages

Source `76076850` used renderer binary SHA-256 `d847c8c5568a549b976972f8cdffc8e2cf8e1d99ec1cbfccb7a499bfe9cfa56d`. A visible flat blank view was captured for the repaired `render` stage. Every retained measurement window lasted twelve seconds, had 13/13 focused samples, and used a 1280×1198 compositor window.

| Stage | Process CPU, one core | Threads | Update-rate telemetry |
| --- | ---: | ---: | --- |
| Native window | 0 ticks | 1 | Event-driven native wait |
| Bevy core | 0.16664% | 25 | Event-driven native wait |
| Blank GPU, reactive `render` | 0.08331% | 28 | 0.2 updates/s |
| Blank GPU, `continuous` | 216.31478% | 28 | Mean 1212.906 updates/s; logged interval range 1071.767–1306.683 |

The renderer comparison changes only the Winit policy: `render` uses the reactive desktop policy, while `continuous` uses the normal game policy. Renderer, camera, Mailbox presentation, default pools, normal pipelining, and absence of an FPS limiter are retained. Update rate is not presented FPS.

Whole-host busy CPU was 5.799% for native, 5.407% for core, 7.165% for reactive rendering, and 21.071% for continuous rendering. Those values are not single-process attribution. Reactive CPU/GPU clock ceilings were 4504–4937 / 2574–2900 MHz; continuous ceilings were 2274–3799 / 1498–2416 MHz. Raw GPU metrics captured average graphics activity of 0–1 for reactive rendering and 80–86 for continuous rendering, with average GPU clocks 618–682 versus 1945–2326 MHz. No 600 MHz row occurred, but clock limits differ and per-frame efficiency is uncontrolled.

Independent source verification and runtime audit `248` pass the bounded finding: the first major CPU increase appears when the otherwise blank framework enters continuous updates, before project services are registered. This is not an optimization, a same-throughput comparison, or a full-game root-cause conclusion. The evidence has no presented FPS, frame-time pacing, render-thread breakdown, per-thread CPU attribution, or hardware-counter proof. The continuous client remains open as PID 1217890, window 331.

## Optional named-system CPU attribution

Commit `29c3251d` lets the blank GPU stages use the existing opt-in `cpu-system-profile` layer. Only a `cpu-system-profile` feature build with `WOO_CPU_PROFILE_OUTPUT` set installs the layer; ordinary blank-renderer logging remains unchanged. The control does not alter pools, pipelining, presentation, or Winit update policy.

The control exports named Bevy system spans, not complete process ownership. Executor, driver, and uninstrumented work remain residual; any profiling run must measure instrumentation overhead separately. See [[movement-performance#named-span-thread-cpu-diagnostic]] for the profiler's capture window and accounting boundaries.

The native RED used pre-integration feature binary `7aa4` with `WOO_CPU_PROFILE_OUTPUT` set. It logged updates for 46 seconds but wrote no profile, proving that the former blank-renderer path bypassed the layer. `29c3251d` is the integration change. Native GREEN produced 1,910 positive named system/thread spans in an initial smoke capture.

A retained focused ten-second pair used the same feature binary (`5a36…`), default pools, and 1280×989 window, with 11/11 focused samples in each retained run. Unprofiled process CPU was 236.080% of one core with mean logged main-update rate 1404.792/s; profiled was 241.581% with 1067.050/s. The first 13-row pair had 0/13 focused samples and is discarded. The unprofiled rate came from 263 seconds after launch while profiled samples began eight seconds after launch, and clocks differ. This pair therefore does not isolate profiler overhead, does not measure presented FPS, and supports no optimization claim.

The five-second profile reports 12.487799 CPU-seconds between each reporting thread's first and last selected span; this is not whole-process CPU. Selected-span self CPU was 6.708256 seconds, named-system self CPU 3.055642 seconds, and 5.779543 seconds lay outside selected spans within those intervals. Largest named-system self entries were `submit_pending_command_buffers` (390.551 ms across 5,411 calls), `mark_dirty_trees` (214.325 ms), `apply_extract_commands` (164.715 ms), and `prepare_windows` (162.395 ms). No dominant callback or bulk CPU root cause follows. Independent data audit 268 confirmed the arithmetic, identity and focus, while rejecting the pair as a controlled overhead measurement because clocks/activity and startup age differ. Named output is verified; bulk CPU attribution and isolated instrumentation overhead remain open.

Commit `55a7abe9` additionally selects Bevy's existing `producer_mark_dirty`, `consumer_mark_dirty`, and `par_traversal_mark_dirty` worker spans. Test `5712ae71` first proved they were absent, then proved positive CPU export after selection. This measures worker spans independently; it does not charge cross-thread worker CPU to `mark_dirty_trees`, count distinct tasks, or establish the source candidate as the bulk cause. Instrumented futures enter spans for both polling and final drop. Bevy source shows `mark_dirty_trees` creates its consumer and traversal workers before scanning changed transforms.

The first native capture still lacked worker entries: Bevy 0.19's aggregate `trace` feature omits `bevy_transform/trace`. `d37278bb` enables that existing dependency feature only for CPU diagnostics; `19d8b432` records the dependency edges without package upgrades. The subsequent five-second capture exported producer self CPU 55.044 ms/4,340 entries, consumer 22.192 ms/13,020 entries, and traversal 277.001 ms/195,265 entries. Parent system self CPU was 296.424 ms/4,340 calls. Entry counts include polling and drop; these are not distinct-task counts. First-to-last selected-span CPU was 12.295116 seconds, selected self CPU 6.991881 seconds, leaving 5.303235 seconds outside selected spans. Profiler overhead and scheduling outside these spans remain unresolved.

The accompanying ten-second process sample recorded 251.179% one-core CPU and about 930.945 logged main updates/s. Focus was 0/11, recorded informationally: the user waived focus gating and the main thread took no focus. GPU ceilings ranged 646–2454 MHz, so no cross-run efficiency or causal focus claim follows. These measurements prove worker-span output, not bulk CPU ownership.

## Isolated dirty-tree removal

`aeff05db` and `ccc9b47a` allow one existing `--remove-system-after` request after `--service-window continuous`; other service modes remain exclusive. Parser tests demonstrated RED before the extension and GREEN 5/5 afterward. The experiment removes only `main:PostUpdate bevy_transform::systems::mark_dirty_trees` through the existing controller.

With the CPU profiler layer disabled but the same tracing-feature binary retained, one process logged the unique removal at 20.000 seconds. Ten-second process CPU samples measured 190.182% before and 187.484% afterward. Nearby logged update-rate means were 847.874/s and 1005.674/s respectively; these are not presented FPS. Pools, pipelining, window and binary remained unchanged. GPU ceilings reached 600 MHz in both windows and CPU ceilings varied, preventing an isolated efficiency claim. Bulk CPU remained high. The preceding profiler-enabled pair overlapped 81% host busy CPU and a GPU ceiling collapse, so it is not causal evidence.

A separate same-binary run removed the callback at 0.000 seconds. Its subsequent profile retained 170 other span names across 18 reporting threads but contained neither the target system nor its producer, consumer or traversal spans. This proves the intervention stopped that work, not that it solved bulk CPU usage. Artifacts: `data/diagnostics/movement-perf-20260905/service-window/renderer/named-profile/dirty-removal/`.

Both blank update telemetry and full-game IPC FPS describe app-update cadence, not presentation; their aggregation differs. See [metric provenance and normalization](movement-performance.md#update-rate-metric-provenance-2026-09-07).

## Native instruction attribution

The original continuous blank-renderer profile has 3,109 records totaling 11,618,357,418 sampled cycle period. Offline LLVM symbolization of the preserved matching ELF assigns each of 1,089 executable instruction addresses once to the nearest registry-source frame in its deepest-first inline chain. It resolves 9,137,307,152 period (78.61%); this is instruction ownership, not runtime caller ancestry or inclusive callback cost.

| Nearest source package | Sampled period share |
| --- | ---: |
| `bevy_ecs` | 19.6184% |
| `concurrent-queue` | 15.7290% |
| `async-executor` | 6.7951% |
| `async-task` | 5.5316% |

The hottest runnable-queue instruction resolves to `concurrent_queue::bounded::Bounded<Runnable>::pop` at `bounded.rs:268`: it reads the tail after a fence and returns `Empty` when tail equals head. ECS `Context::tick_executor` resolves to its `try_lock` at `multi_threaded.rs:356`; system completion pushes resolve to the bounded queue's slot-stamp compare/CAS at `bounded.rs:182`. The hottest mutex instruction is the standard library's bounded spin loop. These establish repeated queue probes, task dispatch/completion, and synchronization as real blank-renderer CPU operations.

`spawn_system_task` wraps each runnable Bevy system in a task and reports completion; `tick_executor` consumes completions and advances ready work. This does not establish that all queue work is empty or wasteful, identify the upstream systems behind generic helpers, or select a safe callback removal. In particular, an empty Bevy query scope returns after `f(scope)` when its scoped-task queue is empty (`bevy_tasks::task_pool::scope_with_executor_inner`); it does not enter the executor-draining path. The blank camera still keeps render/window/camera work populated. Missing decoded ancestry covers 95.7148% of total period, so bulk callback ownership remains unresolved. Further profiler testing stopped at the user's request; this analysis reused preserved artifacts only.

A separate full-scene capture had 2,138 samples, 1,898 from the executable across 1,512 addresses. Its nearest-source counts were `bevy_ecs` 27.689%, `concurrent-queue` 7.016%, `fixedbitset` 5.940%, `async-executor` 5.005%, and `async-task` 2.479%. Those are sample-count fractions, unlike the blank cycle-period fractions, and the capture used `--inworld-stage no-npcs-ui` plus terrain/render isolation. It is neither an intact-game baseline nor a literal Empty scene; do not compare these percentages causally.

## Local profiler entry overhead

The ignored test-only benchmark, relocated without behavioral change by `93cef709` to `src/cpu_system_profile/overhead_benchmark.rs`, compares the real `CpuSpanLayer` against registry-only tracing. It precreates spans, runs 50,000 equal-work enter/drop iterations per case, and alternates disabled/enabled order across three rounds while timing the current thread's CPU. Paired median added CPU was **2.202289 µs per entry** for 154 distinct span names and **2.355397 µs per entry** for 1,031 names.

This is local single-thread entry/exit overhead for this build and capture shape. It excludes span construction and does not reproduce concurrent scheduling, nesting, cache state, or other tracing work. Do not subtract it from recorded process or selected-span CPU, treat it as a whole-application bound, or infer scheduler causation from the update-normalized correlation. Independent verification of `93cef709` confirmed unchanged benchmark behavior, passing formatting, and zero readability findings in both affected Rust files; existing compile/list and benchmark evidence was reused without rerunning measurements.

## Measurement scope

Measure process/thread idle CPU externally after the window is visible, and record host CPU, window/focus state, GPU activity, clocks, and limits alongside it. Do not add an FPS overlay or diagnostic server. Native and core stages have no game workload or continuous rendering. Renderer stages add only blank-frame work; their deliberately different update policies do not establish a same-throughput game optimization.

## Verified native baseline

Commit `c3568e59`, binary SHA-256 `8cde96f5b86da49cf4cf6bf854a9e0d1cd0d9aabf048db12d0300508eace3b4a`, launched from an empty working directory. A visible flat window resized from 1280×1198 to 1480×1198 and exited through the window manager's close request.

Both 12-second idle samples recorded **zero CPU ticks**, one thread in sleeping state, and `do_epoll_wait`. At the host's 100-Hz accounting resolution, each result is below approximately **0.0833% of one CPU core**, not proof of mathematical zero. Native screenshots decode to the expected uniform background before and after resizing.

No game IPC socket, game-asset descriptor, GPU-device descriptor, or GPU-driver mapping was observed. A reopened instance had no owned TCP/UDP sockets; its event-loop timerfd was disarmed. The second instance was left visible for the next initialization step.

Independent verification passed routing tests (2/2), formatting, locked checking, readability, dependency-version review, and native artifact review. Proof lives under `data/diagnostics/movement-perf-20260905/empty-window-baseline/`, including `runtime/`, `visible/`, and `verification/verifier-report-c3568e59.md`.

## Sources

- [empty-window baseline spec](../../specs/empty-window-baseline.md) — native boundary and measurement scope
- [service-window baseline spec](../../specs/service-window-baseline.md) — staged contract and opt-in blank-renderer CPU-attribution control
- [main startup](../../../src/main.rs) — early diagnostic routing and error exit
- [CLI parsing](../../../src/cli_args.rs) — exclusive argument selection
- [empty window runtime](../../../src/empty_window.rs) — shared winit/softbuffer loop and optional app update
- [service window runtime](../../../src/service_window.rs) — minimal Bevy-core plugin boundary
- [native instruction attribution](../../../data/diagnostics/movement-perf-20260905/service-window/renderer/continuous/native-profile/instruction-owner-analysis.json) — one-address blank-renderer source attribution
- [full-scene instruction attribution](../../../data/diagnostics/movement-perf-20260905/settled-low-fps/cpu-system-isolation/profile-deep-dwarf/instruction-owner-analysis.json) — sample-count-only comparison scope
- [profiler overhead benchmark](../../../src/cpu_system_profile/overhead_benchmark.rs) — test-only local entry-overhead control
- [Cargo manifest](../../../Cargo.toml) — direct dependency features

## See Also

- [[movement-performance]] — comparative game-runtime CPU investigation results and named-span profiler accounting
- [[rendering-pipeline]] — full-game rendering architecture beyond the blank diagnostic scene
