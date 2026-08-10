# Procedural Cloud Regeneration

Before commit `b2b07e5b`, in-world sky updates synchronously regenerated one procedural cloud texture every five seconds. The work consumed most sampled CPU during regeneration even though the shader already animates cloud UVs over time.

## Finding

`src/rendering/skybox/cloud_texture.rs` defines 512×1024 cloud images with six simplex-noise octaves. Each image contains 524,288 pixels; each pixel evaluates two six-octave fractal-noise fields. `create_procedural_cloud_maps()` generates three textures at startup.

Before `b2b07e5b`, `update_procedural_cloud_maps()` regenerated the next texture synchronously from `Update` while `InWorld` or `CharSelect` was active. The profiler's self samples placed approximately 60% of sampled CPU in `simplex2`, `simplex_corner`, `fbm_simplex`, `gradient`, and related cloud functions.

## Resolution

Commit `b2b07e5b` removed runtime regeneration and its `Update` registration. The three startup textures, texture dimensions, six octaves, cloud parameters, and visual settings remain unchanged. `assets/shaders/sky.wgsl` continues to animate clouds by offsetting sampled UVs with time-derived cloud parameters, so runtime pixel regeneration was not required for cloud motion.

## Corrected Performance Evidence

Earlier screenshot-derived values (**27.98 FPS**, **27.89 FPS**) and the single `game-engine-cli performance` result (`28.14 FPS`) are not valid performance baselines. Screenshots and the stale **15.68 FPS** overlay are visual artifacts; the overlay can remain unchanged while no frames complete.

The corrected root-cause evidence was collected before commit `89f58874`:

- Five seconds without CLI requests produced **zero completed SSAO extraction frames**.
- `ping`, `status`, and `performance` each waited approximately **1 second**.
- Repeated main-thread stacks waited in `bevy_app::sub_app::SubApps::update`.
- The render worker blocked in wgpu Vulkan `Queue::present` through Wayland `wl_display_dispatch_queue`/`ppoll`, receiving events every approximately **0.96–0.97 seconds**.
- The Vulkan Wayland surface supports **Mailbox** and **FIFO** presentation modes.

The existing `vsyncEnabled=false` path selected **Mailbox** and removed the presentation stall. Commit `89f58874` now maps VSync-enabled mode to `PresentMode::Mailbox`; VSync-disabled mode remains `AutoNoVsync`.

With Mailbox, fully visible and unfocused in-world evidence was:

- **142 frames / 5.009 seconds:** **28.35 FPS**
- Six-sample CLI mean: **29.32 FPS**, **34.37 ms** frame time
- CLI request mean: **38.7 ms**
- Process CPU: **226.57% of one core**
- Process GPU gfx busy: **58.01%**
- System GPU busy mean: **60.33%**
- Network: `InWorld`, `connected=true`

The cloud fix is confirmed to remove the procedural simplex hotspot: later profiler evidence contained no `simplex`, `fbm`, `cloud_density`, or `generate_cloud` rows. The earlier FPS comparison depended on invalid screenshot/CLI-stall evidence, so no FPS effect is attributed to the cloud fix.

## Empty-Stage UI Processing Boundary

The preserved empty-stage run emitted **853,196** repeated `UIActionBar.BLP` blacklist lines, totaling **75.9 MB**, and starved IPC. The root cause was upstream scheduling: `UiRenderEnabled(false)` disabled only the ui-toolkit render subchain, while game-UI builders, registry/layout processing, button input, texture-related frame work, and game-UI observers continued to run.

`game-engine` commit `508891a6` gates in-world UI builders and sync systems before the cumulative `Ui` stage, including minimap, action bars, unit frames, all registered in-world frame plugins, group frames, in-world game-menu paths, quest sparkles, and nameplate/health-bar observers. Teardown remains unconditional. `ui-toolkit` commit `50e4a17` adds default-enabled `UiProcessingEnabled` around the entire chained UI `Update` schedule while preserving the inner `UiRenderEnabled` and text gates.

The Bevy performance panel is independent of this boundary: its startup, diagnostics, and overlay text systems are registered by `FpsOverlayPlugin` separately from ui-toolkit processing. Added tests cover each pre-`Ui` stage, enabled defaults, processing pause/re-enable behavior, and overlay survival.

Machine-side empty-stage relaunch proof is recorded in `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` using `game-engine` behavior commit `508891a6` and `ui-toolkit` commit `50e4a17`. The client reached connected `InWorld` with `connected_links=1`, `local_players=1`, and `remote_entities=133`. The toolkit UI tree and `MainActionBar` filter were empty. Engine stderr had zero `[UI]` lines and zero `UIActionBar.BLP` blacklist lines, plus zero font-parse failures, GPU-OOM mentions, device-loss mentions, and panic mentions; `ping` and `performance` remained responsive. That client was left running for Alessio's visual inspection, then exited independently before diagnostic replacement; current live-client state and the launch gate are recorded below. The three captured performance samples are not comparative evidence and no FPS improvement is claimed.

## Permanent Empty-to-Npcs Camera Render-Bundle Gate

