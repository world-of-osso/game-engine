# Native skybox debug

Offline Godot `--screen skyboxdebug` must preserve the original authored-sky debug behavior in `src/scenes/skybox_debug/`, subject to the no-fallback requirement below. CLI parsing lives in `src/startup_args_data.rs`; launcher routing lives in `launcher/src/main.rs`. This is a feature contract within [Godot conversion](godot-conversion.md), not native completion evidence. See [skybox architecture](../wiki/systems/skybox.md) for source lookup and rendering boundaries.

## What it must do

### CLI

- [ ] Accept exactly the original paired `--skybox-fdid <u32>`, `--light-skybox-id <u32>`, `--skybox-time-ms <u32>` and valueless `--skybox-verify`. IDs and milliseconds accept 0 through 4294967295; parsing preserves the exact numeric value without conversion or clamping.
- [ ] Forced FDID and LightSkybox ID are mutually exclusive in either order, with an explicit error naming both flags. Missing values, invalid unsigned 32-bit values and unknown options produce meaningful errors.
- [ ] Preserve first-target and first-value semantics of existing client options. Repeated paired options still require a value; later values do not replace the first. Verification defaults false and repeated verification remains true.
- [ ] Route these options after Godot's `--`, preserving engine order, client order, explicit-separator behavior, non-UTF8 argument transport and existing routing errors. Verification consumes no following engine argument. Introduce no new flags.

### Offline scene and source selection

- [ ] Enter the selected authored-sky debug scene without authentication, server contact or global UI. Use the selected authored source unless an original forced FDID or LightSkybox ID selects another source; apply the millisecond time override when supplied.
- [ ] Unknown or unavailable authored sources fail explicitly. Never substitute a hardcoded fallback asset or invent a default model or light data.
- [ ] Default composition preserves original reference objects and authored/procedural composition semantics. `--skybox-verify` isolates authored sky against black without reference objects, visible procedural baseline or procedural fog.

### Camera and live rendering

- [ ] Start orbit focus at (0, 1, 0), distance 7.5; honor the existing camera FOV option. Left drag orbits and wheel zooms. Add no new controls.
- [ ] Display live authored sky geometry, textures/materials and animation, maintaining the original camera-relative sky behavior and time-dependent evaluation. Fixed bone time preserves original `f32` conversion/remainder for positive-duration clips and the requested `f32` phase for zero-duration clips; it must not enter ordinary advancement, which clamps zero-duration time to zero.
- [ ] Preserve original `f23343bb` sky sampling on every batch: Repeat U/V, linear mip 0 only. `src/rendering/character/m2_texture_composite.rs::build_composited_texture_handle` explicitly sets Repeat and linear sampling, including plain GPU uploads. Advanced `load_repeat_texture` uses the same GPU loader: `src/asset/blp.rs::gpu_image_from_dxtn` uploads only `dxtn.images.first()` with `Image::new`; RGBA uploads also contain only mip 0. All four native sky sampler slots use `filter_linear, repeat_enable`, even when shared native images retain authored mips for other consumers. No sampler classifier, cache/image stripping, new flags or fallback. Preserve texture FDIDs/images, source flags/identity metadata, render priority, culling, blending, geometry, camera and animation.
- [ ] Prove rendered authored content and real orbit/zoom input independently in the running native scene. Parser acceptance, scene dispatch or a nonempty screenshot alone cannot establish parity.

## How it works

- [Skybox architecture](../wiki/systems/skybox.md)
- [Godot conversion architecture and evidence](../wiki/systems/godot-conversion.md)
- [Original authored lookup](../skybox-authored-lookup.md)

## Implementation inventory

