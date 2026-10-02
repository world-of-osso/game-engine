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
- [ ] Escape may open the existing main-menu/Options overlay in offline SkyboxDebug, without authentication or Log Out. Camera FOV and mouse-sensitivity edits persist to existing settings and apply to the retained scene immediately; menu input must not orbit the camera. Preserve existing Escape/Done/Resume semantics and InWorld menu gating.
- [ ] Display live authored sky geometry, textures/materials and animation, maintaining the original camera-relative sky behavior and time-dependent evaluation. Fixed bone time preserves original `f32` conversion/remainder for positive-duration clips and the requested `f32` phase for zero-duration clips; it must not enter ordinary advancement, which clamps zero-duration time to zero.
- [ ] Preserve texture FDIDs/images, source flags/identity metadata, render priority, culling, blending, geometry, camera and animation. Visual acceptance follows [the conversion contract](godot-conversion.md#cross-engine-acceptance); forced mip-0 sampling is not a product requirement.
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
- `godot/shaders/sky_m2.gdshader` — authored texture-stage combinations retain the original linear color domain without extra gamma roundtrip; independent owner removed screenshot-driven forced-mip-0 policy in `ba05872c` (post-removal saved runtime evidence below).

## Tests asserting this spec

- `godot/core/src/startup_args_data_tests.rs` — parsed values, boundaries, verification, first-value/target semantics, missing/invalid values and mutual exclusion. Saved Depot core GREEN at `f23343bb` covers these parser cases; not native parity.
- `godot/core/src/skybox_debug_data.rs` tests — cached LightSkybox653 → FDID5412968/flags15, LightParams5615 → skybox653, and explicit unknown-row error; not runtime/render proof.
- `launcher/tests/process.rs` — three `skybox` process cases asserting exact recorded Godot argv with fake build/Godot executables; not native runtime proof.
- `src/scenes/skybox_debug/tests.rs` — legacy source, composition, FOV and camera-relative behavior references; not native acceptance.
- `godot/tests/skybox_debug_screen.gd` — production observer for cached source cases, real orbit/zoom, composition and hide/restore image attribution. Bounded coastal and corrected cloud dark/active/live runtime proof is retained below; historical incorrect cloud contribution oracles remain recorded. Attribution is not original-expected pixel parity.

- `godot/tests/skybox_linear_combine.gd` — real production-shader GPU single-stage control and two-stage linear-product contract; source difference reproduced before correction. Not complete asset/image parity.
- `godot/tests/skybox_static_sampler.gd` and `godot/tests/skybox_mipmap_sampling.gd` — synthetic GPU sampling diagnostics. Historical forced-mip-0 witnesses do not define product acceptance. Independent `ba05872c` deletes the static sampler fixture and restores the mip-consumption witness; post-removal synthetic GPU assertions and exact process exits are retained below.

## Retained proof limits — 2026-10-01

Primary evidence: `/tmp/claude/retained-conversion-20/main-proof-ledger.md`, `sampler-source-verification.md`, `seek-independent-verification.md`, and the exact logs below. Early exploratory claims superseded by MAIN/source verification are not current findings. Goal20 remains **OPEN/bounded**, not whole-project PASS; no transferred capability is closed.

| Exact scope | Retained result | Proof limit |
|---|---|---|
| Fixed phase/state seek | `ba3cd12f` state-backed zero-duration RED; `f23343bb` dedicated seek GREEN, core12/native15, exit0 (`sky-state-pose-red-full.log`, `sky-helpers-integrated-green-{depot,full}.log`) | Genuine zero-duration seek/state-phase fix; not all-track acceptance |
| Cloud FDID5412968, authored0/100000 | Source-selected 54 batches have opacity0; original100000 is also black, comparator0/921600 failed pixels (`cloud-zero-opacity-root.md`, `cloud-zero-opacity-tracks.json`, `cloud-original-comparison-valid-json.log`) | Authored dark phases, not a production render failure; earlier positive-contribution oracle was wrong |
| Cloud active200000/live | Fixed observer exit0,70176 contribution pixels; natural live reaches200001ms and changes16771 samples (`cloud-authored-200000-retry.log`, `cloud-live-active-matrix.log`) | Bounded source/animation visibility, not all phases |
| Source/input/fog/layers/camera | Coastal fixed/live observers exit0; corrected source/default composition retries exit0; persisted FOV105/sensitivity.007 observer exit0 (`coastal-authored-100000.log`, `coastal-live.log`, `cloud-light-default-0-retry.log`, `cloud-fdid-default-0-matrix.log`, `persisted-camera-105-007.log`) | Preserve exact fixture scopes; same-scene live Options remains unsupported offline |
| Linear texture-stage arithmetic | `b3e546e2` removes extra gamma roundtrip; six GPU witnesses pass (`sky-linear-combine-green.log`), process124 | Genuine stage-arithmetic fix, not general shutdown or full-image acceptance |
| Historical sampler detour | `ca400060` clamp candidate contradicted original Repeat source; subsequent all-sky forced-mip-0 candidate was introduced to chase screenshots | Wrong detour, not product policy. Independent `ba05872c` removes forced-mip-0 shader hints and the static fixture; post-removal runtime evidence is retained below |
| Native `6b1c7650fb975b884adcfbb3570c833c11354c40` | Depot `lc0jr8r3x3`, installed SHA-256 prefix `062521`; build provenance `mip0-correction-build-inputs.txt`/`mip0-correction-native-build.log`. Stage GPU assertions pass, several processes124; coastal functional observer0 (`skybox_static_sampler-mip0-final.log`, `skybox_mipmap_sampling-mip0-final.log`, `skybox_debug_screen-mip0-final.log`) | Saved candidate evidence, not proof of independently changing current policy; general shutdown deferred |
| Coastal image diagnostics | 450491 initial,450489 phase-ready,450481 after linear correction,450454 clamp candidate; `6b1c7650` comparator451234/921600, exit1, max.95686275/mean.04712256 (`coastal-mip0-original-comparison.log`) | Actual numeric diagnostic failures retained, not product failures under linked acceptance. No threshold relaxation or fitted mask; premature FrameReady, missing Color-case and clamp explanations are not established |
| Real performance runs1/2 | Coherent distributions/memory observations; run1 pending5803; run2 nominal60.077382s/289 intervals with changing readiness (`performance-run1-verification.md`, `performance-runner{1,2}.log`, `perf-settled-boundary.md`) | Neither proves steady performance; no comparable baseline or invented product budget |

## Post-removal native evidence — 2026-10-01

Saved evidence under `/tmp/claude/retained-conversion-20/`: `final-native-build-inputs.json`/`final-native-build.log` record native `ee61569f5371b91a54fef8b125d1607bc0024b6b`, Depot `0gvshwskx4`, build exit0, installed DLL SHA-256 `681b43cdf70ea88b5b02043b1d1558ff6aad19971b38801c0b8e65d9ca421322`. `final-native-mips-results.json` records unchanged before/after DLL hashes and shader SHA-256 `6652b9ddc064b222cbc728fb126be41c40fbd4cdc52898b779e90473604ef8ef`; `final-native-mips-additional-results.json` records the same DLL for corrected Light653 and persisted-camera runs. These replace the source-only post-`ba05872c` boundary, not historical candidate results.

| Exact saved case / full log | Assertions / observation | Process boundary |
|---|---|---|
| `final-native-mips-mips.log` | Real RGBA8 authored mip chain: three explicit-control and three production-shader witnesses all return expected green `(0,128,0,255)`; six assertions PASS, native mips consumed | Exit124 at timeout60; **not process PASS**, shutdown deferred |
| `final-native-mips-linear.log` | Three single-stage `[64,96,128]` and three linear-product `[9,31,61]` witnesses PASS at unchanged two-code tolerance | Exit0; bounded arithmetic, not full-scene parity |
| `final-native-mips-cloud-{dark,active,live}.log` | Regular cloud fixed100000: all54 selected batches transparent, shown/hidden change0 and stable restore; fixed200000:70074 contribution pixels; natural live reaches200043ms after186779ms wait and changes16793 samples | Each exit0; live timeout300, not a fixed-time shortcut; all-track coverage unproved |
| `final-native-mips-coastal-{fixed,live}.log` | Fixed100000 UV.5/stable and114853 contribution pixels; live UV/render change29022 samples | Each exit0; bounded observers, no original-expected pixel oracle |
| `final-native-mips-light653.log` → `final-native-mips-light653-fixed0.log` | First command omitted time, therefore LIVE; natural source activation requires200000ms. Corrected command explicitly sets `--skybox-time-ms 0`: transparent authored output, radial fog and reference126932/procedural230400 contribution pixels | First timeout180 exits124, preserved command-boundary timeout; corrected fixed0 exits0, not live completion proof |
| `final-native-mips-persisted-camera.log` | Saved FOV105/sensitivity.007 with coastal fixed100000 UV.5 and104557 contribution pixels | Exit0; persisted camera only, not unsupported offline same-scene live Options |
| `final-sky-tests-depot.log` / `final-sky-tests-full.log` | Saved `--lib sky` core12/native15 pass; `final-sky-test-inputs.txt` records `e0f0661f36d63e5863cc317e43923a73fe7825cb` | Exit0; filtered tests, not full suite or warning-free: core unused imports plus native unused import/two unused-mut warnings retained |

Cross-engine pixel comparisons remain optional diagnostics under [visual acceptance](godot-conversion.md#cross-engine-acceptance), not blockers; retained numeric failures and thresholds are unchanged. Performance results are retained in `data/diagnostics/retained-performance-20261001/run{3,4}/result.json` and the MAIN proof ledger. Run3 exits1/no timeout, wall436.336s: loading131.018181s/max2308.163ms, transition217.609448s; nominal60.294225s/224 intervals is **INVALID steady** despite zero start/end queues and nine tiles. Its missing intermediate snapshot prevents cause attribution. Seven RSS/HWM markers and input hashes are intact; endRSS948748KiB/HWM2289608KiB; checkout advanced through other-owner commits, not a frozen source run.

Run4 at fixture `6520c861` runtime-proves first-invalid-snapshot capture without changing the readiness predicate/window: exit1/no timeout, wall176.290s; loading33.356928s/max439.653ms, transition62.578282s; nominal60.201259s/626 intervals is **INVALID steady**. Stored `first_readiness_change` has terrain.pending_count0/world_objects.pending0/unit_visuals_pending1 and the same nine parsed tiles; final queues are0/0/0. Existing fixture error string `world_objects.pending changed` is imprecise: the first stored violation is a unit visual job, not terrain/object work. Seven RSS/HWM markers and before/after input hashes are intact, checkout unchanged; endRSS1001228KiB/HWM2666660KiB. Neither run supplies a comparable baseline, controlled improvement or product-budget proof. Default Map2703 data, unsupported offline live Options and deferred shutdown remain **OPEN**; retained goal20 and full conversion remain **OPEN**.

## Known gaps (current cycle)

- [x] Final bounded scope audit16–20: independent report `/tmp/claude/retained-conversion-20/final-independent-verification.md` accepts source/render evidence and measurement coherence at `ded19eb7`, while rejecting settled-window acceptance. This completes evidence accounting, not retained/full-conversion acceptance. Mip timeout124, inherited format/build warnings and remaining gaps stay open.
- [ ] Supply authoritative default `Light.map2703` data, currently absent: explicit default-source blocker, no fallback.
- [ ] Complete remaining source/animation/input coverage and unsupported same-scene live Options proof without promoting bounded FOV105/.007 evidence to all options.
- [ ] Establish a valid settled performance window and comparable baseline; readiness changes invalidate steady claims.
- [ ] Complete retained skybox acceptance under the linked conversion contract. General shutdown is deferred; transferred character/clothing, water/appearance/tooling and whole conversion remain open.

## Out of scope

- Authentication, global UI, new controls, new CLI flags and platform expansion: not part of original offline debug behavior.
- Hardcoded fallback asset substitution: forbidden, not a compatibility option.
- Whole-client conversion acceptance: governed by the broader conversion spec.
