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
- [ ] Explicit user approval (`ask_questions`: **Allow offline Options**) permits Escape to open the existing main-menu/Options overlay in offline SkyboxDebug, without authentication or Log Out. Camera FOV and mouse-sensitivity edits persist to existing settings and apply to the retained scene immediately; menu input must not orbit the camera. Preserve existing Escape/Done/Resume semantics and InWorld menu gating.
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
- `godot/tests/skybox_authored_pose.gd` — pinned cached coastal FDID525142 bone6, sequence0/duration320000: requested17040001ms → original f32 phase80000ms. Independently byte-derived authored translation/packed rotation assert production loader/player/Skeleton3D local and skeleton-space transforms over four observations without pose writes. MAIN command: `./scripts/agent/agent-run sky-authored-pose "$GODOT_BIN" --path godot -s res://tests/skybox_authored_pose.gd -- --screen skyboxdebug --skybox-fdid 525142 --skybox-time-ms 17040001 --skybox-verify`. Fixture `b4888571`/`81ec36c3` has actual pinned runtime proof below; not natural-live, zero-duration authored, GPU deformation or pixel-parity proof.

- `godot/tests/skybox_live_options.gd` (`80d2762d`) — physical Escape/menu/Options input, FOV105/sensitivity.007 → FOV110/sensitivity.004 → restore; asserts retained source/data, camera and input behavior. MAIN observed genuine missing-menu RED exit1 before the access-gate change. Current bounded real-UI runtime and normal exit are proven below; fixture presence/build success alone is not GREEN.

- `godot/tests/skybox_linear_combine.gd` — real production-shader GPU single-stage control and two-stage linear-product contract; source difference reproduced before correction. Not complete asset/image parity.
- `godot/tests/skybox_static_sampler.gd` and `godot/tests/skybox_mipmap_sampling.gd` — synthetic GPU sampling diagnostics. Historical forced-mip-0 witnesses do not define product acceptance. Independent `ba05872c` deletes the static sampler fixture and restores the mip-consumption witness; post-removal synthetic GPU assertions and exact process exits are retained below.

## Retained proof limits — 2026-10-01

Primary evidence: `/tmp/claude/retained-conversion-20/main-proof-ledger.md`, `sampler-source-verification.md`, `seek-independent-verification.md`, and the exact logs below. Early exploratory claims superseded by MAIN/source verification are not current findings. Goal20 remains **OPEN/bounded**, not whole-project PASS; no transferred capability is closed.

| Exact scope | Retained result | Proof limit |
|---|---|---|
| Fixed phase/state seek | `ba3cd12f` state-backed zero-duration RED; `f23343bb` dedicated seek GREEN, core12/native15, exit0 (`sky-state-pose-red-full.log`, `sky-helpers-integrated-green-{depot,full}.log`) | Genuine zero-duration seek/state-phase fix; not all-track acceptance |
| Cloud FDID5412968, authored0/100000 | Source-selected 54 batches have opacity0; original100000 is also black, comparator0/921600 failed pixels (`cloud-zero-opacity-root.md`, `cloud-zero-opacity-tracks.json`, `cloud-original-comparison-valid-json.log`) | Authored dark phases, not a production render failure; earlier positive-contribution oracle was wrong |
| Cloud active200000/live | Fixed observer exit0,70176 contribution pixels; natural live reaches200001ms and changes16771 samples (`cloud-authored-200000-retry.log`, `cloud-live-active-matrix.log`) | Bounded source/animation visibility, not all phases |
| Source/input/fog/layers/camera | Coastal fixed/live observers exit0; corrected source/default composition retries exit0; persisted FOV105/sensitivity.007 observer exit0 (`coastal-authored-100000.log`, `coastal-live.log`, `cloud-light-default-0-retry.log`, `cloud-fdid-default-0-matrix.log`, `persisted-camera-105-007.log`) | Preserve exact fixture scopes; historical InWorld-only access prevented offline same-scene live Options; later bounded runtime proof below supersedes that access gap |
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
| `final-native-mips-persisted-camera.log` | Saved FOV105/sensitivity.007 with coastal fixed100000 UV.5 and104557 contribution pixels | Exit0; persisted camera only, not offline same-scene live Options runtime proof |
| `final-sky-tests-depot.log` / `final-sky-tests-full.log` | Saved `--lib sky` core12/native15 pass; `final-sky-test-inputs.txt` records `e0f0661f36d63e5863cc317e43923a73fe7825cb` | Exit0; filtered tests, not full suite or warning-free: core unused imports plus native unused import/two unused-mut warnings retained |