Commit `3b144afc` makes the accepted diagnostic behavior permanent for cumulative scene stages `Empty`, `Character`, `Skybox`, `Terrain`, and `Npcs`. On `WowCamera`, those stages remove only `TemporalAntiAliasing`, `ScreenSpaceAmbientOcclusion`, `DepthPrepass`, `NormalPrepass`, `MotionVectorPrepass`, `TemporalJitter`, and `MipBias`.

`Lighting` and later stages restore the graphics-option-driven path: TAA restores `Msaa::Off`, TAA, SSAO, motion vectors, temporal jitter, and mip bias; non-TAA settings restore the configured MSAA behavior and depth/normal prepasses. An unconfigured/default stage also retains the normal graphics-option-driven path. Bloom, render-scale resolution, CAS, depth of field, camera identity/components, tonemapping, shadow filtering, spatial audio, UI, networking, IPC, and the FPS overlay remain unchanged. The implementation restores prepasses removed by the Empty-to-Npcs gate before applying the normal anti-aliasing/SSAO synchronization.

The temporary selector was removed in `e9d3d470`; no `--disable-wow-camera-post-process` compatibility path remains. Alessio accepted the diagnostic cause from the old comparison, and machine samples remain supporting evidence only.

## Strict Empty World-Camera Boundary

Commit `96e1308a` (`Skip world camera at Empty stage`) makes `Empty` stricter than the render-bundle gate above: `spawn_world_environment` creates the world `WowCamera`/`Camera3d` only when the cumulative stage includes `Character`. `Empty` therefore has no world camera; `Character` and every later configured stage still retain one. The standalone performance panel and its UI camera remain available, and the permanent `Empty`–`Npcs` post-process gate from `3b144afc` is unchanged. The commit's tests prove zero world cameras in `Empty` and one world camera across `Character` re-entry. This audit updated documentation only; no live relaunch or performance claim was made.

Historical live proof `/tmp/claude/game-engine-perf/empty-camera-post-process-live.json` belongs to PID `3367453`, socket `/tmp/game-engine-3367453.sock`, and the preserved diagnostic binary from `b6468868`. That client exited by `2026-08-09T06:05:43Z`; the socket is gone, and stderr ends with `WindowCloseRequested` followed by `AppExit Success`. No coredump, OOM kill, crash, or agent lifecycle action was observed. Its connected Empty workload and zero-error observations document the accepted diagnostic cause only; they predate `96e1308a` and do not verify the strict world-camera boundary.

## Remote Interpolation Stage Boundary

Commit `538e8329` (`Gate remote interpolation at Npcs`) separates replicated target synchronization from remote visual interpolation. Connection/auth and replication receive remain active in every connected InWorld stage, and `sync_replicated_transforms` still updates `InterpolationTarget`/`RotationTarget` when replicated network state changes. `interpolate_remote_entities` now runs only from cumulative `Npcs`; `Empty`, `Character`, `Skybox`, and `Terrain` no longer mutate remote visual `Transform`.

The behavioral RED test run is recorded in `/tmp/claude/game-engine-perf/strict-empty-interpolation-red.log`: `registered_interpolation_does_not_move_remote_before_npcs` failed at `Empty` with a nonzero translation while the `Npcs` test passed. After the stage gate, `/tmp/claude/game-engine-perf/strict-empty-interpolation-green.log` records both boundary tests passing.

The live replacement comparison is recorded in `/tmp/claude/game-engine-perf/strict-empty-live-20260809.json` and `/tmp/claude/game-engine-perf/strict-empty-interp-live-20260809.json`. The pre-gate client used game-engine commit `96e1308a31940ed5c03046b574ca0b52fe15d8e2`, PID `2592665`, start ticks `182782909`, and socket `/tmp/game-engine-2592665.sock`; it reported `remote_entities=134`. The post-gate client used commit `538e83290769c70a6980ec0903db74bf2981c0fb`, PID `2960624`, start ticks `182917558`, and socket `/tmp/game-engine-2960624.sock`; it reported `remote_entities=133`. Both scene-tree captures reported zero `WowCamera` and zero `Camera3d`. Server PID `82964` remained start ticks `178818377` before and after both captures.

Across the approximately 10-second passive windows, aggregate CPU changed from `325.49%` to `288.12%`, compute-pool CPU from `229.17%` to `197.62%`, and client DRM gfx occupancy from `6.93%` to `5.60%`. The direction is consistent with removing remote interpolation, but frame/FPS direction is not acceptance evidence: runqueue delay changed from `2.389` to `4.565` seconds and the live entity count and other conditions drifted. A subsequent three-sample elevated eu-stack capture for PID `2960624` (`/tmp/claude/game-engine-perf/strict-empty-interp-eu-stack-20260809.txt`) contained no `interpolate_remote_entities` stack. Approximately `2.9` CPU cores still remain, so the Empty-stage root-cause loop continues; no final performance acceptance claim is made.

## Current In-World Performance Investigation

A separate empty-stage investigation found replicated-unit semantic NOOPs before any visual-stage conclusion: the preserved client kept 133 `RemoteEntity` entries (132 NPCs plus one local player), rewrote stable `Transform`/`Visibility` state every frame, and received server payloads caused by equal movement/gravity writes. Fixes are committed in `game-engine` `3c77d346` and `game-server` `2927382`/`ae81c65`; the corrected empty-stage relaunch confirms connectivity and no UI log flood, but supplies no comparative FPS evidence. See [[replicated-unit-noops]].

