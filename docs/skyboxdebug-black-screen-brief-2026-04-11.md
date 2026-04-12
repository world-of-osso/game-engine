# Skyboxdebug Black Screen Brief

## Findings

- `de4de85` is a known-good baseline for `skyboxdebug` rendering again.
  - Reverting to that commit restores visible skybox output.
  - That proves the later breakage was introduced by follow-up skybox changes, not by the base screen wiring.
- `skyboxdebug` is no longer a totally black viewport.
- That is not proof the authored skybox works.
  - Earlier purple screenshots could be explained by the removed magenta `SkyboxDebugDepthProbe` plus the procedural baseline.
  - After removing that probe, any remaining non-black result has to come from the sky itself, the reference plane, or other surviving scene content.
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
- One-at-a-time retesting produced a clearer regression split.
  - Preserving authored batch draw order does not break the working baseline.
  - Mapping skybox blend mode `7` to `AlphaMode::Blend` does not break the working baseline.
  - Preserving authored skybox `batch.transparency` does break the working baseline.
    - With that change applied, the skybox becomes broken again.
    - Reverting just that change restores the working state.
- `LightSkybox.db2` flag decoding is now traced well enough to drive `skyboxdebug` composition.
  - The likely flags field is `field[1]`, not the FDID field.
  - `LightSkyboxID 653` resolves flag bits `0b01111`, which matches the current wowdev reading of:
    - full-day skybox
    - combine procedural and authored sky
    - procedural fog-color blending
    - force sunshafts
  - `skyboxdebug` now uses those flags to decide whether default mode keeps the procedural sky dome and distance fog visible.
- Screenshot coverage for the flag path should stay on alive-scene authored skyboxes.
  - Using `deathskybox.m2` as a generic "flags = 0" control was a bad fixture.
  - The current screenshot regression uses only `LightSkyboxID 653`.
  - The no-blend path is covered by unit tests with a synthetic zero-flags fixture instead of rendering a death-state skybox.
- The debug scene keeps a procedural sky/fog environment now.
  - That is no longer unconditional.
  - In default mode, it now depends on authored `LightSkybox::Flags`.

## Assumptions

- The startup-path fixes above are real even if authored skybox rendering is still broken.
- `de4de85` should be treated as the stable comparison point for incremental skybox work.
- The authored skyboxes are still not proven to render correctly.
  - That still needs a control without helper visuals and without treating "non-black" as success.
- The current `LightSkybox::Flags` behavior is only partially implemented.
  - `CombineProceduralAndSkybox` and `ProceduralFogColorBlend` now affect `skyboxdebug`.
  - The other documented sky-affecting flags still need follow-up work.
- Keeping the procedural sky/fog bootstrap in `skyboxdebug` is acceptable because this screen is a debug harness, not a shipping gameplay scene.

## What This Rules Out

- Not a GPU or driver compatibility problem.
- Not a missing authored skybox lookup problem.
- Not a “black source texture” problem.
- Not just the earlier single-texture combine bug.
- Not just an “IPC screenshot taken too early” problem.
- Not the claim that authored skybox rendering is already proven.
- Not "just a general skybox material change" anymore.
  - At least one specific material change is now isolated as bad: preserving authored `batch.transparency` in the skybox material path.

## Theories

- `11xp_cloudsky01.m2` still uses more modern effect ids (`0x8012`, `0x8016`) than the older legacy combiner path.
- That means there may still be a separate fidelity problem after this fix set:
  - the debug harness is no longer fully black
  - but modern multi-stage authored skybox effect behavior may still need a more complete `M2Effect` implementation later
- Skybox-authored `transparency` values are probably not directly usable in the current shader contract.
  - The shader/material path may already encode opacity elsewhere, so multiplying by authored batch transparency again likely suppresses the dome too aggressively.
  - Until that contract is understood, forcing `transparency = 1.0` is the safer behavior for skyboxdebug.

## Current Best Read

- The pure black-screen startup failure in the `skyboxdebug` harness is fixed.
- `de4de85` is the current known-good skybox baseline.
- The decisive fixes were:
  - internal shader handle registration for `SkyboxM2Material`
  - delayed direct-entry scene setup
  - skybox-following-camera translation instead of orbit focus
  - guarded `uv_b` access in the WGSL path
- The currently safe forward changes are:
  - preserving authored batch order (`priority_plane`, `material_layer`)
  - treating blend mode `7` as blended instead of additive
  - looping/authored skybox UV animation fixes
  - `LightSkybox::Flags`-driven procedural sky/fog composition in `skyboxdebug`
- The currently unsafe forward change is:
  - preserving authored skybox `batch.transparency`
- Authored skybox correctness is still unproven.
- Earlier purple-cube output was not valid proof that the authored skybox worked.
