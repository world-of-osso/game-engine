# Graphics effect configuration

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Independent graphics controls use the existing `options_settings.ron` graphics section. Runtime settings and persistence live in `src/game/state/client_options.rs` and `client_options_storage.rs`. See [rendering pipeline](../wiki/systems/rendering-pipeline.md).

## What it must do

- [x] Persist `particleEffectsEnabled`, existing `bloomEnabled`, `antiAlias` (`None`, `Msaa4x`, `Taa`), and `ssaoEnabled` independently; retain particle density and bloom intensity.
- [x] Missing new fields preserve current default output: particles enabled, bloom disabled, MSAA4x selected, contact shading disabled.
- [x] Reject enabled SSAO combined with MSAA4x with an actionable configuration error; do not silently change either setting.
- [x] Load configuration at startup; do not introduce another configuration file or CLI flags. Native SSAO is restart-required.
- [x] Disabled particles omit emitter/simulation, Hanabi processing/render registration, and weather particle effects while preserving character animation, weather state, fog, lighting, and other scene systems.
- [x] Disabled glow, AA, and contact shading remove their corresponding camera effects. Changing one control must not enable another.
- [x] Persist `anisotropicFiltering` (1, 2, 4, 8 or 16 samples) and apply it to the 3D viewport and every world texture sampler (M2, WMO, terrain ground layers, liquids, ground detail, water). Default 16: Retail `textureFilteringMode` defaults to 5 and indexes an anisotropy ladder 1/1/2/4/8/16 (12.1.5.70688; callback and ladder proved from the client image, the final sampler-descriptor field INFERRED, decomp `SAMPLER-AND-PROJECTED-NOTES.md`). Retirement: drop the option if the decomp proves Retail scales anisotropy per texture descriptor instead, or when the client stops using Godot's viewport anisotropic level.
- [x] Persist `graphicsQuality` (`Low`, `Medium`, `High`; default `High`) and read the world camera far plane, WDL horizon range, sun shadow splits (world and character-select campsite), directional shadow atlas size and unit shadow radius from its row (`godot/core/src/graphics_quality_data.rs`). `High` keeps the client's settings before presets (far 1000, horizon 3108, 2 splits) with retail's largest shadow atlas, 2048 (`shadowTextureSize` at shadowMode 3-4; Godot's default 4096 before); remote unit shadows fade out by ordered dither over the last tenth of its 50 yd unit shadow radius from the world camera (retail `entityShadowFadeScale` 50, radius in yards INFERRED; computed in the M2 shader from global `unit_shadow_camera`/`unit_shadow_radius`, no per-frame material writes) and drawn remote units beyond it cast no shadow, while the local player and camera-rejected units keep theirs. `Medium`/`Low` scale `High` by retail block 2/block 0 over block 11 (decoded blocks VERIFIED, the ratio mapping INFERRED). The row applies when the client starts; no Options control exists yet.
- [x] Preserve unrelated saved options and normal scene-stage isolation.

## Native Godot SSAO mapping

- `ssaoEnabled` defaults to `false`. Native Graphics exposes `SSAO (Requires Restart)`. Options commits persist the selection, but never change live SSAO or the depth prepass; both take effect on the next process launch. Other graphics controls remain live. File edits alone are not watched. Depth of Field is removed entirely (2026-10-07 user decision), not retained as a disabled switch.
- Existing RON/Serde unknown-field handling ignores removed `depthOfField` (and unknown snake-case `depth_of_field`) keys. Loading preserves other settings; the next save omits removed keys. No migration layer. Concrete old-file regression: `godot/core/src/client_options_data_tests.rs::removed_depth_of_field_old_file_loads_and_is_not_saved` writes a concrete old file and exercises production load/save.
- Startup application uses `godot/rust/src/display_options.rs`. GDExtension `Servers` initialization reads the existing saved options before RenderingServer initialization and sets the prepass to the saved SSAO boolean. The controller uses this fixed startup choice, not the pending saved selection, for root-viewport `WorldLighting/Environment`, including late entry/replacement. Cameras, portrait environments and UI layers are untouched. Startup Off creates no controller and preserves original Environment identity.
- SSAO On duplicates Environment, enables SSAO, and uses Godot defaults radius 1, intensity 2, power 1.5. Light affect and AO-channel affect are 1 for authored diffuse lighting; remaining SSAO values are unchanged. Terrain/M2/WMO preserve zero engine ambient/reflections with IRRADIANCE/RADIANCE overrides rather than `ambient_light_disabled`, which bypasses AO. Emissive/unshaded output is excluded. SSAO requires Forward+; the SSAO + MSAA4x validation error remains, without implicit AA changes.
- `project.godot` keeps `rendering/driver/depth_prepass/enable=false`, preserving the deliberate a41c02c1 CPU-bound submission decision. Only saved SSAO On enables the prepass at startup; never edit project.godot at runtime or toggle the prepass after renderer initialization. Cached opaque pipelines use EQUAL depth testing after a prepass and can discard geometry if it is later disabled. Startup Off changes no Environment values or camera resources; its complete fixture image must be byte-identical to the prepass-disabled master-material reference. This preserves the prior performance configuration, not a new benchmark claim. No generated settings files.
- Consumer proof: `godot/tests/ssao_options.gd`. Pixel oracle: `ssao_options_pixels.gd` compares fresh saved-On/Off processes against the prepass-disabled master image, requiring contact darkening, unchanged isolated flat terrain, exact Off and exact UI. Pending toggles must leave all live image bytes unchanged; restart back to saved Off must restore exact master pixels. Renderer comparison: `ssao_renderer_probe.gd`, same standard/authored scene, defaults/affect settings, Dozen versus Lavapipe. Lavapipe passes both saved Off/On pixel fixtures, including exact master-material Off/UI and unchanged flat terrain. Dozen still corrupts SSAO with valid prepass; it is not a passing pixel proof or a reason to weaken the oracle. See [investigation](../wiki/investigations/native-ssao.md).

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

- `godot/core/src/graphics_quality_data.rs` (rows), `godot/core/src/client_options_data_tests.rs::graphics_quality_defaults_to_high_and_round_trips` (persistence), `godot/tests/unit_outdoor_admission.gd` (unit shadow radius casting), `godot/tests/unit_shadow_fade_pixels.gd` (fade band pixels).
- `src/game/state/client_options_tests.rs`
- `tests/unit/camera_post_process_tests.rs`
- `src/rendering/particles/tests.rs` — startup registration and deferred emitter behavior.

## Known gaps (current cycle)

- [x] Persistence and validation: six focused behavioral tests pass at `c4376e6e`.
- [x] Camera lifecycle: 13 focused behavioral tests pass at `24d97a50`.
- [x] Particle startup/spawn gating: 23 targeted tests pass at `cab4207b`.
- [x] Particle-off weather behavior: six weather tests pass at `b0e1f2bf`.

## Out of scope

- Graphics UI controls beyond the restart-required SSAO row, or live file watching; native application uses the existing startup and Options-commit paths.
- CPU improvement claims or benchmarks; switches alone do not establish savings.
- Renderer compatibility redesign to combine SSAO and MSAA.