The enabled UI control measured **12.332 FPS** and **81.157 ms** from six unprofiled `game-engine-cli performance` samples with **71 remote entities** (`game-engine` `00e7b3b0`, `ui-toolkit` `5ead575`). The all-text-disabled diagnostic measured **37.415 FPS** and **26.785 ms** with **76 remote entities** (`game-engine` `6806717c`, `ui-toolkit` `43a2784`). These runs did not use identical revisions or exact workloads. The large delta strongly implicates UI-text-associated rendering with **moderate confidence**, but does not prove that text is the sole cause or identify which text stage owns it.

An equality-guard experiment in `ui-toolkit` commit `33fa74d` stopped unchanged main-text component assignments from advancing Bevy change ticks. The behavioral test passed, but the enabled-text live measurement was **11.373 FPS** and **90.230 ms** after 120 seconds, so the experiment was rejected and reverted by `36d4692`. This is not a performance fix.

The engine process has measured approximately **224–277% CPU** of one core, with about **236%** in Bevy's Compute Task Pool. The approximately **74% GPU busy** measurement is adapter-wide on shared `card1`/`renderD128`, not engine-specific. Existing callgraph evidence is dominated by wgpu buffer transitions/unmaps and related render-resource work; `bevy_sprite_render::text2d::extract_text2d_sprite` has low self-cost in that profile. Downstream causality remains unresolved: extraction can feed later GPU/resource work, but the profile does not prove that text alone owns the worker load. Profiled FPS and frame time are attribution-invalid and are never used as performance evidence.

Transient reconnect failures occurred during two fixed-binary launches, then cleared with a later unchanged logged launch that reached InWorld with 70 remote entities. Account state was healthy; no stale-session or token explanation was established. An earlier direct M2 launch ended at a Friz font parse failure; the font files were overwritten at 18:23 UTC, the pre-overwrite bytes were not preserved, and the current Friz/Arial bytes pass the exact Bevy parser. The initial paired run then used the same binary/source/options/server/token/environment, ten readiness polls, one local player, stable recorded world/material invariants, 125-second holds, and six unprofiled samples. Enabled measured **31.767 FPS / 54.177 ms**; disabled measured **36.843 FPS / 54.482 ms**. However, the first performance sample in each condition immediately followed an expensive `dump-scene` request and inherited its long diagnostic frame (**188.06 ms** enabled after **215 ms** scene latency; **210.88 ms** disabled after **266 ms** scene latency). The recorded readiness workloads also differed (**135** versus **133** remote entities). Therefore this initial comparison is **preliminary/inconclusive pending a clean repeat** with a prospective performance warm-up; it supports no M2 performance conclusion or fix. Selector/tests/flag were removed in `58e2f9c2`, then restored temporarily in `a7784e70` for the clean repeat.

The RED test failed because SSAO remained present with `Msaa::Sample4`. The exact GREEN test `sync_camera_graphics_post_process_keeps_ssao_compatible_with_anti_aliasing` passes. This removes the per-frame Bevy SSAO/MSAA error path. No runtime FPS improvement is claimed until the restarted engine is measured with the corrected configuration.

IPC screenshots are visual captures, not frame-timing measurements. The screenshot `FPS: 1.00` artifact remains a visual artifact and is not used as performance evidence.

## Demand-Driven IPC Status Refresh

Commit `abf68fd9` (`Refresh IPC status snapshots on demand`) removes unconditional InWorld status-snapshot rebuilding from every `Update`. IPC now uses three ordered sets: `Receive` drains the socket channel into `PendingIpcCommands`; `RefreshStatus` runs only the snapshot systems selected by the queued requests; `Dispatch` formats and executes those requests after refresh. Equipment IPC commands remain before `RefreshStatus`, so an export observes equipment changes in the same update.

The request dependency matrix is explicit:

| Request | Refreshed snapshot(s) |
|---|---|
| `NetworkStatus` | network |
| `TerrainStatus` | terrain |
| `SoundStatus` | sound |
| `CharacterStatsStatus` | character stats |
| `EquippedGearStatus` | equipped gear |
| `ExportCharacter` | character stats, equipped gear, equipment appearance, character roster |
| `MapPosition`, `MapTarget`, `MapWaypointAdd`, `MapWaypointClear` | map |
| Every other IPC request, including `Ping` and `Performance` | none |

Multiple queued requests coalesce these refresh flags, while command order remains FIFO for dispatch. The duplicate `sync_map_status_snapshot` registration in `game/networking/mod.rs` was removed; map status is now refreshed through the demand path, preserving waypoint/graveyard fields while updating zone and player coordinates.

Behavioral RED evidence is recorded in `/tmp/claude/game-engine-perf/status-demand-red.log` and `/tmp/claude/game-engine-perf/status-refresh-matrix-red.log`; both captured the expected pre-implementation compile boundary. GREEN evidence is recorded in `/tmp/claude/game-engine-perf/status-demand-green-2.log` and `/tmp/claude/game-engine-perf/status-refresh-matrix-green-2.log`: the idle-update/map-refresh boundary and request dependency matrix pass. Formatting evidence is recorded in `/tmp/claude/game-engine-perf/status-demand-cargo-fmt-check.log` and `status-demand-cargo-fmt-check-final.log` (empty stderr). Rust readability audit artifacts are under `/tmp/claude/game-engine-perf/status-demand-readability/`.

