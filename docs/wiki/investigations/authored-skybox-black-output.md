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
- `LightSkybox.db2` has enough decoded flag data to drive `skyboxdebug` composition now.
  - `field[1]` decodes as flags.
  - `field[2]` decodes as the authored skybox FDID.
  - `LightSkyboxID 653` carries the blend bits that keep procedural sky and fog visible in default debug mode.
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

## Modern Shader Trace (2026-04-21)

The modern authored cloud skybox path is now traced end-to-end in tests (`m2_spawn_material_tests.rs` + `skybox_m2_material_tests.rs`) for `0x4014`, `0x8012`, and `0x8016`.

### Batch translation + stage binding (`m2_spawn_material.rs`)

For `11xp_cloudsky01.m2`, traced batches currently map as:

| Shader ID | Authored `texture_count` | Authored extra stages | Bound stage flags (`has_second/third/fourth`) | Resolved `combine_mode` |
| --- | --- | --- | --- | --- |
| `0x4014` | `2` | `0` | `1 / 0 / 0` | `0x000E` (static fragment-mode table path) |
| `0x8012` | `3` | `1` | `1 / 1 / 0` | `0x8012` |
| `0x8016` | `4` | `2` | `1 / 1 / 1` | `0x8016` |

Notable detail: `0x4014` cloudsky batches currently do **not** set `uses_texture_combiner_combos`, so the loader takes the static fragment-mode mapping and emits `0x000E` instead of preserving raw `0x4014`.

### UV routing (`resolve_skybox_uv_modes`)

Current UV mode routing contract:

- `0x4014`: follows authored per-batch UV booleans (`use_uv_2_1`, `use_uv_2_2`)
- `0x8012`: forced to `[0, 0, 0, 0]` (primary UV for all stages)
- `0x8016`: forced to `[0, 0, 0, 1]` (stage 4 routed to secondary UV set)

### WGSL combine semantics (`assets/shaders/m2_skybox.wgsl`)

Current shader combine implementation includes explicit cases for legacy/older modes (`0x4014`, `0x0010`, `0x0011`, `0x4016`, `0x8015`, `0x8001`, `0x8002`, `0x8003`), but **does not** define explicit combine logic for `0x8012` or `0x8016`.

The fragment path currently combines only `texture1` + `texture2`; `third_texture` and `fourth_texture` are bound but not consumed in combine logic. This is the traced gap that explains why modern multi-stage authored batches still fail to reproduce reference-client visuals.

## Sources

- [skybox-authored-lookup.md](../../skybox-authored-lookup.md) — authored lookup chain and forced override commands
- [skybox_debug/mod.rs](../../../src/scenes/skybox_debug/mod.rs) — debug skybox resolution and spawn path
- [m2_spawn_material.rs](../../../src/rendering/model/m2_spawn_material.rs) — `skybox_batch_needs_effect_combine()` classification
- [skybox_m2_material.rs](../../../src/rendering/skybox/skybox_m2_material.rs) — authored skybox material path
- [m2_skybox.wgsl](../../../assets/shaders/m2_skybox.wgsl) — skybox shader combine behavior
- [m2_spawn_material_tests.rs](../../../src/rendering/model/m2_spawn_material_tests.rs) — modern shader stage/UV trace coverage
- [skybox_m2_material_tests.rs](../../../src/rendering/skybox/skybox_m2_material_tests.rs) — WGSL combine-coverage trace for `0x4014` / `0x8012` / `0x8016`
- [wow_client/src/gx/m2.c](/home/osso/Repos/wow_client/src/gx/m2.c) — local reference client showing texture-combiner combo table resolution

## See Also

- [[skybox]] — authored lookup chain and debug commands
- [[rendering-pipeline]] — skybox material render path
