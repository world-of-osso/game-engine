# InWorld scene isolation

InWorld scene isolation provides cumulative rendering stages for controlled diagnosis. Stage ordering lives in `src/game/state/inworld_scene_stage.rs`; implementation details and measured investigations are documented in [procedural cloud regeneration](../wiki/investigations/procedural-cloud-regeneration.md).

## What it must do

### Stage defaults

- [x] Preserve the full existing scene when no `InWorldSceneStage` resource is configured.
- [x] Keep stages cumulative from `Empty` through `Ui`.

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