Rebuilt status-demand client PID `2093844` remained connected in strict Empty and reported **490.62 FPS** while consuming **369.05% of one core** over 10 seconds. Because no matched pre-change run exists and the live replicated workload changed, this does not isolate a CPU effect from demand-driven status refresh. It proves only that status cleanup did not satisfy the Empty CPU gate by itself.

## Strict Empty Diagnostic Frame Pacing

Commit `4fb2e5c9` (`Pace strict Empty stage at 10 FPS`) adds an explicit diagnostic limiter after demand-driven status cleanup. Only `GameState::InWorld` with the exact `InWorldSceneStage::Empty` selects a fixed **100 ms** interval (**10 FPS**) through the existing `First` frame-limiter boundary. `Character`, later cumulative stages, and every other game state continue using the persisted/global `GraphicsOptions` frame-rate limit; the pacing change does not alter `PresentMode` or saved graphics options. The FPS overlay, networking, and IPC remain registered. This controls frame cadence; it does not remove render or application work.

The behavioral RED compile boundary is recorded in `/tmp/claude/game-engine-perf/empty-frame-interval-red.log`. Valid GREEN evidence is `/tmp/claude/game-engine-perf/empty-frame-interval-green-2.log`; the module-wide client-options GREEN run is `/tmp/claude/game-engine-perf/empty-frame-client-options-green-2.log`. Formatting evidence is `/tmp/claude/game-engine-perf/empty-frame-cargo-fmt-check.log`; Rust readability artifacts are under `/tmp/claude/game-engine-perf/empty-frame-readability-final/`.

Before pacing, rebuilt status-demand client PID `2093844` reached `InWorld` with one link and one local player, reported **490.62 FPS**, and consumed **369.05% of one core** during the passive 10-second window. Identity/readiness evidence is `/tmp/claude/game-engine-perf/status-demand-empty-readiness.json`; CPU evidence is `/tmp/claude/game-engine-perf/status-demand-empty-cpu-2093844.json`.

The paced replacement, PID `2130439`, reported **9.98 FPS / 100.23 ms**, remained connected with one link and one local player, and retained the visible performance panel. Terrain counts were zero; scene evidence contained zero `Camera3d` and zero displayed NPCs. The passive 10-second measurement was **11.20% of one core**, so the required `<=10%` gate still failed. Evidence: `/tmp/claude/game-engine-perf/empty-10fps-launch-identity.json`, `empty-10fps-readiness.json`, `empty-10fps-cpu-2130439.json`, and `empty-10fps-2130439.webp`. Stderr had no panic, device loss, or OOM; it contained one nonfatal Lightyear missing-despawn error. The client later exited cleanly through `WindowCloseRequested` and `AppExit Success`. Alessio rejected lower frame pacing as a fix and chose to keep 10 FPS temporarily only as an investigation aid. Character remains blocked while the per-frame application/render cost is decomposed and removed.

## Strict Empty Camera/Input Update Boundary

Commit `c446d81c` (`Gate camera updates at Character stage`) adds an exact cumulative-stage guard around the chained camera/input systems. Strict `Empty` now skips `sync_camera_options`, `camera_input`, `cursor_grab`, `player_movement`, and `camera_follow`; `Character` and every later cumulative stage retain them. The standalone FPS/IPC path and networking remain separate.

Source inspection identified `player_movement` as avoidable Empty work: each dispatched frame allocated collision/pathing collections and initialized mesh-raycast setup despite `Empty` having no world `Camera3d`, terrain, or displayed character. The guard removes that setup rather than changing movement or collision behavior for `Character` and later stages.

Behavioral RED evidence is `/tmp/claude/game-engine-perf/character-stage-guard-red.log`. Focused GREEN evidence is `/tmp/claude/game-engine-perf/character-stage-guard-green.log`; module GREEN evidence is `/tmp/claude/game-engine-perf/character-stage-tests-green.log`. Formatting evidence is `/tmp/claude/game-engine-perf/character-stage-cargo-fmt-check.log`; Rust readability artifacts are under `/tmp/claude/game-engine-perf/character-stage-readability/`. The protected `src/rendering/camera/camera.rs` movement instrumentation was preserved through partial staging and is not part of the `c446d81c` source change.

Rebuilt PID `2176863` remained connected with one link/player, reported **10.01 FPS / 99.89 ms**, zero terrain, zero `Camera3d`, and zero displayed NPCs. Its passive 10-second CPU result was **11.10% of one core**, versus **11.20%** for the prior paced client; remote entities also changed from 70 to 75, so the 0.10-point difference is not accepted as a measurable improvement. Evidence: `/tmp/claude/game-engine-perf/character-camera-gate-{launch-identity,readiness}.json`, `character-camera-gate-cpu-2176863.json`, and `character-camera-gate-2176863.webp`. The 10 FPS limiter remains a temporary investigation aid, and Character remains blocked pending the `<=10%` Empty gate.

## Strict Empty FPS Frame-Time Graph Boundary

