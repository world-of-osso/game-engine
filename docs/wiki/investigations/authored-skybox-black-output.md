# Authored Skybox Black Output

Forced authored WoW skyboxes still render effectively black in `skyboxdebug`. The older default warband-scene repro was misleading: scene 1 was reaching `deathskybox.m2` through a global `Light.csv` fallback row, not a local campsite-authored skybox choice.

## Reproduction

Run:

```bash
cargo run --bin game-engine -- --screen skyboxdebug screenshot data/skyboxdebug-default-2026-04-11.webp
cargo run --bin game-engine -- --screen skyboxdebug --light-skybox-id 653 screenshot data/skyboxdebug-light653-2026-04-11.webp
```

Observed runtime logs:

- current default scene 1 path now falls back to `data/models/skyboxes/costalislandskybox.m2`
- forced `--light-skybox-id 653` resolves `data/models/skyboxes/11xp_cloudsky01.m2`

Measured image output:

- forced `653` screenshot center pixel: `srgba(0,0,0,0)`, mean brightness: `0.0059299`

## What This Proves

- The authored lookup chain works well enough to reach the known-good `LightSkyboxID 653 -> 11xp_cloudsky01.m2` path.
- The old default `deathskybox.m2` control was not trustworthy for warband scene 1 and should not be used as proof of authored correctness.
- The remaining failure is downstream of lookup, in the authored skybox render path shared by `skyboxdebug`.

## Fixed

The single-texture authored skybox bug is fixed.

Specifically:

- `deathskybox.m2` has single-texture skybox batches with `shader_id=0x0010`
- the engine now explicitly marks whether a skybox batch actually has a second texture bound
- the WGSL path now skips `second_texture` sampling and combine logic when that second texture is missing

The current regression tests cover both the CPU material contract and the authored `deathskybox.m2` asset path, so this specific bug should not regress silently.

## Remaining Problem

The remaining black-output bug is not the single-texture case anymore.

Traced batch inputs on 2026-04-11:

- `11xp_cloudsky01.m2` is not single-texture at all: it loads 54 batches, all with a real `texture_2_fdid`
- the `11xp_cloudsky01.m2` batch shader id set is `{0x4014, 0x8012, 0x8016}`

`11xp_cloudsky01.m2` still renders black, which means the remaining bug is downstream of the single-texture fix. The current gap is that the engine still treats the raw M2 `shader_id` as a direct shader opcode. The local reference client resolves some modern M2 shader ids through texture-combiner combo tables instead.

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
