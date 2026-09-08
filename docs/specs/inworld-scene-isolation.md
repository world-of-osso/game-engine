# InWorld scene isolation

InWorld scene isolation provides cumulative rendering stages for controlled diagnosis. Stage ordering lives in `src/game/state/inworld_scene_stage.rs`; implementation details and measured investigations are documented in [procedural cloud regeneration](../wiki/investigations/procedural-cloud-regeneration.md).

## What it must do

### Stage defaults

- [x] Preserve the full existing scene when no `InWorldSceneStage` resource is configured.
- [x] Keep stages cumulative from `Empty` through `Ui`.
- [x] Provide opt-in `--inworld-stage no-npcs-ui` for settled-FPS diagnosis: omit NPC/remote-player visuals, nameplate creation/update, and game UI processing/rendering while preserving the full scene's local-character policy, terrain, skybox, lighting, particles, camera effects, networking, and standalone FPS overlay. Remote replicated state remains available; this is not a server-side NPC disable.
- [x] Keep this diagnostic separate from `terrain`, which also excludes lighting and particles. Remove the selector when this isolation experiment is retired.

### Empty registration boundary

Empty is a startup-only diagnostic; returning to the full rendering pipeline requires restarting without that stage.

- [ ] `Empty` omits PBR and its light provider together; it must not retain consumers whose provider resources were removed.
- [ ] Empty terrain, water, M2, and sky material stores remain available for data/status access without registering their PBR material pipelines.
- [ ] Target-circle visual systems are not registered in Empty; target data resources remain available.
- [ ] Empty must not impose its own frame-rate cap. Only the user's configured global frame limiter applies; disabling that option leaves Empty uncapped.
- [ ] Non-Empty stages retain their normal rendering plugins. Camera, UI overlay, and renderer progress must remain available in Empty.

### MSAA-only control

- [x] Accept opt-in `--no-msaa`: override configured 4x MSAA to single-sample rendering in the camera render bundle, without changing the saved graphics option.
- [x] Keep SSAO compatibility based on the original configured anti-alias mode so this diagnostic does not enable SSAO. Preserve TAA when independently configured, depth/normal prepasses, common camera effects, and lighting.
- [x] Keep the composited UI camera's MSAA synchronized with the active 3D camera after graphics updates, including diagnostic restoration and preserved pre-Lighting sampling. Preserve UI clear/order behavior and readable changing FPS digits; do not apply 3D post-processing to the UI camera.

### Pipelined-rendering control

- [x] Accept opt-in `--no-pipelined-rendering`: omit Bevy's pipelined-rendering plugin while retaining the render app and its rendering work. Execute render-app frames sequentially on the calling thread rather than via the separate render-thread handoff.
- [x] Preserve normal pipelining without the flag and retain scene/camera/FPS-overlay settings. Report the selected diagnostic mode. Compare CPU time and FPS together because removing overlap can change throughput.

### Timed exact-name callback removal

- [x] Accept repeatable `--remove-system-after <main:SCHEDULE|render:SCHEDULE> <EXACT_SYSTEM_NAME> <SECONDS>`. Preserve names exactly, sort requests by deadline within each safe execution phase, and reject invalid owners, empty names, missing fields, non-unsigned seconds, and duplicate targets. Do not interpret the three values as asset paths.
- [x] Resolve the registered schedule and callback by exact runtime names. Require a unique callback and a single-member implicit type set; remove only that set with `RemoveSystemsOnly`, never a broader explicit group. Missing or ambiguous selection must fail before target removal.
- [x] Use extraction as the safe barrier for main-world and ordinary render-schedule removals. Defer `render:ExtractSchedule` until extraction completes, then remove it in Render cleanup. Preserve main-world and schedule registries, including consecutive removals without `MainWorld` present during cleanup.
- [x] Register no controller without requests. Report each actual removal with time, world, schedule, and name. The six former dedicated callback-removal flags are rejected with no compatibility aliases. Other visual/terrain diagnostics remain unchanged.

#### Retired-flag migration

| Retired flag | Current exact-name request |
|---|---|
| `--freeze-indirect-parameters-after <SECONDS>` | `--remove-system-after render:Render bevy_render::batching::gpu_preprocessing::write_indirect_parameters_buffers <SECONDS>` |
| `--freeze-batched-instances-after <SECONDS>` | `--remove-system-after render:Render bevy_render::batching::gpu_preprocessing::write_batched_instance_buffers<bevy_pbr::render::mesh::MeshPipeline> <SECONDS>` |
| `--freeze-gpu-clusters-after <SECONDS>` | `--remove-system-after render:Render bevy_pbr::cluster::gpu::prepare_clusters_for_gpu_clustering <SECONDS>` |
| `--freeze-mesh-collection-after <SECONDS>` | `--remove-system-after render:Render bevy_pbr::render::mesh::collect_meshes_for_gpu_building <SECONDS>` |
| `--freeze-camera-follow-after <SECONDS>` | `--remove-system-after main:Update game_engine::rendering::camera::camera_follow::camera_follow <SECONDS>` |
| `--freeze-message-send-after <SECONDS>` | `--remove-system-after main:PostUpdate MessagePlugin::send <SECONDS>` |
- [ ] Use after initial loading in a stationary scene. Removing a callback can freeze data or change downstream work; it does not prove that work unnecessary. Preserve view, connection, focus, and comparable throughput for attribution. A delayed frame can make multiple deadlines due; reject such an interval as a one-change comparison. The cutoff includes measurement time, not additional FPS stabilization.