Commit `8cac2b03` disables only `FpsOverlayConfig.frame_time_graph_config` for exact strict Empty while keeping the FPS overlay enabled and its visible FPS text/config active. Empty therefore avoids the frame-time graph's per-frame diagnostic-history reads and shader-storage-buffer updates; the green frame-time graph is absent. `Character` and later stages preserve the graph. The performance panel, networking, IPC, and temporary 10 FPS pacing remain active.

The initial `8cac2b03` startup-only disablement was incomplete: later visibility writers in `apply_loaded_client_options`, `sync_hud_visibility_toggles`, and `apply_snapshot_to_world` restored the graph setting. The screenshot `/tmp/claude/game-engine-perf/empty-fps-graph-2246158.webp` visibly proves the failure with a solid red frame-time graph in strict Empty.

Commit `cc5780a8` makes all three overlay visibility paths stage-aware. Exact Empty keeps FPS text/overlay enabled while forcing the frame-time graph disabled; Character and later stages restore graph visibility. RED evidence is `/tmp/claude/game-engine-perf/empty-fps-graph-options-red.log`; GREEN evidence is `/tmp/claude/game-engine-perf/empty-fps-graph-options-green.log` and `/tmp/claude/game-engine-perf/empty-fps-graph-all-green-2.log`. Formatting evidence is `/tmp/claude/game-engine-perf/empty-fps-graph-options-cargo-fmt-check-final.log`; Rust-readability artifacts are under `/tmp/claude/game-engine-perf/empty-fps-graph-options-readability/`.

Rebuilt PID `2283621` remained connected with one link/player and 78 remote entities, reported **9.95 FPS / 100.45 ms**, and retained the FPS text with no graph in `/tmp/claude/game-engine-perf/empty-fps-graph-options-2283621.webp`. Terrain, world camera, and displayed-NPC counts remained zero. The passive 10-second process measurement was **9.80% of one core**, meeting the numerical `<=10%` threshold. Evidence: `empty-fps-graph-options-launch-identity.json`, `empty-fps-graph-options-readiness.json`, and `empty-fps-graph-options-cpu-2283621.json`. Stderr contains no panic, device loss, or OOM; one nonfatal Lightyear missing-despawn error remains. Because Alessio explicitly kept 10 FPS only as a temporary investigation aid, this numerical result is not accepted as the final Empty design and does not authorize Character advancement.

## Particle/Hanabi Empty render boundary

Commit `beead231` (`Register particles only at Particles stage`) removes `ParticlePlugin` and Hanabi registration for exact `Empty` through `Lighting`. `Particles`, `Ui`, and unconfigured normal runs retain the plugin. The prior emitter systems already had a stage run condition, but the pre-fix strict-Empty profile still sampled `bevy_hanabi::render::VfxSimulateNode::run` at **2.18% self CPU**; artifact family: `/tmp/claude/game-engine-perf/character-camera-gate-2176863.perf-*.txt`. This is source attribution from PID `2176863` before `beead231`, not a post-fix measurement.

Behavioral RED evidence is `/tmp/claude/game-engine-perf/empty-particle-plugin-red.log`; GREEN evidence is `/tmp/claude/game-engine-perf/empty-particle-plugin-green.log`. Formatting evidence is `/tmp/claude/game-engine-perf/empty-particle-plugin-cargo-fmt-check.log`; readability artifacts are under `/tmp/claude/game-engine-perf/empty-particle-plugin-readability/`.

Rebuilt PID `2297374` matched commit `beead231`, remained `InWorld` and connected with one link/player and 80 remote entities, reported **10.22 FPS / 97.85 ms**, and retained the FPS text. Terrain, world-camera, and displayed-NPC counts remained zero; screenshot: `/tmp/claude/game-engine-perf/empty-particle-plugin-2297374.webp`.

Three identity-bound passive 10-second process samples on that binary measured **12.50%**, **12.70%**, and **9.90%** of one core. The first two fail the `<=10%` gate and the third crosses it, so the result is not stable. The post-fix callgraph `/tmp/claude/game-engine-perf/empty-particle-plugin-perf-2297374.data` contained no Hanabi symbol. Its largest self-sample shares instead included M2 material-specialization parameter validation (**10.19%**), PipeWire audio conversion (**9.43%**), Lightyear link/UDP iteration (**9.36%**), and render-view preparation/scheduling. Those percentages are shares of sampled CPU cycles, not percentages of a Linux core. The profile verifies Hanabi absence during the interval but does not establish a total CPU reduction. Temporary 10 FPS pacing remains, and Character remains blocked.

## Network reset flush boundary

Commit `ce0ce2d0` (`Gate network reset flush until due`) adds a due predicate before the exclusive `flush_pending_network_world_reset` system. Frames with no pending reset or a not-yet-reached target now skip the exclusive system; the due path preserves the earliest target, one-frame deferral, exactly-once reset, and existing reset contents.

The final pre-fix Empty PID `2390217` profile sampled the flush wrapper at **16.12%**. Because that interval may include a real one-time startup reset, it did not establish steady-state CPU savings. RED evidence is `/tmp/claude/game-engine-perf/network-reset-due-gate-red.log`; GREEN is `network-reset-due-gate-green.log`; formatting is `network-reset-due-gate-cargo-fmt.log`; Rust-readability artifacts are under `network-reset-due-gate-readability/`.

