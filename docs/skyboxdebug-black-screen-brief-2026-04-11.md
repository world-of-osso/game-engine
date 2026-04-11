# Skyboxdebug Black Screen Brief

## Findings

- `skyboxdebug` is no longer black for either authored test target.
  - Forced `--light-skybox-id 653` (`11xp_cloudsky01.m2`) now produces a non-black screenshot.
  - Default lookup (`deathskybox.m2`) now also produces a non-black screenshot.
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

- The current non-black screenshots are enough to close the specific `PLAN.md` verification item.
- The authored skyboxes are now rendering through a stable enough path for investigation, even if some modern M2 effect semantics may still be incomplete.
- Keeping the procedural sky/fog bootstrap in `skyboxdebug` is acceptable because this screen is a debug harness, not a shipping gameplay scene.

## What This Rules Out

- Not a GPU or driver compatibility problem.
- Not a missing authored skybox lookup problem.
- Not a “black source texture” problem.
- Not just the earlier single-texture combine bug.
- Not just an “IPC screenshot taken too early” problem.

## Theories

- `11xp_cloudsky01.m2` still uses more modern effect ids (`0x8012`, `0x8016`) than the older legacy combiner path.
- That means there may still be a separate fidelity problem after this fix set:
  - the scene is no longer black
  - but modern multi-stage authored skybox effect behavior may still need a more complete `M2Effect` implementation later

## Current Best Read

- The black-screen blocker is fixed for `skyboxdebug`.
- The decisive fixes were:
  - internal shader handle registration for `SkyboxM2Material`
  - delayed direct-entry scene setup
  - skybox-following-camera translation instead of orbit focus
  - guarded `uv_b` access in the WGSL path
- Any remaining work is about authored skybox correctness or fidelity, not total black output.
