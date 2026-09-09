# Graphics effect configuration

Independent graphics controls use the existing `options_settings.ron` graphics section. Runtime settings and persistence live in `src/game/state/client_options.rs` and `client_options_storage.rs`. See [rendering pipeline](../wiki/systems/rendering-pipeline.md).

## What it must do

- [ ] Persist `particleEffectsEnabled`, `depthOfField`, existing `bloomEnabled`, `antiAlias` (`None`, `Msaa4x`, `Taa`), and `ssaoEnabled` independently; retain particle density and bloom intensity.
- [ ] Missing new fields preserve current default output: particles enabled, depth of field and bloom disabled, MSAA4x selected, contact shading disabled.
- [ ] Reject enabled SSAO combined with MSAA4x with an actionable configuration error; do not silently change either setting.
- [ ] Load configuration at startup; do not introduce another configuration file or CLI flags.
- [ ] Disabled particles omit emitter/simulation and Hanabi processing/render registration, without disabling character animation or other scene systems.
- [ ] Disabled blur, glow, AA, and contact shading remove their corresponding camera effects. Changing one control must not enable another.
- [ ] Preserve unrelated saved options and normal scene-stage isolation.

## How it works

- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)
- [Particle system](../particle-system.md)

## Implementation inventory

- `src/game/state/client_options.rs` — runtime options, defaults and startup loading.
- `src/game/state/client_options_storage.rs` — RON serialization and saving.
- `src/rendering/camera/camera_post_process.rs` — camera effects.
- `src/app_setup.rs` — particle plugin registration.
- `src/rendering/particles/` — emitter creation and simulation.

## Tests asserting this spec

- `src/game/state/client_options_tests.rs`
- `tests/unit/camera_post_process_tests.rs`
- Particle startup/spawn behavioral tests added with implementation.

## Known gaps (current cycle)

- [ ] Persistence, validation and effect-off coverage pending implementation.

## Out of scope

- New graphics UI controls or live file watching; this change uses the existing startup configuration path.
- CPU improvement claims or benchmarks; switches alone do not establish savings.
- Renderer compatibility redesign to combine SSAO and MSAA.