Post-fix PID `2402583` remained focused, `InWorld`, and connected with one link/player; it retained FPS text and zero terrain, `Camera3d`, or displayed NPCs. Its steady profile contained neither `flush_pending_network_world_reset` nor `network_world_reset_is_due`. Twelve passive windows after the same 30-second warm-up measured **12.05% mean**, **11.75% median**, and **14.00% maximum** CPU; **0/12** met `<=10.0%`. The top post-fix samples moved to general Bevy scheduling, PipeWire, Vulkan/render-pass encoding, Lightyear netcode, SSAO query maintenance, and text/UI work. The due gate is behaviorally valid but the performance hypothesis is rejected; Character remains blocked.

## Initial Reconnect Disconnect Marker

Lightyear `NetcodeClient` requires an initial `Disconnected { reason: None }` marker. Before `0d215316`, the InWorld disconnect observer treated that required marker as a real connection loss while `GameState::InWorld` and `ReconnectPhase::PendingConnect`; it queued another reset and replaced the client entity and client ID about every 100 ms before handshake. The resulting storm produced thousands of failed reconnect attempts and prevented any live Who verification.

Commit `0d215316` ignores only a reasonless marker with no forced-disconnect notice during `PendingConnect`. Reasoned pending failures, connected disconnects, forced disconnects, auth/token and character-selection behavior, world-reset behavior, and retry handling remain unchanged. RED evidence is `/tmp/claude/game-engine/reconnect-initial-marker-red.log`; GREEN evidence is `/tmp/claude/game-engine/reconnect-initial-marker-green.log` (**14 passed**); formatting is `/tmp/claude/game-engine/reconnect-initial-marker-fmt.log`; readability artifacts are `/tmp/claude/game-engine/reconnect-initial-marker-readability.json` and `/tmp/claude/game-engine/reconnect-initial-marker-readability-metrics/`. HEAD `0d215316` is rebased on `origin/master` `e09944e9` with 85 local commits. This is source/test proof only; no rebuilt live-runtime completion claim exists yet.

## Audio backend Empty boundary

Commit `463e9e47` (`Disable audio backend without sound flag`) makes no-sound mode omit Bevy's `AudioPlugin`, which `DefaultPlugins` previously registered even when project `SoundPlugin` was disabled. `--sound` retains Bevy audio and project `SoundPlugin` exactly. Outside `src/sound/`, only an optional `AudioSink` status query and optional `SoundSettings` exist; neither requires `AudioPlugin`.

The pre-fix strict-Empty PID `2402583` profile sampled PipeWire audio conversion at **7.61%** plus CPAL/ALSA output-thread work. RED evidence is `/tmp/claude/game-engine-perf/audio-plugin-registration-red.log`; GREEN is `audio-plugin-registration-green-3.log`; formatting is `audio-plugin-registration-cargo-fmt-3.log`; readability artifacts are under `audio-plugin-registration-readability/`.

Post-fix PID `2468254` remained focused, `InWorld`, and connected with one link/player and 81 remote entities; it retained FPS text and zero terrain, `Camera3d`, or displayed NPCs. No audio backend thread or PipeWire/CPAL/ALSA profile symbol remained. Twelve passive windows after the same 30-second warm-up measured **9.61% mean**, **9.55% median**, and **10.60% maximum** CPU; **8/12** met `<=10.0%`. The next profile's largest project-owned idle sample was `PvpRuntimeState` parameter validation at **11.76%** of sampled CPU, while motion-blur query and UI extraction were larger framework samples. Audio removal materially lowered the mean, but the strict all-window CPU gate and Character advancement remain blocked.

## Who runtime idle change boundary

Commit `2c265ffa` (`Avoid dirtying idle Who runtime`) fixes a client-side no-op in `send_pending_queries`. The old `while let Some(query) = runtime.pending_queries.pop_front()` called `pop_front()` on an empty queue every `Update`; the mutable dereference advanced `WhoRuntimeState` change ticks despite no query, send, reply, or snapshot work. A read-only empty-queue guard now returns before the destructive loop.

The guard preserves FIFO query sending, the immediate `who is unavailable: not connected` reply, inbound `WhoStateUpdate` handling, reconnect/reset cleanup, and later-stage behavior. RED evidence is `/tmp/claude/game-engine-perf/who-idle-change-tick-red.log`; GREEN evidence is `/tmp/claude/game-engine-perf/who-idle-change-tick-green.log`; `cargo fmt` passed, and Rust-readability evidence is under `/tmp/claude/game-engine-perf/who-idle-change-tick-readability/`. This correction records the root cause and behavioral preservation only: it makes no CPU-savings or Character-readiness claim before runtime measurement.

## Strict Empty Gizmo Registration Boundary

Commit `1503cd1c` (`Disable gizmos in strict Empty`) makes the fixed diagnostic stage explicit at Bevy plugin registration. Exact `InWorldSceneStage::Empty` disables both `bevy::gizmos::GizmoPlugin` and `bevy::gizmos_render::GizmoRenderPlugin`; the rest of the application remains registered. Unconfigured/default runs, `Character` and later cumulative stages, debug modes, and screenshot/default paths retain gizmos. Source inspection found no project gizmo consumers, so this boundary removes registration/render work without changing project behavior.