### Named-span CPU attribution

- [ ] With the diagnostic-only `cpu-system-profile` feature and `WOO_CPU_PROFILE_OUTPUT` set, capture a bounded five-second sample starting ten seconds after profiler setup and write aggregate JSON. Without the environment variable, install no CPU-accounting layer; normal builds do not include it.
- [x] Report calls, inclusive thread CPU, and nested-span-exclusive thread CPU for named systems, schedules, and executor spans. Account independently for each thread and span entry; exclude blocked time and never subtract CPU from another thread. `observed_cpu_ns` covers only the first-to-last captured selected-span clock observations, not the entire five-second window; threads without observations are absent. The boundary counter counts entries inside the window that exit afterward.
- [ ] Preserve rendering, worker counts, CPU availability, and frame-rate policy. Fail explicitly on clock or output errors. Treat results as instrumented attribution, not a normal-build performance comparison; uninstrumented task work and profiling overhead limit coverage.

### Camera-follow diagnostic

- [x] Selecting only the camera-follow callback retains its last transform while unrelated systems remain registered. Use only in a stationary scene; this is not a movement or camera-control optimization.

### Application-message sender diagnostic

- [x] Selecting only PostUpdate `MessagePlugin::send` retains unrelated send-group members; queued application payloads stop draining without removing the whole group.
- [ ] Use only after login in a stationary client. Outgoing application messages remain queued and may accumulate; do not interact with gameplay while frozen. Verify connection health and incoming synchronization throughout the comparison; disconnects or changed workloads invalidate CPU/FPS attribution. This is not a networking optimization.

### Directional-shadow diagnostic

- [x] Accept opt-in `--no-directional-shadows` for the InWorld world-environment directional light. Set only `DirectionalLight.shadow_maps_enabled` false; retain the light entity, illuminance, transform, ambient light, cascade configuration, shadow-map resource, camera effects, and normal lighting.
- [x] Preserve enabled directional shadows without the option. Do not affect standalone or other scene setup paths.

### Frame-time-graph control

- [x] Accept opt-in `--no-frame-time-graph`: keep numeric FPS visibility under the existing HUD preference, but hide the frame-time graph and stop its per-frame shader-buffer updates.
- [x] Keep the override effective after loaded-option/HUD/menu updates. Do not persist it to user settings or disable unrelated systems.
- [x] Retain the graph plugin/material/hidden node registration; this skips graph drawing and update work, not all Bevy UI infrastructure.

### Terrain-rendering-off control

- [x] Accept opt-in `--no-terrain-meshes`: retain each streamed tile's logical root and height/streaming data, but create no ground-terrain meshes or their materials/images. Preserve other rendering settings and isolation flags.
- [x] Keep initial loading completion, tile lifetime/unloading, and player/camera height state functional. Background parsing and height queries remain; this is a settled rendering-workload control, not removal of all terrain CPU code.

### Untextured-terrain diagnostic

- [x] Accept opt-in `--no-terrain-textures`: render streamed terrain with a flat, lit material shared by that tile's chunks without terrain texture bindings. Preserve chunk geometry, culling metadata, lighting/camera settings, and other isolation flags.
- [x] Skip terrain image registration and custom terrain-material creation on this path. Background terrain/texture parsing remains unchanged; this is not an asset-loading benchmark.
- [x] Record that this also bypasses the custom multilayer terrain shader and its per-frame material updates. Results must not be attributed to texture sampling alone. Default textured behavior remains unchanged.

### Water and skybox diagnostics

- [x] Accept opt-in `--no-terrain-water`: omit streamed ADT water surfaces and water-height registration while preserving terrain geometry/textures. Combine with `--no-terrain-objects` to exclude WMO liquids too.
- [x] Accept opt-in `--no-skybox`: suppress sky-dome entities and authored InWorld skybox visual loading/updates. Preserve the camera, game-time progression/controls, directional/ambient lighting, fog, and environment-map lighting.
- [x] Preserve normal behavior without these flags. These are visual-isolation controls, not a minimal renderer: shared sky/cloud resources and empty material plugins may remain initialized.

### Terrain-object diagnostic

- [x] Accept opt-in `--no-terrain-objects`: skip `_obj*` companion parsing, doodad/WMO preloads, object spawning, and later object-LOD swaps. Preserve root terrain, `_tex0` materials, water, heightmap registration, networking, terrain streaming, lighting, particles, and non-object scene behavior.
- [x] Without the option, preserve existing object parsing/preload/spawn behavior. This temporary control changes the playable world and collision coverage; use only for the settled terrain-only FPS comparison. Remove it after the experiment.

