# Authored Skybox Black Output

`skyboxdebug` resolves authored WoW skyboxes correctly, but the rendered result is still effectively black. The failure reproduces through both the default warband-scene lookup and a forced known-good authored override, which rules out lookup failure as the primary issue.

## Reproduction

Run:

```bash
cargo run --bin game-engine -- --screen skyboxdebug screenshot data/skyboxdebug-default-2026-04-11.webp
cargo run --bin game-engine -- --screen skyboxdebug --light-skybox-id 653 screenshot data/skyboxdebug-light653-2026-04-11.webp
```

Observed runtime logs:

- default `skyboxdebug` resolves `data/models/skyboxes/deathskybox.m2`
- forced `--light-skybox-id 653` resolves `data/models/skyboxes/11xp_cloudsky01.m2`

Measured image output:

- default screenshot center pixel: `srgba(0,0,0,0)`, mean brightness: `0.00612541`
- forced `653` screenshot center pixel: `srgba(0,0,0,0)`, mean brightness: `0.0059299`

## What This Proves

- The authored lookup chain works well enough to reach two different skybox M2 files.
- The black output is not just a bad default skybox choice, because the forced known-good override fails the same way.
- The failure is downstream of lookup, in the authored skybox render path shared by `skyboxdebug`.

## Current Suspect

The strongest current suspect is the `SkyboxM2Material` path. The shader always samples `second_texture` and applies combine behavior for shader ids like `0x0010` and `0x0011`, while the CPU-side skybox material classification still allows single-texture batches with those shader ids to stay on the base-texture path.

That mismatch would explain why different authored skyboxes can both resolve correctly and still render to black.

## Sources

- [skybox-authored-lookup.md](../../skybox-authored-lookup.md) — authored lookup chain and forced override commands
- [skybox_debug/mod.rs](../../../src/scenes/skybox_debug/mod.rs) — debug skybox resolution and spawn path
- [m2_spawn_material.rs](../../../src/rendering/model/m2_spawn_material.rs) — `skybox_batch_needs_effect_combine()` classification
- [skybox_m2_material.rs](../../../src/rendering/skybox/skybox_m2_material.rs) — authored skybox material path
- [m2_skybox.wgsl](../../../assets/shaders/m2_skybox.wgsl) — skybox shader combine behavior

## See Also

- [[skybox]] — authored lookup chain and debug commands
- [[rendering-pipeline]] — skybox material render path
