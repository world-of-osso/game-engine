# InWorld scene isolation

InWorld scene isolation provides cumulative rendering stages for controlled diagnosis. Stage ordering lives in `src/game/state/inworld_scene_stage.rs`; implementation details and measured investigations are documented in [procedural cloud regeneration](../wiki/investigations/procedural-cloud-regeneration.md).

## What it must do

### Stage defaults

- [x] Preserve the full existing scene when no `InWorldSceneStage` resource is configured.
- [x] Keep stages cumulative from `Empty` through `Ui`.
- [x] Provide opt-in `--inworld-stage no-npcs-ui` for settled-FPS diagnosis: omit NPC/remote-player visuals, nameplate creation/update, and game UI processing/rendering while preserving the full scene's local-character policy, terrain, skybox, lighting, particles, camera effects, networking, and standalone FPS overlay. Remote replicated state remains available; this is not a server-side NPC disable.
- [x] Keep this diagnostic separate from `terrain`, which also excludes lighting and particles. Remove the selector when this isolation experiment is retired.

### MSAA-only control

- [x] Accept opt-in `--no-msaa`: override configured 4x MSAA to single-sample rendering in the camera render bundle, without changing the saved graphics option.
- [x] Keep SSAO compatibility based on the original configured anti-alias mode so this diagnostic does not enable SSAO. Preserve TAA when independently configured, depth/normal prepasses, common camera effects, and lighting.
- [x] Keep the composited UI camera's MSAA synchronized with the active 3D camera after graphics updates, including diagnostic restoration and preserved pre-Lighting sampling. Preserve UI clear/order behavior and readable changing FPS digits; do not apply 3D post-processing to the UI camera.

### Pipelined-rendering control

- [x] Accept opt-in `--no-pipelined-rendering`: omit Bevy's pipelined-rendering plugin while retaining the render app and its rendering work. Execute render-app frames sequentially on the calling thread rather than via the separate render-thread handoff.
- [x] Preserve normal pipelining without the flag and retain scene/camera/FPS-overlay settings. Report the selected diagnostic mode. Compare CPU time and FPS together because removing overlap can change throughput.

### Render upload controls

- [x] Accept opt-in `--freeze-indirect-parameters-after <SECONDS>`: after startup elapsed time, remove only Bevy's `write_indirect_parameters_buffers` system from the render schedule. Preserve dependency ordering, allocated buffers, other render systems, camera settings, and existing scene exclusions.
- [x] Accept `--freeze-batched-instances-after <SECONDS>` independently: remove only `write_batched_instance_buffers<MeshPipeline>` at its cutoff. Support separate cutoffs so an earlier exclusion stays applied while measuring the next one; also handle multiple due removals in one extraction pass without replacing the schedule registry.
- [x] Accept `--freeze-gpu-clusters-after <SECONDS>`: remove only the private Bevy `prepare_clusters_for_gpu_clustering` callback, resolved by its exact runtime name and implicit type set. Keep existing buffers, other callbacks, CPU availability, and rendering rate policy unchanged; moving/changing scenes are not supported by this diagnostic.
- [x] Report actual removal time and fail explicitly unless exactly one target system is removed per cutoff. Without either flag, add no removal controller. Reject missing/invalid seconds and do not interpret values as asset paths.
- [x] Use only after initial uploads in a stationary diagnostic scene. Frozen draw metadata can invalidate moving/changing scenes; CPU differences may include downstream rendering effects and are not proof of unnecessary upload work. The cutoff includes reference sampling and is not an FPS-stabilization delay. Trace spans are overlapping instrumented wall time, not exclusive CPU cost; a trace lead must not be treated as proof that removing its callback changes CPU or FPS.

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

- [x] In exact configured `InWorldSceneStage::Empty`, disable Bevy `LightPlugin` together with `GizmoPlugin` and `GizmoRenderPlugin` so nested `LightGizmoPlugin` resources are not recreated.
- [x] Preserve LightPlugin, gizmos, and their render behavior for unconfigured/default runs, `Character` and later stages, debug modes, and screenshot/default paths.

### Camera rendering

- [x] Remove TAA, SSAO, depth/normal/motion prepasses, temporal jitter, and mip bias from `WowCamera` before `Lighting`.
- [x] Restore graphics-option-driven TAA/SSAO and required prepasses when the stage advances to `Lighting`.
- [x] Restore depth and normal prepasses with MSAA at `Lighting` without enabling TAA or SSAO.
- [x] Preserve camera identity, transforms, MSAA state before `Lighting`, tonemapping, shadow filtering, spatial audio, bloom, sharpening, and depth-of-field synchronization.
- [x] Keep the standalone performance overlay active while game UI and early camera rendering are isolated.

## How it works

- [InWorld performance investigation](../wiki/investigations/procedural-cloud-regeneration.md)
- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [UI system](../wiki/systems/ui-system.md)

## Implementation inventory

- `src/game/state/inworld_scene_stage.rs` — cumulative stage ordering and predicates.
- `src/rendering/camera/camera_post_process.rs` — stage-aware WowCamera render-bundle synchronization.
- `src/main.rs` — startup stage selection and pre-UI processing gates.
- `src/app_setup.rs` — exact-Empty LightPlugin/gizmo plugin boundary.
- `src/rendering/terrain/terrain{,_background_parse,_streaming,_spawn}.rs` — streamed object/water loading and flat-material controls.
- `src/rendering/skybox/mod.rs` — skybox-visual gates and dome removal with lighting retained.

## Tests asserting this spec

- `tests/unit/camera_post_process_tests.rs` — pre-Lighting removal, Lighting restoration, MSAA behavior, default behavior, and preserved common camera effects.
- `tests/unit/main_tests.rs` — stage parsing, default/full-scene behavior, UI processing gates, and performance-overlay survival.
- `tests/unit/pipeline_isolation_tests.rs` — render-frame delivery and calling-thread versus render-thread execution through the pipelining selector.
- `src/rendering/render_upload_isolation.rs` tests — separate removal deadlines, continued unrelated work, and CLI value handling.

## Known gaps (current cycle)

- [ ] Replace the preserved diagnostic client with the permanent stage-aware Empty build after explicit lifecycle permission.
- [ ] Re-run the structured Empty human gate before advancing to `Character`.

## Out of scope

- Globally disabling camera post-processing in normal full gameplay.
- Changing tonemapping, MSAA selection, shadow filtering, window behavior, networking, IPC, or UI rendering.
- Advancing to later cumulative stages before Empty is accepted.