The preceding identity-bound profiler was PID `2655273`. Its exact `perf` interval was **59.950 seconds**, with **2,153 samples** and **0 lost samples**. `GizmoBuffer<LightGizmoConfigGroup>::queue` was the strongest current project-retained target at **1.51% sampled CPU**. This is a share of sampled cycles, not 1.51% of one Linux core, and it identifies a target rather than proving causal savings.

RED evidence is `/tmp/claude/game-engine-perf/empty-gizmo-registration-red.log`; GREEN evidence is `/tmp/claude/game-engine-perf/empty-gizmo-registration-green.log`; Rust-readability evidence is under `/tmp/claude/game-engine-perf/empty-gizmo-registration-readability/`. The exact profiler artifacts are `/tmp/claude/game-engine-perf/final-polling-loops-perf-2655273.data`, `final-polling-loops-perf-2655273-header.txt`, `final-polling-loops-perf-2655273-record.log`, `final-polling-loops-perf-2655273-self.txt`, `final-polling-loops-perf-2655273-children.txt`, and `final-polling-loops-perf-2655273-script.txt`.

Rebuilt candidate PID `2846177` (SHA `84f6ecc9590979a2fba498b8206549d33ebff7fdc27878a16331acccc19811da`) completed the A/B proof. After a 30-second warm-up, **10/12** passive windows passed, with **9.47% mean** and **10.60% maximum** CPU versus baseline PID `2655273` at **8/12**, **9.97% mean**, and **10.90% maximum**. The passive mean improved by **0.50 percentage points**. A 60-second profile collected **1,936 samples** with **0 lost samples** and no `Gizmo` symbol, confirming the targeted path disappeared. Commit `1503cd1c` is retained; the final `<=10.0%` every-window gate remains open.

Readiness held one client/server with healthy IPC/admin ping, approximately **10.09 FPS**, zero displayed NPCs/cameras/terrain, no panic/device-loss/OOM, and no Character advancement. One startup `Received despawn for an entity that does not exist` warning was recorded without blocking readiness.

## M2 Effect Material Empty registration boundary

Commit `0a1a1bfb` omitted the full `M2EffectMaterialPlugin` for exact `Empty` and unintentionally removed the lightweight `Assets<M2EffectMaterial>` resource required by active consumers. PID `2339740` panicked in scene setup before IPC readiness; PID `2365242` later panicked in `sync_equipment`. Their stale sockets were removed only after identity-safe verification. These panic logs are failure evidence, not successful runtime proof.

Commit `49304144` (`Skip inactive scene setup validation`) attempted optional-resource and run-condition compatibility but was incomplete and is forward-reverted by `e7f98704` (`Keep Empty M2 assets without render plugin`). The final architecture initializes only lightweight `Assets<M2EffectMaterial>` for exact `Empty`, omitting Bevy `EntitiesNeedingSpecialization` and material/render schedules. `Character` and later stages, plus unconfigured and debug runs, retain the full plugin.

Corrective RED evidence: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-red.log`; GREEN: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-green.log`; formatting: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-cargo-fmt.log`; Rust readability: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-readability/`. Earlier panic artifacts remain failure evidence: `/tmp/claude/game-engine-perf/empty-scene-setup-validation-red.log` and `/tmp/claude/game-engine-perf/empty-scene-setup-validation-red-3.log`.

