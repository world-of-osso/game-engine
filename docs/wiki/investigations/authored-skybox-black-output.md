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
| Cloud active 200000 ms and natural live | Regular `5cc16630` observer exits0 at fixed200000: 70176 sampled authored pixels, UV1.09091/stability. Live exits0 after naturally reaching clock200001ms in197364ms wall wait; four-second observation changes16771 sampled pixels. | Actual visibility and natural animation, not active original-image parity. Earlier diagnostic timeout124 and SETUP failures remain historical. |
| Coastal FDID 525142, 100000 ms | Diagnostic comparison fails450491 pixels; phase-ready regular `5cc16630` capture also fails450489, maximum error.95686, mean.04682. | Both actually sample SkyBatch0 UV≈.5; the first JSON record is SkyBatch22, not batch0. Premature capture is not established as the cause. Numeric image mismatch remains a diagnostic, not a product failure under [cross-engine acceptance](../../specs/godot-conversion.md#cross-engine-acceptance). |
| Corrected dormant/composition oracle `5cc16630` | First SETUP attempts fail30s CASC wait; delayed bounded retries keep that bound and exit0 for Light653/default0, direct-FDID/default0 and authored100000. | Actual fog, reference/dome contributions, transparent authored output and restore proven boundedly. Historical incorrect positive-contribution RED remains; no tolerance relaxation. |

Source cloud keys stay dark until 200000 ms for the selected initial tracks; later selected tracks have different activation times. The April modern-stage/UV coverage remains bounded CPU/source evidence, not active-phase rendered equivalence. Original capture provenance and strict comparison do not close full camera, all-track, full-scene, retained-goal or conversion acceptance. Shared-host contention also limits [[world-entry-stalls#Retained integrated performance — 2026-10-01|retained performance evidence]].

## Native texture-stage color boundary

Original `m2_skybox.wgsl` combines decoded sRGB texture samples in linear space. Native `sky_m2.gdshader` instead inserted gamma encoding before combination and decoding afterward. The production-shader GPU fixture `skybox_linear_combine.gd` isolates that difference with opaque 1×1 RGB `[64,96,128]` textures: single-stage control passes exactly, while original-linear product expects `[9,31,61]` and native returns `[16,36,64]` at all three observed pixels (exit1, unchanged two-code tolerance). Independent Python transfer math confirms the reference; a wrong precomputed green28 guard was corrected to31 before the final RED.

The native sky shader now combines the already-linear `source_color` samples directly (`b3e546e2`). Six GPU witness assertions pass in `sky-linear-combine-green.log`; process exit124 is not shutdown PASS. The complete coastal comparison still reports450481 mismatches after this fix, not evidence that this was the dominant image difference. The other retained genuine fix is the dedicated zero-duration seek/state-phase path (`f23343bb`); see [bounded native proof](../../specs/native-skybox-debug.md#retained-proof-limits--2026-10-01).

### Historical sampler detour and current boundary

`ca400060` introduced static clamp sampling from an incomplete loader trace. `sampler-source-verification.md` falsified that claim: original composited and plain GPU sky uploads explicitly use Repeat; neither the missing Color-case nor premature FrameReady explanation was established. The later forced-mip-0 policy pursued screenshot matching, not an original product requirement, and was removed by independent `ba05872c` (shader hints/static fixture). Post-removal saved native evidence is now recorded below; historical candidate results are not current sampler policy.

Saved native `6b1c7650`/Depot `lc0jr8r3x3` (installed SHA-256 prefix `062521`) has passing stage GPU assertions but several exit124 processes; the coastal functional observer exits0. Its full-image comparator reports451234/921600 failures, max.95686275/mean.04712256, exit1 (`coastal-mip0-original-comparison.log`). Earlier450491/450489/450481/450454 results remain actual numeric diagnostics, not product failures under linked acceptance. User visual review covers the reviewed images only; whole-client acceptance, default map2703 Light data, general shutdown and goal20 remain open. Real performance runs1/2 are coherent observations, not steady acceptance: readiness changes invalidate the nominal settled window and no comparable baseline exists. Other-owner root-format issues and warnings remain untouched.

### Post-`ba05872c` actual native evidence

Native `ee61569f5371b91a54fef8b125d1607bc0024b6b`, Depot `0gvshwskx4`, build exit0 and installed DLL SHA-256 `681b43cdf70ea88b5b02043b1d1558ff6aad19971b38801c0b8e65d9ca421322` are recorded by `final-native-build-inputs.json`/`final-native-build.log` under `/tmp/claude/retained-conversion-20/`. `final-native-mips-results.json` records unchanged DLL hashes and production shader SHA-256 `6652b9ddc064b222cbc728fb126be41c40fbd4cdc52898b779e90473604ef8ef`; corrected supplemental commands and matching DLL are in `final-native-mips-additional-results.json`.

- `final-native-mips-mips.log`: authored RGBA8 mip chain, three explicit-control plus three real production-shader witnesses all assert expected green `(0,128,0,255)` PASS. Process exits124 at timeout60: **not process PASS**, deferred shutdown remains unresolved. Restored native mips are runtime-observed, not only source-observed.
- `final-native-mips-linear.log`: six single-stage/linear-product witnesses PASS with unchanged two-code tolerance; process0. Earlier linear GREEN timeout124 remains historical, not retroactively converted to process PASS.
- Regular `final-native-mips-cloud-{dark,active,live}.log` and `final-native-mips-coastal-{fixed,live}.log`: each process0. Cloud fixed100000 proves transparent54-batch output/change0/stable restore; fixed200000 contributes70074 pixels; natural live reaches200043ms after186779ms wait and changes16793 samples. Coastal fixed100000 contributes114853 pixels at stable UV.5; live changes29022 samples. Bounded observers, not all-track or original-image parity.
- First `final-native-mips-light653.log` command omitted time and ran LIVE with timeout180; natural cloud activation needs200000ms, so exit124 remains a command-boundary timeout. Corrected `final-native-mips-light653-fixed0.log` explicitly requests time0 and exits0 with transparent authored output, radial fog, reference126932/procedural230400 pixels. `final-native-mips-persisted-camera.log` exits0 with saved FOV105/sensitivity.007, coastal fixed100000 and104557 contribution pixels; not offline same-scene live Options runtime proof.
- `final-sky-tests-depot.log`/`final-sky-tests-full.log`: saved filtered core12/native15 tests pass, exit0; input revision `e0f0661f36d63e5863cc317e43923a73fe7825cb` in `final-sky-test-inputs.txt`. Core unused imports and native unused import/two unused-mut warnings remain; not warning-free or full-suite proof.

[Exact native coverage and open gaps](../../specs/native-skybox-debug.md#post-removal-native-evidence--2026-10-01) retain the assertion/process distinction. Cross-engine pixel diagnostics are not blockers under the conversion visual contract; actual numeric failures and unchanged thresholds remain historical evidence. Final `data/diagnostics/retained-performance-20261001/run{3,4}/result.json` and MAIN ledger reconcile performance as follows:

| Case | Stored observation | Proof boundary |
| --- | --- | --- |
| Run3 | Exit1/no timeout, wall436.336s; loading131.018181s/max2308.163ms, transition217.609448s; nominal60.294225s/224 intervals changes readiness despite start/end queues0/0/0 and nine tiles. Seven RSS/HWM markers, unchanged input hashes; endRSS948748KiB/HWM2289608KiB. | **INVALID steady**; missing intermediate snapshot prevents cause attribution. Checkout advanced through other-owner commits. |
| Run4, fixture `6520c861` | Exit1/no timeout, wall176.290s; loading33.356928s/max439.653ms, transition62.578282s; nominal60.201259s/626 intervals. Runtime-captured `first_readiness_change`: terrain.pending_count0/world_objects.pending0/unit_visuals_pending1, unchanged nine parsed tiles; final0/0/0. Seven RSS/HWM markers, unchanged input hashes and checkout; endRSS1001228KiB/HWM2666660KiB. | **INVALID steady**; first stored violation is a unit visual job, not terrain/object work. Fixture string `world_objects.pending changed` is imprecise. Capture is runtime-proven, not pending; predicate/window unchanged. |

Run1/2 gaps remain. No comparable baseline, controlled improvement or product-budget claim follows from these shared-host runs; transferred character/clothing remains open. Default Map2703 data, deferred shutdown, retained goal20 and full conversion remain **OPEN**. Approved offline access/live values are now bounded runtime-observed below; settled performance and comparable baseline remain unproved. Owner index/log and other dirty files are outside this update.

## Approved offline Options — bounded runtime observed

Explicit `ask_questions` answer **Allow offline Options** approves existing main-menu/Options access without authentication or Log Out; existing UI ownership is unchanged. [Exact requirement, build hashes, retained logs and proof matrix](../../specs/native-skybox-debug.md#approved-offline-options--bounded-runtime-observed) are recorded in the native spec.

Two production boundaries were required: `744fa656` widens physical Escape access to actual SkyboxDebug, preserves InWorld `gameplay_input_allowed` and passes the logged-in flag for offline Log Out visibility; `09dc90ad` yields sky input while parent `GameMenuUI` exists and resets drag. Historical Depot `13lg139ksk` remains access-build provenance; new Depot `dlhhzs6pll` exits0 at `09dc90ad`, DLL SHA-256 `c2669a283a38568a7ca053d41c2bffc4b931de7362d2c1a7b65c16f53c437ee0`, full build stdout/stderr retained.

`/tmp/claude/retained-conversion-20/offline-options-red.log` retains genuine old-menu RED exit1. `offline-options-access-*` shows Main opens but Options click is consumed by sky input, RED exit1. `offline-options-pointer-*` reaches Camera after the pointer fix, then fails a nonproduction Handle-shape oracle: expected887.666671342734, actual887. Test-only `48132be1` updates `80d2762d` by removing that unguarded subpixel assertion; no fitted pixel tolerance or proven rounding cause.

Final `offline-options-semantic-*` retains actual UI105/.007 → 110/.004 → restore, canonical data/live FOV and real orbit yaw assertions −.016/−.028. Escape Options→Main→Closed, Done/Resume, underlying offline sky camera/M2, no authentication/Log Out and unchanged legacy file are retained. **All final assertions PASS; process124 at timeout120.** Offline access/live values are bounded runtime-observed, not whole-case PASS, normal exit or all Options acceptance. Independent verifier remains active/pending final-commit review, not accepted. Historical persisted-camera and sampler/process evidence is unchanged; general shutdown, default-data, settled performance/comparable baseline and full conversion remain **OPEN**.

## Sources

- [Conversion acceptance SSOT](../../specs/godot-conversion.md#cross-engine-acceptance) — direct user clarification; no whole-client acceptance.
- `/tmp/claude/retained-conversion-20/main-proof-ledger.md`, `sampler-source-verification.md`, `seek-independent-verification.md` — MAIN reconciliation and primary-source corrections; early exploratory claims are superseded where contradicted.
- `/tmp/claude/retained-conversion-20/{mip0-correction-build-inputs.txt,mip0-correction-native-build.log,skybox_static_sampler-mip0-final.log,skybox_mipmap_sampling-mip0-final.log,skybox_debug_screen-mip0-final.log,coastal-mip0-original-comparison.log}` — saved candidate provenance, assertion/process boundaries and numeric diagnostic.

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
