# Wiki Log

Chronological record of wiki operations.

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
