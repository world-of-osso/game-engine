# Graphics effect configuration

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Independent graphics controls use the existing `options_settings.ron` graphics section. Runtime settings and persistence live in `src/game/state/client_options.rs` and `client_options_storage.rs`. See [rendering pipeline](../wiki/systems/rendering-pipeline.md).

## What it must do

- [x] Persist `particleEffectsEnabled`, `depthOfField`, existing `bloomEnabled`, `antiAlias` (`None`, `Msaa4x`, `Taa`), and `ssaoEnabled` independently; retain particle density and bloom intensity.
- [x] Missing new fields preserve current default output: particles enabled, depth of field and bloom disabled, MSAA4x selected, contact shading disabled.
- [x] Reject enabled SSAO combined with MSAA4x with an actionable configuration error; do not silently change either setting.
- [x] Load configuration at startup; do not introduce another configuration file or CLI flags.
- [x] Disabled particles omit emitter/simulation, Hanabi processing/render registration, and weather particle effects while preserving character animation, weather state, fog, lighting, and other scene systems.
- [x] Disabled blur, glow, AA, and contact shading remove their corresponding camera effects. Changing one control must not enable another.
- [x] Preserve unrelated saved options and normal scene-stage isolation.

## Native Godot DOF / SSAO mapping

- `depthOfField` and `ssaoEnabled` are independent booleans, default `false`; neither has a numeric range. The native Graphics UI currently exposes neither control. Authored Options commits preserve these hidden saved fields and apply their values live; direct file edits alone are not watched.
- Startup and live edits use `godot/rust/src/display_options.rs`, like bloom/TAA. The world-effects controller targets only root-viewport `WorldCamera` and `WorldLighting/Environment`, including late world entry and replacement. Portrait/subviewport cameras and UI layers are excluded. Both Off skips controller creation; turning either Off restores its original resource identity without changing unrelated fields.
- DOF On installs `CameraAttributesPractical`: near/far blur enabled, both distances 15 world units, both transitions 5 units, blur amount 0.1. The 15-unit focal distance preserves the retired client's authored focus. Practical blur is an approximation, not a conversion of Bevy's f/0.125 aperture or 64-pixel circle-of-confusion cap: Godot uses its own blur kernel/quality and amount. No claim of pixel-identical Retail optics or autofocus. DOF requires Forward+ or Mobile (not Compatibility); this client uses Forward+.
- SSAO On duplicates the world `Environment`, enables `ssao_enabled`, and uses radius 1 world unit, intensity 2, power 1.5 (Godot defaults). Remaining `ssao_*` defaults remain unchanged. This implements contact shading, not Retail kernel equality; only visible opaque depth/normal geometry participates. SSAO requires Forward+. The existing SSAO + MSAA4x configuration error remains; no implicit AA switch.
- Proof: `godot/tests/dof_ssao_options.gd` checks saved startup/live booleans, concrete resource values, independent On/Off, Off identity, world replacement and portrait exclusion. `dof_ssao_options_pixels.gd` checks owned floor/box crease darkening, a far checker plane's edge reduction, exact restored Off baseline and exact higher-layer UI pixels. These fixtures do not prove arbitrary material transparency, full Retail optics, or performance savings.

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
