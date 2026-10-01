# Authored Skybox Black Output

Cloud `11xp_cloudsky01.m2` black output at authored times 0 and 100000 ms is explained by source opacity keys, not an established production renderer defect. Original/native cloud RGB matches at 100000 ms; active-phase and coastal parity remain open. The older default warband-scene repro was misleading: scene 1 reached `deathskybox.m2` through a global `Light.csv` fallback row, not a local campsite-authored choice.

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
- Historically, the remaining failure was attributed downstream of lookup; the retained source-opacity evidence below supersedes a renderer-bug inference for cloud times 0/100000. This does not describe ordinary Azeroth InWorld sky: that live path selected `LightParamsID 12 → raw LightSkyboxID 0`. The independent procedural-dome lifecycle restoration required a later visibility correction in `58d4b12a`; see [[procedural-sky-dome-visibility]].

## Fixed

The single-texture authored skybox bug is fixed.

Specifically:

- `deathskybox.m2` has single-texture skybox batches with `shader_id=0x0010`
- the engine now explicitly marks whether a skybox batch actually has a second texture bound
- the WGSL path now skips `second_texture` sampling and combine logic when that second texture is missing

The current regression tests cover both the CPU material contract and the authored `deathskybox.m2` asset path, so this specific bug should not regress silently.

## Historical unresolved trace

The April black-output observation was not the single-texture case; it did not establish a renderer defect.

Traced batch inputs on 2026-04-11:

- `11xp_cloudsky01.m2` is not single-texture at all: it loads 54 batches, all with a real `texture_2_fdid`
- the `11xp_cloudsky01.m2` batch shader id set is `{0x4014, 0x8012, 0x8016}`

The initial trace suspected direct raw M2 shader-opcode handling versus the reference client's texture-combiner tables. That missing-modern-combiner claim is superseded by the trace below; black output alone does not identify a combiner bug.

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

Current shader combine implementation has explicit `0x8012` and `0x8016` branches and consumes optional third/fourth stages. `0x8012` combines `texture1 * mix(texture2, texture3, texture3.a)`; `0x8016` uses the same pattern with the fourth-stage alpha mask when present. The older claim that those branches or bindings were absent is stale.

The unresolved authored gap is pixel correctness: the current formulas have CPU stage/UV coverage but no reference-client GPU proof that their operation order and alpha source reproduce WoW's combiner semantics. This remains independent of the fixed ordinary InWorld dome omission.

## Retained original/native evidence — 2026-10-01

| Case | Retained proof | Boundary / status |
| --- | --- | --- |
| Cloud FDID 5412968, 0/100000 ms | Raw M2/SKIN decode resolves all 54 batches to tracks dark at both times; actual native 100000 state has 54 transparency values of 0. Batch-zero track keys `[0,133333,200000,543333,576667,800000]` carry signed values `[0,0,32767,32767,0,0]`, step-sampled by the source evaluator. | Source-authored zero opacity, not parser/sampler failure. Track 2 is nonzero but not selected by these batches. No production renderer fix for this black output. |
| Cloud original 100000 ms | Fresh original Bevy build `f23343bb`, 19m09s, exit0; lossless `grim` 1280×720 capture is black. Strict comparator at `da429bfd` accepts metadata and compares all 921600 RGB pixels without mask/fit: failed pixels 0, maximum error 0, exit0. | Bounded dark-phase image match only. Nominal source camera coordinates rounded to six decimals; FOV matched semantically, not full camera/projection equivalence or other-phase parity. Four original build warnings remain historical. |
| Diagnostic active 200000 ms | Observed selected opacity 1 and retained PNG. | Normal-exit attempt timeout124 despite PNG: not whole-case PASS. Separate `cloud-authored-200000-final.log` attempt fails startup wait, exit1. |
| Coastal FDID 525142, 100000 ms | Premature diagnostic comparison records 450491 mismatched pixels, exit1. | **OPEN** pending phase-ready capture; not a renderer-bug assertion. Earlier native fixed/live contribution observations are not original-pixel parity. |
| New oracles `d2cec014` / `5cc16630` | Runtime remains pending: SETUP failed the 30-second CASC wait under shared-host contention. | Preserve historical incorrect positive-contribution RED at dark cloud phases; setup failure is neither new behavioral RED nor GREEN. |

Source cloud keys stay dark until 200000 ms for the selected initial tracks; later selected tracks have different activation times. The April modern-stage/UV coverage remains bounded CPU/source evidence, not active-phase rendered equivalence. Original capture provenance and strict comparison do not close full camera, all-track, full-scene, retained-goal or conversion acceptance. Shared-host contention also limits [[world-entry-stalls#Retained integrated performance — 2026-10-01|retained performance evidence]].

## Sources

- `/tmp/claude/retained-conversion-20/cloud-zero-opacity-root.md` and `cloud-zero-opacity-tracks.json` — raw source keys, batch lookup and asset hashes; older `cloud-zero-pixels-root.md` / `main-proof-ledger.md` unknown-root statements are historical.
- `/tmp/claude/retained-conversion-20/{cloud-render-state.json,cloud-active-state.log,original-bevy-build.log,cloud-original-comparison-valid-json.log,coastal-original-comparison-valid-json.log,cloud-authored-200000-final.log}` — retained observations, failures and strict comparison; newer oracle/setup and timeout status supplied by scope handoff.
- `data/diagnostics/retained-original-cloud-100000-f23343bb/` — original PNG, runtime, scene dump and capture receipt; compositor capture is distinct from lossy IPC WebP.
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
