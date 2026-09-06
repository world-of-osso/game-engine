# InWorld scene isolation

InWorld scene isolation provides cumulative rendering stages for controlled diagnosis. Stage ordering lives in `src/game/state/inworld_scene_stage.rs`; implementation details and measured investigations are documented in [procedural cloud regeneration](../wiki/investigations/procedural-cloud-regeneration.md).

## What it must do

### Stage defaults

- [x] Preserve the full existing scene when no `InWorldSceneStage` resource is configured.
- [x] Keep stages cumulative from `Empty` through `Ui`.
- [x] Provide opt-in `--inworld-stage no-npcs-ui` for settled-FPS diagnosis: omit NPC/remote-player visuals, nameplate creation/update, and game UI processing/rendering while preserving the full scene's local-character policy, terrain, skybox, lighting, particles, camera effects, networking, and standalone FPS overlay. Remote replicated state remains available; this is not a server-side NPC disable.
- [x] Keep this diagnostic separate from `terrain`, which also excludes lighting and particles. Remove the selector when this isolation experiment is retired.

### Untextured-terrain diagnostic

- [x] Accept opt-in `--no-terrain-textures`: render streamed terrain with a shared flat, lit material without terrain texture bindings. Preserve chunk geometry, culling metadata, lighting/camera settings, and other isolation flags.
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

## Known gaps (current cycle)

- [ ] Replace the preserved diagnostic client with the permanent stage-aware Empty build after explicit lifecycle permission.
- [ ] Re-run the structured Empty human gate before advancing to `Character`.

## Out of scope

- Globally disabling camera post-processing in normal full gameplay.
- Changing tonemapping, MSAA selection, shadow filtering, window behavior, networking, IPC, or UI rendering.
- Advancing to later cumulative stages before Empty is accepted.
