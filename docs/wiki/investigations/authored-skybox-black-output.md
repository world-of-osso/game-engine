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

The strongest current suspect is the `SkyboxM2Material` path.

Traced batch inputs on 2026-04-11:

- `deathskybox.m2` has single-texture skybox batches with `shader_id=0x0010`
- `11xp_cloudsky01.m2` is not single-texture at all: it loads 54 batches, all with a real `texture_2_fdid`
- the `11xp_cloudsky01.m2` batch shader id set is `{0x4014, 0x8012, 0x8016}`

The engine now explicitly marks when a skybox batch does not have a second texture bound, so single-texture `0x0010` batches no longer have to sample an unbound texture.

That did **not** fix `11xp_cloudsky01.m2`, which means the remaining black-output bug is downstream of that single-texture case. The current gap is that the engine still treats the raw M2 `shader_id` as a direct shader opcode. The local reference client resolves some modern M2 shader ids through texture-combiner combo tables instead.

## Sources

- [skybox-authored-lookup.md](../../skybox-authored-lookup.md) — authored lookup chain and forced override commands
- [skybox_debug/mod.rs](../../../src/scenes/skybox_debug/mod.rs) — debug skybox resolution and spawn path
- [m2_spawn_material.rs](../../../src/rendering/model/m2_spawn_material.rs) — `skybox_batch_needs_effect_combine()` classification
- [skybox_m2_material.rs](../../../src/rendering/skybox/skybox_m2_material.rs) — authored skybox material path
- [m2_skybox.wgsl](../../../assets/shaders/m2_skybox.wgsl) — skybox shader combine behavior
- [wow_client/src/gx/m2.c](/home/osso/Repos/wow_client/src/gx/m2.c) — local reference client showing texture-combiner combo table resolution

## See Also

- [[skybox]] — authored lookup chain and debug commands
- [[rendering-pipeline]] — skybox material render path
