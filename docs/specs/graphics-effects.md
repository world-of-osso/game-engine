# Graphics effect configuration

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Independent graphics controls use the existing `options_settings.ron` graphics section. Runtime settings and persistence live in `src/game/state/client_options.rs` and `client_options_storage.rs`. See [rendering pipeline](../wiki/systems/rendering-pipeline.md).

## What it must do

- [x] Persist `particleEffectsEnabled`, existing `bloomEnabled`, `antiAlias` (`None`, `Msaa4x`, `Taa`), and `ssaoEnabled` independently; retain particle density and bloom intensity.
- [x] Missing new fields preserve current default output: particles enabled, bloom disabled, MSAA4x selected, contact shading disabled.
- [x] Reject enabled SSAO combined with MSAA4x with an actionable configuration error; do not silently change either setting.
- [x] Load configuration at startup; do not introduce another configuration file or CLI flags.
- [x] Disabled particles omit emitter/simulation, Hanabi processing/render registration, and weather particle effects while preserving character animation, weather state, fog, lighting, and other scene systems.
- [x] Disabled glow, AA, and contact shading remove their corresponding camera effects. Changing one control must not enable another.
- [x] Preserve unrelated saved options and normal scene-stage isolation.

## Native Godot SSAO mapping

- `ssaoEnabled` defaults to `false`. Native Graphics UI has no SSAO row; existing Options commits preserve and apply this saved field live. File edits alone are not watched. Depth of Field is removed entirely (2026-10-07 user decision), not retained as a disabled switch.
- Existing RON/Serde unknown-field handling ignores removed `depthOfField` (and unknown snake-case `depth_of_field`) keys. Loading preserves other settings; the next save omits removed keys. No migration layer. Concrete old-file regression: `godot/core/src/client_options_data_tests.rs::removed_depth_of_field_old_file_loads_and_is_not_saved` writes a concrete old file and exercises production load/save.
- Startup/live application uses `godot/rust/src/display_options.rs`. The controller targets root-viewport `WorldLighting/Environment`, including late entry/replacement. Cameras, portrait environments and UI layers are untouched. First Off creates no controller; live Off restores original Environment identity without modifying the required depth prepass.
- SSAO On duplicates Environment, enables SSAO, and uses Godot defaults radius 1, intensity 2, power 1.5. Light affect and AO-channel affect are 1 for authored diffuse lighting; remaining SSAO values are unchanged. Terrain/M2/WMO preserve zero engine ambient/reflections with IRRADIANCE/RADIANCE overrides rather than `ambient_light_disabled`, which bypasses AO. Emissive/unshaded output is excluded. SSAO requires Forward+; the SSAO + MSAA4x validation error remains, without implicit AA changes.
- `project.godot` enables `rendering/driver/depth_prepass/enable` as a stable prerequisite. The pinned renderer otherwise skips SSAO generation despite sampling its output. Never toggle this project setting live: cached opaque pipelines use EQUAL depth testing after a prepass and can discard geometry if the prepass is later disabled. Off changes no Environment values or camera resources; its complete fixture image must match the master-material reference. No performance-savings claim for Off.
- Consumer proof: `godot/tests/ssao_options.gd`. Pixel oracle: `ssao_options_pixels.gd` requires contact darkening, unchanged isolated flat terrain, exact restored Off and exact UI. Renderer comparison: `ssao_renderer_probe.gd`, same standard/authored scene, defaults/affect settings, Dozen versus Lavapipe. Lavapipe passes both saved Off/On pixel fixtures, including exact master-material Off/UI and unchanged flat terrain. Dozen still corrupts SSAO with valid prepass; it is not a passing pixel proof or a reason to weaken the oracle. See [investigation](../wiki/investigations/native-ssao.md).

## How it works

- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [Particle system](../particle-system.md)

## Implementation inventory

- `src/game/state/client_options.rs` — runtime options, defaults and startup loading.
- `src/game/state/client_options_storage.rs` — RON serialization and saving.
- `src/rendering/camera/camera_post_process.rs` — camera effects.
- `src/app_setup.rs` — particle plugin registration.
- `src/rendering/particles/` — emitter creation and simulation.
- `src/rendering/weather.rs` — weather state, fog, and weather particle effects.

## Tests asserting this spec

- `src/game/state/client_options_tests.rs`
- `tests/unit/camera_post_process_tests.rs`
- `src/rendering/particles/tests.rs` — startup registration and deferred emitter behavior.

## Known gaps (current cycle)

- [x] Persistence and validation: six focused behavioral tests pass at `c4376e6e`.
- [x] Camera lifecycle: 13 focused behavioral tests pass at `24d97a50`.
- [x] Particle startup/spawn gating: 23 targeted tests pass at `cab4207b`.
- [x] Particle-off weather behavior: six weather tests pass at `b0e1f2bf`.

## Out of scope

- New graphics UI controls or live file watching; native application uses the existing startup and Options-commit paths.
- CPU improvement claims or benchmarks; switches alone do not establish savings.
- Renderer compatibility redesign to combine SSAO and MSAA.
