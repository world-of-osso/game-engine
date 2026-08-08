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

## Current In-World Performance Investigation

The enabled UI control measured **12.332 FPS** and **81.157 ms** from six unprofiled `game-engine-cli performance` samples with **71 remote entities** (`game-engine` `00e7b3b0`, `ui-toolkit` `5ead575`). The all-text-disabled diagnostic measured **37.415 FPS** and **26.785 ms** with **76 remote entities** (`game-engine` `6806717c`, `ui-toolkit` `43a2784`). These runs did not use identical revisions or exact workloads. The large delta strongly implicates UI-text-associated rendering with **moderate confidence**, but does not prove that text is the sole cause or identify which text stage owns it.

An equality-guard experiment in `ui-toolkit` commit `33fa74d` stopped unchanged main-text component assignments from advancing Bevy change ticks. The behavioral test passed, but the enabled-text live measurement was **11.373 FPS** and **90.230 ms** after 120 seconds, so the experiment was rejected and reverted by `36d4692`. This is not a performance fix.

The engine process has measured approximately **224–277% CPU** of one core, with about **236%** in Bevy's Compute Task Pool. The approximately **74% GPU busy** measurement is adapter-wide on shared `card1`/`renderD128`, not engine-specific. Existing callgraph evidence is dominated by wgpu buffer transitions/unmaps and related render-resource work; `bevy_sprite_render::text2d::extract_text2d_sprite` has low self-cost in that profile. Downstream causality remains unresolved: extraction can feed later GPU/resource work, but the profile does not prove that text alone owns the worker load. Profiled FPS and frame time are attribution-invalid and are never used as performance evidence.

Transient reconnect failures occurred during two fixed-binary launches, then cleared with a later unchanged logged launch that reached InWorld with 70 remote entities. Account state was healthy; no stale-session or token explanation was established. The temporary M2 UV-update selector reached a valid paired direct run after restoring zero-byte local UI-toolkit font files from the existing cache. Both conditions used the same binary/source/options/server/token/environment, ten readiness polls, one local player, stable world/material invariants, 125-second holds, and six unprofiled samples. Enabled measured **31.767 FPS / 54.177 ms**; disabled measured **36.843 FPS / 54.482 ms**. Disabling M2 UV updates raised FPS but worsened frame time by **0.305 ms**, so the required directional criterion failed and the hypothesis is **rejected**. Selector/tests/flag were removed in `58e2f9c2`; no production fix is claimed.

The RED test failed because SSAO remained present with `Msaa::Sample4`. The exact GREEN test `sync_camera_graphics_post_process_keeps_ssao_compatible_with_anti_aliasing` passes. This removes the per-frame Bevy SSAO/MSAA error path. No runtime FPS improvement is claimed until the restarted engine is measured with the corrected configuration.

IPC screenshots are visual captures, not frame-timing measurements. The screenshot `FPS: 1.00` artifact remains a visual artifact and is not used as performance evidence.

## Sources

- [rendering-pipeline](../systems/rendering-pipeline.md) — pipeline summary and known performance history
- [game-engine app setup](../../src/app_setup.rs) — UI plugin registration and diagnostic override cleanup
- [game-engine networking](../../src/game/networking/mod.rs) — connection and transport lifecycle evidence
- [game-engine disconnect handling](../../src/game/networking/disconnect.rs) — reconnect/reset behavior
- `ui-toolkit/src/plugin.rs` — text/UI render diagnostic resources and current shadow-only GREEN test
- `/tmp/claude/game-engine-perf/ui-enabled-systemd-control-stable.json` — enabled unprofiled control
- `/tmp/claude/game-engine-perf/ui-text-disabled-unprofiled.json` — all-text-disabled unprofiled diagnostic
- `/tmp/claude/game-engine-perf/ui-text-fix-logged-observation.json` — rejected equality-guard measurement and logged reconnect observation
- `/tmp/claude/game-engine-perf/main-render-16k-self.txt` — calibrated profiler attribution; profiled FPS excluded
- `/tmp/claude/game-engine-perf/m2-uv-{enabled,disabled,comparison}.json` — valid direct paired M2 UV measurement and rejection
- [cloud_texture.rs](../../src/rendering/skybox/cloud_texture.rs) — texture dimensions, startup generation, and simplex implementation
- [skybox/mod.rs](../../src/rendering/skybox/mod.rs) — runtime regeneration removal and sky update registration
- [sky.wgsl](../../assets/shaders/sky.wgsl) — time/cloud-parameter UV scrolling
- [game-engine-cli command dispatch](../../src/bin/game-engine-cli/command_dispatch.rs) — IPC screenshot capture and performance commands
- [client options](../../src/game/state/client_options.rs) — VSync setting and presentation-mode selection
- [camera post-process](../../src/rendering/camera/camera_post_process.rs) — anti-aliasing synchronization and SSAO compatibility
- [camera post-process tests](../../tests/unit/camera_post_process_tests.rs) — RED/GREEN compatibility behavior

## See Also

- [[rendering-pipeline]] — skybox and known rendering bottlenecks
- [[skybox]] — authored and procedural sky composition