Cross-engine pixel comparisons remain optional diagnostics under [visual acceptance](godot-conversion.md#cross-engine-acceptance), not blockers; retained numeric failures and thresholds are unchanged. Performance results are retained in `data/diagnostics/retained-performance-20261001/run{3,4}/result.json` and the MAIN proof ledger. Run3 exits1/no timeout, wall436.336s: loading131.018181s/max2308.163ms, transition217.609448s; nominal60.294225s/224 intervals is **INVALID steady** despite zero start/end queues and nine tiles. Its missing intermediate snapshot prevents cause attribution. Seven RSS/HWM markers and input hashes are intact; endRSS948748KiB/HWM2289608KiB; checkout advanced through other-owner commits, not a frozen source run.

Run4 at fixture `6520c861` runtime-proves first-invalid-snapshot capture without changing the readiness predicate/window: exit1/no timeout, wall176.290s; loading33.356928s/max439.653ms, transition62.578282s; nominal60.201259s/626 intervals is **INVALID steady**. Stored `first_readiness_change` has terrain.pending_count0/world_objects.pending0/unit_visuals_pending1 and the same nine parsed tiles; final queues are0/0/0. Historical fixture error string `world_objects.pending changed` was imprecise: the first stored violation is an outstanding replicated-unit appearance request, not terrain/object work. The counter alone does not identify its entity or distinguish worker loading from completion awaiting main-thread consumption. Seven RSS/HWM markers and before/after input hashes are intact, checkout unchanged; endRSS1001228KiB/HWM2666660KiB. Neither run supplies a comparable baseline, controlled improvement or product-budget proof. Default Map2703 data and general shutdown beyond the owned Options fixture remain **OPEN**; offline access/live values are now bounded runtime-observed below; retained goal20 and full conversion remain **OPEN**.

## Readiness phase diagnostics — bounded run5/6 observations, 2026-10-02

- Replicated-unit appearance requests emit one main-thread `UNIT_VISUAL_REQUEST` record with `server_id`, `request_id`, `existing_appearance`, `describe_unit`, `observed_process_frame` and `observed_ticks_usec`. `existing_appearance` means an appearance existed before this request; false means the stored appearance was `None`, not proof of a new unit node. Description uses the existing helper, not gear-correctness evidence.
- Matching result consumption emits one `UNIT_VISUAL_RESULT_CONSUMED` record with `server_id`, `request_id`, `existing_visual`, `describe_unit`, `prepared_result_ok`, `visual_attached` and the same clock fields. Request ID correlates the request's initial/replacement classification without extra state. `existing_visual` observes a visual before attachment, a distinct boolean from prior appearance. `prepared_result_ok` observes the worker result only; `visual_attached` observes the visual stored after the existing attachment function, not appearance correctness. Detached/stale results are not matching replicated-unit consumption events.
- Every existing workload snapshot adds `observed_process_frame` and `observed_ticks_usec`. First-invalid metadata is captured immediately after its account-state sample, before predicate/tile comparison; other snapshot metadata observes snapshot construction. Events use Godot Engine process-frame counter and Time monotonic microseconds since engine startup, shared with the fixture in that process; not wall time, runner launch-relative time, worker-completion time or GPU/presentation time. Request clocks are observed at the logging boundary after enqueue. Result clocks are observed at the logging boundary after the result attachment attempt, not at worker completion or the exact request-take time; `visual_attached` remains the actual new-visual presence after that attempt, which frees the previous visual on both success and failure. Same-frame records still require tick ordering; these sequential observations are not atomic state captures.
- Readiness/tile failures use broad diagnostics rather than object-queue attribution. Existing external fields, readiness predicate, parsed-tile equality, measurement window and limits remain unchanged; no queue, priority, scheduling, clock mutation or appearance implementation changes. Logging has observer overhead and makes no performance-improvement claim.
- Behavioral Python RED/GREEN uses a real runner/child process with a concrete valid `PERF_REPORT`: object pending0, readiness changed, unit visuals1. This proves diagnostic wording only. Native compilation and run5/6 event-to-sample observations are retained below; historical run4 cannot be attributed retrospectively. No character/clothing correctness or final acceptance claim.

## Unit lifecycle diagnostics — bounded run6 observations, 2026-10-02

- `godot/rust/src/lib.rs` logs `REPLICATION_REMOVAL` on existing projection despawn/nonunit-removal branches: server ID, reason, snapshot presence, optional current ModelDisplay ID and NPC/Player presence. Despawn has no surviving snapshot; absent observations are `None`, not false component-presence claims.
- `godot/rust/src/world.rs` logs `WORLD_UNIT_REMOVED` only for an actual map-entry removal and `WORLD_UNIT_RESET` once per entry cleared by reset; empty resets/absent-ID removals produce no unit-removal event. Each includes server ID, prior appearance presence/description and pending request presence/ID. `UNIT_APPEARANCE_CLEAR` records only a changed Some → None appearance before assignment, with those prior fields plus current snapshot ModelDisplay Option and NPC/Player presence. Unchanged upserts emit no lifecycle event. No snapshot is available inside remove/reset.
- All events observe main-thread process-frame and monotonic microsecond clocks at logging boundaries. Existing request/result data semantics, appearance behavior and workload remain unchanged; no API/CLI/env flag, protocol, queue or suppression changes. Logs distinguish an actual removal/reset before a same-ID request from retained-node appearance clearing; they do not establish why server relevance or component state changed. Clock stamps are not worker-completion observations.
- Supplied run5 first-invalid sample: frame2045/tick90286277, terrain0/objects0/visuals1. Latest matching NPC4294959785/display36654 request121: queued frame2044/tick90178477, consumed frame2045/tick90335984. All14 same-ID requests report `existing_appearance=false`; removal/reinsert versus retained appearance clear remains unresolved. Future logs cannot retrospectively prove run5 cause.
- Diagnostic-only `d52d9c0f` adds lifecycle observations without changing workload, readiness predicate, measurement window, queues or appearance behavior. Actual run6 evidence follows; independent audit accepts bounded source/emission/provenance, not steady performance.

### Actual run5/6 evidence

Evidence: `/tmp/claude/retained-conversion-20/CURRENT.md`, `performance-run6-attribution-summary.json`, and immutable `data/diagnostics/retained-performance-20261001/run{5,6}/result.json` / `stdout.log`. Primary results/events supersede stale source-only or runtime-pending headings. Run6 Depot `p7prn8z8k6` build0 records source observation `0b3e829e526b13236aff676275b6ae6c1ff82bcb`, native SHA-256 `2c9b919a73633ef8ccf8955c5a51c3b3ace0b70b375c0525ad166821ea7d87d8`; this is not a frozen-checkout or deployed-server-source attestation. Run5 used previous Depot `4mpjg3p4jr`, not this artifact.

| Saved case | Actual observation | Proof limit |
|---|---|---|
| Run5 | Child1/no timeout; loading29.891663s, drain50.674771s; nominal60.164406s/446 intervals, p50/p95/p99/max106.206/271.745/312.507/337.365ms. First-invalid frame2045/tick90286277: terrain0/object0/visual1, unchanged nine tiles; request121 correlation above. 140 requests/137 active consumes | **INVALID steady**; no retrospective lifecycle cause proof |
| Run6 | Child1/no timeout, wall78.470204s; loading6.204538s/max159.265ms, drain9.123740s; nominal60.014829s/2137 intervals, p50/p95/p99/max27.631/33.194/43.036/66.171ms, all below100ms | Timing satisfies fixture policy, not product budget; readiness **INVALID**, not PASS |
| Run6 first invalid | Frame1632/tick17828308: terrain0/object0/visual1, unchanged nine tiles. NPC4294959728/display32729 request97 queued frame1631/tick17803820; consumed frame1634/tick17896361, result OK/visual attached | Source/time correlation, not captured private pending-key or worker-completion proof. No matching removal for this NPC; do not attribute first invalid to same-ID re-entry |
| Run6 lifecycle | 113 requests/113 active consumes; 17 replica `despawned` events and 17 actual removals; zero appearance-clear/reset events. Repeated removal/reintroduction concerns other NPCs, including4294959785/4294959768 | Distinct from first-invalid candidate. Replica despawn does not establish authoritative relevance/death/respawn cause |

No controlled improvement baseline, GPU-time/VRAM/leak, general shutdown or full-conversion acceptance. All six nominal steady windows remain invalid. Independent `/tmp/claude/retained-conversion-20/lifecycle-diagnostics-independent.md` accepts bounded source/emission/provenance **PASS**, with steady **FAIL**. Its own introduced naming finding (`resolve_unit_appearance` hides file/cache effects) is trivially fixed by `130fbf90` renaming it to `load_unit_appearance`; independent `authored-pose-independent.md` accepts rename semantics/readability, scoped formatting and saved Depot `2x2qqvn5dt` build0. Installed SHA-256 is `a7f523bc608285c8ec3ee1ef6180aacad57d78a1e80ad4435db5687f8d30b3df`; earlier run6 runtime retains its recorded artifact scope. Retained/full conversion remains OPEN.

## Approved offline Options — bounded runtime observed

User-approved **Allow offline Options** permits existing offline main-menu/Options access. Production `744fa656` widens only physical Escape access to the actual SkyboxDebug child, preserves InWorld `gameplay_input_allowed`, and supplies the logged-in flag that hides offline Log Out. Production `09dc90ad` makes sky input yield while parent `GameMenuUI` is present and resets dragging. Existing Options/settings semantics and UI ownership remain unchanged.

Historical Depot `13lg139ksk` access build exits0, supplied artifact SHA-256 `e7b8be049fe48abc9cada78fe38b367675d629f8297967d3d7ab88439868fc7b`. New native Depot `dlhhzs6pll` build exits0 at `09dc90ad3cf5ac34c5388c229a8886ef9cd259e8`, DLL SHA-256 `c2669a283a38568a7ca053d41c2bffc4b931de7362d2c1a7b65c16f53c437ee0`. Full stdout/stderr and inputs are retained in `/tmp/claude/retained-conversion-20/offline-options-build{1,2}-{inputs.json,stdout.log,stderr.log}`.

| Exact retained boundary | Observation | Proof limit |
|---|---|---|
| `offline-options-red.log`, fixture `80d2762d` | Genuine old-menu missing-menu RED, exit1 | Pre-access production failure retained |
| `offline-options-access-{stdout,stderr}.log` / `access-result.json` | Access gate opens Main; sky input prevents Options click, RED exit1 | Access alone did not fix pointer ownership |
| `offline-options-pointer-{stdout,stderr}.log` / `pointer-result.json` | Pointer fix opens Camera; fixture rejects Handle expected887.666671342734 versus actual887 | Nonproduction layout oracle failure; rounding cause not proven |
| `offline-options-semantic-{stdout,stderr}.log` / `semantic-result.json`, fixture `48132be1` | Actual UI FOV105/.007 → 110/.004 → restore; canonical data and live camera FOV, real orbit yaw assertions −.016/−.028; Escape Options→Main→Closed, Done/Resume, retained offline sky camera/M2, no authentication/Log Out and unchanged legacy file; final assertions PASS | Process124 at timeout120: **not whole-case PASS or normal-exit proof** |

Test-only `48132be1` updates `80d2762d` by removing the unguarded subpixel Handle-shape assertion, not fitting a pixel tolerance. Semantic saved-value/camera/input assertions remain. Offline access and these live values are bounded runtime-observed, no longer unsupported or pending; not all Options acceptance. Independent bounded audit `/tmp/claude/retained-conversion-20/offline-options-independent-verification.md` accepts source/live-value evidence, scoped formatting and current12+15 tests at `49decd65`; process124 remains explicitly NOT PASS. Prior persisted-camera, sampler and process provenance remains unchanged.

## Owned Options normal exit — pinned acceptance, 2026-10-02

The existing canonical pin `4bf02dd0` selects `4.7.2-pr123946` and was already deployed before MAIN's old timeout; MAIN had instead selected official `4.7.2`. User authorized this owned Options exit investigation. Selecting the existing pin and verifying its SHA-512 required no new application/native code, driver, kernel, service, compositor or renderer changes. [Godot Wayland exit RCA](../wiki/investigations/godot-wayland-exit-hang.md) is the shutdown RCA SSOT; its upstream status and patch mechanism are not duplicated here.

MAIN's current 30 faithful real-UI cases all assert PASS and return outer exit0 over 78.035s: ten Cage `-D` cases directly log Godot child0; twenty unwrapped no-`-D` cases return outer0. Same actual GPU/Vulkan, audio, default UI, source and retained camera; FOV105/sensitivity.007 → 110/.004 → restore. Native DLL SHA-256 stays `c2669a283a38568a7ca053d41c2bffc4b931de7362d2c1a7b65c16f53c437ee0`. Verified pinned binary SHA-512: `1bd2e9575be0bf1b885abe34cf0f928adb3212cd2b38aa443f62e4d82b117c281d4358e432b33209046f69fbe4a99604ca74303568ba3b823840f2b052aa4594`.

Exact 30 logs: `data/diagnostics/retained-options-pinned-acceptance/`. Inputs/results/summary: `/tmp/claude/retained-conversion-20/options-pinned-acceptance-{inputs,results,summary}.json`. Independent verifier is active and will inspect this new documentation commit; no acceptance of that commit is claimed yet.

First live PID1940217 was Cage in allocator code, a transient observation, not a proven persistent hang. Post-unload `--collect` unit defaults are not valid exit-status evidence. Old wrapped direct exit0 observations demonstrate intermittence, not a root fix for old124. No hung Godot stack was captured in this case; individual historical124 exits cannot be attributed to an unobserved race. The old semantic timeout124 remains historical, not retroactive PASS. Current owned Options normal-exit boundary is no longer unsupported; this does not prove general shutdown beyond this fixture. At the Options proof, earlier native GL sampler/old mip124 cases were not retested or relabeled. The later bounded mip-sampler proof below records a new process0 observation without relabeling historical124. Default Map2703 data, settled performance/comparable baseline and retained/full-conversion gaps remain open.

## Pinned mip-sampler process proof

`data/diagnostics/retained-mipmap-pinned-acceptance/` retains exact argv/input hashes, full stdout/stderr, two GPU captures and outer exit0 at1.436866s. Existing canonical `4.7.2-pr123946` replaces the obsolete official-runtime input of the historical124 case; shader and fixture are unchanged. Explicit control and production sampler each pass three concrete green-mip pixel assertions. Cage `-D` directly records Godot child normal exit0. Current DLL is the run6 diagnostic artifact recorded above. This closes this synthetic fixture's bounded assertion/process boundary, not original-asset visual parity or general shutdown. Historical124 cause remains unknown; no code/driver/compositor change or forced cleanup was made. Independent `/tmp/claude/retained-conversion-20/lifecycle-diagnostics-independent.md` accepts bounded saved synthetic GPU/assertion/process **PASS**: six assertions, child0/outer0. Authored pose is excluded from that audit; no general shutdown acceptance.

## Pinned authored pose — bounded actual runtime

Fixture `b4888571`/`81ec36c3` observes original coastal FDID525142 authored bone6 through the production loader/player/Skeleton3D, with an expectation independently derived from raw M2 bytes, not test-written poses. Actual artifacts are repository `data/diagnostics/retained-authored-pose-pinned/{inputs.json,result.json,stdout.log,stderr.log}` (not filesystem-root `/data/`). Original sequence0 duration320000ms converts requested17040001ms to f32 17040000ms before remainder80000ms. All four observations match expected local and global skeleton-space poses at epsilon0.0002 without test pose writes.

Cage directly records normal child0; result records outer0, wall2.327133s. Logs retain leaked13 texture/1 shaped-text/2 font allocations,26 Texture RIDs and22 ObjectDB instances. This is one positive-duration fixed authored phase, not clean-resource/general-shutdown, natural-live, zero-duration-authored, GPU pose/deformation or pixel-parity proof. Separate independent `/tmp/claude/retained-conversion-20/authored-pose-independent.md` accepts raw-byte oracle, eight local/global transform observations and bounded process proof at `ac53b56d`. The later rename/build does not recertify runtime on a new binary.

### Camera focus invariant — rejected arbitrary-translation gap

Original `sync_skybox_to_camera` uses `OrbitCamera.focus`, not arbitrary camera translation. Existing production observer `check_pose` under real drag/wheel asserts the matching focus invariant. The proposed arbitrary-translation gap is **REJECTED**: it is not the original contract or a missing acceptance requirement. This does not broaden camera/input proof beyond the existing fixture.

## Known gaps (current cycle)

- [x] Final bounded scope audit16–20: independent report `/tmp/claude/retained-conversion-20/final-independent-verification.md` accepts source/render evidence and measurement coherence at `ded19eb7`, while rejecting settled-window acceptance. This completes evidence accounting, not retained/full-conversion acceptance. Historical mip timeout124 remains recorded; later pinned synthetic mip assertion/process proof is above. Inherited format/build warnings and other remaining gaps stay open.
- [ ] Supply authoritative default `Light.map2703` data, currently absent: explicit default-source blocker, no fallback.
- [ ] Complete remaining source/animation/input coverage; bounded offline same-scene live FOV/sensitivity is runtime-observed, not all Options acceptance; current owned-fixture normal-exit proof is bounded above, not general shutdown.
- [ ] Establish a valid settled performance window and comparable baseline; readiness changes invalidate steady claims.
- [ ] Complete retained skybox acceptance under the linked conversion contract. General shutdown is deferred; transferred character/clothing, water/appearance/tooling and whole conversion remain open.

## Out of scope

- Authentication, global UI beyond the explicitly approved existing main-menu/Options overlay, new controls, new CLI flags and platform expansion: out of scope.
- Hardcoded fallback asset substitution: forbidden, not a compatibility option.
- Whole-client conversion acceptance: governed by the broader conversion spec.