Final PID `2390217` matched `e7f98704`, reached `InWorld`, and remained connected with one link/player and 71 remote entities. It reported **10.04 FPS / 99.60 ms**, retained the FPS text, and had zero terrain, `Camera3d`, or displayed NPCs. Screenshot: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-2390217.webp`. Stderr contained no panic, device loss, or OOM.

After a prospective 30-second warm-up, twelve contiguous passive 10-second windows measured a **10.24% mean**, **10.20% median**, and **10.60% maximum** of one core; only **2/12** windows met the predeclared `<=10.0%` threshold. Artifact: `/tmp/claude/game-engine-perf/empty-m2-asset-specialization-cpu-stability.json`. The post-fix profile contained no Hanabi, `M2EffectMaterial`, or M2 `EntitiesNeedingSpecialization` symbol. Its largest self-sample shares instead included network-reset flushing (**16.12%**), Wayland marshaling (**14.78%**), byte-buffer release (**12.09%**), PBR morph-uniform parameter work (**7.10%**), and pending character-roster refresh (**5.40%**). These are sampled CPU-cycle shares, not percentages of a Linux core. The M2 specialization boundary is verified, but the overall Empty CPU gate still fails; temporary 10 FPS pacing and the Character block remain.

## Sources

- [rendering-pipeline](../systems/rendering-pipeline.md) — pipeline summary and known performance history
- [networking](../systems/networking.md) — Who query runtime and Empty-stage performance boundaries
- `src/who.rs` — Who runtime queue/send/receive/reset behavior
- `game-engine` commit `2c265ffa` — idle Who change-tick guard
- `/tmp/claude/game-engine-perf/who-idle-change-tick-red.log` and `who-idle-change-tick-green.log` — behavioral RED/GREEN evidence; `cargo fmt` passed and readability evidence is under `who-idle-change-tick-readability/`
- [game-engine app setup](../../src/app_setup.rs) — strict-Empty gizmo disablement and diagnostic plugin registration
- [game-engine main](../../src/main.rs) — stage-aware gizmo policy wiring and screenshot/default retention
- `game-engine` commit `1503cd1c` — disable Bevy gizmo plugins only for explicit Empty
- `/tmp/claude/game-engine-perf/empty-gizmo-cpu-2846177.json` and rebuilt 60-second perf artifacts — accepted A/B proof; final every-window CPU gate remains open
- `/tmp/claude/game-engine-perf/empty-gizmo-registration-red.log`, `empty-gizmo-registration-green.log`, and `empty-gizmo-registration-readability/` — gizmo registration RED/GREEN/readability evidence
- `/tmp/claude/game-engine-perf/final-polling-loops-perf-2655273.data` and companion header/self/children/script artifacts — 59.950-second PID 2655273 profiler evidence
- [game-engine networking](../../src/game/networking/mod.rs) — connection and transport lifecycle evidence
- [game-engine disconnect handling](../../src/game/networking/disconnect.rs) — reconnect/reset behavior
- `ui-toolkit/src/plugin.rs` — `UiProcessingEnabled` full-schedule gate, inner render/text gates, and pause/re-enable test
- [game-engine UI stage gates](../../../src/app_setup.rs) — pre-`Ui` plugin registration boundary
- [in-world stage predicates](../../../src/game/state/inworld_scene_stage.rs) — cumulative stage ordering
- `game-engine` commit `508891a6` — stop game-UI work before the `Ui` stage
- `game-engine` commit `0d215316` — ignore the initial reconnect disconnect marker
- `ui-toolkit` commit `50e4a17` — gate the complete toolkit UI update chain
- `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` — connected empty-stage relaunch proof and retained live-client state
- `/tmp/claude/game-engine-perf/ui-enabled-systemd-control-stable.json` — enabled unprofiled control
- `/tmp/claude/game-engine-perf/ui-text-disabled-unprofiled.json` — all-text-disabled unprofiled diagnostic
- `/tmp/claude/game-engine-perf/ui-text-fix-logged-observation.json` — rejected equality-guard measurement and logged reconnect observation
- `/tmp/claude/game-engine-perf/main-render-16k-self.txt` — calibrated profiler attribution; profiled FPS excluded
- `/tmp/claude/game-engine-perf/m2-uv-{enabled,disabled,comparison}.json` — preliminary direct M2 UV pair; first samples were contaminated by the preceding `dump-scene`, and remote-entity workloads differed; clean repeat pending
- [cloud_texture.rs](../../src/rendering/skybox/cloud_texture.rs) — texture dimensions, startup generation, and simplex implementation
- [skybox/mod.rs](../../src/rendering/skybox/mod.rs) — runtime regeneration removal and sky update registration
- [sky.wgsl](../../assets/shaders/sky.wgsl) — time/cloud-parameter UV scrolling
- [game-engine-cli command dispatch](../../src/bin/game-engine-cli/command_dispatch.rs) — IPC screenshot capture and performance commands
- [client options](../../src/game/state/client_options.rs) — VSync setting and presentation-mode selection
- [camera post-process](../../src/rendering/camera/camera_post_process.rs) — anti-aliasing synchronization and SSAO compatibility
- [camera post-process tests](../../tests/unit/camera_post_process_tests.rs) — RED/GREEN compatibility behavior
- [world environment](../../../src/game/state/game_state.rs) — strict Empty world-camera spawn boundary
- [world environment tests](../../../tests/unit/main_tests.rs) — Empty has no world camera; Character retains one across re-entry
- [IPC plugin](../../../src/ipc/plugin.rs) — Receive/RefreshStatus/Dispatch sets, pending commands, and request dependency matrix
- [status synchronization](../../../src/status_sync.rs) — demand-gated snapshot systems and map refresh
- [game networking registration](../../../src/game/networking/mod.rs) — duplicate map snapshot registration removal
- [camera systems](../../../src/rendering/camera/camera.rs) — camera/input/player movement/follow systems and preserved movement instrumentation
- [in-world stage predicates](../../../src/game/state/inworld_scene_stage.rs) — Character-stage guard and cumulative-stage behavior
- `/tmp/claude/game-engine-perf/character-stage-guard-red.log`, `character-stage-guard-green.log`, and `character-stage-tests-green.log` — behavioral RED/GREEN evidence
- `/tmp/claude/game-engine-perf/character-stage-cargo-fmt-check.log` and `character-stage-readability/` — formatting/readability evidence
- `/tmp/claude/game-engine-perf/status-demand-red.log` and `status-refresh-matrix-red.log` — RED compile-boundary evidence
- `/tmp/claude/game-engine-perf/status-demand-green-2.log` and `status-refresh-matrix-green-2.log` — GREEN behavioral evidence
- `/tmp/claude/game-engine-perf/status-demand-cargo-fmt-check-final.log` — formatting check evidence
- `/tmp/claude/game-engine-perf/status-demand-readability/` — Rust readability audit artifacts

## See Also

- [[rendering-pipeline]] — skybox and known rendering bottlenecks
- [[skybox]] — authored and procedural sky composition
- [[replicated-unit-noops]] — separate empty-stage networking/ECS no-op workload
- [[ui-system]] — game-UI stage gates and toolkit processing boundary