- `src/startup_args_data.rs` — shared flat `StartupArgs` fields and explicit CLI parsing.
- `godot/core/src/skybox_debug_data.rs` — strict cached LightSkybox/LightParams reads through the original WDC5 decoder and shared flags; missing rows fail explicitly.
- `godot/rust/src/skybox_debug/source.rs` — same-map clear-Light lookup and cached M2 selection; no other-map or hardcoded-model substitution. Existing legacy light priority uses the same shared pure function.
- `launcher/src/main.rs` — original options routed to Godot user arguments.
- `src/main.rs` — preserved Bevy skybox resource insertion.
- `src/scenes/skybox_debug/mod.rs` — original scene, composition and camera behavior.
- `src/scenes/skybox_debug/resolution.rs` — original authored lookup and existing source-fallback boundary.
- `godot/rust/src/startup.rs` — native offline screen dispatch; options reach the controller without account authentication.
- `godot/rust/src/skybox_debug.rs` — original composition flags, current orbit camera, real mouse input and authored material clock; live options come from the existing `CameraOptionsFile`.
- `godot/rust/src/character_select/sky.rs` — shared authored M2 loading with exact source metadata and fixed-time material/bone sampling; character-select default entry remains unchanged.
- `godot/rust/src/animation/mod.rs` — dedicated validated fixed-time seek samples sky bones without ordinary clip advancement; ordinary playback remains unchanged.
- `godot/rust/src/skybox_debug/environment.rs` — original procedural/reference layer; custom radial linear fog is shader-owned, not stock Environment fog. Physical-unit/image equivalence remains unproved.
- `godot/shaders/sky_m2.gdshader` — all four stages repeat with linear/no-mip sampling; authored texture-stage combinations retain the original linear color domain without extra gamma roundtrip. Rust specializes render mode only; shared texture/cache policy remains unchanged.

## Tests asserting this spec

- `godot/core/src/startup_args_data_tests.rs` — parsed values, boundaries, verification, first-value/target semantics, missing/invalid values and mutual exclusion. Saved Depot core GREEN at `f23343bb` covers these parser cases; not native parity.
- `godot/core/src/skybox_debug_data.rs` tests — cached LightSkybox653 → FDID5412968/flags15, LightParams5615 → skybox653, and explicit unknown-row error; not runtime/render proof.
- `launcher/tests/process.rs` — three `skybox` process cases asserting exact recorded Godot argv with fake build/Godot executables; not native runtime proof.
- `src/scenes/skybox_debug/tests.rs` — legacy source, composition, FOV and camera-relative behavior references; not native acceptance.
- `godot/tests/skybox_debug_screen.gd` — production observer for cached source cases, real orbit/zoom, composition and hide/restore image attribution. Bounded coastal runtime proof is retained below; cloud authored attribution fails. Attribution is not original-expected pixel parity.

- `godot/tests/skybox_linear_combine.gd` — real production-shader GPU single-stage control and two-stage linear-product contract; source difference reproduced before correction. Not complete asset/image parity.
- `godot/tests/skybox_static_sampler.gd` — shares the actual production Fog21 shader on an isolated quad with synthetic textures. Original-repeat witnesses expect wrapped blue below U=0 and wrapped red above U=1; authored smaller green mip levels distinguish mip-0-only red sampling from mip selection. Independent repeat/linear/no-mip controls must pass. This tests GPU sampling behavior, not shader source substrings, whole authored-image parity or all-batch coverage. Existing fixed-phase cases remain unchanged.
- `godot/tests/skybox_mipmap_sampling.gd` — explicit mipmap control expects authored green smaller levels; unchanged production shader expects original red mip 0 with the same synthetic texture, UVs and two-code tolerance. Permanent original-contract regression, not asset or full-scene parity. Skin and linear-combine fixtures remain unchanged.

## Retained proof limits — 2026-10-01

Evidence: `/tmp/claude/retained-conversion-20/main-proof-ledger.md` and adjacent named logs. These are revision-scoped saved results, not a fresh acceptance run. Native runtime evidence uses `f23343bb` with observer fixture `0eb4784e`; installed build-ID/tool-return provenance still needs recovery, not inference from checkout HEAD. Contract checkboxes remain open.

