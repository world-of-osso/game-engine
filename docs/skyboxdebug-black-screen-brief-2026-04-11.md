# Skyboxdebug Black Screen Brief

## Findings

- `skyboxdebug` is no longer a totally black viewport.
- That is not proof the authored skybox works.
  - The debug scene still spawns a magenta `SkyboxDebugDepthProbe`.
  - A purple or merely non-black screenshot can be explained by that probe and the procedural baseline, not by a correctly rendered authored skybox.
- The earlier "non-black screenshot" proof standard was wrong.
- One real root cause was shader registration, not authored skybox content.
  - `SkyboxM2Material::fragment_shader()` used a path-loaded shader ref.
  - That could hit `PipelineCacheError::ShaderNotLoaded` for `shaders/m2_skybox.wgsl` during live startup.
  - Switching to `load_internal_asset!` plus a fixed shader handle removed that startup race.
- A second real root cause was the debug scene itself.
  - Direct-entry `SkyboxDebug` setup on `OnEnter` was too early.
  - Delaying setup until the first active `Update` makes the scene initialize reliably.
- A third real bug was authored skybox anchoring.
  - The authored skybox was following the orbit focus point instead of the actual camera translation.
  - Centering it on the camera fixes the enclosing-shell behavior.
- The skybox material/shader path also needed robustness fixes.
  - `m2_skybox.wgsl` now guards `uv_b` access so meshes without `UV_1` do not compile the wrong path.
  - Single-texture authored skybox batches still avoid multiplying against a missing second texture.
- The debug scene keeps a procedural sky/fog environment now.
  - That gives `skyboxdebug` a reliable non-black baseline even while authored skyboxes are being iterated.

## Assumptions

- The startup-path fixes above are real even if authored skybox rendering is still broken.
- The authored skyboxes are still not proven to render correctly.
  - That needs a control without the magenta probe and without treating "non-black" as success.
- Keeping the procedural sky/fog bootstrap in `skyboxdebug` is acceptable because this screen is a debug harness, not a shipping gameplay scene.

## What This Rules Out

- Not a GPU or driver compatibility problem.
- Not a missing authored skybox lookup problem.
- Not a “black source texture” problem.
- Not just the earlier single-texture combine bug.
- Not just an “IPC screenshot taken too early” problem.
- Not the claim that authored skybox rendering is already proven.

## Theories

- The current purple output may mainly be the debug probe, with the authored skybox still failing behind it.
- `11xp_cloudsky01.m2` still uses more modern effect ids (`0x8012`, `0x8016`) than the older legacy combiner path.
- That means there may still be a separate fidelity problem after this fix set:
  - the debug harness is no longer fully black
  - but modern multi-stage authored skybox effect behavior may still need a more complete `M2Effect` implementation later

## Current Best Read

- The pure black-screen startup failure in the `skyboxdebug` harness is fixed.
- The decisive fixes were:
  - internal shader handle registration for `SkyboxM2Material`
  - delayed direct-entry scene setup
  - skybox-following-camera translation instead of orbit focus
  - guarded `uv_b` access in the WGSL path
- Authored skybox correctness is still unproven.
- The current purple-cube result is not valid proof that the authored skybox works.