### Terrain-material diagnostic

- [x] Accept opt-in `--freeze-terrain-materials-after <SECONDS>` (unsigned integer): after that startup real-time deadline, stop terrain animation-time and environment-map material updates together. Preserve the material assets, last values, terrain drawing, and all other material systems.
- [x] Report the first frozen update with elapsed time, configured deadline, and material count. Without the option, preserve existing updates.
- [x] Use only for a settled-scene comparison with terrain already loaded before the deadline. Animated terrain textures and environment-map synchronization remain frozen afterward; newly loaded tiles are not part of this diagnostic. Remove the selector when the experiment is retired.

### Plugin registration

- [x] In exact configured `InWorldSceneStage::Empty`, disable `GizmoPlugin` and `GizmoRenderPlugin`.
- [x] Keep `LightPlugin` registered in Empty as a PBR resource provider: `PointLightShadowMap` and scattering-medium assets are required even with no `Light` entities.
- [x] Preserve LightPlugin, gizmos, and their render behavior for unconfigured/default runs, `Character` and later stages, debug modes, and screenshot/default paths.

### Camera rendering

- [x] Remove TAA, SSAO, depth/normal/motion prepasses, temporal jitter, and mip bias from `WowCamera` before `Lighting`.
- [x] Restore graphics-option-driven TAA/SSAO and required prepasses when the stage advances to `Lighting`.
- [x] Restore depth and normal prepasses with MSAA at `Lighting` without enabling TAA or SSAO.
- [x] Preserve camera identity, transforms, MSAA state before `Lighting`, tonemapping, shadow filtering, spatial audio, bloom, sharpening, and depth-of-field synchronization.
- [x] Keep the standalone performance overlay active while game UI and early camera rendering are isolated.

### InWorld environment lighting

- [x] Active `WowCamera` entities in InWorld receive generated environment lighting when neither generated nor baked environment lighting is already present, using current interpolated sky colors at the current game time.
- [x] Late camera activation initializes lighting once; unchanged cameras do not allocate replacement cubemaps. Existing environment overrides and unrelated cameras remain untouched.
- [x] Initialization requires the Lighting stage, but not skybox visuals. Global ambient remains zero and initialization does not add fog.

Exposure, shadows, and existing fog update behavior are outside this initialization change.

## How it works

- [InWorld performance investigation](../wiki/investigations/procedural-cloud-regeneration.md)
- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [UI system](../wiki/systems/ui-system.md)

## Implementation inventory

- `src/game/state/inworld_scene_stage.rs` — cumulative stage ordering and predicates.
- `src/rendering/camera/camera_post_process.rs` — stage-aware WowCamera render-bundle synchronization.
- `src/main.rs` — startup stage selection and pre-UI processing gates.
- `src/app_setup.rs` — exact-Empty gizmo-plugin boundary, required LightPlugin provider registration, and opt-in profiling-layer installation.
- `src/cpu_system_profile.rs` — bounded named-span thread-CPU accounting and JSON output.
- `src/system_isolation.rs` — shared exact-name removal and safe main/render/extraction phases.
- `src/system_isolation/args.rs` — repeatable selector parsing and retired-flag rejection.
- `src/rendering/terrain/terrain{,_background_parse,_streaming,_spawn}.rs` — streamed object/water loading and flat-material controls.
- `src/rendering/skybox/mod.rs` — skybox-visual gates, independent InWorld camera IBL initialization, and dome removal with lighting retained.

## Tests asserting this spec

- `tests/unit/camera_post_process_tests.rs` — pre-Lighting removal, Lighting restoration, MSAA behavior, default behavior, and preserved common camera effects.
- `tests/unit/main_tests.rs` — stage parsing, default/full-scene behavior, UI processing gates, and performance-overlay survival.
- `tests/unit/pipeline_isolation_tests.rs` — render-frame delivery and calling-thread versus render-thread execution through the pipelining selector.
- `src/cpu_system_profile.rs` tests — nested, boundary, independent-thread, concurrent same-span, and blocked-sleep CPU accounting.
- `tests/unit/system_isolation_tests.rs` — cross-world deadlines, extraction cleanup, retained pose/payloads, exact identity, and defaults.
- `src/system_isolation/args.rs` tests — argument validation, retired flags, duplicate targets, and ordering.

- `src/rendering/skybox/tests/inworld_ibl.rs` — current-color cubemap initialization, late activation/idempotence, disabled visuals, override preservation, and state/stage guards.

## Known gaps (current cycle)

- [ ] Replace the preserved diagnostic client with the permanent stage-aware Empty build after explicit lifecycle permission.
- [ ] Re-run the structured Empty human gate before advancing to `Character`.

## Out of scope

- Globally disabling camera post-processing in normal full gameplay.
- Changing unconfigured/default tonemapping, MSAA selection, shadow filtering, window behavior, networking, IPC, or UI rendering.
- Advancing to later cumulative stages before Empty is accepted.