| Exact scope | Retained result | Proof limit |
|---|---|---|
| Fixed phase/state seek | `ba3cd12f` state-backed zero-duration behavioral RED (`sky-state-pose-red-full.log`: sampled pose 0); `f23343bb` dedicated seek GREEN (`sky-helpers-integrated-green-{depot,full}.log`), saved command exit 0, core 12/native 15 tests passed | Concrete state/phase/source/options tests, not all-track or rendered parity; two pre-existing terrain `unused_mut` warnings remain |
| Coastal FDID525142, authored-only, fixed 100000 ms | MAIN exit 0; UV translation 0.5 stable; 114853 sampled authored-contribution pixels; fixed stability, hide/restore and physical orbit checks (`coastal-authored-100000.log`) | Bounded rendered attribution/input proof, not original-expected pixels or default composition |
| Coastal FDID525142, authored-only, live | MAIN exit 0; UV 0.04731 → 0.068335; 27511 changed sampled pixels (`coastal-live.log`) | One concrete UV/material track; no time-identical live hide/restore assertion or all-track proof |
| Cloud FDID5412968, authored-only, fixed 0 and 100000 ms | Both exit 1, authored contribution 0; fixed UV respectively 0 and 0.545455 (`cloud-authored-first.log`, `cloud-authored-100000.log`) | Loaded 54 surfaces and advancing phase do not prove visibility; zero phase alone does not explain failure; no full-cloud visible/parity claim |
| Reference comparator `f6aa48f5` | MAIN exit 0 (`comparator-selftest.log`) | Synthetic offline self-test only; no matching current original PNG comparison |
| Historical static Fog21 sampler, native artifact `f2337ce` | MAIN exit 1 (`sky-static-sampler-typed-red.log`): controls PASS; four repeat witnesses failed the then-wrong clamp oracle; three minification witnesses genuinely failed original red-mip0 sampling by producing green | Primary `f23343bb` source withdraws the clamp and advanced-authored-mip claims. Repeat was not a production defect. Prior untyped conditional-array setup failure is separate. Actual Fog21 has 125 vertices, U −0.9992379 through 2.0007615, one texture, no UV animation and false secondary flags. Corrected repeat oracle and production no-mip policy have no new GPU proof |
| Withdrawn `ca400060` sampler candidate, compiled native `74ab` | Static-only clamp specialization implemented the wrong contract; removed by this correction | Old runtime/build evidence does not cover corrected source. Fresh MAIN Depot build and both corrected GPU fixtures required; no GPU GREEN or full-parity claim |
| Whole coastal comparison before this correction | 450454 mismatches retained for the wrong-branch candidate; changing that branch changed little. Earlier `b3e546e2` comparison retained 450481/921600 failing RGB pixels | Historical failures remain; fresh corrected Depot artifact must be compared against original with unchanged whole-image criteria |

## Known gaps (current cycle)

- [ ] MAIN builds corrected source on Depot, runs both original-contract sampler regressions, then repeats strict whole-coastal comparison. Existing compiled `74ab` includes the withdrawn clamp branch; it cannot prove this correction. If strict images still fail after this third whole-scene candidate, stop code changes and ask the diagnostic question rather than invent another fix.

Static sampler source correction does not close the retained whole-coastal 450481 RGB mismatches. The `b3e` color correction proves a synthetic component, not dominant mismatch attribution; full-image comparison remains pending.

- [ ] Establish cloud authored-render failure root cause and matching original evidence. [Historical Bevy black-output investigation](../skyboxdebug-black-screen-brief-2026-04-11.md) documents earlier failures/fixes, not an established cause for this retained failure. No current original matching lossless PNG or verified original binary-to-revision provenance exists; preserve the original oracle and comparison criteria.
- [ ] Supply authoritative default `Light.map2703` data, currently absent: explicit default-source blocker, not permission to substitute another map/model. Default composition, rendering special cases and physical-unit/image equivalence remain unproved; resolve the legacy source-fallback semantic boundary without a replacement default.
- [ ] Run nondefault persisted/live camera options and complete independent rendered/input/material/animation coverage. Saved effective FOV90/sensitivity0.003 and pure option tests do not establish those runtime cases.
- [ ] Run integrated loading/frame-time/memory performance with comparable baseline. Concurrent GPU clients and unavailable requested disabled VSync preclude an isolated performance claim from retained sky runs.
- [ ] Complete retained skybox acceptance without closing whole conversion. Transferred water/appearance/remaining tooling are excluded here and unresolved; whole conversion remains **OPEN**.

## Out of scope

- Authentication, global UI, new controls, new CLI flags and platform expansion: not part of original offline debug behavior.
- Hardcoded fallback asset substitution: forbidden, not a compatibility option.
- Whole-client conversion acceptance: governed by the broader conversion spec.
