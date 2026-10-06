## 2026-10-06 — Authored portrait cameras and Modern HUD pixel overlap

[Portrait camera/key investigation](systems/portrait-party-frames.md#authored-camera-and-frontal-key--2026-10-06): loaded HD type-0 camera selection was already correct; fixed reversed portrait key direction and corrected the all-human offline fixture. Eight Human/Blood Elf/Dwarf/Orc male/female head-angle cases, native frontal/reversed pixel lighting and both-skin roster captures pass. Modern reported 72×70 bounds intersect in 596 drawn pixels (53×33 envelope); native default scaling intersects in 11,243 drawn pixels. Approved positions unchanged. Proof/crops: `data/diagnostics/portraitcam-2026-10-06/`. Shutdown warnings remain; no live/reference-image parity claim.

## 2026-10-05 — Runtime portrait party heads (party3)

[[portrait-party-frames#Step 3 — runtime member portraits]] records reuse of the masked unit portrait renderer, member-name resource ownership, Retail offline/dead/ghost/low-health rules and no range fade. Compact remains default with zero party renders. At native `5f33198a` plus fixture `978772c5`, both skin processes pass concrete roster changes, actual grey offline head pixels and viewport cleanup; five native Rust/six UI-model tests and native build also pass. Inspected runtime-head raster passes bounded scope. Global RID/ObjectDB shutdown warnings remain, higher than the static baseline; never-replicated appearances, settings UI and live acceptance remain open. Proof: `data/diagnostics/party3-2026-10-05/proof.md`.

## 2026-10-05 — Stormwind loading boundary

[[stormwind-loading]] records instrumented tile-gate timeout, per-placement costs, WMO-child discovery and unrelated terrain build order. [World-loading policy](../specs/world-loading.md) now defines a spatial entry bubble without discarding distant work. Before/after evidence remains in `data/diagnostics/swload-2026-10-05/`.
## Native guild rank settings — client continuation

[[banks#Rank settings model]] now records the native guild/communities entry, both-skin
rank/settings widgets, hierarchy-gated member context menu and GuildChannel bridge.
Client writes wait for authoritative state; only purchased tab-name entries are edited
from the server's fixed eight-tab rights vector. 27 targeted Rust tests pass across
widget/model/transport/micro scopes. At `1e62844b`, native build and both inspected
1920×1080 captures pass after a real-raster disclaimer overlap regression. Logs retain
texture/font shutdown leaks; live server and full communities parity are not claimed.
Proof: `data/diagnostics/guildranks-2026-10-05/proof.md`. No server edits/builds or port 5000 use.

## 2026-10-05 — Paperdoll teardown double free

[[character-preview-double-free]] records a real-engine reproduction of `gd.rs:911`, the parent-before-preview teardown bug, and the process/reconnect ownership audit. Reset now releases the preview before its RegistryUi parent. At `dc7f80b9`, formatting, locked local extension build and three headless lifecycle cycles pass (each command exit 0). All 144 native Rust free sites inventoried; no other matching unguarded double-free established. Original suspend incident attribution remains conditional.
## 2026-10-05 — Portrait roster wiring and Retail online-state rules (party2)

[[portrait-party-frames#Step 2 — roster and source corrections]] records member roster/live-state wiring with compact default and a persisted style flag; settings UI/runtime heads/live acceptance remain later steps. Offline health is full/desaturated; power full with half-grey tint, not invented desaturation. Native projection now desaturates sampled pixels. Step-1 crown already matches Retail BOTTOM→TOP (-10,-6); retain source rect and regression-test it rather than moving it cosmetically. `27b41020` passes two Godot roster/projection tests and 12 portrait UI tests. Native raster then exposed a later desaturation overwrite; `a220f937` fixes it after both-skin raster RED. Final Modern/Forever recaptures pass inspected grey-health/dim-blue-power/source-crown scope; prior shutdown leaks remain. Proof: `data/diagnostics/party2-2026-10-05/proof.md`.

## 2026-10-05 — Product-bound party sheet offline proof (party1)

[[portrait-party-frames]] records local 70205 bytes matching frozen 69913 bar coordinates, bounded product-file binding through the existing authored UI loader, and the corrected player-mana crop. At `dea89b5a`, targeted 9 portrait + 4 status-bar tests and native build pass; inspected Modern/Forever offline recaptures pass member art/fills. Shutdown leaks remain; no pet/reference parity, roster/runtime portraits/settings/live proof. Compact default retained. Earlier blockers below are historical, not current input state.

## 2026-10-05 — Static portrait party family (party1)

[[portrait-party-frames]] adds source-cited Retail/Camelot member/pet geometry and distinguishes both verified c60 mappings: conditional CharacterFrameOnParty element 33561 → set-1 member 38477 → atlas 3960 shares Camelot's player sheet; ordinary Party element 21081 → member 39017 → atlas 4019 uses uipartyframec60. Static state-only component, behavioral tests, registry-golden capture and offline preview; no roster/portrait-runtime/settings/live acceptance. Compact remains default. Targeted registry/source tests pass 12/12 and extension builds, but owned native captures expose missing Retail sheet 4681512 and a mismatched 69913-vs-69933 base sheet at shared FDID 4631591. Static raster acceptance is blocked; do not wire as complete. Source-record equality is not physical-texture proof.

## 2026-10-04 — Canonical toolkit integration and retained Skyborn blocker

[Build guidance](../remote-builds.md) and active source links now use canonical `ui-toolkit/core` after toolkit merge `96dbda4`; historical branch-path proof remains attributed to its original revision. [[character-creation]] records unmerged Skyborn partial support, inferred server masks and current build-70205 local-CASC root-initialization failure from the retained main probe. No builds/tests rerun; native Skyborn support remains unverified.

## 2026-10-03 — Replica acknowledgment test capture (replicaflake)

[[godot-replication]] records the deterministic Connecting → Connected reproduction: the old Tap paired buffered arrival with an acknowledgment from a different consumption boundary. Capture now brackets replicon's Connected PreUpdate and OnEnter receive calls; product worker and exact byte comparison unchanged. Regression RED reproduces `[b"\0\0"]` versus `[]`; unchanged test GREEN 1/1 and full network library 58/58 at `5ae0e4e7`, with natural one-minute loads 35.05/36.38. Exact proof and revisions in [[godot-replication]].

## 2026-10-03 — Pinned Godot adds PR #123546: cold import crash (coldimport)

[[godot-cold-import-crash]]: the launcher's and deploy.sh's cold `--import` crashed 134/139 intermittently from a Godot ClassDB race hit by threaded `.glsl` imports; the pin becomes `4.7.2-pr123946-pr123546` (upstream PR #123546 backported). Symbolized build: 0/40 cold imports vs 3/20 (`data/diagnostics/coldimport-2026-10-03/`).

## 2026-10-03 — Desktop/local trial independently accepted

Independent followup at `0c8f7275` accepts the bounded [[build-hosts]] trial: refreshed launcher format/check/readability and 25 process tests; unchanged Python proof retained; server admin/UDP and authored RTX Login capture inspected. [Operational SSOT](../remote-builds.md#desktop-runtime-capability-boundary) retains Dozen, audio, full-world, shutdown and snapshot limits. Docs-only acceptance reconciliation; no broad checks rerun.

## 2026-10-03 — Desktop/local trial bounded evidence reconciliation

[[build-hosts]] links the [operational SSOT](../remote-builds.md): both extension exports, desktop CLI and 16 camera tests, fully staged admin/authenticated UDP server fixture, and inspected authored GPU Login capture. Rerun commands and gaming host selection documented; snapshot pinning, full-world/parity/audio/shutdown and independent code-gate exclusions retained. Docs only; no runtime/test reruns.

## 2026-10-03 — Skyriding part 2: vigor and abilities (skyride2)

[[mounts]]: Skyriding Charges (cat 2391) spent by Surge Forward / Skyward Ascent through the spell pipeline; the local player's `SpellGo` flaps the shared `Glider` (impulse capped at AddImpulseMaxSpeed 100), Aerial Halt air friction; live `skyriding_abilities_live.gd` (`data/diagnostics/skyride2-2026-10-03/`).

## 2026-10-03 — Elastic-tree evidence reconciliation

[[elastic-trees]]: accepted coarse annotation, saved lit contact/recovery and private mounted-input/network proof reconciled at `34b6ada6`; spec checks distinguish placement reuse from source-only FDID caching. Preserved uniform-buffer root cause, prototype boundaries, pending live-image inspection and latest-source checks. Docs only; no reruns.

## 2026-10-03 — Skyriding part 1: physics (skyride1)

[[mounts]]: skyriding momentum flight on `CAN_ADV_FLY` (shared-protocol `skyriding`, FlightCapability 11), launch/glide/dive/landing; live `skyriding_live.gd` on a private server (`data/diagnostics/skyride1-2026-10-02/`).

## 2026-10-02 — Scene light as global shader uniforms (settlegate)

[[world-entry-stalls#scene-light-rebound-every-frame--2026-10-02]]: "World did not settle" in Stormwind was the live clock changing the light every frame and rebinding every material (400-710 ms/frame); light and fog are now global uniforms written once per change. Stormwind settle 488 s → 32 s.

## 2026-10-02 — Mounts and steady flight (flymount)

[[mounts]]: Godot mount model and rider seat, flight controls after Retail JUMP/SITORSTAND, `PlayerInput.flying`; live `flying_mount_live.gd` PASS on a private server (`data/diagnostics/flymount-2026-10-02/`).

## 2026-10-01 — Launcher pins patched Godot 4.7.2-pr123946

[[godot-wayland-exit-hang]]: launcher and shell helpers now use official 4.7.2 plus upstream PR #123946, built by `scripts/godot/build-patched-godot.sh` and SHA-512 pinned; missing or mismatched binary fails with build instructions, no fallback. `PYTHONHASHSEED=0` makes the build bit-reproducible (`editor/editor_builders.py` embeds Python `hash()` of the docs). Pinned binary: 0/120 hangs of `m2_animation.gd` via `quit-hang-loop.sh` (`data/diagnostics/godotpatch-2026-10-01/loop-anim-seeded.txt`). Retire when an official release contains #123946.

## 2026-10-01 — Server game time, WDL horizon, item particles (worldvis)

[Retail lighting](systems/retail-lighting.md#sky-dome-godot): time of day from `LoginSetTimeSpeed`. [Terrain](systems/terrain.md#horizon-godot): WDL horizon. [m2-particles](../specs/m2-particles.md): item model emitters.

## 2026-10-01 — In-world LightSkybox models (worldvis)

[Retail lighting](systems/retail-lighting.md#sky-dome-godot): LightSkybox models collected over the LightParams blend (SkyBoxCollector) and drawn on the camera; Twilight Highlands fixture.

## 2026-10-01 — Godot ground detail (worldvis)

[Ground detail](systems/terrain.md#ground-detail-godot): 12340 detail-doodad scatter/mesh port matching solarityclient's native fixtures; retail GroundEffect CSV exports; live Northshire clutter fixture. Spec [ground-detail](../specs/ground-detail.md).

## 2026-10-01 — Catalog waits at world entry

[[world-entry-stalls#catalog-waits--2026-10-01]]: the first inventory snapshot waited 9.2-54.9 s in `OnceLock::get_or_init` for the background item catalog load. Item catalog/icons, NPC gear rows and the spell visual catalog are now read without waiting; the entrance and minimap catalogs load from client start. Items received early update when the catalog loads (`GET_ITEM_INFO_RECEIVED` model); spec line in [cursor-item](../specs/cursor-item.md). Character-select model catalogs remain synchronous.

## 2026-10-01 — Retained skybox/performance bounded reconciliation

[[authored-skybox-black-output]] records source zero opacity and bounded 100000 ms original/native RGB match, preserving active timeout, coastal phase-ready gap and oracle SETUP failures. [[world-entry-stalls]] records actual run1/run2 failures and readiness RED/GREEN without settled/baseline/budget/leak acceptance. Retained/conversion goals remain open; docs only.

## 2026-10-01 — M2 free material-null errors

[[godot-material-null-free]]: M2 batches bound materials as surface overrides, which Godot releases before freeing the RenderingServer instance; a batch freed before its first draw read the freed material. `eb619da4` binds them as the material override; regression `godot/tests/m2_free_material.gd`. [[rendering-pipeline#godot-m2-batch-materials]] notes the binding.

## 2026-10-01 — Native SkyboxDebug source/test-first docs audit

[Evidence SSOT](systems/godot-conversion.md#native-skyboxdebug--sourcetest-first-contribution-runtime-green-pending) records contribution through `da694c79`/`f786be27`, original CLI transport, cached no-fallback source and owned renderer/controller/environment. Supplied parser/launcher RED/GREEN are bounded; MAIN's actual pre-production Vulkan/CASC startup rejected the screen and exited1. Current build/new independent gate pending, no native runtime GREEN; core data-helper tests unexecuted. [Architecture](systems/skybox.md#native-offline-skyboxdebug) separates shader-owned fog and legacy fallback. Pixel/default-source/fog/physical-unit/bone/shutdown gaps and root quest-format FAIL retained; no other feature closure. Three owned docs only; no index/new page, code/build/runtime/tests/delegation/operations.

## 2026-10-01 — Godot M2 batch materials on WebWowViewer retail rules

[[m2-format#batch-shaders]] now records retail pixel/vertex shader resolution, render flags 0x2/0x8/0x10, texture weights, float-quaternion texture transforms and wrap flags; [[rendering-pipeline#godot-m2-batch-materials]] records the Godot binder/shader and its real-model oracle proof (16 named batches, baseline 14/16 RED). Parity row added (Partial: decals, transparent sort, Bevy/Retail comparisons open).

## 2026-10-01 — Native ExportScene bounded MAIN-observed GREEN

[Evidence SSOT](systems/godot-conversion.md#native-exportscene--accepted-bounded-pass) records MAIN-accepted independent1573 bounded PASS at `3ea4580c`: shared original JSON, actual native public export/write error, compensated transforms, retained legacy decoder and root check0. Post-READY `d1981968` RED and introduced re-export warning remain historical. Native runtime0/Depot0 are retained by source equivalence after the include correction. Full semantics/UI/actions/performance/shutdown remain open; root formatting still fails.

## 2026-10-01 — LiquidObject missing rows knowledge preservation

[Investigation SSOT](investigations/northshire-pale-water.md#liquidobject-ids-without-db2-rows--resolved) preserves supplied MAIN base/copy, cached content identity and bounded XFTH membership evidence; authoritative consumer/overlay semantics remain unresolved. Docs only, no new independent data/runtime proof.

## 2026-10-01 — Native JS negative startup bounded docs audit

[Evidence SSOT](systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail) adds tests-only absent-focus typing rejection/successor suppression and distinguishes historical parse-error setup from RED and observer-owned exit from production auto-exit. MAIN accepts independent1568 bounded source/readability/supplied-runtime PASS; broad JS/root-format FULL goal and retained warnings remain open. Three owned docs only; no code/tests/build/runtime/delegation/operations or index change.

## 2026-10-01 — Native JS bounded docs audit

[Evidence SSOT](systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail) records MAIN Login GREEN `ea9c4482`, independent1537 functional subset PASS/overall FAIL, five runtime action variants and genuine timeout RED `73cb16e0`/`61bfdc12`. Root script routing and native contract links corrected; pending fixes, unresolved MH2O errors, warnings and conversion/shutdown/transferred exclusions retained. No new page or index change.

## 2026-10-01 — Merchant ordering/content bounded MAIN-observed GREEN

[Merchant system SSOT](systems/merchant-frame.md#native-reply-ordering-and-tooltip-content--bounded-main-observed-green) records ordered same-channel replies `55648101`, original tooltip content `33e81860`, real-UDP RED/GREEN, five portable tests and Depot `0xdh43jcwk` physical runtime0. Independent1529 pending; forced cleanup, historical failures, inherited WMO warning and transferred-owner exclusions retained. Parity links updated; full merchant/conversion, junk tooltip and per-item repair remain open.

## 2026-10-01 — Native IPC bounded MAIN-observed GREEN

[Evidence SSOT](systems/godot-conversion.md#native-ipc-diagnostics--bounded-main-observed-green-independent-gate-pending): six public diagnostics, rendered WebP and own-instance cleanup have MAIN-observed GREEN; independent1524 pending. Historical failures, unported consumers, semantic parity and shutdown/resource exclusions retained. Parity remains Partial, conversion open.

## 2026-10-01 — Polymorph video with audio

[[spell-visuals]] Polymorph section: the `POLY_GRAB` recording now uses 30 fps wall-clock slots and adds Master-bus audio, muxed by `scripts/agent/grab-video.py`. Added a `set_specialization` fixture API, because a level-10 mage on game-server 40241f5 defaults to Arcane and has no Frostbolt. Findings: the client renders only 5-7 fps in the headless cage, the spy and sheep float above the mage, and stalls can cause reconnects.

## 2026-09-30 — Merchant cursor test-only checkpoint
## 2026-10-01 — Native merchant cursor MAIN-accepted bounded PASS
||||||| 71e0796f
## 2026-09-30 — Merchant cursor test-only checkpoint

[SSOT proof matrix](systems/godot-conversion.md#native-merchant-cursor-buy--main-accepted-bounded-pass) supersedes gate-pending `588ec432` and `7ec8c814`: matching Depot build and five full runtimes exit0 at native `837e2c1e`/`b073e4dd`. Exact owned embedded-slot Buy,900ms pre-response COMMIT and peer-only inventory/gold final now MAIN-accepted under independent gate1446 bounded functional/source/format/five-flow PASS. Rename-only fixture SIDE_EFFECT fix `afa2d71a` accepted without new compile proof; helper-length/root/foreign debt retained. Historical RED, inherited debt/auth timeout/ObjectDB warning and shutdown exclusions retained; no whole merchant cursor/SELL/global ownership/full conversion acceptance. Docs only; no source/PLAN/data edits, tests/builds/delegation/ops.

## 2026-10-01 — Merchant cursor actual RED and bounded source checkpoint (historical)

[SSOT proof matrix](systems/godot-conversion.md#native-merchant-cursor-buy--main-accepted-bounded-pass) supersedes prepared docs `e5c98513`: actual authenticated first pickup RED101, then ordered owner-tagged Merchant inputs `b073e4dd` and shared cursor dispatch/private owner check `837e2c1e`. First vendor → own embedded bag only; matching build, five runtime regressions and gate1446 pending, no accepted GREEN. Existing startup1429/tooltip1415 scopes retained; inherited debt/auth timeout/ObjectDB warning/general shutdown open. Docs-only commit; no source/PLAN/data edits, tests/builds/delegation/operations.

## 2026-09-30 — Merchant cursor test-only checkpoint (historical)

[SSOT](systems/godot-conversion.md#native-merchant-cursor-buy--main-accepted-bounded-pass) records tests `d825108c`/`2c00a751`, MAIN registration `3cb53ab4`, literal Linen/embedded-slot Buy+COMMIT barrier, strict phases and intentional owned cleanup. First Native Depot/authenticated RED pending; no production cursor implementation or new user screen. Spec/parity links and invocation docs reconciled without changing product policy; accepted startup1429/tooltip1415 and existing merchant-click/bag scopes retained. Docs only; no builds/tests/gates/operations.

## 2026-09-30 — Startup equipment bounded acceptance reconciled

MAIN read FULL and accepts independent1429 bounded PASS. [Acceptance SSOT](systems/godot-conversion.md#occupied-startup-equipment--bounded-main-observed-green) retains exact startup/actions, byte-equivalent extraction/direct4/helper18, required rendering assembly and original15-second merchant0/Loading2738ms/strict3opens3closes/successful child exit. Earlier pending entries and failures remain historical; auth cause, ObjectDB leak, inherited debt and full conversion remain unresolved. Docs only; no tests/builds/operations.

## [2026-09-30] audit | Original-bound merchant runtime passes; independent gate pending

[SSOT](systems/godot-conversion.md#isolated-merchant-fixture-rendering-gap--corrected-source-proof-pending): `51a3e85f` final log0, Loading2738ms under original15 seconds, strict three opens/closes and placement/reset/pointer/quiet-reopen proof, normal child success. Newly observed one ObjectDB leak warning is unattributed; shutdown investigation deferred, no clean-resource/general-shutdown acceptance or pre-existing claim. Independent1429 pending MAIN acceptance; native5a unchanged, no redundant build/tests.

## [2026-09-30] audit | Merchant diagnostic passes; original-bound acceptance pending

[SSOT](systems/godot-conversion.md#isolated-merchant-fixture-rendering-gap--corrected-source-proof-pending) records rendering fix/build0, unexplained pre-auth retry101, and test-only `6bea` timing/strict merchant exit0 (Loading4162ms, normal child0). No auth-root-cause or180-second necessity claim. `51a3e85f` restores15 seconds; final retry/independent1429 pending. Native5a startup/readability and unchanged tooltip/pure proofs retained; combined gate OPEN, no general shutdown clearance.

## [2026-09-30] audit | Merchant fixture rendering links corrected; gate OPEN

[SSOT](systems/godot-conversion.md#isolated-merchant-fixture-rendering-gap--corrected-source-proof-pending) records native5a source/build/actions and own readability resolution, actual merchant pre-login exit101, and fixture-only `54eaef69` required-link correction. Fresh fixture build/merchant retry and read-only1425 acceptance pending; prior startup failures and valid unchanged startup proof retained. Full conversion OPEN; Mail/Auction untouched. Docs only; no tests/builds/operations.

## [2026-09-30] audit | Startup equipment followup pending after lifecycle split

[SSOT](systems/godot-conversion.md#occupied-startup-equipment--bounded-main-observed-green) records gate1421's new own complexity finding and source-only `5a3ebf7b` lifecycle/application split. Fresh build/actions/merchant-click and independent1423 acceptance pending MAIN proof; no metric clearance. Prior first-GREEN remains historical functional evidence, not current integration proof. Inherited readability remains uncleared; inventory snapshot is not Appearance/mesh proof. Docs only; no tests/builds or operations.

## [2026-09-30] audit | Startup MainHand inventory MAIN observed GREEN

MAIN observed GREEN for single startup MainHand inventory at native `ee2d3e47` + `2d1829fc`, test `57b30f57`. Independent1421 active/report pending, not accepted until MAIN confirms. Tooltip1415/docs `2142e85f` acceptance retained. [SSOT](systems/godot-conversion.md#occupied-startup-equipment--bounded-main-observed-green) records fresh Depot `04kwqv77h7` build0/runtime0, pre-Equip GUID9170105, authoritative GUID9170005 replacement, exact one Equip/two Destroy and original popup flow. Intentional child3934038 kill/reap/readers0 is not shutdown; broader coverage excluded. Supersedes pending-GREEN docs `cf9c3d63`.

# Wiki Log

## 2026-10-06 — Rendered Stormwind resource scheduling

[[stormwind-loading]]: owned UDP 5286 fixture measured 37.788-second warm entry / 38.315-second first draw with 494/494 prerequisites. Instrumented IO, decode, parse, meshes, materials, uploads, CPU/GPU viewport time and pipeline counters. Terrain/object loading slices increased to 64 ms; unchanged interactive budgets and entry bubble. `b1fd59d7` same-setup rendered entry **11.715 s**, first draw **12.048 s**, 494/494 ready, zero failures; WMO slices 130→17. Eight targeted Rust tests passed; owned headless full drain **9.699 s**. Exact phase tables and measurement limits recorded in the investigation.

## 2026-10-01 — Zaralda fixture acceptance blocker

Updated [[test-assets]] and index with prepared/executed `7903cb5e` fixture and native BLOCKED 0/3. Linked [server Midnight SSOT](../../../game-server/docs/wiki/investigations/midnight-economy-content.md#native-acceptance-blocker) rather than duplicating catalog/data/CLI facts. No runtime fix, new probe, build or service operation; concurrent engine work preserved. Overall goal open.

## [2026-09-30] systems | Login handshake timeout

Updated [[godot-conversion]]: Netcode client timeout 60 s → 10 s and a 5 s handshake timeout whose reason the login screen shows (`login_connection_loss.gd` GREEN, 5.8 s).

## [2026-09-30] audit | Occupied startup equipment authentic RED; minimal consumer committed

[Owned checkpoint](systems/godot-conversion.md#occupied-startup-equipment--bounded-main-observed-green): test `57b30f57` distinct startup MainHand25/count1/GUID9170105 versus bag9170005; exact pre-Equip equipment/bags/no carried icon/split/popup oracle and later replacement/original one Equip/two Destroy retained. MAIN Depot `rxnbcnxs5z` native build0/existing WMO warning; actual startup log exit101 equipment empty after authenticated READY/exact bags, before Equip input. Missing typed network receive/account decode/merchant apply chain; minimal account/merchant `ee2d3e47` committed using original inventory apply, transport1419 `2d1829fc` committed (typed receive after InventorySnapshot, before InventoryDelta). Integration Depot build/verifier1421 active; runtime GREEN pending; occupied startup still missing, not EquipmentAppearance/mesh proof. No separate-owner architecture/server/protocol change. Tooltip gate1415/docs `2142e85f` retained; protected Mail/Auction branches untouched, ownership inquiry outstanding without freeze. Docs-only reconciliation; no test/build/gate reruns.

## [2026-09-30] audit | Foreign-chat World rejection main-observed GREEN; independent bounded PASS accepted

[Owned evidence](systems/godot-conversion.md#foreign-chat-world-rejection--bounded-main-observed-green) records diagnostic RED on `b3bf65a2`, bounded `5f6782b5` rejection, new Depot build and three parent0 drag/actions/cursor logs. Independent gate1406 accepted bounded PASS; own hits unchanged, no cross-layer winner/global ownership claim. Existing exclusions retained; deliberate SIGKILL is not shutdown. Docs-only reconciliation; no tests/builds/operations.

## [2026-09-30] audit | Exact standalone drag followup independently accepted bounded PASS

Reconciled existing specs, parity coverage, wiki and index against `/tmp/claude/verify-native-bags-drag.md`. [Exact coverage](systems/godot-conversion.md#standalone-drag--exact-bounded-proof-and-pending-followup): `4677d732` independent functional/build/fmt PASS; Depot `hv02mf7mfc` and full native drag/actions/cursor logs all0. Same/held/noncursor negatives0; separated124px/rapid-frame837/ordinary-click each one Swap0/0→1/0, deltas withheld until unchanged INITIAL client proof then authoritative3/2. `b3bf65a2` independently accepted bounded PASS (`/tmp/claude/verify-native-bags-drag-followup.md`): source/event semantics and own readability findings resolved; fresh supplied Depot `0c4hdfxff4` build exit0 and full drag/actions/cursor logs parent0. Twelve inherited findings remain uncleared. Test-only `cc3765b1`/`7bd81e29` confer no production proof. Root/pure/invite unchanged proofs retained, no reruns. Intentional SIGKILL/reap/readers0 is not normal shutdown. No whole-file readability, threshold boundary/nondefault-scale runtime, NPC/global ownership or full-goal acceptance. Prior REDs and capture limits retained; previous pending-build checkpoint below is historical. Docs only; accepted saved followup evidence, no verification/build/runtime reruns.

## [2026-09-30] system | Standalone authored bag drag implemented; build/runtime pending

[Current evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass): production `4677d732` + projection `c61fc93d` implement ordered physical `FrameClick`/global-release `PointerUp` → opt-in `BagInput`. Successful empty-cursor pickup records logical press/source target; release consumes the record, dispatching existing effects only for >=4 logical pixels and a distinct target, without optimistic inventory. Held-cursor release has no origin. Own visible mouse-enabled frames use original strata/level/raise priority, actual global rectangles, scaled insets and action ancestors. Authentic `615bd810` RED and earlier failures retained. Main build `/tmp/claude/native-bags-drag-first-green-build.log` and runtime remain pending; cross-registry/NPC ownership, drag GREEN, full goal and normal shutdown unproved. Reconciled existing specs/wiki/index only; main-owned cursor spec already records implementation.

## [2026-09-30] audit | Accepted bounded actions followup; authentic drag RED

[Checkpoint evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass) reconciled against `/tmp/claude/native-bags-proof-ledger.md` and `/tmp/claude/verify-native-bags-actions-followup.md`: exact `ed135319` priority comparison, full captured build/actions PASS, fresh scoped fmt0, input cognitive19→9 and own naming fixed. Main accepts applicable own findings fixed; inherited input33-body-line/complex-condition/bumpy-road findings deferred, not feature bugs or whole-file clearance. Latest test-only `615bd810` authentic drag RED parent101 supersedes report's pending-RED status: three quiet release controls zero swaps; separated124 logical pixels leaves icon held/no Swap. Initial parsing failure (`8f14db08` fix) and second wrong mouse-disabled-title oracle remain failures. Authored absent-bag2 mouse-enabled button supplies inert noncursor control. Drag implementation NOT started at checkpoint; no drag/full-goal, normal-shutdown, startup EquipmentSnapshot, mesh or NPC/global-owner acceptance. Historical entry below records its earlier checkpoint, not current status.

## [2026-09-30] audit | Actions verifier PASS; readability followup and drag RED pending

[Existing evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass) records independent1377 functional/compile/format PASS, accepted own input complexity/length and neutral resolve naming findings, not clean readability. `ed135319` preserves startup→popup→chat→binding precedence and names `dispatch_bag_destroy_results`; followup proof pending. Test-only `233cbd63` compiles via Depot `0ml790cpjz`, full stdout/stderr saved, existing WMO warning only. Main rerunning actions after input change; actual drag RED pending. Historical capture limitations/failed builds and unchanged root/pure proof retained; no full-goal, shutdown, drag PASS or startup-equipment/mesh claim.

## [2026-09-30] audit | Bounded native equip/destroy GREEN; verifier pending

Reconciled existing conversion/cursor specs, parity matrix and wiki against `/tmp/claude/native-bags-proof-ledger.md`, `/tmp/claude/cursor-destroy-popup-proof-ledger.md` and actual `/tmp/claude/native-bags-actions-first-green.log`. [Exact evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass): shared original popup/key `bf1b1a73` pure10/10; native fix `a0b295b4`, read-only diagnostics `6bc3cde0`, Depot `jv155pp2xb` exit0 with existing WMO warning only. Main-observed actions parent0 proves one Equip/two Destroy, authoritative bags empty/MainHand retained, PoorNo quiet/cursor cleared, rare disabled+Enter inert, Unicode32/scalar Backspace/lowercase DELETE. Historical texture wrong-boundary failure, actual right-click RED, equip→world-drop RED, import and match-arm build failures retained. Independent actions verifier pending, not accepted gate; full goal, drag, meshes/startup equipment, NPC/global owner and normal shutdown unproved. Docs only; no tests, ops or delegation.

## [2026-09-30] audit | Bounded equip observation, destroy RED and unverified host

Updated existing conversion/cursor specs, parity matrix and wiki from authoritative `/tmp/claude/native-bags-proof-ledger.md` and `2223d6d3`. [Exact evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass): historical `ln9jmltc2m` import failure fixed by `2086200d`; `0mbvzk13ft` build exit0. Saved actions parent101 observes one EquipItem0/5, authoritative bag clear/MainHand25 guid9170005 count1 and quiet EquipDone, then PoorNo missing StaticPopup1 RED. `2223d6d3` native world-drop/shared-popup/result-drain/keyboard implementation is NOT GREEN; portable extraction1375 pending before compilation. No full gate, startup-equipment, mesh, destroy, drag or shutdown acceptance. Prior window/cursor proof preserved; docs only, no ops/tests/delegation, PLAN/data excluded.

## [2026-09-30] audit | Native standalone equip sender, GREEN pending

Reconciled existing conversion docs against `093f3816` and `/tmp/claude/native-bags-proof-ledger.md`. [Actions evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass): fixture build exit0; first run missing texture is not equip RED; same-binary second run after local CASC extraction reproduces missing right-click equip. Production now sends original catalog-gated EquipItem from authoritative inventory only. GREEN pending; no equip PASS, equipment mesh/startup equipment, destroy/drag or shutdown acceptance. Prior bounded window/cursor proof unchanged. Docs only; no build/test/ops/delegation; PLAN/data excluded.

## [2026-09-30] audit | Independent bounded native cursor PASS accepted

Accepted `/tmp/claude/verify-native-bags-cursor.md`, superseding earlier independent-pending entries only within [bounded cursor evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass). EXIST/SUBSTANTIVE/WIRED pass; actual root package check errors0/two existing parser warnings, root fmt and 13-path scoped fmt pass. Saved original policy8/icon1 and exact once Swap/Split, authoritative deltas, event-pointer/icon/source-lock, Escape/source-return/stale proof accepted without reruns. Test-only `19b1de2d` bags-actions remains pending actual RED/build; no production RightEquip/Destroy. New fixtures do not invalidate unchanged original pure scope. Pending request timing/interleaving NOT logged; child SIGKILL is not normal shutdown. No all-cursor/global-window/full-conversion acceptance. Docs-only reconciliation, not goal completion; PLAN remains unstaged.

## [2026-09-30] audit | Bounded native cursor UI/network GREEN

Reconciled [conversion evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass), specs/matrix/index and [startup investigation](investigations/local-animation-loading-gate.md) after `40497cb7`. Barrier `f7b137fb`/`76487261` now reaches READY; no exact pending-request timing claim. Diagnostic `9dc97a47` establishes actual event/OS pointer producer DataMismatch; `40497cb7` retains actual event position across handled/UI-blocked and keyboard-only split input, without test calibration. Existing pure8/icon1 pass; Depot `7p31d9cmcw` native+fixture exit0 with existing WMO warning only. Actual third GREEN parent0 completes quiet return/Escape/stale, bags open, exact once Swap0/0→1/0 and Split0/0→1/1 count2, typed2+Enter, icon center/source lock and authoritative final1/2. Child3813397 intentional SIGKILL/readers0 is not shutdown. Independent cursor agent1360 pending; full goal and exclusions remain open, legacy checked tests preserved. Earlier pending entries below are historical, superseded only within this bounded proof. Docs only; PLAN remains unstaged.

## [2026-09-30] investigation | Local animation Loading → InWorld gate

Added [startup investigation](investigations/local-animation-loading-gate.md); reconciled conversion system/spec/index. Cursor `2b3fa596` has saved Depot build exit0, pure policy 8/8 and icon 1/1; actual first GREEN exits101 before cursor readiness on missing local authored visual. Async world entry `5e26bc9b` permits pending visuals during Loading; `f7b137fb` gates local animation on existing InWorld barrier, retaining the post-readiness error. Log lacks request timing/state booleans: no specific captured-interleaving claim. Post-gate rebuild/GREEN/independent proof pending; legacy checked tests preserved, full goal open, shutdown deferred. Docs only; no source/build/test/push/delegation; protected PLAN unstaged.

## [2026-09-30] audit | Standalone window PASS and pending native cursor

Reconciled conversion spec/matrix, cursor native coverage and [bounded bag evidence](systems/godot-conversion.md#native-standalone-bags--bounded-window-and-cursor-pass) from independent window/actual Bevy reports and proof ledger. Saved window/runtime/ten portable tests PASS; actual Bevy compile PASS with three historical warnings. `9f954a3b` own-warning cleanup not rechecked. Actual authenticated slot0 cursor RED parent101; `9e301fb5` portable policy/icon and `2b3fa596` native consumer implemented, build/runtime/independent gate pending. Legacy cursor checkboxes preserved; remaining parity/readability gaps and intentional SIGKILL/normal-shutdown deferral retained. Full conversion open; docs only, no tests/builds/runtime operations.

## [2026-09-30] investigation | World-entry stalls

Added [[world-entry-stalls]]. The "Account" step at world entry (17-48 s in base) built every replicated unit's visual synchronously. "World objects" overran its 8 ms budget with whole-model and whole-WMO units. Unit visuals and ADT objects now load on `AssetLoader` workers, and the main thread builds them within 8 ms budgets: WMOs a slice of batches at a time, terrain chunk by chunk. The loading screen also waits for the local player's model. Back-to-back A/B (`world_entry_frames.gd`, two rounds at load 14-44): longest loading frame 17.6-47.7 s → 0.22-0.91 s. Gaps: the first in-world HUD frame, the per-unit build cost of humanoid NPCs, and object throughput under heavy load.

## [2026-09-30] audit | Standalone bag RED and pending host integration

Updated conversion spec, parity matrix and [system evidence](systems/godot-conversion.md#native-standalone-bags--red-host-implementation-proof-pending) for tests `67870535`/`a3e34213` and host `91bbc703`. Saved runtime RED reaches authoritative BAGS_READY then fails missing MainMenuBarBackpackButton, exit 101; Depot `d11bz93c7w` is compiled baseline only. Portable agents 1343/1344 APIs and actual GREEN remain pending. Standalone visibility, inventory interactions and unified window ownership remain unproven; full conversion open. Docs-only update; no tests/build/runtime operations.

## [2026-09-30] system | Godot replication without an ECS replica

New [[godot-replication]]: the network worker no longer runs replicon's client; it forwards raw replicon payloads and acks mutations, and the host-owned `Replica` decodes them into per-type columns. Wire layout, ack flow, fingerprint reasoning, codec schema and proof recorded; `UnitSnapshot` mentions in merchant, chat, spellbook, nameplate, movement and NPC appearance docs now point at the `Replica`.

## [2026-09-30] verification | Bounded native loot reach

Reconciled LootFrame spec/matrix with [saved reach proof](systems/godot-conversion.md#native-loot-reach--bounded-runtime-proof): inclusive corpse reach and friendly dispatch accepted; original four cases/42 UI checks retained. Hostile/role-response gaps remain; post-DONE exit 101 RenderingServer-null is FAIL/deferred. Settings-reload commits remain test preparation, not proof. Docs only; no clean full-conversion or source-pinning claim.

## [2026-09-30] implementation | Bounded native receiving mail

Updated [[trade-and-mail]] and the MailFrame spec to distinguish preserved Bevy full mail from the native AH receiving dependency. `9ed690bf` Depot focused source proof: metadata 1/1, owned UDP 3/3, model/registry/interaction 4/4. Real mailbox M2/picking, matching role/contents gate, authoritative claims and receiving-only authored UI are implemented. Committed GDScript fixture remains unrun until main's CLI proof; no extension install, native live run, backend/shared change or full-AH acceptance. Two pre-existing terrain test unused-mut warnings remain outside this slice.

## [2026-09-30] verification | Native Options and loot overflow

Recorded the [bounded functional gate](systems/godot-conversion.md#native-options-and-loot-money-overflow--rendered-red-fixes-awaiting-main-rendering); reconciled LootFrame spec/matrix proof, leaving deferred termination and full-conversion acceptance open.

## [2026-09-30] implementation | Server-global native auction browse

Updated [[auction-house-ui]] and spec after `5b9cb76c`: native browse now consumes server-global distinct-item pages, authoritative unit price and `u64` stock; drilldown/sell remain flat with real auction IDs. Snapshot adds `groups` and active endpoint flag; fixture reads groups. Targeted Depot native model 8/8, owned UDP 1/1 and shared frame 1/1 passed. Model RED missing-API compilation and wire RED missing-reply timeout recorded separately. Host/runtime proof remains with main after game-cli; no integration/ops run.

## [2026-09-30] implementation | Bounded native auction client

Updated [[auction-house-ui]] and its spec: native NPC/gossip protocol host, portable trading validation, exact-item/category queries, all fetched-row/server-page navigation and corrected duration labels. Targeted current model 8/8, retained owned UDP 1/1 and current host compile/range test 1/1 passed; no Godot runtime executed. Main owns game-cli-first integration and native smoke; no runtime/full-AH acceptance claim.

## [2026-09-30] update | Server missile timing

[[spell-visuals]] Server timing: game-server `fa5e689` delays missile hits by TrinityCore's `max(dist, 5) / Speed + LaunchDelay` (`Spell::HandleDelayed`). Frostbolt's damage number no longer arrives with `SpellGo`. About 200 ms early remains: the server clock starts at the cast, while the client releases at the M2 event. Not re-captured on the client.

## [2026-09-30] feature | Retail class resource bars

Every player class bar now ports its Retail template and mixin: art, animation groups and visibility gates. Rogue and druid combo points, chi, soul shards (with Destruction fragments), essence, death knight runes, holy power and Arcane Charges are covered. `ui/screens/class_bars/` holds a small `AnimationGroup` player plus one module per bar. Live proof against a private server: rogue, paladin, warlock, mage and a caster-form druid (bar hidden). Monk and Evoker cannot be created on the server. Runes wait on per-rune cooldowns from the powers protocol. See [ui-system](systems/ui-system.md#unit-frames).

## [2026-09-30] feature | Retail melee sounds; synthetic miss/interrupt PCM removed

[Melee sounds](systems/spell-visuals.md#melee-sounds): WeaponSwingSounds2 swoosh at `$CSS`, WeaponImpactSounds impact and CreatureSoundData injury at `$CAH`, SoundDeathID on NPC death clips. Hit reactions no longer cut a unit's own swing. [[sound]] outcome section rewritten; `OutcomeSpells` and `sound-outcome` fixture removed.

## [2026-09-30] evidence | Shared cache standalone and bounded native gates accepted

Updated [asset pipeline](systems/asset-pipeline.md#local-extraction) and index with main-accepted standalone `25debb1` proof: persisted-marker fresh-process recovery of exact valid local-CASC bytes, positive-cache preservation, explicit failure context, format/check/readability. Main accepts `/tmp/claude/verify-negative-cache-native-integration.md` bounded saved-artifact PASS: Depot `s1q4qhb120` build2 exit 0 compiles native consumer/fixture; extension load and eight Options helper PASS markers plus Menu/owned UDP/Exit runtime exit 0. Native cold-marker recovery remains unverified; standalone fresh-process regression supplies that separate proof. No independent binary-identity attestation, native cold-marker/full-conversion/deploy/current unfrozen whole-tree acceptance; native compiler/runtime warnings retained in report. Contract remains in sibling [asset-cache spec](../../../asset-resolver/docs/specs/asset-cache.md). Isolated engine docs only; asset repo untouched, user-dirty files preserved. Source unfrozen; shutdown paused.

## [2026-09-30] evidence | Bounded TargetSelf Options rebind; gate accepted

Updated existing Options parity row, [native main-menu boundary](systems/godot-conversion.md#native-main-menu-boundary) and index. Test-only `685cbe83`, `data/diagnostics/target-binding-options/run1.log` exit 0: real Keybindings/Targeting capture F1 → T → F1, exact labels/unique canonical ownership, old keys inactive, same-player ring/TargetFrame; selection cleared/Menu closed/input released. T avoids fixed F10 Edit Mode. Previous character-select/camera/Min-Max/Zoom/marker/full Menu UDP/Exit assertions pass; unchanged compiled `mb61bqgcj2`, no production change/new Depot. `/tmp/claude/verify-native-target-binding-options.md` accepted bounded saved-runtime/source PASS (eight PASS messages, not eight framework cases); no all-bindings/conflict/server-ack/movement-rebind/fresh-process acceptance. Docs only; Cargo.lock, PLAN.md, user-data/data preserved; no tests/GPU/build/network/delegation. Source unfrozen/shutdown paused; canonical window released for main to merge, not agents.

## [2026-09-30] evidence | Bounded Camera Zoom Speed; gate accepted

Extended existing [camera boundary](systems/godot-conversion.md#native-camera-options-boundary), matrix and index. Zoom Speed test-only `0bc0789f`: main `data/diagnostics/camera-zoom-options/run3.log` exit 0, unchanged compiled `mb61bqgcj2`, no production change/new Depot. Authored 2/20 canonical numeric saves; actual wheel +2 targets 17; 48 consecutive JSON post-draw samples compared against independent f32 lerp using observed dt, fixed `1e-4`. Speed 8/logical 15 restored; previous helpers/full Menu/UDP/Exit pass. Runs 1/2 camera phases passed but full fixture failed missing WMO texture-cache prerequisites; local CASC cache prepared, no data staging/test waivers. Main accepts `/tmp/claude/verify-native-camera-zoom-options.md` **bounded saved-artifact and source PASS**: 48 run3 samples (24 per speed), own f32 histories/consecutive frames, max error 1.91e-6 within fixed `1e-4`, canonical 8/logical 15 restore and all Menu assertions. Sampled dt recurrence observed, not universal timing; compiled identity not independently certified; no general dt-boundary/physical-collision/Follow Rate/fresh-process/full-conversion acceptance. Source unfrozen; shutdown paused. Cache preparation covered nine known textures plus 200 TXIDs from active Training Hall’s 203 models. Docs only; Cargo.lock, PLAN.md, user-data and data preserved; no GPU/tests/build/network/delegation.

## [2026-09-30] evidence | Bounded Camera Min/Max; gate pending

Extended [camera boundary](systems/godot-conversion.md#native-camera-options-boundary), matrix and index. Test-only GDScript `67a55dc5` + `d182d61a`: `data/diagnostics/camera-distance-options/run2` exits 0 using unchanged compiled Depot `mb61bqgcj2`; no production change/new build. Authored Max 12 clamps logical distance 15 → 12, then Min 10 saves canonical numeric bounds. Real wheel reaches 10/12 with finite, bounded, monotonic logical-distance checks over 120 frames. Bounds 2/40 restored; wheel factor 1.5 restores logical default distance 15, not Zoom Speed 8. Run1 Min-save failure: endpoint outside control; captured-drag fixture fix, not production. Existing character-select/camera/marker/full Menu UDP/Exit pass. Main accepts `/tmp/claude/verify-native-camera-distance-options.md` **bounded saved-artifact PASS**: 363 logged logical-distance values (3 baselines + 360 post-input frames), independently checked finite, bounded and monotonic; real authored-slider/wheel input and canonical numeric restoration, largest endpoint error 1.91e-6. No independent compiled-build identity proof. Physical rendered/collision distance and zoom/follow-rate acceptance excluded; no all-camera/startup/fresh-process/full-conversion proof. Source editable; shutdown paused.

## [2026-09-30] evidence | Bounded character-select MouseSensitivity; gate accepted

Extended existing [camera Options boundary](systems/godot-conversion.md#native-camera-options-boundary) and matrix from test-only `576a374a`/`c46515ec`, `data/diagnostics/charselect-sensitivity-options/run1.log` exit 0: authored Menu/Camera 0.003/0.006 numeric saves, physical left-drag +4 pixels asserts rendered-basis yaw against original shared math (expected −0.012/−0.024, not logged raw observed deltas). Opposite drags restore baseline/scene/roster/character pose; mouse sensitivity restored to 0.003. Existing gameplay camera/marker/full Menu UDP/Exit pass. Main accepts `/tmp/claude/verify-native-charselect-sensitivity-options.md` **bounded saved-runtime-artifact and source PASS**: original yaw/persistence/actual rendered-basis assertions within 0.00001 radians/restore/roster+Menu; actual displaced yaw not printed, no offline displacement recomputation or zero-error claim; character-select pitch/character creation excluded; no all-camera/all-account-scenes/startup/fresh-reload/full-conversion claim. Docs only; no production change/new Depot/tests/build/GPU/network/delegation. Cargo.lock, PLAN.md, user-data and data preserved. Source unfrozen; shutdown paused.

## [2026-09-30] evidence | Bounded Camera Look/FOV gate accepted

Extended existing [camera boundary](systems/godot-conversion.md#native-camera-options-boundary) and matrix with test-only `2fc386d1`/`edfb4ec6` main slider `run1.log` exit-0 evidence: authored 0.02/105° and restored 0.01/90° save/pitch/physical-FOV checks, numeric f32 tolerance, prior InvertY/marker/full Menu assertions passing. No production change/Depot build; `mb61bqgcj2` unaffected. Main accepts `/tmp/claude/verify-native-camera-slider-options.md` **bounded saved-runtime-artifact and source PASS**: original ranges/math, 4 canonical numeric/2 pitch/2 physical-FOV checks and retained release/InvertY/marker/Menu; no rendered-pixel proof; no fresh-process/all-camera/full-scene acceptance. Source unfrozen; shutdown deferred.

## [2026-09-30] evidence | Bounded CameraInvertY gate accepted

Recorded [CameraInvertY evidence](systems/godot-conversion.md#native-camera-options-boundary): test-only `9671006a`/`47cc143f`/`563968a2`, no production change; owned-UDP/Vulkan Menu `run4.log` exit 0. Authored canonical Off/On persistence, RMB-held +4-pixel pitch deltas −0.03999999165535/+0.03999999165535, released-motion no-change and Off restoration pass alongside marker/original menu assertions. Runs 1–3 fixture failures retained; unchanged Depot `mb61bqgcj2` reused without GDScript-only rebuilds. Main accepts `/tmp/claude/verify-native-camera-invert-options.md` **bounded saved-runtime-artifact and source PASS**; no all-camera/startup/fresh-process/full-conversion acceptance. Shutdown paused; no source pins.

## [2026-09-30] evidence | Bounded target-marker Options PASS; post-extraction gate PASS

Updated [target-marker evidence](systems/godot-conversion.md#native-nameplate-options-boundary), parity row and index from saved ledger/logs: main red5 exit 101 → green3 exit 0, Depot `cshqvl6qmg` native/fixture `1d24222c`. Authored saved Off/On/new F1 self-ring visibility and original menu movement/decoded UDP/Exit pass; green1/2 historical W failures retained. Prior independent `/tmp/claude/verify-native-target-marker-options.md`: bounded behavior/compile PASS with readability findings. `4ef38394` extracts fixture defaults persistence to address introduced complexity; five-line production consumer unchanged. Main Depot `mb61bqgcj2` final-build/final-runtime exit 0 repeats bounded marker/menu proof; main accepts `/tmp/claude/verify-native-target-marker-final.md` bounded follow-up PASS: behavior-preserving extraction, create cognitive 16→15/helper 1 and focused fmt, reusing saved Depot build/runtime exit-0 proof. Existing long functions excluded; no whole-file readability clearance. Corrected fixture wording to canonical options file seeded with `()`, not a `canonical()` call. No all-HUD/full-conversion/startup-false/fresh-process/existing-ring live-edit proof. Docs only; shutdown deferred/source pin removed.

## [2026-09-30] evidence | Bounded TAA + Bloom gate accepted; termination unresolved

Bounded main-observed [TAA + Bloom controller evidence](systems/godot-conversion.md#bloom-production-controller-observer--bounded-main-observed-evidence): `data/diagnostics/taa-controller-bloom-observer/run2.log` exit 0 (`run-2361912-575`), actual production TAA + Bloom controllers, nine HDR 64×48 frames. Isolated original GPU `gpu-run2.log` exit 0 (`gpu-oracle-2363313-828`): 540672 main channel comparisons including post-TAA scene, 0 failures/max error 0; 3072 confidence and 54 jitter comparisons confirm. Oracle owns independent original GPU history, freezes predictions before comparison, and uses fixed HALF ULP + 2e-5 without calibration. Positive Bloom RGB delta with preserved alpha is observation, NOT Bloom numeric parity. Main accepted independent gate `/tmp/claude/verify-taa-controller-bloom-observer.md`: **bounded saved-artifact numeric/callback/Bloom-observation PASS**. Independently recomputed 540672 main + 3072 confidence + 54 jitter comparisons, zero failures/max error 0; same-frame callback order/restoration checked; saved capture and binary integrity checks passed. Bloom remains observation only, not numeric parity. Capture run1 and isolated GPU run1 both exit 124 after bounded PASS/report with all cleanup markers complete; timeout now also occurs without native extension, so cannot be attributed only to native code/logging. Reliable termination remains **FAIL/unresolved**; clean run2 does not fix intermittency. Latest isolated diagnostic `gpu-run3.log` also exits 124. Saved `gpu-run3-gdb-stdout.txt` records main join-frame `r12=0x7f7a48ffa6c0`, the same pointer listed for the “Wayland Events” thread; analysis pending, not a root-cause finding. No full-application/general parity, scale or LDR acceptance. Prior CPU 3806 failures remain; full conversion open, no Handled claim.

## [2026-09-30] fix | Login status shows every connection loss

[Login feedback](systems/godot-conversion.md): `SessionEffect::ShowFeedback` projects disconnect feedback without a screen change; live fixture `godot/tests/login_connection_loss.gd` (server on UDP 5093 killed while connecting) RED before, GREEN at `5400ba7d`. Handshake loss still waits for the 60 s netcode timeout.

## [2026-09-29] evidence | Bounded native controller scale, verifier pending

Bounded main-observed [scale controller evidence](systems/godot-conversion.md#scale-production-controller-observer--bounded-main-observed-evidence): `data/diagnostics/taa-controller-scale-observer/run4.log` exit 0 (`run-2355548-639`), actual controller scale 1 → 0.5 → 1, native internal 64×48 → 32×24 → 64×48, nine HDR frames, no Bloom. Original GPU `gpu-run1.log` exit 0 (`gpu-oracle-2356067-1155`); `result.json` confirms 304128 channels/0 failures/max error 0.00006103515625 (not bit-exact), plus 6912 confidence and 54 jitter comparisons/0 failures. Fixed one expected HALF ULP + 2e-5; independent original GPU history and raw-extents RESET, predictions frozen before actual comparison. Main accepts `/tmp/claude/verify-taa-controller-scale-observer.md` as bounded saved-artifact numeric/runtime PASS: independently recomputed 304128 main + 6912 confidence + 54 jitter comparisons, zero failures; one resolved channel differs by one HALF ULP (maximum 0.00006103515625), not bit-exact. README hash drift is documentation-only, not a runtime or numeric failure; original README bytes were not preserved. Historical manifest records remain unchanged; reruns and ordinary documentation edits are not blocked. Captured raw inputs, outputs and frozen predictions retain SHA-256 integrity checks. Preserve run1 compile failure, run2/run3 exit 124 after bounded PASS; run3 disposal/free complete and quit(0) at 1014 ms yet pthread join blocked, root cause/join target unknown. Clean run4 does not fix intermittency. NOT Bevy scaled physical-history equivalence; no app/Bloom/full-scene/general parity acceptance. Prior CPU 3806/native oracle 124 unresolved; full goal open, no Handled claim. Docs only; no GPU/tests/build/network/delegation; Cargo.lock, PLAN.md and user-data preserved.

## [2026-09-29] evidence | Bounded LDR controller, independent saved-artifact gate PASS

Main-observed [LDR controller evidence](systems/godot-conversion.md#ldr-production-controller-observer--bounded-main-observed-evidence): `data/diagnostics/taa-controller-ldr-observer/run1.log` exit 0 (`run-2329200-1277`), actual `configure(true,false)`, nine 64×48 frames, phases 0..7,0, restored projection/SRGB allocations. Original-shader `gpu-run1.log` exit 0 (`gpu-oracle-2329568-1119`), own SRGB history frozen before comparisons: 430080 main comparisons/zero failures/max encoded-code error 0 plus 3072 confidence comparisons/zero failures and 54 jitter comparisons/zero failures (confirmed in `result.json`). Fixed <=1 encoded RGB/linear UNORM alpha code and decoder residual <=one HALF ULP + 2e-5; independent 256-code hardware decoder, no calibration. Independent verifier `/tmp/claude/verify-taa-controller-ldr-observer.md` accepted **bounded saved-artifact PASS**: independently recomputed 430080 main + 3072 confidence + 54 jitter comparisons, zero failures. Decoded nearest/clamp FLOAT observations, not raw production attachment-byte identity; frozen hand-translated GLSL legacy shader bytes verified, upstream translation equivalence not audited. No full-application/Bloom/render-scale acceptance. Prior HDR proof preserved; CPU 3806 failures/native timeout 124 unresolved. No app/Bloom/render-scale/general parity acceptance; full goal open, no Handled claim. Docs only; no tests/GPU/build/network/delegation; Cargo.lock, PLAN.md and user-data preserved.

## [2026-09-29] evidence | Bounded nine-frame production controller, independent gate PASS

Updated existing AA spec/matrix, [controller evidence](systems/godot-conversion.md#production-controller-observer--bounded-main-observed-evidence) and index from saved `taa-controller-observer/run1.log` and `gpu-run1.log`, both exit 0. Main observed nine consecutive 64×48 HDR frames, phases 0..7,0, restored projection/raw before-after readbacks; isolated original GPU oracle 430080 channels/0 failures/max error 0 plus 3072 confidence comparisons/0 failures. Expected motion uses negated raw motion plus independently computed HALTON delta; oracle owns expected history, never production feedback. `configure(true,true)` models HDR metadata, no actual Bloom. Independent gate 1153 (`/tmp/claude/verify-taa-controller-observer.md`) accepted bounded PASS: 430080 main + 3072 confidence + 54 jitter comparisons, zero discrepancies; all 62 capture/36 oracle binaries finite, source-dataflow audit passed. Recorded production-source hashes are provenance, not rerun gates; upstream translation fidelity remains outside this saved-artifact scope. Controller-owned SubViewport only; GameClient/saved Options/authored scene/LDR/full-scene/Bloom/render-scale and general acceptance excluded. Prior CPU 3806 failures/native timeout 124 unresolved. Docs only; no tests/builds/network/delegation/production changes.

## [2026-09-29] evidence | Bounded production TAA observer, independent gate PASS

Updated existing AA spec/matrix and [system evidence](systems/godot-conversion.md#production-effect-observer--bounded-main-observed-evidence), linked from index. Main read saved production capture and isolated original-shader GPU oracle logs: exits 0, three HDR zero-jitter frames, 135168 numeric comparisons/0 failures plus 3072 constant-confidence comparisons/0 failures. Preserved failed CPU ideal-bilinear/half gap (3806/135168), observed-not-fitted reset confidence difference and native-project timeout 124 after numeric report; teardown/logger cause unproven. Independent verifier 1148 (`/tmp/claude/verify-taa-production-observer.md`) bounded PASS: 135168 main channels + 3072 confidence comparisons, zero failures. One frame-2 resolved channel differs by one HALF ULP within unchanged tolerance; not bit-exact parity. Saved-artifact audit only, no new runtime proof. Separate controller nine-frame evidence/agent 1150 pending excluded. Callback-only scope; full conversion open. Docs only; no tests/builds/network/delegation/production edits.

## [2026-09-29] source audit | AA/HDR texture sampling

Recorded checked [source boundaries](systems/godot-conversion.md#aahdr-texture-sampling--bounded-source-audit) in the existing conversion page; index links there. No default HDR bug; scoped legacy TAA bias does not establish missing mip detail for one-level legacy BLP uploads. Native complete DXT chains remain a separate sampling-parity concern, not grounds for global −1. Local pinned Godot spatial sampler bias checked; rendered equivalence/full parity remain open. Docs only; no builds/tests/network/delegation/merge.

## [2026-09-29] evidence | Bounded native AA, raster formats and lifecycle

Updated existing spec/matrix, [AA evidence](systems/godot-conversion.md#native-antialias--bounded-missing-consumer-evidence) and index. Native `20dde8b5` startup/unrelated authored option commits each exit 0: None/MSAA4X/Taa yield 0/846/698 intermediate pixels among 4318 samples with exact UI; independent `/tmp/claude/verify-native-aa-bounded.md` PASS. Raster formats `cdb9ac98` exit 0: 128 cases, 16352 comparisons, 0 failures/engine errors, fixed tolerances. Lifecycle `bb1731f8` exits 0 for saved-file Taa → None → MSAA4X → Taa via authored frame-cap toggle commits, replacement camera, projection fields over 32 frames, settled pixels and exact UI; Independent verifier 1121 (`/tmp/claude/verify-taa-formats-lifecycle.md`) bounded PASS covers 128 format cases/16352 checks, mode/replacement, resize/isolation and full-scale Bloom; explicitly excludes later motion/half-scale. Authored `25f04e62` no-server `--screen charcreate` exits 0 with owned saved Taa, real creation camera/model + UI and preview-only hide/restore pixel controls: actual-model-visible smoke, not image parity or full-game parity. Independent verifier 1129 (`/tmp/claude/verify-taa-supplementary.md`) bounded PASS for exact motion `7560106b`, half-scale `c6b1b86b` and authored `25f04e62` saved artifacts: 39 independently decoded captures, no runtime reruns. Covers sampled opaque motion interiors/settled disocclusion, functional half-scale Taa + RCAS + Bloom, offline preview hide/restore attribution and exact UI; not temporal/HDR/image parity, authenticated creation or GPU-resource lifetime proof. Float32 boundary precision explains count differences; counts are not goldens. Independent identity input sampling corrects invalid CPU ideal sRGB oracle (RGB 187: GPU 0.49609375 versus ideal 0.496933043), not TAA-output calibration; tolerances unchanged. Test-only extent `d47ca7fd`/parse fix `cb9e7306`, Bloom `31ac35f2`, half-scale `c6b1b86b` and motion `7560106b` exit 0: resize/restore and independent SubViewport pixel isolation, saved Taa + Bloom startup/live, production scale 0.5 + RCAS + TAA + Bloom, analytic object/camera white-patch motion and bounded disocclusion, exact UI. Final motion edges 1096 versus startup 698, not pixel-identical convergence. Exact combined numeric/HDR equivalence, renderScale exceptions, first-reset/per-frame history, skinned/transparent motion, full-scene temporal parity, multivendor and export remain unproven. Full conversion stays open; no Handled claim. Saved logs read; no delegation/build/GPU/operational execution.

## [2026-09-29] evidence | Native AA source integration, acceptance pending

Updated existing spec/matrix, [source inventory/proof matrix](systems/godot-conversion.md#native-antialias--bounded-missing-consumer-evidence) and index. Source integration `20dde8b5` applies persisted None/MSAA4X through both 3D/2D MSAA and custom root NativeTaa, with stock TAA false. Camera ownership, eight-phase pre-draw jitter/post-draw restoration and per-camera Bloom-required Hdr metadata retained after Off are present. Effect `061332b1` sRGB/HALF copy/MRT ping-pong, raster kernel `b9c4c888` and velocity conversion `5232da86` are source-only. All current runtime acceptance awaits Depot/GPU; no Handled claim. Historical pre-consumer Taa RED `90cf7e66` cleanly exits 1: saved Taa, MSAA 0/stock false, 0 intermediate/4318 edges (`data/diagnostics/taa-native-red/run-90cf7e66.log`). Independent `/tmp/claude/verify-taa-kernel-and-jitter.md` PASS covers previous HALF-only `d3a8ebd4` (135664 comparisons) and separate 30-frame fractional-jitter diagnostic, not new variants/controller. Raster sRGB harness agent 1114 pending. Legacy `MainPassResolutionOverride` extraction gap remains source-only, unproven at runtime, not a new format contract. Docs only; no code/build/GPU/tests/agents.

## [2026-09-29] evidence | Bounded standalone TAA kernel and jitter lifecycle

Updated existing AA spec/matrix, [scoped wiki evidence](systems/godot-conversion.md#native-antialias--bounded-missing-consumer-evidence) and index from saved artifacts. Kernel `1757485b`, CPU corrections `32ee3be0`/`5de2722d`: 37 goldens pass. Independent harness `e9129cc1`/parse fix `1de24ddf`/physical-oracle correction `d3a8ebd4`: saved exit 0, 48 fixtures/135664 comparisons, 0 failures/engine errors; 8×8 dyadic physical probe replaces invalid 7×5 CPU oracle without changing 17×9 temporal cases or tolerances. Separate jitter lifecycle saved exit 0: 30 frames including 16 fractional frames, projection restoration and raw-vector assertions; source/argv/environment hash manifest saved, no fractional-centroid quality test. Independent kernel/jitter verification pending. Production consumer/controller, history quantization, MSAA and integrated temporal acceptance remain open; no stock-TAA equivalence or completion claim. No code/runtime/build reruns or source-policy change.

## [2026-09-29] evidence | Independent public TAA-input audit

[Scoped AA evidence](systems/godot-conversion.md#native-antialias--bounded-missing-consumer-evidence) now records `/tmp/claude/verify-taa-public-inputs.md`: 800 finite decoded floats, 180 fixed-oracle interior samples, integer-offset depth cross-section centroids, normal disposal and complete saved stderr audited. No rerun, fractional-jitter, temporal accumulation, executed-source hash attestation or production AA acceptance.

## [2026-09-29] evidence | Bounded persisted native antiAlias missing consumer

Added adjacent antiAlias conversion-spec subsection, Missing matrix row and [scoped wiki evidence](systems/godot-conversion.md#native-antialias--bounded-missing-consumer-evidence) from the saved AA ledger. Recorded `cdd6bfeb` actual native Msaa4x assertion with timeout 124, not clean RED; ALBEDO correction `efc05be9`; distinct geometric fixture `d0cebe67` None startup/unrelated 144 FPS-cap commit control exit 0, exact UI/finite image, 0 intermediate/4318 edge samples and main-inspected PNG. Stock TAA differences remain source-only. Public projection/velocity 20-frame diagnostic exits 0, feasibility only; independent report absent/pending. All production AA modes and separate temporal oracle remain open. No new UI/CLI contract, code, runtime/build/test execution, or bloom-proof changes.

## [2026-09-29] evidence | Bounded native bloom lifecycle and shutdown observations

Updated existing bloom spec, matrix, [[godot-conversion#native-bloom--bounded-in-progress-evidence]] and index for test `72cb71fe` plus comment-only `48e626c3`. Main's verbose lifecycle log exits 0: startup/replacement halo 0.21182088022276 versus disabled 0; observed owning-window/viewport/capture resize to 1440×720 gives halo 0.21571181157681 versus disabled 0. Whole independent SubViewport image unchanged within 1/255 with finite-channel checks; exact higher UI preserved. Main inspected `lifecycle-resized-enabled.png`. Fixture frees client/cameras/SubViewport; ROOT controller persists until normal SceneTree shutdown, not explicit disposal or real-world logout/transfer proof. Three isolated production-bloom 12-frame shutdown controls exit 0 without ObjectDB warning; original warning unattributed, not fixed. Combined functional `e7db5844` verbose run exits 0 without ObjectDB warning/errors: saved/observed Render Scale 0.5, production RCAS startup routing plus bloom and fixture MSAA4X; startup/live halo 0.21015625604196 versus disabled 0, intensity 1 halo 1, exact UI/emission preserved. Combined capture inspection and verifier 1071 remain pending. Not persisted MSAA Options or combined HDR/numeric parity proof. Broad/rapid resize, real-world transfer, explicit controller disposal, general MSAA and remaining platform/full-scene parity stay open. Recipes remain wiki-only; no runtime/build reruns.

## [2026-09-29] evidence | Bounded native bloom numeric and startup/live proof

Updated conversion spec, capability matrix, [[godot-conversion#native-bloom--bounded-in-progress-evidence]] and index from saved logs/ledger only. Production `8c704b87` retains packed compute downsampling and restores legacy raster tent/hardware additive upsampling; unchanged Rust `90fc989b`/controller `72030edd` reuses Depot `s4lftbfxrw`. Independent raster oracle + CPU constant goldens: 20,268,551 comparisons, 0 failures/engine errors, exit 0, original tolerances. CPU `6082282d`: independently 8604 checks. Actual enabled startup/Off/On/intensity and `e60ec838` disabled startup → first On after existing camera pass persistence/emission/exact UI with halos 0/0.21182088/1. Initial enabled run has two unattributed ObjectDB instances warnings; paired verbose and lazy runs have none, not a proven fix. Broad camera logout/transfer, production resize/MSAA, bloom+RCAS, export/vendor paths and full-scene/HDR parity remain unproven. Supersedes prior pending CPU/native/numeric evidence; recipes remain wiki-only. No runtime/build reruns.

## [2026-09-29] evidence | Native bloom in progress

Updated [[godot-conversion#native-bloom--bounded-in-progress-evidence]] and index only. Saved `06da11eb` native RED exits 1 after controls/persistence and disabled bright-source validation: no startup halo. Initial black unshaded-emission fixture was a corrected precondition, not a runtime bug. Public `POST_TRANSPARENT` compute probe exits 0 for linear tint, scale/resize, exact UI edges and disposal; Vulkan packed storage/sampling support is true. Both are feasibility evidence, not bloom. Test-only CPU reference `7b0d911e`/`6082282d` excludes packed rounding; reported 8604 assertions lack verified saved proof, so no pass credited. Integrated controller `72030edd`/Rust `90fc989b` remain pending effect/build acceptance. Recorded legacy HDR retention, RCAS `With<Camera>` query and filter semantics; no direct Godot Glow equivalent or full parity claim.

## [2026-09-29] verification | Bounded native RCAS rendered GREEN

Recorded production `543ca754`, integrated `27727fa0`, and actual native fixture `94e7e02a` proof in [[godot-conversion#bounded-rcas-compatibility-investigation]]. Depot `9n5kpbsc6l` built in 62.1 seconds, exit 0; interrupted `zp1dlvr07r` supplies no compilation proof. Native Vulkan run exits 0 for startup 0.75/live 0.5 RCAS oracle, live 1.0 bypass, unchanged discriminating UI edge pixels, and finite opaque black patch. Main inspected the capture. Independent final evidence remains pending; full legacy scene parity and other outstanding Options remain open. Updated both conversion specs and index; recipes remain wiki-only.

## [2026-09-29] fix | Native FontString shadows

The Godot projection ignored FontString `shadow_color`/`shadow_offset`, so the tracker's dark-gold quest title and grey objective line had no shadow and were nearly invisible on grass. Labels now get `font_shadow_color` and `shadow_offset_x/y` (WoW y-up flipped). RED on the previous build: "QuestBlock28766HeaderText has no (1, 1) black shadow"; GREEN `world_minimap_quest.gd` exit 0 with background-relative glyph/shadow pixel counts (title 85/26 vs grass 40/0, line 50/43 vs 0/0). Applies to every native FontString with a shadow. See [[quest-ui]].

## [2026-09-29] implementation | Godot minimap and objective tracker

Branch `minimapquest`: native `MinimapCluster` (TOPRIGHT) and `ObjectiveTrackerFrame` (TOPRIGHT −110, −275). Pure tile/composite/blip/zone/clock logic in `godot/core/src/minimap_data.rs`; the tracker reuses the shared Bevy component through `from_watched`. Two projection fixes were needed: onclick on textures/font strings now clicks, and dynamic-texture pixel updates redraw. Live `godot/tests/world_minimap_quest.gd` exits 0 on a private server (tile `azeroth/map32_48`, rendered pixels = composite, arrow along W movement, quest blips, zoom; tracker "Beating Them Back!" 0/6 and collapse). Core `minimap_data` 8/8 on Depot `--test`. See [[minimap]], [[quest-ui]] and the [minimap spec](../specs/minimap.md).

## [2026-09-29] implementation | Native chat frame

The Godot client had no chat. It now hosts the shared `ChatFrame1` screen on a `ChatFrameUI` RegistryUi. The Bevy channel mapping, whisper recording, tab entries, scroll hold and view builder moved into shared files, so both clients format and route chat the same way. The edit box gained the retail `Say: ` header and ChatFontNormal. `godot/tests/world_chat_flow.gd` passes against a private server: geometry, MOTD, W typing without moving, server echoes of say/yell/emote, the offline-whisper error, local command lines, wheel scroll, history, and Escape without opening the game menu. See [[chat-frame]].

## [2026-09-29] port | Spell visual kit sounds

Frostbolt's cast sound was the synthetic 140 ms CastStart sweep (`sound_cast.rs`). No DB2 sound was played.

**Changes.**
- SoundKit and SoundKitEntry are exported from local CASC 12.1.0.69933, and `SpellVisualMissile.SoundEntriesID` is now exported.
- The catalog resolves `SpellVisualKitEffect` type 5.
- `spell_sounds.rs` plays one weighted entry per SoundKit on the kit's unit, looping (0x200) while a held kit lasts.
- Frostbolt now plays precast_start + precast_loop (85501/85500), cast (85502) at `SpellGo` and impact (85503) on arrival.

**Sweep removed.** The synthetic CastStart sweep (`CastSpells`) and its sound-click cast stages are removed.

**First-cast freeze.** The fixture's 0.03-0.14 s cast was a 25-30 s main-thread stall: the catalog was rebuilt from the CSVs at the first cast, because checkouts at different formats overwrote each other's cache file. Godot's 8-step delta cap hid the stall's length from the client clock. The catalog now loads on a worker thread from client start, into a per-format cache file.

**Still not played.** `$SCD` (CreatureSoundData).

Details in [[spell-visuals]].

## [2026-09-29] verification | Native Render Scale bounded buffer/UI GREEN

The actual GPU RED at `e3a5c92b` measured 1280×720 instead of 960×540 for saved startup 0.75. Exact `ca2f75c0` built through Depot `40n2xh4n2l` (exit 0; `data/diagnostics/render-scale-depot-green-build.log`). Offscreen Vulkan GREEN then measured startup 0.75 = 960×540 with a 1280×720 target; live 0.5 = 640×360; live 1.0 = 1280×720; resized 1.0 = 1600×900; and resized 0.5 = 800×450 (`data/diagnostics/render-scale-green/green-ca2f75c0.log`, exit 0). The same 2D geometry/red-pixel probe and all final targets pass; captures are in `data/diagnostics/render-scale-green/captures/`. This is bounded compositor-buffer/UI proof, not all 3D scenes, visual equality, bloom, or full parity. Legacy paired CAS/sharpening below 0.999 remains a full-conversion obligation. See [[godot-conversion]] and the [parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-29] fix | Native particle-density placement snapshot

`79d792b0` captures density per doodad-emitter placement; `TerrainObjects` updates only the default after Options saves. Existing placements retain their captured rate, future registrations use the new default, and original `NO_GLOBAL_SCALE` emitters ignore global density. The controlled disposable 197007 portal copy clears only its six `NO_GLOBAL_SCALE` bits and preserves the original asset hash. In the actual `GameClient`, real Options 100→10 keeps known-pool quads at 485.67→488.0; owned same-map `NewWorld` creates a fresh placement at 46.67. Depot `kdhjvgmnt3` and runtime GREEN both exit 0 (`data/diagnostics/portal-density-depot-green-build-retry1.log`, `data/diagnostics/portal-density-green-79d792b0.log`). This is state/rate proof, not retail pixels, audible output, or full parity. See [[godot-conversion]] and [M2 particle spec](../specs/m2-particles.md).

## [2026-09-29] verification | Native persisted M2-particle startup gate

Merged `d23012b4` + `e5671528` prove the persisted `particleEffectsEnabled` startup boundary through the downloaded Depot `d1q5w3x6v3` fixture and authenticated Azeroth `GameClient`. Both runs exit 0: actual placed `sw_magicdistrict` MODD 1112 portal meshes have no particle pools/emitter state when disabled; enabled has six positive MultiMesh visible-instance counts (42, 21, 21, 21, 21, 2; 143 total; scene totals 711 pools/emitters). Evidence: `data/diagnostics/portal-particles-depot-build-e5671528.log` and `data/diagnostics/portal-particles-{disabled,enabled}-e5671528.log`. This is headless runtime state proof only—not pixels, audibility, live-toggle behavior, density, or full Godot parity. Updated [M2 particles](../specs/m2-particles.md) and the [parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-29] fix | Native FontString width-bound word wrapping

The authored merchant Options descriptions exceeded the row at 0.75/1.25 because Godot Labels with wrapping off grew their minimum width beyond the authored 370 logical px. Legacy text bounds width and uses word-boundary layout. `9c12ce9e` requests word wrapping in native FontString projection. Depot `jcmsl5lx24` built revision `9c12ce9e` with `native_input_fixture` (exit 0; existing `NativeWmoGroup::fdid` warning); the owned merchant-click runtime exits 0 and confirms full text, two-line long descriptions, 370-logical-px label width, row/content/root containment, and non-overlap at both scales (`data/diagnostics/options-fontstring-wrap-{depot-build.log,green/merchant-click-runtime-short.log}`). Main inspected `merchant-options-125.png`. This is a bounded single-owner rendered check, not pixel equality, all-UI-owner coverage, or conversion parity. See [[ui-system]].

## [2026-09-29] implementation | Godot RegistryUi scale ownership

Shared live-owner traversal now drives native UI scale and click draining, with startup and frame-end sync. Tooltip and entrance-bar coordinate producers use the logical viewport of the scaled canvas. The first-visible login assertion was RED on `e8599fb0` and GREEN on `34b12ea5`; owned reset-windows and merchant-click fixtures also pass against `34b12ea5` (merchant script `dd11bdd2`). The owned sound-click fixture at `b63c64ec` adds known Slam to the rightmost slot: controlled old `34b12ea5` library + new fixture RED placed the tooltip's right edge at 711 px in an 800 px viewport (`data/diagnostics/ui-scale-edge-old34-native-b63-fixture-red.log`); Depot `c22n43vc57` built the corrected library and fixture, then sound-click GREEN preserved existing casting/visibility/click assertions (`data/diagnostics/ui-scale-b63c64ec-sound-click-edge-green.log`). Entrance-bar coordinate conversion has source-space proof only because the owned fixtures do not show an entrance. See [[ui-system]].

## [2026-09-29] investigation | Spell missiles leave at the cast clip's release event

**Symptom.** Frostbolt's missile launched the instant `SpellGo` arrived. The hands were still at the chest in ReadySpellDirected 51, and the missile hit before SpellCastDirected 53 thrust the arm.

**Retail rule.** A pending missile is released by the caster's cast clip firing the M2 event `$CSL`, `$CSR` or `$CST` (wowdev.wiki/M2 Events). HumanMale and HumanFemale HD fire `$CSL` at 200 ms of clip 53. `SpellMisc.Speed` is in yd/s (TrinityCore `Spell.cpp:2515`). The 7.7 yd Frostbolt flight at 35 yd/s really does last 0.22 s.

**Changes.**
- M2 event parser (`m2_event.rs`; `.skel` models read AFM2 timestamps from `.anim`, each file loaded once).
- The action layer reports a pending release event.
- `SpellEffects` holds the missile until the event fires, then launches it facing the target.

**Server gap, not fixed.** game-server applies the damage and combat log in the same tick as `SpellGo`, so the number shows about 0.4 s before the impact. TrinityCore delays each hit by `max(dist, 5) / Speed`.

Details in [[spell-visuals]].

## [2026-09-29] verification | Options integration correction

`253f8238` merged verified Options `e8599fb0`; `818d7c53` reconciled `ensure_art` without losing concurrent `67e6e430`. At `4c0acc91`, `958e612a` fixed merchant-fixture hostility with real friendly `UnitFactionTemplate`s: vendor `NpcFlags` alone does not prevent auto-attack. Existing Depot `fg7w9m1g7w` and four owned modes pass (three retained, merchant fresh). Original `CombatEvent` still reaches outcome audio and visuals once; `SpellGo` and later frame steps survive `FrameError::Client`. Pure Rust tests were not run under Depot-only constraints. Updated [[godot-conversion]] and [parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-29] systems | Polymorph

Added a Polymorph section to [[spell-visuals]]. Polymorph 118 works end to end against the game-server `polymorph` branch: the spy's display swaps in place to the sheep and back, the target ring resizes with the swap, and a sheep no longer tries to hold the spy's weapons. Finding: Movie Maker compresses server time about 3.6x at this scene's 8 fps, so the fixture can also grab wall-clock frames (`POLY_GRAB`).

## [2026-09-29] investigation | Selecting a target no longer starts auto-attack

**Bug.** The server started auto-attack, and combat, on every `SetTarget` to an attackable unit. A Tab-cycling mage swung unarmed at each dummy it selected, and 0/1 damage numbers appeared before its Frostbolt.

**Retail rule.** Selecting only sets the selection. Auto-attack starts from `CMSG_ATTACK_SWING`, which the client sends on:
- a right-click on an attackable unit
- Auto Attack
- casts of SPELL_ATTR1/ATTR2 auto-attack spells, such as the warrior's Attack 88163 and Slam

**Changes.**
- New `AttackSwing`/`AttackStop` messages and `AttackStart`/`AttackStopped` echoes (shared-protocol `06534c1`).
- The server attacks only on request (game-server `9db412f`, `2572257`).
- The Godot client sends the requests ([[spellbook-action-bar]]).

**Proof.** Re-recorded in [[spell-visuals]].

**Picking.** The player's M2 header box (about 11 x 5.5 x 4 yd for HD human male, the animation extents) swallowed every click on a unit in melee range in front of it. The pick now raycasts each broad-phase unit's drawn triangles, CPU-skinned in the current pose, like the Bevy client's `MeshRayCast`; the box only rejects units the ray misses. Right-click then attacked the dummy live ([[spell-visuals]]).

## [2026-09-29] port | Godot combat animations and spell visuals

The Godot client now plays combat and spell animations and spell effects.

- **Melee.** Swings (Attack 16-19 by main-hand class), victim reactions and the Ready stance are driven by `CombatEvent`.
- **Spell visuals.** Kits come from the local-CASC SpellXSpellVisual → SpellVisualEvent → SpellVisualKit chain, with the visual chosen by caster `PlayerCondition`. They cover:
  - kit clips through SpellVisualAnim/AnimKit
  - kit M2 models on attachments
  - Frostbolt's missile
  - impact kits on hit or primary units
- **Protocol.** New `SpellGo` message, broadcast by the server to every client replicating the caster.
- **Particles.** Keyframed `emissionRate`/`enabledIn` tracks made Battle Shout's burst visible.

The Bevy combat anim constants (51/46/...) were wrong for Retail. Warrior (Slam, Battle Shout) and mage (Frostbolt) videos are in `data/diagnostics/spellcast-anim-2026-09-29/`. Added [[spell-visuals]]; updated [[animation]].

## [2026-09-29] system | Depot-native Godot extension build documented

Root `cargo run`/`rd` retains a local std-only launcher and normal local Godot import/launch while `scripts/depot-build.py --root <checkout>` builds the Linux x86_64 GDExtension remotely. The documented boundary includes source-only snapshots, shared remote caches with locked target sharing, atomic worktree-local library install, explicit no-local-Cargo failure behavior, and prerequisite tools. The $100 monthly figure is conditional budget planning, not a configured billing cap; provider limits and cache GC require main verification. No gameplay, renderer, or conversion-parity claim follows.

Added [Remote Godot builds](../remote-builds.md); updated [[godot-conversion]] and the [Godot conversion specification](../specs/godot-conversion.md).

## [2026-09-28] investigation | Godot missing assets no longer end the session

One missing cursor BLP disconnected the Godot client: every frame-step error stopped the account. Only `Account` transport failures (`SessionError`) stop it now; asset failures are logged once and stay absent. See [godot-conversion](systems/godot-conversion.md#frame-failure-policy).

## [2026-09-29] ui | Native MerchantFrame placement slice

`MerchantFrame` alone uses selected-character `ui_layout.ron` placement, shared 24-unit title dragging, UI-scale clamp, reopen and Options reset. `ContainerFrame0` and `StackSplitFrame` do not move with it; native Panel L/R ordering and raise parity remain open. Extended owned merchant-click fixture covers vendor ray interaction, saved placement, backpack independence and click-audio behavior. Baseline installed binary at `80d2cbc6` RED on unscaled merchant root. Main-owned Depot build `70561683` passed with aligned `godot/Cargo.lock` after an initial `--locked` failure; the downloaded binary plus script `9f64af5f` passed the owned UDP fixture (`data/diagnostics/merchant-placement-green-9f64af5f.log`). Updated [[merchant-frame]] and [window manager spec](../specs/window-manager.md).

## [2026-09-29] ui | Native SpellBookRoot managed placement implementation

`SpellBookRoot` now uses canonical selected-character window positions and the first Panel slot (16, 104), with title-only drag, logical clamp, save/reopen and reset alongside `WorldMapFrame`. The extended authenticated three-process fixture RED on the old downloaded binary at `a4b86a5e` (missing `SpellBookRoot`). Main-owned Depot build `c66bdfef` passed; the downloaded fixture with corrected script `e0d02e78` exits 0, including MAP_SAVED, MAP_REOPENED, authored reset, and fresh-process checks (full log `data/diagnostics/spellbook-placement-fixture-e0d02e78.log`; it also records listfile-lock and missing-local-CASC diagnostics). Merchant coexistence and left/right panel stacking remain unsupported. Updated [[spellbook-action-bar]], [[world-map]], and [window manager spec](../specs/window-manager.md).

## [2026-09-29] investigation | Optional Depot fixture export

At `be6aeedb`, `scripts/depot-build.py --root <checkout> --fixture native_input_fixture` completed on Depot project `003c4ttwqh`, build `5nqxfxrzpt`, in 206.042 s. It exported the default library and installed `target/debug/examples/native_input_fixture` beneath the originating checkout. The directly launched downloaded fixture then passed local owned-UDP `sound-click` in 30.787 s. Default builds remain library-only; allowlisted `native_npc_visual_fixture` was not remotely built or run. Verifier 963 is pending. This is bounded build/export and fixture proof, not a performance or parity claim. Evidence: `data/diagnostics/options-depot-20260929/{fixture-build,exported-fixture-runtime}.log`.

Updated [Remote Godot builds](../remote-builds.md), [[godot-conversion]], and [[depot-cross-worktree-freshness]].

## [2026-09-29] investigation | Depot cross-worktree Cargo freshness

A build from worktree A, then older B, then A failed with `E0425` although A's exact source was snapshotted. The stale state was the shared Cargo target cache. `e4b213a8` refreshes staged compile-input timestamps only after obtaining the shared target lock; the repeated A/B/A sequence rebuilt dependencies and passed. The first full refreshed Options build passed in 49.274 s (`003c4ttwqh` / `cpw7crx3ww`) and installed the 272,736,240-byte extension. Verifier 954 then isolated pinned Godot 4.7.2 and proved `GameClient` registration, `Node3D` instantiation, and scene-tree attachment; this is bounded class-load proof only. Cleanup's 98.28 GB allocated-cache result is not treated as physical-space reclamation.

Added [[depot-cross-worktree-freshness]]; updated [Remote Godot builds](../remote-builds.md) and [[godot-conversion]].

## [2026-09-29] system | Native main action bar HUD visibility

Committed `hud.show_action_bars` now hides the entire cached native main bar without clearing its slots or bound-key casts; restoring it reuses the same node and slot, then restores pointer casting. Owned-UDP `sound-click` passes the hidden-key `SpellCastIntent`, intentional restored click, and subsequent right/release/keyboard/reopen no-replay checks, plus CastStart request/repeat/inactive/reset/mute/removal stages (`/tmp/claude/actionbar-consumer-targeted-green.log`, commit `67e6e430`, base `539f1b86` plus scoped diff). Verifier941 is pending; no independent-pass claim. Updated [[spellbook-action-bar]], [spellbook/action-bar spec](../specs/spellbook-action-bar.md), and the [Godot parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-29] system | Original CombatEvent native outcome audio

Final gate `bf0dfde4` PASS rebuilds `target/debug/libgame_engine_godot.so` and then proves 65 reliable ordered original `CombatEvent` UDP messages reach 65 spatial outcome players exactly once, including beyond the 64-entry `CombatLogEvent` deque. The shared Impact/Heal/Miss/Interrupt PCM and gain policy, ignored/zero/unresolved suppression, master/effects/mute/music behavior, remote removal, and forced-disconnect reset remain covered. Account dispatch preserves direct `CombatEvent` routing before spell-state/log handling; changed routing functions are readability-clean, with only pre-existing whole-file/helper findings. Current-server spell results still send only `CombatLogEvent`; the original producer remains dormant for those results, with no server change, log-event mapping, or fallback. No audible/hardware or full-parity claim. Evidence: `/tmp/claude/verify-native-outcomes-complete.md`. Updated [[sound]] and [Godot parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-29] system | Owned UDP merchant click audio

The owned `native_input_fixture merchant-click` mode ray-picks a replicated vendor and sends `InteractNpc`; only then does its owned UDP server send `InteractionOpened`, `InventorySnapshot`, and `VendorInventory`. The authenticated client's authored buyback-tab and close-button left-down reach its Effects player; right press, release, Escape, and reopen stay quiet. The server observes two opens and two closes. Final exit-0 output: `/tmp/claude/merchant-click-f1609b4e-final.log`. Optional scenery missing-texture diagnostics remain within the explicit fixture exception. Verifier918 independent review is pending. This does not prove all-owner coverage, audible/hardware output, purchase/sale, mutable sound settings, or full parity. Updated [[sound]] and [[merchant-frame]].

## [2026-09-29] system | Owned UDP spell-button click audio

`e8a90de4` drains active spellbook/action-bar and merchant owners into native UI-click playback. The isolated `native_input_fixture sound-click` mode then supplies a replicated local player plus actual `KnownSpellsSnapshot` and `ActionBarSnapshot` over owned UDP. Its final pinned-Godot log records action-bar **finished** and spellbook **active** before release (the assertion accepts either state within 40 ms), both at owned Effects gain `0.44`; right/release/keyboard/reopen remain quiet. Merchant is wired but has no owned real-runtime proof: rejected live attempts and production tests without a sound-observation API do not count. After fresh-cache SQLite contention, `795ec1f3` stages a read-only canonical-cache backup; final runtime has no listfile lock, though optional spell icons remain unavailable in local CASC. `78cf942c` adds the proof; `4580bebb` records it. Evidence: `data/diagnostics/native-spell-click-{build,extension-build,final}.log`. Verifier915 is pending; no independent-pass claim. Audible output, mutable Options gain/mute in this fixture, merchant clicks, and full parity remain unproven. Updated [[sound]] and [[spellbook-action-bar]].

## [2026-09-28] investigation | Godot WMO doodad cost, global WMO culling, MWDS, doodad light

WMO doodads' per-frame cost was the Rust cull (hash sets, per-batch FFI reads), not node processing; `6cb884ec` halves it. The Stockade global WMO is portal-culled; MODF 0x80 MWDS sets and WWV's WMO doodad light are ported. Updated [[godot-stormwind-fps]], [[wmo-format]], [[wmo-retail-lighting]], [[godot-conversion]].

## [2026-09-28] ui | First native managed WorldMap window

`WorldMapFrame` now uses character-scoped canonical `ui_layout.ron` for saved logical top-left, scaled title dragging and Wide-slot reset. The owned-loopback Options fixture covers effective 5/6-scale drag, canvas right-click navigation/button exclusion, resize clamp, then a second authenticated Godot process renders the saved placement before reset; the reset returns the map to its slot and a third process reads the reset file (`/tmp/claude/world-map-fresh-green-attempt.log`). Simultaneously open map + Options reset remains untested through the user interface. Updated [[world-map]].

## [2026-09-28] ui | Reset fixture authored-input preflight

The isolated Reset Window Positions fixture sources staged inputs from the canonical Git checkout and links real customization requirement/race/equipment data rather than fabricated requirement headers. The `07586b86` run exposed an omitted race-model CSV before reset. A loader-list audit then added all 12 customization/cache source CSVs to the upfront manifest; its targeted missing-input test passes. The equipped-player scenario and reset assertions remain unchanged. At `ba535fcd`, the owned-loopback fixture exits 0 after the authored reset, same-process reload and fresh-process persistence checks (`/tmp/claude/reset-windows-ba535fcd-runtime.log`). Updated [[godot-conversion]].

## [2026-09-29] ui | Native Reset Window Positions

Final bounded review PASS (`/tmp/claude/verify-native-window-reset-final.md`): the authenticated equipped character still loads from canonical authored customization inputs; Reset removes only ID 17's `window_positions`, retains ID 18, account-wide edit layout, and Options modal data, then a fresh process reads the persisted result. Fixture `ZoneLight` rows only provide parseable startup controls (map 99999, no polygon); they do not prove lighting behavior. Native managed-window movement remains absent. Updated [[godot-conversion]], [[ui-system]], and the [window-manager spec](../specs/window-manager.md).

## [2026-09-28] ui | Native local-player unit frame

Selected local replicated name, level, health, combat, and powers feed the shared `PlayerFrame`; rest uses the existing rest-area update. `421eca38` final bounded verification retains authored values, a live health change, off/on visibility, and local-despawn clearing, then fresh-runs the owned UDP fixture to authenticated post-disconnect `CharacterSelect` without a retained local-player position (`/tmp/claude/verify-native-player-frame-final.md`). Visible teardown is only asserted before reconnect: despawn hides `PlayerFrame` and clears its name. No PlayerFrame is inspected after reconnect. Child exit, reader joins, and disposable fixture-root removal are harness lifecycle proof, not UI teardown. Combat/rest icon rendering and post-reconnect PlayerFrame teardown remain unproven. The default fixture's pre-auth `FogDensity` staging failure is separate and out of scope. Updated [[godot-conversion]] and [conversion spec](../specs/godot-conversion.md).

## [2026-09-28] ui | Native target-frame HUD visibility

The native target-frame cluster now consumes `hud.show_health_bars` through the shared unit-frame state. The existing owned-UDP NPC Options fixture proves whole-cluster off/on hide/restoration with the same reselected NPC and separately retained nameplate label (RED/GREEN: `/tmp/claude/target-frame-{red,green}-6073af82.log`). Escape clears selection before opening Options; the fixture does not claim otherwise. The later local-player frame is documented above; native Reset Window Positions is documented separately. See [[godot-conversion]] and [conversion spec](../specs/godot-conversion.md).

## [2026-09-29] ui | Native nameplate Options independently verified

Independent verification of `6d2f7cd6` fresh-runs the private-loopback authored Options fixture against a replicated NPC: HUD health-bars hides frame/fill while retaining the label; camera-to-health-body fade remains separate from viewer-to-unit CVar eligibility; colorblind NPC label restoration leaves health fill unchanged (`/tmp/claude/verify-native-nameplate-options.md`). `6073af82` independently passes root `cargo fmt --check` and locked root binary compilation (`/tmp/claude/verify-nameplate-root-adapter.md`). The pure color test asserts exact player cyan and NPC yellow labels, but no live player plate is reachable: player-vs-player attacks are rejected, friendly-player plates default off, and no authored control changes that. Test-only player attempts remain RED (`/tmp/claude/nameplate-player-*.log`) and changed no source. Full nameplate parity remains open. See [[nameplate-design]] and [nameplate spec](../specs/nameplate-style.md).

## [2026-09-29] system | Native local-player footstep playback

`d3882c9a`/`ece3da3c` add authored local Ogg footstep catalog selection and owned 3D emitters for the selected local player after animation tick, with shared phase/selection and streamed terrain/WMO surfaces. `b773ecfd` final independent verification reuses valid production fmt/check proof, confirms the existing worktree GDExtension library, and fresh-runs the refactored fixture to PASS in 18.65 seconds through movement playback, idle/stop, Options gain/mute/music behavior, and player-removal release. All four temporary fixture CSV links were removed. The retained final log has 668 optional-scenery `WorldObjects` missing-texture errors, but zero script/non-WorldObjects errors, timeouts, or fixture-exit failures; it is not a whole-runtime-clean claim. Fixture readability now passes (cognitive 3, cyclomatic 9). Audible/hardware and full parity remain unproven. Updated [[sound]], [[terrain]], and [Godot parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Native WMO footstep surface override

Shared the legacy root-wide material priority and inclusive smallest-volume placement selection with the native terrain reader. Global WDT and streamed MODF surfaces are queryable before node/physics completion; native footstep audio trigger/playback remains absent. Verifier853 is pending without an independent PASS. Updated [[sound]] and [[terrain]].

## [2026-09-28] system | Native terrain surface query

The native tile reader caches local GroundEffectTexture/TerrainTypeSounds DB2s once and stores shared-policy classifications for each parsed `_tex0`/height-grid chunk. Streamed terrain exposes an exact half-open surface query, cleared on reset. `2209dce4` subsequently adds native WMO material/bounds lookup; native footstep audio trigger/playback remains absent. Updated [[terrain]] and [[sound]].

## [2026-09-28] investigation | Invalid character-creation customization combos

`27d02d52` evaluates ChrCustomizationReq/ReqChoice. A Human warrior is now offered 16 skins instead of 24, and a picked choice repairs the options it depends on. `e18ebee4` composites BlendMode 4/6/7/9 and keeps non-body texture types out of the body atlas, so tan skin 4978 with face 27 renders tan instead of teal. Created [[charcreate-invalid-customization-combos]]; updated [[character-creation]] and [[character-texture-compositing]].

## [2026-09-28] system | Godot vertical swimming and breath bar

Swimmers now float: Space ascends and X (`SitOrStand`) descends at swim speed, clamped to the surface and seabed; the loopback probe now expects the floating height. The server still clamps y to the seabed. Retail MirrorTimer breath bar ported, driven only by GDScript until the server sends mirror timers. Added [[swimming]].

## [2026-09-28] port | Godot M2 particles

Godot draws doodad M2 particle emitters: a shared CPU simulation after WebWowViewerCpp `particleEmitter.cpp` in `godot/core`, and one pooled MultiMesh per (model, emitter). Pools are sized from authored rate × lifetime × 1.15 (cap 500 per emitter, 4096 per pool). Emitters update only while their doodad is drawn and in view, and fade with it. The Stockade portal's six emitters draw. See [godot-conversion](systems/godot-conversion.md#native-m2-particles).

## [2026-09-28] system | Godot nameplates with Retail visibility

Godot draws nameplates only for the target and units fighting the player (`nameplateShowAll` 0), enemies only, within 60 yd, and dims plates behind terrain or WMO collision to 0.4. See [[nameplate-design]].

## [2026-09-28] system | Godot local player speed and stop input

The Godot client now predicts at the server's speed (swim speed, aura multiplier from the replicated `MovementSpeed`) and reports one stop input on release. Updated [[networking]].

## [2026-09-28] investigation | Godot player walks through WMO walls

The Godot player had no wall collision; the original wall ray now runs against the WMO wall bodies before the slope rule. Updated [[stockade-entrance]] and [[collision-system]].

## [2026-09-28] system | WMO interior fog (MFOG) in Godot

`03db2144` ports WebWowViewerCpp `WmoObject::checkFog` into the shared lib and blends its result into the Godot scene fog. Cultists' Quay now uses cave fog RGB (21, 80, 99) at weight 1; Retail is still brighter and bluer there. Updated [[retail-lighting]], [[campsite-fog-and-wmo-selection]] and [[wmo-retail-lighting]].

## [2026-09-28] investigation | Godot camera outside WMO walls

The Godot camera ray had only terrain bodies to hit, so WMO walls did not bound it; WMO wall physics bodies from the shared collision faces now do, built lazily within a frame budget. Updated [[stockade-entrance]] and [[collision-system]].

## [2026-09-28] investigation | Godot Stormwind FPS

Retail scenery distance for doodads (landed in `8fdc22d0`), NPC animation LOD (`0be4373f`) and WMO portal culling (`1723b9cf`) take Stormwind from 13–20 to ~20–25 FPS and ~6.4–7.4k to ~4.3–5k draws. Created [[godot-stormwind-fps]].

## [2026-09-28] system | Shared terrain surface selection prerequisite

`70e047ff` extracts dominant ADT effect, texture, and footstep-surface selection into `terrain_surface_data`, shared by root and `godot/core`. Core concrete layer tests and existing root dominant-selector tests pass; root terrain/footstep loaders, Bevy GroundEffect cache/loading, and unresolved-Dirt behavior remain unchanged. This shared prerequisite did not itself add a native runtime consumer or footstep playback; verifier839 is pending. `3d6df314` separately adds native per-chunk DB2/listfile surface metadata and `StreamedTerrain::surface_at`; `2209dce4` adds native WMO material/bounds lookup through `surface_at_position`. Native footstep audio trigger/playback remains absent; verifier849 and verifier853 are pending without independent PASS claims. Updated [[terrain]], [[sound]], and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Shared footstep phase remains a prerequisite

`2f8a7bfb` moves the Bevy half-cycle observer into shared `FootstepPhaseTracker`; Bevy carries it as `FootstepTracker`. Native `WowAnimationPlayer::footstep_phase` read-only exposes the selected clip's index/ID, duration, and clock for a future observer. `c5b83b15` restores the root adapter's movement-policy import. No native footstep playback or fallback behavior changed; verifier835 and tests remain separate pending work. Updated [[sound]] and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Shared ground-effect DB2 parsing prerequisite

Moved pure GroundEffectTexture/TerrainTypeSounds WDC5 parsing and ordered name-to-surface classification into `src/sound/ground_effect_data.rs`, exposed to root and `godot/core`. Root retains filesystem/cache/CASC and clutter; core byte fixtures cover accepted layouts and error handling. No native terrain runtime integration. Updated [[sound]].

## [2026-09-28] system | Shared footstep selection prerequisite

Moved pure footstep classification and catalog selection into `src/sound/footstep_data.rs`, available to root and `godot/core`; Bevy retains existing loading and handles. Moved five existing policy tests to core and added tied-seed/no-eligible cases. No native playback or adapter fallback behavior change; verifier831 is pending. Updated [[sound]] and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | UI-click and Sound Defaults independently verified

`5274d0c0` independently passes the authenticated world fixture: master changes to 0.25, Music/Ambient/Effects become 0.1125/0.075/0.025, then Defaults restores Music/Ambient to 0.45/0.3. This proves Defaults after material state mutation.

`618d153c` independently passes a fresh owned `GameClient/LoginUI/ConnectButton` left-down click at default FPS (Effects active) and 5 FPS (Effects finished). The earlier failure was fixture timing: the 40 ms click ended before a frame could observe playback. The correction changes only GDScript test sequencing; root/native checks and the protected lock proof at `9145a605` remain applicable because no native source changed. Audible/hardware output, full PCM/audio parity, and conversion parity remain unproven; the matrix remains 0 handled. Updated [[sound]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Native UI click parity slice

Shared the legacy normalized-phase click generator and 0.55 gain through `ui_click_data`. Projected left pointer-down on an actionable frame or ancestor reaches `GameClient`'s owned `NativeSound` Effects player; disabled buttons and non-pointer actions do not trigger it. Targeted PCM and headless Godot fixture cover behavior and player volume/mute. This is not audible-output or full-parity proof. Updated [[sound]].

## [2026-09-28] system | Real-client native sound fixture is bounded

`ce2a8c92` authenticates an owned loopback `GameClient`, loads MCNK area 9 → root zone 12, observes Music `53492`, and observes no zone-12 ambient track. It then drives authored Options Sound controls and observes native player volume/mute/music-enable state. The retained [fixture log](../../data/diagnostics/native-sound-client-final.log) records those markers and exit-0 result.

This is player-state/volume evidence, not audible-output or full-parity proof. Fixture marker discovery is a harness sentinel rather than a production RED; mutation assertions are harness sensitivity only. Independent verifier817 remains pending. Updated [[sound]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] investigation | Native audio shutdown leak timing is bounded

`741344ca` adds `NativeSound`: owned Music/Ambient Godot players consume the shared catalogs, decode local MP3/Ogg/WAV bytes, reject FLAC, sequence tracks by zone, cache streams, and apply live sound options. Its targeted headless fixture exits 0 for playback state, natural completion, zone changes, and lifecycle; missing-catalog and FLAC errors are expected. Verbose exit reports 16 leaked audio stream/playback/Ogg-packet objects and no native sound/player nodes.

A pure-GDScript control with the same two player classes, playback/state steps, stop/stream-clear/free sequence, one frame, and a real 0.5-second timer exits 0 with zero leaked classes. The otherwise-identical immediate variant exits 0 but leaks six audio objects. Timing therefore rules out Rust-specific node ownership as necessary, but does not prove all sixteen native objects share one owner. The native timed copy has no retained result log, so it is not independent proof. The Godot 4.7.2 snapshot supports deferred mix-path cleanup and a Dummy-driver shutdown join without a forced final mix. No production delay or audible-output claim follows. Source formatting/readability remediation `809` and root-adapter compilation remain open.

Created [[native-audio-shutdown-leaks]]; updated [[sound]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Native Options corrections and standalone startup regressions are bounded

`a15b441e` routes and projects the bounded Options controller, but wrote edits to legacy `data_root/ui/options_settings.ron`; its reported flow therefore did not prove canonical persistence. `7b1d60ff` writes the loaded canonical XDG file and corrects capture handling. Actual REDs record outside-slider release persistence failure (`data/diagnostics/options-capture-avcdvc_6/red.log`) and Ctrl+Shift+R capture failure (`data/diagnostics/options-modifier-5vspamlj/red.log`). Committed `options_menu_flow.gd` GREEN (`data/diagnostics/options-proof-pa459243/green.log`) proves isolated-XDG canonical save and new-client reload at 240, unchanged legacy bytes, external EULA/realm preservation, graphics Defaults, Ctrl precedence, Shift capture, ignored modifier-only keys, and mouse capture. `verifier767` independently verifies that `7b1d60ff` canonical-persistence/capture boundary: Godot `cargo fmt --check` and `cargo check -p game-engine-godot` exit 0 with the one existing `NativeWmoGroup::fdid` warning. This verifier predates the startup commits.

`6d318ee2` restores Logout for explicitly logged-in standalone `gamemenu` by keeping `logged_in` true, matching prior `show_game_menu(true)` behavior. `/tmp/claude/options-startup-logout-red.log` exits 1 with hidden `MenuBtnLogout`. The intermediate `/tmp/claude/options-startup-logout-green.log` exits 0 but contains a Godot Rust panic (`menu has UI`), so is invalid proof. `80ee3350` breaks action draining after `request_logout()` frees the overlay, avoiding that expectation panic. Final Vulkan AMD cage evidence (`/tmp/claude/options-startup-logout-final.log`) exits 0 with no Godot errors and retains standalone `gamemenu` without Login or a countdown. Both native builds exit 0 with the one existing `NativeWmoGroup::fdid` warning (`/tmp/claude/options-startup-fix-build.log`, `/tmp/claude/options-logout-lifetime-build.log`). Independent follow-up remains pending. Modal drag/reset-window-positions remains incomplete (agent774 is in progress); HUD/audio consumers, visual parity, full conversion, and matrix result remain 0 handled. `7fcfd8d8` display application proof remains bounded to VSync/FPS cap behavior, not visual parity.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Native UI scale is bounded

`364a29ac` mirrors the legacy effective-scale policy: `InWorld` is `max(min(width / 1920, height / 1080), 2/3) × clamp(user_scale, 0.75, 1.5)`; other screens use the clamped user scale alone. Registry canvases use physical viewport divided by that scale as their logical layout size and scale the projection root back to physical pixels. Map pointer conversion likewise divides physical input by scale before layout/UV handling. The FPS and logout overlays remain intentionally unscaled separate CanvasLayers. Development evidence is `ui_scale.gd` exit 0 for 0.75/1.25 persistence, recentering, category/Done, and physical-pixel input (`/tmp/claude/godot-ui-scale-green.out`), scale helper 1/1 (`/tmp/claude/cargo-ui-scale.out`), map helper 1/1 (`/tmp/claude/cargo-ui-scale-map.out`), and native build exit 0 with existing `NativeWmoGroup::fdid` warning (`/tmp/claude/cargo-ui-scale-build.out`). No independent gate, authenticated live-map click, visual parity, or full conversion claim. Native source has no playback consumer corresponding to Bevy `runtime_music`, `runtime_ambient`, or `runtime_assets`; Options audio remains settings-only.

Updated [[ui-system]], [[world-map]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Authored Options projection remains unwired

`d620c812` adds full authored `GameMenuViewModel` projection through `GameMenuModel::from_view` and `RegistryUi` full-view show/update APIs, plus Slider projection and viewport-level captured drag events. Agent-reported targeted `options_views` 3/3 and native slider 1/1 pass. This does not route the real game menu or integrate its controller: a real headless Options click at `d620c812` correctly reports `menu_options not converted` and creates no Options panel. Native Options and full conversion remain incomplete; matrix result stays 0 handled.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Options policy is shared; native parity remains absent

`0c68d89e` centralizes Options drafts, view construction, action/reset policy, and apply snapshots in shared `options_menu_data`; `80b3c664` migrates the original Bevy runtime adapters to that policy while retaining immediate root apply and host-owned persistence. Independent verifier754 records root Options 26/26 and game-menu 12/12; native `ui-model` policy 3/3, views 2/2, and menu 2/2; root formatting plus `cargo check --bin game-engine` and native `ui-model` check exit 0 (`/tmp/claude/options-policy-*.log`). The root `skeleton_afid` dead-code warning is existing. Native Options routing, live consumers, and persistence remain missing. Source sharing is not native parity or a final gate; the detailed matrix remains 0 handled.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] investigation | Camera collision recovery lag

`fe9faa73` recovers camera collision from the stored pulled-in distance instead of the lagging camera pose, so the camera keeps following a running player after a pull-in. Created [[camera-collision-recovery-lag]].

## [2026-09-28] system | M2 test boundary verified; native AFID poses are bounded GREEN

Historical `261ed778` filtered `game-engine-core --lib nameplate_style_data` evidence fails before requested tests because two cache-backed original-root tests reference unavailable `asset_cache` and `load_anim_data` (`/tmp/claude/core-m2-harness-red-261ed778.log`). `c8972a67` moves the unchanged bodies from pure `m2_anim` to `tests/unit/asset/m2_file_loader_tests.rs`, registered by `file_loader.rs` under `#[cfg(test)]`. Final targeted proof passes: root authentic-loader tests 2/2, pure native parser tests 13/13, and root/native `cargo fmt --all --check` (`/tmp/claude/core-m2-test-{root-loader,native-parser,fmt}.log`). The moved tests remain active and their assertions are preserved.

`008f8856` adds the required external-animation callback through SKID/MD20 parsing; callbacks provide SKID/MD20 bytes, with no root-SKID fallback. `f8c8eafa` adopts fixed-master `ac0ef0ba`'s equivalent callback/prefetch implementation rather than duplicate host code; `cecd15f0` is the integrated production state. `aa4c9d52`'s build failure is a concurrent protocol removal of `PlayerInput.elapsed_secs`, fixed by importing master without branch wire changes. The genuine pre-implementation native RED sees SitGround 97 at zero posed bones (`/tmp/claude/native-afid-runtime-red.log`). The framed Vulkan GREEN at `7b1863ae` records SitGround 97/index 78 at 125 posed bones and Sleep 100/index 137 at 87 (`/tmp/claude/native-afid-runtime-framed.log`). Main visually read the 1280×720 PNGs in `data/diagnostics/godot-conversion/external-animation-{97,100}.png`; valid headers/signatures and the ledger support saved captures, not independent visual pixel parity. They show an uncustomized all-geoset HumanMale, not character appearance, material, or full-parity proof.

`7b1863ae` consolidates duplicate core cases while retaining five cases and FDID-metadata assertions. Bounded final AFID proof passes native fmt/check (one existing `NativeWmoGroup::fdid` warning), core 15/15 (5 AFID + 3 metadata + 7 asset parsing), native animation 12/12, and UI-model 4/4 (`/tmp/claude/verify-native-afid-final.md`). Parent root fmt/check also exit 0 with the imported root `skeleton_afid` dead-code warning. Current-protocol Vulkan/UDP logout integration exits 0 with live-connection, token-relogin, and rest-area-immediate-Login markers (`/tmp/claude/logout-runtime-afid-integration.log`).

Canonical master is dirty and advanced beyond imported `ac0ef0ba`, so no merge-back or current-master validation is claimed. No handled-matrix result, general animation parity, full appearance/material/animation parity, or full conversion claim follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Original Options authored view is integrated into the native UI model

`261ed778` integrates original `game_menu_component`, `options_menu_component`, `options_menu_active_sections`, and `options_menu_sections` through `godot/ui-model`. `95ede457`'s pure nameplate presets/editor/`NameplateBarThickness` live in `nameplate_style_data`; `nameplate_style` preserves the original UI enum path by reexport. Input/nameplate aliases preserve source compatibility and persisted type/method identity without a new nameplate system. Root/native reuse one portable behavioral-test source; native proof is GREEN 2/2 (`/tmp/claude/options-view-green-integrated.log`) for all 13 body labels/actions, section replacement, and decimal volume updates. Direct `serde` is removed from `ui-model`; local `game-engine-core` remains justified. Root fmt/check pass at `261ed778` (`/tmp/claude/options-root-{fmt,check}-261ed778.log`) and the protected root lock hunk is restored. Historical `261ed778` filtered core `--lib` evidence fails with two unavailable `asset_cache` and two unavailable `load_anim_data` references (`/tmp/claude/core-m2-harness-red-261ed778.log`); `c8972a67` repairs that test boundary pending final targeted proof. This is source exposure only: `GameMenuModel` still builds the main-menu tree, `ACTION_OPTIONS` and `ACTION_ADDONS` still warn, and no native Options routing, apply/persistence, audio/HUD behavior, AddOns runtime, final gate, or handled-matrix claim exists.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Standalone GameMenu startup reaches bounded GREEN

`77a9b4cc` adds `SessionScreen::GameMenu` and startup routing for `--screen gamemenu`/`--state gamemenu`, matching original `GameState::GameMenu`: logged-in menu on entry, no authentication or Login underlay, and Return/Escape dismisses only the overlay while state remains `GameMenu`. Historical missing-support CLI RED is `/tmp/claude/startup-menu-red-f39581b4.log` (exit 1); build is `/tmp/claude/startup-menu-build-77a9b4cc.log` (exit 0 with existing afid and two WMO warnings). Root-launcher Vulkan fixtures at `6e8c14b7` exit 0 for Escape, resume, and Exit (`/tmp/claude/startup-menu-{escape,resume,exit}-6e8c14b7.log`). Main inspected `startup-game-menu.png`: six slate/gold buttons on a dark blank background, no Login underlay; this is bounded rendering inspection, not pixel equality. `f7280c8d` standalone Logout RED exposed a visible InWorld-only countdown; `27637f64` confines sync visibility to `InWorld`, and its standalone GREEN exits 0 (`/tmp/claude/startup-menu-logout-green-27637f64.log`). The required full InWorld UDP Vulkan rerun after that visibility change exits 0 (`/tmp/claude/logout-runtime-27637f64.log`) through combat/rest/countdown/W-cancel/repeated-request/expiry/Login quiet/live connection/relogin/rest-instant markers. Independent733 at `27637f64` passes `cargo fmt --all --check` and `cargo check -p game-engine-godot --lib`; the check retains three existing warnings, and changed-Rust readability finds no violations (`/tmp/claude/verify-startup-menu-final.md`). Options/AddOns and full parent-menu parity remain open; matrix result stays 0 handled.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Native main-menu overlay reaches bounded runtime GREEN

`c3e5a59c` shares the original logged-in main-menu tree; `17d9ea46` resolves its default-panel skin. `e0c744e1` enables Panel through the existing nine-slice renderer and opens the overlay from character-select `MENU` and in-world Escape. Final bounded proof at `6f37e119`: root/native `fmt --check` and targeted checks pass; root shared-menu proof remains 25 tests because only an unused adapter changed; native model proof is current at 2/2. The real Vulkan fixture exits 0 (`/tmp/claude/game-menu-runtime-6f37e119.log`): repeated roster-key preservation → charselect overlay/block/dismiss → world modal block → decoded UDP → Return restoration → release quietness → Exit. Main inspected `data/diagnostics/godot-conversion/game-menu.png`. The unused root adapter warning is resolved; three native pre-existing dead-code warnings and the fixture's intentional flat 42-line marker length debt remain. This is bounded behavior/render inspection, not pixel equality or full parity. Options, AddOns, and standalone `--screen gamemenu` remain open; Support is a placeholder. Final bounded native logout proof is `d59712cd`: the owned UDP Vulkan fixture exits 0 (`/tmp/claude/logout-runtime-d59712cd.log`) through replicated combat blocking; cleared combat; rest `Some` then `None`; original countdown; real W cancellation with decoded UDP; repeated request without reset; expiry to Login; retained world/camera identity; hidden overlay; quiet post-Login input; retained endpoint-keyed token bytes; then an owned-server remote position update reaches the same retained Godot node while screen remains Login before token relogin to selected character 17 and rest-area immediate Login. This closes the live-connection proof gap without production transport/world/token changes. Main inspected `data/diagnostics/godot-conversion/logout-countdown.png`. Fresh fixture `fmt --check` and focused fixture check pass at `d59712cd`; unchanged production library proof remains `1106b064`. Only W has runtime cancellation proof; the other seven configured cancellation keys and custom bindings remain source-only. Existing fixture marker-table dispatcher readability debt and three native pre-existing dead-code warnings remain. The full matrix remains 0 handled.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Lateral swimming reaches real A/D input

Test-only `61a8bf98`, fixture-only `6de10e60`/`00a9871e`, and fixture-phase extraction `e883fe1b` leave native production unchanged. Main rebuilt `e883fe1b` (`/tmp/claude/swimming-lateral-build-e883fe1b.log`, exit 0) and reran its root-launched Vulkan GPU loopback (`/tmp/claude/swimming-lateral-runtime-e883fe1b.log`, exit 0): W dry→wet, idle Space, wet W+Space, A→SwimLeft 43, D→SwimRight 44, reverse S wet→dry, no jumping, and released UDP quiet. Water remains 4,608 changed / 4,607 translucent pixels; main again inspected `swimming.png` and `swimming-shoreline.png`. Scoped final verification (`/tmp/claude/verify-swimming-lateral-final.md`) confirms source/behavior/runtime evidence, fresh Godot `fmt --check`, and targeted `native_input_fixture` check. It also confirms all existing fixture guards remain after named phases replace the prior `run()` cognitive 67/cyclomatic 62 finding: `run()` is 3/10. Strict readability does not fully pass: `advance_fixture_marker()` remains cognitive 10/cyclomatic 28 (>20), its explicit Stage×marker transition table deliberately retained without suppression or threshold-only abstraction.

This is not buoyancy, speed, real-server, whole-world visual, parity, or full-conversion proof.

Updated [[godot-conversion]], [[terrain]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Native ADT MH2O water restored

`b412f7e4` restores original ADT MH2O rendering: shared Bevy/core water geometry and procedural-normal bytes, native normal/fresnel/specular/depth-alpha shader execution, shared-clock sampling, and tile/reset cleanup. `/tmp/claude/adt-water-runtime-clock-typed.log` exits 0: authored geometry covers the fixture point; 4,608 pixels change and are translucent; paused clock pixels remain stable; a 2-second advance changes at least 64 pixels; swim/UDP/release, freed water-node, and client-exit checks complete. Main read `swimming.png`, `swimming-shoreline.png`, and `swimming-water-isolated.png`: blue translucent water and authored shore are visible.

`a160bdbf` is an import-order-only follow-up. Final root and Godot `fmt --check` pass; earlier root/native checks, core geometry 3/3, normal-byte 2/2, and runtime pixels/clock/client-free proof remain valid. Native checks retain two known WMO warnings; the narrow normal test retains one unused-import warning.

`d44ef597` proves ordinary reconnect water reset/recreation in the owned UDP fixture (`/tmp/claude/adt-water-reconnect-d44ef597.log`, exit 0): 65 seconds of server silence exceeds the 60-second Netcode timeout; `PendingConnect` clears terrain/units and invalidates old Water-node/material weakrefs. Token authentication preserves the selected character through roster reordering, terrain refresh creates distinct water/material IDs, their shared clock advances, input restores, and the headless client exits cleanly. The fixture now spawns at verified actual-ADT Y=112.87991 then Y=117.38283 instead of stale Y=83; production physics is unchanged.

`6bd26680` adds bounded same-map transfer resource/state proof in the built native fixture (`/tmp/claude/water-transfer-build-6bd26680.log`; `/tmp/claude/adt-water-transfer-6bd26680.log`, both exit 0): `INITIAL_READY` → `TRANSFER_LOADING` → `TRANSFER_WATER_READY` → `TRANSFER_READY`; `NewWorld` releases old Water-node/material weakrefs, recreates distinct water/material IDs whose shared clock advances, and recreates terrain while retaining the player's identity/model and facing 0.5. The fixture preserves its authored `TransferAborted` error and emits exactly one `WorldPortAck`. Its independent verifier passes Godot `cargo fmt --check` and the native transfer-fixture `cargo check` (`/tmp/claude/verify-water-transfer-6bd26680.md`).

`7059e412` adds an explicit `GODOT_TEST_VISUAL=1` sampled-render branch to transfer/reconnect fixtures while retaining their default headless path and existing lifecycle/Ack assertions; shader, physics, and protocol are unchanged. Main's example build exits 0 (`/tmp/claude/water-transitions-gpu-build-7059e412.log`). Owned UDP transfer and reconnect runs on Vulkan AMD both exit 0: each recreated destination-water helper reports 4,608 changed translucent pixels, stable frozen-clock pixels, and animation after +2,000 ms (`/tmp/claude/adt-water-transfer-gpu-7059e412.log`; `/tmp/claude/adt-water-reconnect-gpu-7059e412.log`). Main inspected `data/diagnostics/godot-conversion/transfer-water-isolated.png` and `reconnect-water-isolated.png`: 96 px isolated viewport samples using copied production meshes/materials over red backing, not whole-scene captures. Cage/Xwayland shutdown warnings occur after successful fixtures, not Godot runtime failures. This is bounded sampled post-transition GPU proof, not different-map/destination or real-server coverage, whole-world pixels, visual equality, or full parity. Independent GPU/source audit and targeted compilation pass; `2e8ff63f` fixes import order and final Godot formatting passes (`/tmp/claude/verify-water-transitions-gpu-final.md`), reusing unchanged runtime proof.

`3341c9b6`'s missing `WorldTerrain/Tile32_48/Water` is the genuine production RED. Historical fixture marker/type/compile/parse failures remain historical fixture errors, not production defects. No waves, foam, liquid-type-specific textures, WMO water, buoyancy, performance, visual equivalence, parity, or full-conversion claim.

Updated [[godot-conversion]], [[terrain]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] system | Cached authored shore swimming fixture exits 0

`d7dae274` commits only a GDScript screenshot-image type correction; native production is unchanged. The actual cached `azeroth(32,48)` fixture exits 0 (`/tmp/claude/swimming-runtime-typed-image.log`): at X=-8558, W/S/Space crosses dry Z522 → deep Z500 → dry Z522, observes 5 → Swim 42 → SwimIdle 41, suppresses wet Space while stationary and moving, then reaches SwimBackwards 45 (43 is SwimLeft) → WalkBackwards 13 → Stand 0. Bones change after 150 ms; decoded UDP orders swimming false → true → false and is quiet after release. Main read the rendered deep-water PNG: it shows the swimming body and terrain, but no water surface; verifier inspection was PNG metadata only.

Independent verification reran native `cargo fmt --check` and `cargo check -p game-engine-network --example native_input_fixture`, both exit 0. The new `swimming.rs` module has zero readability issues; the parent fixture retains seven existing structural issues. No default or runtime fixture reran. Historical compile/parse failures were fixture/test errors, not production REDs. This is not water-rendering, lateral-swimming, speed, floating-physics, real-server, parity, or full-conversion proof; full conversion remains open.

Updated [[animation]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-28] investigation | NPC stance and gear

Created [[npc-stance-gear]]; updated [[m2-format]] (external `.anim` sequences). NPCs now hold their `UnitPose` (stand state or emote state), render virtual items drawn or sheathed and their display's authored armor; the replication mirror carries `UnitPose`; HumanMale HD Sit/Sleep read their `.anim` files.

## [2026-09-28] investigation | Godot texture VRAM

Created [[godot-texture-vram]]. The Godot client built a new RGBA8 `ImageTexture` per M2 material per placement and per WMO build. `224eb4f8` adds `core::blp::decode_gpu` (DXT kept with mips); `0f36a6cb` shares one texture per FDID/composite. In-world VRAM: 4,994 MiB and rising before, 1,029 MiB settled after.

## [2026-09-28] investigation | Stormwind VRAM

Created [[stormwind-vram]]. The ~6.8 GB Stormwind login came from bevy_hanabi's 65536-particle minimum per EffectAsset slab. With one asset per emitter, 614 slabs held 2.46 GB at t=45 s and kept rising. Identical emitters now share an asset, which leaves 1282 distinct assets. Uncomposited M2/WMO textures upload as BC (599 → 275 MiB). With a scratch `MIN_CAPACITY` 4096 hanabi, the full login settles at 2.13 GB for the client.

## [2026-09-28] system | Exclusive shader-7 overlay diagnostic reaches CPU/GPU GREEN

User-approved `85efccc1` diverges from Bevy's skip for differing-size shader-7 overlays: `imageops::resize` with Triangle filtering resizes the overlay to its base before the unchanged shared composite. `/tmp/claude/wmo-shader7-resize-final-targeted.log` is 12/12 CPU GREEN; `/tmp/claude/verify-overlay-current.md` records native fmt/check PASS. `/tmp/claude/wmo-overlay-movement-runtime-85efccc1.log` exits 0 through movement, idle turns, and jumps, but is separate from the exact probe.

Exclusive `/tmp/claude/overlay-exclusive-runtime-560c1676.log` exits 0 without `PlayerInput`: actual `Wmo373730` placement resolves authored WMO 108238 group 38 shader 7, base FDID 948125 (512×512), and overlay FDID 922678 (128×128). CPU cases 27/32/38 match GPU; main inspected the authored-composite and rendered images. Historical held-input/time-out, wrong-node, and parse failures remain evidence history. Concurrent shared asset/import-cache mutation was user-paused before this success, so it is not a proven cause. Post-merge final proof at `b7dd620c` records Godot `cargo fmt --check` and `cargo check -p game-engine-godot` PASS, with two existing WMO dead-code warnings; `/tmp/claude/overlay-master-runtime-b7dd620c.log` reuses the no-`PlayerInput` authored CPU/GPU overlay diagnostic. No current all-stage/default-fixture movement-plus-overlay claim follows. Shader 5 and other WMO materials, portals, water, doodads, collision, visual parity, and full conversion remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Idle right-drag reaches authored turns before later WMO blocker

`93f38c13` adds actual idle right-drag coverage. Its runtime RED observes yaw delta `-0.12`, expects authored turn 12, and receives Stand 0 (`/tmp/claude/idle-turn-runtime-red-93f38c13.log`). `167ef65b` selects idle turns from normalized consecutive local-facing samples; `8ee6c5ea` makes the 0.02-radian thresholds inclusive. Six targeted tests are reported GREEN.

The rerun preserves unchanged left orbit and reaches both right-drag assertions: 60 samples each, authored 11/12 with changed bones after 150 ms, then Stand 0 (`/tmp/claude/idle-turn-runtime-8ee6c5ea.log`). It exits 101 later at WMO 108238 group 38 material 52 unsupported shader 5. This reaches bounded turn assertions, not a whole-runtime PASS. Verifier649 is asynchronous.

Updated [[animation]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Local running jump reaches landing and resumed Run

`f990867f` adds the running-landing fixture and `5a698a23` drains jumping packets at its landing boundary. `23c8f08f` corrects the fixture's forward-vector check to use current yaw. The actual root-launcher in-world runtime exits 0 (`/tmp/claude/running-jump-runtime-23c8f08f.log`): W+Space observes 37 → 38 → 187 → 5 → 0, changing body motion, rise, resumed displacement, decoded forward-running jumping then nonjump input, and quiet release. Idle jump and grounded Walk/Backward/Left/Right remain covered; production stays at `dac0ab3e`.

This proves one bounded local grounded running-jump path only. Turn, swim, remote locomotion, broader races/equipment, physical all-case coverage, performance, parity, and full conversion remain open. Verifier643 is pending.

Updated [[animation]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Idle local jump reaches authored landing and Stand

`4df9b1e2` adds a root-launched idle-Space fixture that checks actual bone playback and decoded stationary jumping input. Its first runtime exits 101 at missing JumpStart 37 (`/tmp/claude/jump-input-runtime-red-4df9b1e2.log`). `8bebf2c4` implements local JumpStart 37 → Jump 38 → JumpEnd 39 → movement; running forward selects authored JumpLandRun 187 when present, covered by synthetic tests only. `7a6ac0d5` wires the state machine from local `PlayerMovement`, and `dac0ab3e` releases Space independently of airborne clip duration.

The `dac0ab3e` runtime exits 0 (`/tmp/claude/jump-input-runtime-dac0ab3e.log`): idle Space observes 37 → 38 → 39 → Stand 0, changed body poses, rise then ground return, stationary jumping UDP, and quiet release. Existing grounded Walk/Backward/Left/Right coverage also passes. Landing 187 is not runtime-proven. Turn, swim, remote locomotion, performance, parity, and full conversion remain open. Verifier640 is running asynchronously; its result is pending.

Updated [[animation]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Six Godot startup names have bounded CLI proof

Supersedes the earlier pending-runtime note. `login`, `charselect`, `charcreate`, `charcreate-customize`, `loading`, and `inworld` have exit-0 CLI artifacts; parser/account/launcher proof is 6/6, 5/5, and 11/11. The authenticated `inworld` artifact covers mixed-case selection of the second roster entry, although this audit did not independently re-read its numeric source predicate. Four PNGs were inspected; clipped Loading progress means UI presence, not visual parity, and follows shared legacy layout at 720 px rather than a confirmed Godot UI-scale regression. Missing/invalid/unsupported root CLI paths exit 1. Latest authenticated fixtures also validate local locomotion `0 → 5 → 0` and clean shutdown. Twelve other canonical names and `connecting`/`reconnecting` remain explicitly unconverted. Native check and launcher fmt pass. The native fmt failure reported by `bd8282c7` applied only to then-uncommitted terrain-object code; after terrain commit `f015f651`, `/tmp/claude/startup-native-fmt-after-terrain-commit.log` records native fmt exit 0. Full conversion remains open.

Updated [[godot-conversion]], [[animation]], and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Godot startup CLI is separated from native Godot flags

`b10eab7d` keeps native Godot arguments intact and sends launcher-owned `--screen`, `--state`, `--server`, and `--char` after Godot's `--` separator; the process suite is reported 11/11 GREEN. `ae091255` shares pure `ScreenArg`/`StartupArgs` parsing, reported 6/6 GREEN. `37262089` consumes Godot user arguments in `GameClient::ready()`, supporting `login`, `charselect`, `charcreate`, `charcreate-customize`, `loading`, and `inworld`. `charselect` and `inworld` authenticate via saved token or configured credentials; `--char` must name a roster character and `inworld` enters it. `charcreate` is standalone unless `--server` is explicit, then authenticates first. `dev`/`prod` remain server aliases. `e89f4243` prioritizes startup tokens, consumes options once, and validates requested names. `connecting`, `reconnecting`, and legacy destinations outside that set error explicitly as unconverted.

The root-launcher/private-XDG credential fixture at `a69e4f4c` remains RED: startup stays at Login without a manual GDScript connection (`/tmp/claude/screen-cli-runtime-red-a69e4f4c.log`). Native build/runtime proof is pending; this does not close conversion or locomotion gates.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Root Cargo command launches Godot

Root `cargo run`/`rd` now selects the std-only debug launcher rather than compiling the Bevy package. It validates the pinned Godot 4.7.2 binary or `GODOT_BIN`, builds the native library, then execs Godot; no server startup or Bevy fallback. `run-tests.sh --workspace` covers root/launcher tests and Godot tests remain separate. Launcher process tests were reported 7/7 at `41036dfe` before integration/refactoring. At `2d17381e`, bare root `cargo run` reached nested native build and Godot 4.7.2 Vulkan initialization with exit 0 (`/tmp/claude/default-cargo-run-smoke-2d17381e.log`); existing WMO/cage warnings remain. This is bootstrap evidence, not client readiness; independent launcher verification remains pending.

Updated [[godot-conversion]] and the [Godot conversion specification](../specs/godot-conversion.md).

## [2026-09-27] system | Local Godot locomotion animation is wired but unverified

`d3593762` makes the original direction policy portable. Core initially fails on a missing export, then passes 2/2: land chooses Stand, Walk/Run only for forward, backward walk, or strafes; swimming chooses only swim IDs and ignores `running` (`/tmp/claude/movement-animation-{red,green}-4e896a2.out`). `5b0bec54` corrects root library import and constant visibility; no root selector evidence exists.

`2df84899` adds authored-ID selection. The missing-API RED becomes 4/4 GREEN: base-variation selection, repeated family requests retain time/variation/blend, missing IDs do not mutate playback, and loop-mode/interruption retain the outgoing blended pose (`/tmp/claude/native-animation-authored-id-{red,green}.log`). `current_animation_id` is read-only and returns `-1` before binding. Death behavior is unchanged: first authored Death, one-shot final hold, no resurrection/replacement reset.

`bca569a5` adds an input fixture that observes IDs and poses but does not choose clips; Vulkan exits 101 with `Held W did not select authored Run 5: 0` (`/tmp/claude/native-locomotion-input-red-2df84899.log`). `de3c0835` supplies the next local-only wiring after prediction/world advance. It is unbuilt and unverified. Remote entities retain no `MovementState` and Stand; no protocol or displacement inference was added, so blocked forward input can remain Run. Jump, turning, full-fluid behavior, performance, runtime/parity proof, and full conversion remain open.

Updated [[animation]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] investigation | Isolated NPC fixture now reaches the diagnostic boundary

The isolated fixture bootstrap was repaired incrementally: `0f83d8f9` supplies UI and Warband CSV inputs; `54f6bf69` stages cached models and terrain; `36449a60` supplies a default player hair choice while retaining explicit NPC negative choices; `09c5f94d` creates a private SQLite backup, local alias catalog, and community-CSV source symlink; `40fc60a4` recursively stages the skybox directory. The successive runtime logs all exit 101: missing fixture hair (`54f6bf69`), map alias (`36449a60`), and nested skybox model (`09c5f94d`) are bootstrap failures. The last run reaches `INITIAL_READY` through `RESET_READY`, including `TYPE6_MISSING_READY`, then fails because the generic unit logger no longer includes display `910014` in the required type-6 error, although that error is emitted (`/tmp/claude/npc-fixture-bootstrap-runtime-{54f6bf69,36449a60,09c5f94d,40fc60a4}.log`).

`ca0c9204` restores NPC server-ID/display-ID and player-name error context. Its native build exits 0 with the two existing unused-WMO-field warnings (`/tmp/claude/npc-diagnostic-build-ca0c9204.log`); the runtime exits 0 through NPC, lighting, death, visibility, missing-type-6, bound-hair, type-19, effect, and reset stages, observing the corrected `NPC … display 910014` error (`/tmp/claude/npc-diagnostic-runtime-ca0c9204.log`). The headless dummy-renderer `Parameter "material" is null` diagnostic remains nonfatal; this is not error-free evidence. A new verifier audit remains pending. The separate `6acd4ec0` remote-player Vulkan artifact PASS remains valid (`/tmp/claude/verify-remote-player-6acd4ec0.md`).

Updated [[character-rendering]], [NPC appearance](../specs/npc-appearance.md), the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native remote-player Vulkan proof is bounded

`47a8f1ea` first exits 0 under actual Vulkan for the remote fixture, but main image inspection finds the remote female buried 1.366 m. It is not whole-body proof. The independent typed terrain-mesh oracle gives authored floor 114.245974 at remote XZ (`/tmp/claude/remote-player-floor-oracle-typed.log`); `6acd4ec0` adds a floor assertion.

`6acd4ec0` then exits 0 under actual Vulkan (`/tmp/claude/remote-player-runtime-6acd4ec0.log`). Local and remote-female body/gear independently remove and restore on their stable units; other hand references remain independent and paused poses remain continuous. Remote samples are body 3,588 → 3,562 and hands 1,631 → 1,626. Before/after provenance confirms unchanged fixture inputs and native library (`/tmp/claude/remote-player-runtime-6acd4ec0-provenance.json`). Verifier585 independently audits native fmt/check and world 7/7 at unchanged `bc86ff42` library; invalid verifier585 input rerun overlapped edits and is excluded. New artifact verification remains pending. NPC regression proof is blocked on stale isolated project state pending current repair603.

This is bounded remote body/equipment proof. Original remote locomotion remains `Stand`; no new protocol requirement follows. Locomotion, performance, visual parity, and full conversion remain open.

Updated [[character-rendering]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native local-player Vulkan proof is bounded

`bc86ff42` compiles native player-world visuals with two existing WMO dead-field warnings (`/tmp/claude/world-player-build-bc86ff42.log`). The first actual-Vulkan run shows body/hand pixels but exits 101 because the paused-pose probe selects the wrong recursive `M2Animation` (`/tmp/claude/world-player-runtime-bc86ff42.log`). `07ef57e4` corrects that lookup; its 90-second fixture deadline exhausts before InWorld after five preview loads take 88 seconds (`/tmp/claude/world-player-runtime-07ef57e4.log`). `cc38aad1` raises only this evidence-based deadline to 180 seconds, not runtime performance.

The compiled `cc38aad1` fixture exits 0 under actual Vulkan (`/tmp/claude/world-player-runtime-cc38aad1.log`). Stable-unit equipment transitions equipped → empty → restored retain paused body poses and produce body 5,997 → 6,011 and hand 471 → 474 samples. Trace shows recursive lookup would choose `EquipmentOffHand/M2Animation`, rather than `PlayerModel/M2Animation`; Loading/input/camera checks and clean client-free/quit shutdown pass. Main inspected the initial PNG earlier; restored PNG inspection and independent verification remain pending. Remote-player rendering, broader customization, movement-animation coverage, full performance, visual parity, and full conversion remain open.

Updated [[character-rendering]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Independent native equipment verification remains bounded

Independent verifier576 at `9059d471` confirms only cached selected-character equipment evidence: roster replacement, one authored collection chest's visible/deformed pixels, and native attachment behavior. Native `cargo fmt --check` and `cargo check` pass. The two warnings are existing unused WMO fields, unrelated to the audited equipment/policy/attachment files. Six function-length findings are deferred maintainability suggestions, not compile or runtime failures.

Clean-cache local-CASC extraction for chest FDID `2368173` remains open; the earlier absent archive location does not prove stale roots. This does not prove all gear, race/sex coverage, authored animation sequences, visual parity, or full conversion. Next slice: replicated InWorld player visuals—`world.rs` currently projects only NPC visuals. Agent578's reported new test is not proof.

Updated [[character-rendering]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native selected-equipment Vulkan proof is bounded

`d44f3dd4` changes native `PlayerInput.elapsed_secs` to the exact prediction delta and passes 13/13 primitives. Real server `:5000` compatibility remains UNKNOWN because its matching shared-protocol field is uncommitted; no trial refresh/restart occurred. At `640e9f30`, `/tmp/claude/native-attachments-rendered-green.log` exits 0 under real offscreen Vulkan: camera plus `frame_post_draw` observe actual HD/boar attachment lookup, authored rest offset, and combined bone/model rotation, translation, and scale. `e0eba0d7` keeps the attachment-offset child below the bone-binding node. Earlier headless and `skeleton_updated` waits hung, so they do not support a renderer-bug claim.

`/tmp/claude/godot-equipped-runtime-640e9f30.log` exits 0 with compiled Rust `e0eba0d7`: starter items 25/38/39/40/2362 produce clothing/sword/shield pixel changes; scenery, isolated sky/rays, Loading/input gating, UDP movement, camera input, and shutdown also complete. Main inspected a PNG showing blue clothing and weapons. Selection replacement, real-server compatibility, visual parity, and full conversion remain open. The later verifier576 report at `9059d471` supplies bounded cached collection-chest pixels plus native check/format evidence. Running trial `1500671` remains a prior background artifact.

Updated [[character-rendering]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native selected-equipment integration remains unbuilt

`ef697e34` consumes shared policy `2cd73e88`: clothing/cape texture sections and exact geosets; authored M2 gear attaches through native points, collection meshes bind by semantic bone names, and shared transform config `9fc070ab` is read. `008a6326` fixes the `testVector` call. `/tmp/claude/native-equipment-primitives-ef697e34.log` stops on missing shared-protocol `PlayerInput.elapsed_secs` at `godot/src/gameplay.rs:134`; no build, GPU, runtime refresh, attachment-render, or parity proof exists. Clothing primitives remain 6/6 at `e6f2e8b9`. Verifier551 could not locate the reported policy 6/6 artifact, so it is not independent proof.

Updated [[character-rendering]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Portable outfit catalog and concurrent cache import

`a7a33681` shares original outfit/item/display/texture/model/geoset resolution through Bevy-free `godot/core`, retaining the Bevy Resource adapter; `ca1ffc83` indexes local M2 names once per data root; `7b5969d6` restores root helmet extraction. The initial parallel root outfit selector failed 2/4 after absolute/relative data-root imports alternately invalidated one SQLite cache. A real-local-CSV two-importer fixture was RED with `database is locked` (`/tmp/claude/outfit-import-concurrency-red.log`). `e005b96a` canonicalizes source keys and serializes import/freshness decisions: core outfit 3/3 and root adapter 4/4 focused GREEN (`/tmp/claude/outfit-core-green-e005b96a.log`, `/tmp/claude/outfit-root-green-e005b96a.log`). Native selected-player rendering and complete conversion remain separate.

Updated [[character-rendering]] and index.

## [2026-09-27] system | Selected-roster equipment remains RED

`e909b136` records an authenticated fixture sending starter item 25 sword, 38 shirt, 39 legs, 40 feet, and 2362 shield. The actual preview exits 101 with native gear unsupported and missing weapons (`/tmp/claude/godot-equipment-red-e909b136.log`). `2a58d68a` adds native M2 attachment nodes/test, but old-extension headless RED lacks `Attachment5`; integrated-build GREEN remains pending. Existing `0266003e` background and current FPS evidence retain their separate bounded scopes.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] investigation | Native campsite collision/sky fixture blocker

Before `4c9bd0b5`, the authenticated Vulkan fixture times out awaiting `Loading`; v2 corrects the monitor cwd and captures `TerrainMaterials::build_tile` → `Mesh::create_trimesh_shape` waiting on a Vulkan fence at 35 seconds (`/tmp/claude/campsite-live-stack-v2.log`). `4c9bd0b5`'s CPU-geometry collision faces reach `PREVIEW_ENTRY` at 16.5 seconds and pass the campsite ray hit. The full-frame sky delta remains 22 pixels, but `00383009` isolation yields 27,041 pixels and inspected PNGs show default mountains occluding real sky (`/tmp/claude/godot-sky-isolation-4c9bd0b5.log`). `0266003e` changes only the foreground-isolated >=200-pixel assertion. At that same code, `/tmp/claude/godot-full-background-0266003e.log` exits 0: body, both terrain tiles, 118 doodads plus the attached WMO, and isolated sky independently affect GPU pixels; two terrain rays hit; Loading removes the preview and blocks input; InWorld UDP W/release/focus plus native orbit/facing/wheel zoom complete; client free/quit and clean exit follow. This is bounded background/input-lifecycle evidence, not WMO-specific rendering/collision/water/doodads, gear, fifth-layer terrain, FPS/performance, root transfer, visual parity, or conversion completion. Verifier535's independent fmt/check/readability/artifact audit remains pending.

## [2026-09-27] system | Native campsite M2 props and first WMO attachment

`1aebb3c8` shares original authored campsite-object policy; `70ccce02` adds the authored WMO placement. `7ef1dfd9` selects/spawns 76 primary-tile M2 doodads (62 props, 14 waterfall/ripple) and 42 supplemental waterfall/ripple doodads. Its GPU fixture observes background pixels but exits 101 at a later `AwaitWorld` 90-second timeout (`/tmp/claude/godot-campsite-objects-7ef1dfd9.log`), not clean GREEN or full-props acceptance.

`8dc48975` builds typed WMO scene meshes/materials. `db417843` integrates primary-tile WMO FDID `4214993`, UID `48366671`, within 120 units; five real-asset tests pass. No WMO GPU/runtime check exists. Uncommitted sky is excluded. `df22179c` remains clean terrain-only evidence; full scenery, WMO collision/water/doodads, sky, clothing/equipment, and conversion remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Bounded native character-select campsite terrain

`4c5e5735` preserves the original renderer's four terrain texture slots for real campsite chunks with five MCLY layers: all parsed layers remain available, while rendering uses only the first four. This prevents rejection of the authored tile; it is not fifth-layer support.

At `df22179c`, `/tmp/claude/godot-background-runtime-df22179c.log` exits 0. The native GPU fixture attaches authored tiles `31_37` and `31_36`; selected-body and terrain mutations independently change pixels; it observes authored solo-camera/placement 55; then reaches Loading teardown, decoded UDP W/release, camera cleanup, client free, and quit request. Main inspected `data/diagnostics/godot-conversion/character-select-preview.png`. Native fmt/check pass with the existing three WMO warnings; the report file is pending.

Sky, props, waterfall objects, fifth-layer rendering, clothing/equipment, complete scenery, and full conversion remain open. No volatile process/window identifier is recorded.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Godot character-select background partial wiring

`3016fe0a` wires native `CharacterPreview` to the first authored Warband scene and a character-slot placement. It requests primary plus supplemental terrain, synchronizes terrain materials and Retail WDT lighting, applies shared solo framing/presentation scale, and snaps the model/camera above sampled terrain.

No compile or GREEN runtime proof exists. The pre-implementation GPU fixture RED at `a3874f40` exits 101 because neither required tile `31_37` nor `31_36` attached (`/tmp/claude/godot-character-background-red.log`); it does not test `3016fe0a`. Sky, props, waterfall objects, complete scenery, and conversion remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Godot character-select background prerequisites and bounded FPS overlay evidence

`425466b5` shares authored Warband scene/placement records with `godot/core`; `36db5dca` shares the solo character-select camera calculation; and `0d9fc55f` adds explicit initial native terrain tile sets. They make background work possible but do not integrate or render a Godot campsite/background.

For `524fd1c0`, manual inspection verifies a visible actual FPS counter and frame-time graph. Hidden/default and persisted-false visibility runs exit 0. The current visible Vulkan/cage fixture also PASSes and exits 0 after independently inspected saved-visibility, 30→10 FPS-cap, and rendered-graph-pixel assertions (`/tmp/claude/fps-visible-shutdown-current.log`). This successful execution does not explain or fix the historical intermittent visible shutdown timeout. The overlay uses Godot's default font, not Fira Mono. No performance-parity or full-conversion claim follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] investigation | Godot selected-character body preview now has bounded GPU evidence

The former blank-UI evidence remains historical: before `d6b45c13`, `attach_character_ui` had no selected-character spawn; its owned UDP/Godot RED is `/tmp/claude/character-select-preview-red.log` and user screenshot is `/tmp/claude/godot-charselect-user-red.png`. `d4061019` now resolves/caches the selected roster body's local-CASC model path and loads actual race/sex/class/customization, composed textures, and selected geosets; four tests cover core/additional choice effects, body/eye pixels, and missing textures. `264e6eee` is behavior-equivalent helper extraction. `d6b45c13` owns replacement/reset plus preview camera/light; `5c0c9db2` checks removal after `Loading` begins. Actual GPU evidence is `data/diagnostics/godot-conversion/character-select-preview.png`.

`/tmp/claude/character-select-preview-visual-green.log` reaches visible geometry, Loading, world-ready, release, and mouse-camera/wheel markers but exits 101 after timing out waiting for `Stopped`; it is not a passing gate. Instrumentation-only `243a9a42` proves the same preview marker, Loading teardown, decoded UDP, W/mouse/wheel/release, client freeing, and quit request; `/tmp/claude/character-select-shutdown-probe.log` ends PASS/exit 0. The earlier timeout remains unexplained, not fixed. The user client restart on the tested DLL (PID 1371941/window 214) is not deployment or full-client readiness. Gear is explicitly unsupported; campsite/background and clothing/equipment remain separate. `4166b35b` uses a quoted multiline CSV reader for `Map.csv`; the supplied account proof path `/tmp/claude/csv-map-account-green.log` was not locally available, so no account-test result is recorded.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Standalone Godot WMO shader pixels

`8116839b` adds standalone `wmo.gdshader` plus an actual Vulkan fixture. Its 25 pixel cases pass: MOCV interior/exterior/missing behavior; MOMT 6/13, MOCV2, UV2 and missing-UV2 repeat sampling; alpha modes; emissive/unlit; fog; and direct-shadow recovery (`/tmp/claude/native-wmo-shader-green-8116839b.log`). The initial absent-shader RED is `/tmp/claude/native-wmo-shader-red.log`. The first repeat-UV1 oracle was corrected only in the fixture because the original root sampler is linear. Compositor protocol warnings are not shader `ERROR`s. Independent verifier458 is pending; native WMO scene/material binding, portal culling, water, doodads, visual parity, and full conversion remain open.

Updated [[wmo-retail-lighting]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Bounded root tests and native global-WMO asset loading

At `580d7300`, root `cargo fmt --check` and bounded `--bin game-engine` selectors pass 193 tests with no failures; 1,916 tests are listed, so this is not a full suite. The foliage depth-prepass GPU test is excluded. The test-only delta preserves the warning-free `e8466601` production root check. Stockade camera collision is 4/4 GREEN (`/tmp/claude/verify-root-tests-580d7300.md`, `/tmp/claude/verify-stockade-camera-580d7300.log`).

Native `4c7927a6` adds cached local-CASC WMO root/indexed-complete-group loading, mesh batches, raw-group metadata, and retained floor collision; Abbey/Stockade targeted evidence is agent-reported 6/6. `e2537c87` passes global-WMO placement to the map worker. The missing-field RED becomes cached-flags/global-placement 1/1 GREEN, which still warns that collision is unused (`/tmp/claude/native-global-wmo-assets-{red,green}.log`). No scene spawn/render readiness or floor-query consumer exists: global readiness remains `Pending`; WMO runtime/parity remains open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Root integration imports repaired; WMO readiness boundary retained

Supplied verification at `75e5911` records the import-only repair, and at `e8466601` records root `cargo fmt --check` plus `cargo check` PASS with zero warnings (`/tmp/claude/verify-root-imports-75e5911.md`, `/tmp/claude/verify-root-test-migration-e8466601.md`). The former root production-import failures are resolved. Native `50e33def` scope is unchanged; its prior fmt/check evidence remains the native proof.

The root `camera_follow` selector compiled no tests at `e8466601`: one duplicate `game_engine::RealmPreset` type and eight stale `WowCamera` field initializers block the binary. No camera-selector, runtime, or parity claim follows. Local filesystem presence is confirmed for `data/models/{107074,107075,108631,322057,321999}.wmo` and `data/terrain/{777627,777628}.adt`; ignored-file searches do not establish asset absence. Native WMO metadata has no spawn or floor registry, and `loading.rs` retains global-WMO readiness as `Pending`. Full conversion remains open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Independent native input gate

Independent gate at `cd7ea9fa18b9bcdd686d30f457f0148990b6edf6` passes root and `godot/` formatting; `cargo check -p game-engine-godot` is warning-free. The bounded real fixture proves Loading suppression, W motion, decoded `PlayerInput`, release quietness, deferred focus clearing, and post-scope `3f404f26` right-mouse orbit/facing plus wheel zoom. Root `cargo check` fails on five stale-import/`ChatType` errors and is not acceptance proof. `default_realm_preset` and `CHUNK_SIZE` warnings were introduced by the options/water extraction; `b35e5c31` removes them and the duplicate config-directory helper without fresh check evidence. Two `InputBindings` warnings remain outside the bounded input slice. Native experimental trial is available; full conversion/parity remains open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native input bounded terrain-flow GREEN

`c1c05d16` reuses canonical authored-water sampling. `bbb156d1` corrects the fixture from Y=83 to sampled terrain Y=112.879913 and rejects fatal stderr. The earlier Y=83 failure was correct gravity below authored terrain, not a production movement defect (`/tmp/claude/native-input-ground-notification-probe.log`).

`cd7ea9fa` fixes a production `GameClient::on_notification` re-entry: deferred `Window.focus_exited` clears physical input without re-entering the mutable callback. The fixture now tests focus loss while W is held. The warning-free native build exits 0 (`/tmp/claude/native-input-focus-build.log`); a real headless process exits 0 after `Loading` blocks input, `WORLD_READY`, local W motion, decoded server `InputChannel::PlayerInput`, then quiet release (`/tmp/claude/native-input-focus-runtime.log`). The earlier broad callback re-entry is recorded in `/tmp/claude/native-input-integrated-runtime.log`.

GDScript-only `3f404f26` adds native right-mouse orbit/player-facing and wheel-zoom assertions without changing the `cd7ea9fa` binary. Its real fixture exits 0 with no `ERROR` and emits the camera-observed marker (`/tmp/claude/native-input-camera-runtime.log`). This is bounded transform/input proof, not rendered camera parity. WMO/doodad collision/resources, pathing/scripted movement, final-gate verification, and full conversion remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native input primitives and unbuilt movement source

`c8363678` supplies the canonical full persisted options schema to root and `godot/core`; its three core tests are agent-reported GREEN. The recorded root selector is blocked by 135 pre-existing compile errors and 13 warnings (`/tmp/claude/options-root-targeted.log`), so the owned test-reference repair is unrerun.

At `b6540078`, retained input state is 4/4 GREEN, native gameplay state 2/2, local facing 1/1, and shared movement-wire decisions 7/7 (`/tmp/claude/{native-input-state,native-gameplay-state,native-facing,movement-wire}-green.log`). Initial REDs are `/tmp/claude/native-input-state-red.log` and `/tmp/claude/movement-wire-red.log`. These tests prove primitives, not runtime event-to-movement, prediction, UDP, fixture, or parity behavior.

`db2c311e` extracts terrain-only slope blocking and step snapping. The reported 12/12 slope result has no locally discoverable proof log, so it remains pending. `10c75463` adds unbuilt source calls through camera input, terrain prediction, and typed `PlayerInput` sending; exclusive modal windows stop movement. Native terrain ground excludes WMO/doodad floor resources; pathing/scripted producers and water API compilation remain pending. No runtime movement, fixture GREEN, collision, parity, or full-conversion claim follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native full-options camera boundary unverified

`d7c6f5d2` has native initialization load the shared full clamped options schema from absolute legacy `data_root/ui/options_settings.ron`. World-camera synchronization applies follow speed, zoom speed, min/max distance, and FOV. Its `WorldCamera::apply_input` ownership adapter is not invoked; other settings consumers are unwired. Active agent409 supplies the shared schema dependency.

Neither `d7c6f5d2` nor input callbacks `cc8c73a0` have build or runtime verification. The W fixture baseline remains RED. No native input, runtime, parity, or full-conversion claim follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native input pre-callback RED

At pre-callback `bcf38c70` scope with fixture commits `8af67c7e`/`a2b94931`, the main-observed native fixture reached `Loading` with zero UDP, then authored `WORLD_READY`; held W left native position unchanged at `[-8949, 83, 0]`. It exits 101 with child exit 1 (`/tmp/claude/native-input-red.log`). The baseline native build exits 0 with four unused-primitive warnings (`/tmp/claude/native-input-baseline-build.log`).

`cc8c73a0` captures physical-key, mouse/wheel/motion events, clears focus/reset state, and clears frame edges. It is NOT YET BUILT OR VERIFIED: no movement, camera, or sending wiring and no GREEN.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native physical input primitives

`1ee7c51c`/`78696f8e` map physical Godot keys and mouse buttons to portable binding values. Targeted mapping GREEN is 4/4 (`/tmp/claude/input-keys-green.log`) after the missing-module RED in `/tmp/claude/input-keys-red.log`. `79043809` adds retained held/one-frame-edge key/button state, modifiers, accumulated motion/scroll, and focus-loss clearing; targeted state GREEN is 3/3 (`/tmp/claude/physical-input-green.log`) after `/tmp/claude/physical-input-red.log`.

The modules are UNWIRED: no event, projection, binding-match, prediction, UDP, or native-gameplay proof. Independent primitives audit agent406 and final integration gates remain pending.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Portable original movement-input decisions

`c2e3297d` moves original movement-input decisions into Bevy-free `movement_input_data`, exposed by `godot/core`; the root Bevy camera adapter supplies its existing input state. The source retains unnormalized forward/both-mouse accumulation, facing-relative vector and animation priority, autorun/run-toggle ordering, manual-override edges, and backward/strafe speed multipliers.

`/tmp/claude/movement-input-red.log` records the initial missing-export RED. `/tmp/claude/movement-input-green.log` records targeted core GREEN 6/6: dual forward/mouse accumulation, opposed-action vector versus animation priority, modified bindings/scripted forward, toggle ordering, manual overrides, and speed multipliers. Native Godot event production, player prediction, camera input/options integration, and decoded-UDP proof remain absent; no parity or final verification follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native NPC appearance proof corrected

The shared-data matrix is now explicit: `9fdb14b0` profile query is core 7/7 GREEN; `ccf98fb4`/`a75f2d03` cover six geoset-decision cases and core export; `8207fd2c` has four customization-catalog cases; `180ee3c7`/`9c59e807` cover the compositor SQLite query and shared export; and `aecf9697`/`e65ae41f` cover eight NPC selection/geoset/type-6 policy cases and root/core exports. `064381ec` restores a root `Resource` adapter after a concrete compile failure, but that repair has not been verified. `085decc1` adds imported-SQLite-only native-core loading; `29f71b56` records missing-module RED then 3/3 GREEN for cached Human-female model 2/layout 104/full choice 85, HD layout-103 2048×1024 dimensions, and read-only missing-catalog failure without creation (`/tmp/claude/npc-appearance-assets-{red,green}.log`). A new dead-field warning awaits correction.

`27cf3c13`/`e222933f` now apply imported appearance data in native Godot. At `7526c855`, warning-free native build and the actual 24-phase UDP fixture exit 0 (`/tmp/claude/native-npc-type6-{build,green}-7526c855.log`): display `910014` reports missing type 6 for batch 0 without creating a visual, then reset passes. The captured dummy-material-null diagnostic occurs between `BAKED_READY` and `COMPOSED_READY` during baked-model replacement, not proven shutdown-only. Root library fmt/check and core `npcassets` 4/4 remain unchanged at `25d59471` (`/tmp/claude/verify-native-npc-appearance-*`). Verifier376 passes at `fcac4099`: `cargo fmt --check` and `cargo check -p game-engine-godot` exit 0 in `godot/` (`/tmp/claude/final-native-npc-appearance-{native-fmt,native-check}-fcac4099.log`); native sources remain `7526c855`/`0475ef39`. Defer only pre-existing `build_model` assembly length (63 body lines): new batch selection is extracted; cognitive 9/cyclomatic 14; no behavioral or complexity failure authorizes broader refactoring. Effect routing and optional type 19 are source-inspection-only; runtime proof covers missing required type 6 only. Pixels, a successful type-6-texture fixture, importer freshness/rebuild parity, and full parity remain open.

Updated [[godot-conversion]], [character rendering](systems/character-rendering.md#native-godot-wiring), the [NPC appearance specification](../specs/npc-appearance.md), the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native NPC Death development GREEN

WorldUnits `c80488be`, helper `2d8bca7a`, and fixture `254dc68f` now select the first `id == 1` Death sequence, play it once non-looping with the authored 150-ms-clamped blend, and hold its final pose. The original marker remains once per NPC life: resurrection does not rearm it, replacement does not reset it, and a missing Death sequence leaves the current animation unchanged.

The phase-12 RED exits 101 because automatic Death stays Stand and never reaches held Y=3 (`/tmp/claude/native-npc-death-red.log`). Rebuilt native `2d8bca7a` passes its build and the 21-phase real-UDP fixture (`/tmp/claude/native-npc-death-{build,green}.log`): initially-dead and living-to-dead automatic bone motion, final hold, retained identity, and the prior 20 phases. Targeted native unit coverage is 1/1 (`/tmp/claude/cargo-death-green-final.out`). This is development evidence only: independent final-gate, pixels, InWorld, animation parity, and full conversion remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native NPC visibility development GREEN

`df77110d` extracts the original NPC visibility policy into shared core data. `52accdce` projects it through `WorldUnits.update_visibility` after local-player selection, reading retained snapshots and current `world_minutes`; missing selected-local health is alive, while present health needs `current > 0`. It toggles existing `Node3D` visibility without replacing visual/mesh children or maintaining duplicate health/policy state.

The old-DLL RED at `6875a283` correctly fails phase 10 because a hidden-template mesh remains visible (`/tmp/claude/native-npc-visibility-red.log`). The rebuilt native `52accdce` plus fixture `f842d56d` build without warnings and pass 20 real-UDP phases: the retained 11 lighting/lifecycle phases plus nine visibility cases (`/tmp/claude/native-npc-visibility-{build,green}.log`). This is development proof; verifier331 is pending. Fixed native time 1440, non-`InWorld` execution, absent pixel proof, and unchanged root stage/schedule systems leave advancing-clock, stage, parity, and full-conversion claims open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native creature light development GREEN and acquisition closure

The `0f4be666` native fixture exits 0 across 11 real-UDP headless phases (`/tmp/claude/native-creature-light-green-0f4be666.log`): static geometry/texture, visual lifecycle, live sampled-light update, replacement, new-map light, and reset. It asserts native resource values rather than pixels. The synthetic WDT has no ADT and deliberately is not `InWorld`; no readiness, rendered-lighting, visual-parity, or full-conversion conclusion follows. The fixture retains the same shared-original `authoredFogEnd / 36`; local `Position` deliberately preserves prediction, so `MovementControl { controlled: true }` drives the light-sample movement.

Acquisition proof is closed at `b628997e`: core asset-reference tests pass 2/2 and native creature tests pass 3/3 (`/tmp/claude/verify-creature-acquisition-{core,native}.log`). This proves exact cached-path SFID/SKID selection/use and authored texture-FDID collection, not uncached local-CASC extraction or texture caching. Independent agent 323 remains underway, so this is development GREEN, not a gate.

Updated [[retail-lighting]], [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native creature light producer lifecycle

`90f91a5a` centralizes native M2's nine common ambient/direct/sun/fog shader writes in `TerrainLight::bind_model`; `clear_model` nils that same set. Terrain's cube-map binding is unchanged: the M2 shader declares no `environment_map` uniform.

`b628997e` retains the sampled light in `WorldUnits`, binds it to creature visuals on spawn/replacement, rebinds existing visuals on live updates, clears material overrides and retained state on map change/transfer, and drops it during world reset. The synthetic-WDT/no-ADT fixture intentionally stays `Loading`; its RED observes a real `WorldLighting` node, an NPC authored material, and null common uniforms before the producer path (`/tmp/claude/native-npc-light-red-22627188.log`). Native build/runtime GREEN is pending. No rendered-pixel, lighting-correctness, InWorld/readiness, visual-parity, or full-conversion conclusion follows.

Updated [[retail-lighting]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native WorldUnits creature visuals

`7a4add9b` connects ordinary replicated NPC `WorldUnits` nodes to the lazy read-only creature-display catalog and local-CASC model path. A nonzero NPC display resolves model FDID, three texture slots, and scale; its owned `NpcVisualRoot` uses yaw `-PI/2`. Same-display updates retain the child, display changes/removal free and replace/remove it, and world reset frees the owned tree. Players remain outside this path. Query/catalog/load failures report explicit NPC/display errors with no capsule/substitute.

`89980e62` records the pre-integration real-UDP two-unit no-visual RED. At `6f4f850e`, the independent bounded gate passes Godot/root format and compilation checks with exit 0 and no warnings (`/tmp/claude/verify-native-creature-{godot-fmt,godot-check,root-fmt,root-lib-check}.log`). Scoped code is unchanged since fixture-only `8ef9be4b`; its actual owned-UDP fixture passes all nine phases (`/tmp/claude/native-npc-visual-fixture-8ef9be4b.log`). It proves UDP data, mesh/material inspection, visual ownership lifecycle, reconnect reset, and positive tiny-scale clamping. Pixels, lighting, appearance correctness, and NPC visibility policy remain unproven. Native catalog-open/query and absent-row errors, zero-scale default, and acquired no-SFID/adjacent-`.skin` input are source-only; direct-path loading supports the last case. No fallback is claimed. The fixture state machine is test-only (cyclomatic 23; cognitive 14).

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native creature local-CASC acquisition boundary

`771c1f5f` adds a native helper that reuses the local `CascListfileResolver` to cache a creature display's model FDID as `.m2`, primary parsed SFID as adjacent `00.skin`, and optional parsed SKID as `.skel`. It reparses those cached companions through the existing M2 loader, resolves authored batch textures and explicit creature texture slots, and caches the resulting `.blp` files. A missing texture remains a reported native material-loader FDID rather than a placeholder.

At `b628997e`, focused acquisition proof is core 2/2 and native 3/3 (`/tmp/claude/verify-creature-acquisition-{core,native}.log`): cached-path SFID primary-skin and SKID skeleton selection/use, plus authored batch/explicit texture-FDID collection. It does not prove uncached local-CASC extraction or texture caching. `7a4add9b` attaches catalog consumption to replicated NPC world units; the later `0f4be666` runtime fixture covers its bounded lifecycle/light-resource path. Independent verification is underway. NPC appearance parity and full conversion remain open.

Updated [[godot-conversion]], [[asset-pipeline]], [[character-rendering]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Shared creature display SQLite lookup

`8afcffae` extracts the Bevy-free `CreatureDisplay` row and `query_display` SQLite query into source shared by the root crate and `godot/core`. It returns `Result<Option<CreatureDisplay>>`: root preserves its existing error-hiding `Option` cache boundary, while reusable core callers retain SQLite errors. The targeted core RED records the missing export (`/tmp/claude/creature-display-query-red.log`); GREEN passes 2/2 (`/tmp/claude/creature-display-query-green.log`), covering full model/three-skin/scale mapping, absent rows, and a missing-table error.

No native model attachment, catalog consumption, runtime/visual proof, or independent gate is claimed.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Explicit native creature texture slots

`92ecaf25` fixes the source distinction: `Model::skin_fdids` is SFID geometry `.skin` metadata, not creature texture data. Shared M2 batch resolution now takes explicit `[u32; 3]`: type 2/11 uses slot 0, type 12 slot 1, type 13 slot 2, and type 0 stays TXID. `17ef37f5` exposes `WowAssetLoader.load_m2_with_skin_fdids(path, PackedInt64Array)`, rejecting any input except three nonnegative `u32` values; ordinary `load_m2` passes zero slots and never infers from SFID.

The core selector passes 8/8 after the SFID bug RED (`/tmp/claude/m2-skin-textures-{red,green}.log`). `a40ed6e2`/`f2f5b4fa` add and correct the generated M2/SKIN/BLP GPU contract. The pre-API RED is recorded in `/tmp/claude/m2-skin-pixels-red.log`; the first six-case run incorrectly expected black from an unbound Godot sampler (`/tmp/claude/native-m2-skin-pixels-green.log`). At `f2f5b4fa`, the fixture asserts both nil texture binding and the real white default, then exits 0 with seven pixel cases and invalid-input rejection; the native build is warning-free (`/tmp/claude/native-m2-skin-{build,pixels-corrected}.log`). At `2b972da2`, verifier304 records bounded PASS: fresh native `cargo fmt --all --check` and `cargo check -p game-engine-godot`; reused core 8/8 and the unchanged corrected actual-loader fixture's seven pixel cases plus invalid-input group. No production unit-model integration, equipment/customization wiring, appearance parity, visual parity, or full conversion claim follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Bounded M2 effect-UV proof boundary

At `bf3e5bd7`, root sampler tests pass 5/5 after `4814ed00` adds direct `M2TextureUnit` imports (`/tmp/claude/verify-m2-effect-uv-followup-root-tests.log`). The actual-loader Vulkan fixture exits 0 with eight explicit pixel assertions and an enabled-process observation that automatic `/root/M2MaterialClock` processing changes a rendered effect pixel (`/tmp/claude/verify-m2-effect-uv-followup-pixels.log`).

Prior bounded native/root fmt and checks, and reused core5 sampler proof, remain unchanged PASS (`/tmp/claude/verify-m2-effect-uv-summary.md`). The pre-existing out-of-scope `unused import: super::*` test warning remains. Main rejected length-only readability findings because they show no behavioral failure; no scope expansion follows. World light/material animation, character appearance parity, visual parity, and full conversion remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native authored M2 batch binder

`7cc4dfbd` exposes the shared CPU M2 compositor to native code. `5701f466`, `5ba9d672`, `0386e091`, and `fe792f20` replace the native `StandardMaterial` loader with resolved authored-batch `ShaderMaterial`s. Original single/effect routing retains CPU second-texture/overlay composition when required; source blend/cull/depth, UV/transparency, lighting and fog variants are bound; opaque/mask variants omit `ALPHA`; missing texture FDIDs are returned while their samplers stay unbound, with no placeholder palette.

`e86504d7` and `a6ad7d47` repair the root M2 extraction failures. Native build, real HD asset/BLP decode, and real HD animation pass (`/tmp/claude/native-m2-loader-{build,m2_assets,m2_animation}.log`). Independent native fmt/check and core compositor 4/4 pass (`/tmp/claude/verify-native-m2-summary.md`). At `87bab0ff`, root library fmt/check pass (`/tmp/claude/native-m2-root-{fmt,lib}-87bab0ff.log`); the root binary target reaches binder code and is blocked only by unrelated non-exhaustive `ChatType`. The unchanged `1458ccd8` fixture exits 0 with four corrected generated-M2/skin/BLP pixel assertions passing and its process group gone (`/tmp/claude/native-m2-shutdown-{captured,live}.log`). An earlier shutdown timeout is unexplained; no reliability fix is claimed. The verifier's first culling-regression finding was false; no culling fix is recorded. World light/material animation, replacement-texture/geoset APIs, native model/world appearance, and visual parity remain open.

Updated [[character-rendering]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Bounded shared-M2 data proof

`7cc4dfbd` newly re-exports the shared CPU M2 compositor to native code. Agent277 reports bounded core 5/5, root 6/6, standalone CPU 4/4, and native check PASS. `/tmp/claude/verify-m2-data-summary.md` was unavailable during this documentation update, so those supplied counts are not recorded as a current integrated gate; `7cc4dfbd` itself has no check proof.

A root check stopped at M2 batch-data destructuring. `e86504d7` fixes that source error but has not been rechecked; unrelated non-exhaustive `ChatType` remains unresolved. `1c6340aa` supplies a real-loader RED: the original shader is lit black and CPU secondary composition is ignored, while its base case passes. Native material binding is underway, not proven. No native material/render, visual, appearance, or full-conversion parity claim follows.

Updated [[godot-conversion]], [[character-rendering]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Portable M2 render-batch decision boundary

`c3ec6086` extracts original M2 render-batch decisions into portable code. `godot/core` exposes `m2::resolve_render_batches` with a callback FDID resolver that preserves the original texture heuristic; the root `Mesh` wrapper delegates to that pure resolution. Developer core7 proves real HD model 113 resolves 113 batches and 147,966 indices; root UV2 is GREEN. No independent gate has run.

This does not bind materials or prove rendering. The native loader still consumes raw batches into `StandardMaterial`; it does not use resolved decisions, the `55c0167e`/`1d25779c` portable CPU compositor, or agent267 shader work in progress. Material binding, rendered output, visual parity, and full conversion remain open.

Updated [[godot-conversion]], [[character-rendering]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Portable M2 texture-composite boundary

`55c0167e` extracts the existing CPU M2 secondary-texture shader byte composition and positioned/scaled overlay blit into Bevy-free `src/asset/m2_texture_composite_data.rs`. The root `m2_texture_composite` adapter retains cache keys and BLP/cache I/O. Standalone `rustc` tests are 4/4 GREEN: shader byte rounding/alpha behavior, repeated secondary sampling, integer-alpha overlay blending, and scaled/clipped overlays.

This is portable algorithm evidence only. `1d25779c` subsequently commits the shared overlay-blit and 2×-scale helpers used by BLP and the M2 module. Root Cargo and `godot/core` exposure are not yet verified; Godot has no consumer, material binding, rendered output, or visual-parity proof. Uncommitted standalone shader and shared batch-metadata work are not recorded as integration. Full conversion remains open.

Updated [[character-rendering]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Independent compositor and reconnect gate

At `0d9301a1`, independent verification passes root/native fmt and checks, portable exact-RGBA compositor tests 4/4, and root real-asset library tests 9/9 (`/tmp/claude/verify-compositor-reconnect-summary.md`). The root `--bin` selector executed 0 tests and is not evidence; `--lib asset::char_texture::tests` supplies the 9/9 proof. The same gate passes session 15/15, transfer 4/4, and the actual reconnect fixture: `INITIAL_READY` → `WORLD_RESET` → `TERRAIN_REFRESHED` → `RECONNECTED`.

Readability finds no changed-line violation. The only warning is the unchanged unused `super::*` import in `tests/unit/asset/m2_retail_light_tests.rs`. `3586b99e` removes the redundant wrapper/getter and simplifies the fixture. Godot's native model loader has no replacement-texture/geoset API and world players remain model-less, so no Godot native path invokes the compositor. Native character rendering and full conversion remain open.

Updated [[godot-conversion]], [[character-texture-compositing]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Ordinary native reconnect integrated; runtime GREEN pending

`af6cf7ca` adds pure session reconnect phases and captures the in-world selection for token login. It blocks gameplay input until connection, terrain refresh, and selected local-player presence complete reconnect; forced disconnect and failed login/entry clear the state. `7fd6c61d` integrates this in the account/host: it stops and joins the old transport, discards the rest of that batch, then begins token transport; `LoadTerrain` records refresh and the host supplies local-marker completion. `07aa1ebc` aligns the owned fixture with the configured 60-second Netcode timeout.

`/tmp/claude/native-reconnect-red-60s.log` records real `ConnectionTimedOut`. At `7fd6c61d`, the native build exits 0 (`/tmp/claude/native-reconnect-build.log`) and the owned fixture exits 0 (`/tmp/claude/native-reconnect-green.log`): `INITIAL_READY` → `WORLD_RESET` → `TERRAIN_REFRESHED` → `RECONNECTED`; token-login roster reordering retains the captured selection and completion waits for the local marker. Development GREEN only; independent gate pending. No native Godot input adapter, full lifecycle, or full-conversion claim follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Independent native transfer boundary gate

At `e028babadf408e760b8aea7d925116efff90fd11`, independent verification passes root/native fmt and checks, loading 3/3, core camera-input 6/6 and WMO-mesh 2/2, root WMO-mesh 1/1 and `UIErrors` wrapper 4/4, plus native `world_camera_flow` and `enter_world_flow` (`/tmp/claude/verify-native-transfer-summary.md` and listed logs). Root warning findings remain baseline: unused `InputBindings` imports, plus the root `UIErrors` test's unused `super::*`.

The unchanged owned transfer runtime evidence was inspected but not rerun; `native-transfer-green.log` remains earlier evidence, not a fresh e028 result. `UIErrors` and account `RegistryUi` screens are mutually exclusive today, so no ordering fix is justified. Native global-WMO spawn is absent; WMO mesh data is not native WMO rendering, shared camera calculation is not native input, and reconnect has no proof. Full conversion remains open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Typed native transfer wiring remains uncompiled

`237d6aa3` adds typed transfer/session-account state: `NewWorld` starts a pending world port and selects `Loading`; `TransferAborted` becomes a transfer-error event; only native readiness completion may send `WorldPortAck`. `1de253b7`/`7480f999`/`3ae265ea` add and correct the owned Godot UDP transfer fixture, including typed instance-ID comparisons. Session agent evidence is targeted 4/4 only.

`6a8e8d74` projects same-map terrain/material/lighting reset, destination-tile request, selected-player position/facing, readiness acknowledgment, and `UIErrors` ownership into the host. `526a7ca0` now supplies agent233's native overlay/model implementation; combined main wiring has not compiled. The only owned-loopback runtime attempt used the old `c48d2510` DLL: it reached `FIXTURE INITIAL_READY`, ignored `NewWorld`, and timed out at fixture phase 1 (`/tmp/claude/native-transfer-red-runtime-typed.log`). This RED does not test `6a8e8d74`. Independent compile and GREEN fixture verification remain required. Global WMO, WMO model/ground/camera, character-facing movement/native input, and all parity rows remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native loading-readiness wiring, proof pending

`dae1f6e5` wires the shared loading predicate into the native host. Selected-unit position supplies local-player readiness and requests the current center tile; only an attached center tile is loaded; a present global WMO stays pending until native spawn; LoadingUI receives progress/status; completion selects `InWorld`. `191dee01` lets the stable-unit fixture accept either `Loading` or `InWorld`.

No native GREEN/proof exists. `/tmp/claude/native-readiness-red.log` still observes `Loading`; build/proof awaits223. `NewWorld`, `WorldPortAck`, WMO spawn, rendered-world, and parity claims remain open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Shared camera-input calculation boundary

`6c6d994a` moves original in-world mouse/keyboard/wheel camera calculation into Bevy-free `camera_input_data`, reusing `CameraState`, pitch limits, and portable binding matching. The root adapter maps only Bevy capture/events to that shared calculation; `47c9dac8` removes its unnecessary mutable facing binding. Core `camera_input_data` is 6/6 GREEN (`/tmp/claude/camera-input-green.log`), covering mouse orbit/facing and keyboard ordering, pitch bounds/inversion, binding modifiers/opposed actions, keyboard/wheel zoom ordering, and no-player-facing behavior. `/tmp/claude/camera-input-red.log` records the initial unexported-module RED.

No native Godot input adapter, capture, options integration, or runtime proof exists. The post-warning root test attempt is blocked by concurrent unrelated terrain wiring errors (`/tmp/claude/camera-input-bevy-final.log`); this is not an integrated/full gate.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] audit | Godot loading-readiness predicate

Original `check_loading_complete` treats `LocalPlayer` as local-player readiness, not rendered character/model, appearance, or equipment readiness. `tag_local_player` adds that marker only when replicated `NetPlayer.name` exactly matches `SelectedCharacterId.character_name` (`src/game/state/game_state.rs:362-381`, `src/game/networking/player.rs:804-888`). When terrain is required, completion also requires a map and either spawned global WMO or the current-player streaming-center tile loaded; pending/failed terrain remains incomplete. Without terrain, the marker alone completes. Godot still has no native predicate integration or `InWorld` transition. Character/appearance/equipment visual parity remains separately required.

Updated [[godot-conversion]], [Godot conversion specification](../specs/godot-conversion.md), and [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Portable input model and bounded loopback wire proof

`f50dedaa` moves input-binding data into Bevy-free `src/input_bindings_data.rs`: action/section metadata, defaults, portable key/mouse values, persisted tokens, labels, parsing, and matching. `godot/core` exposes that source; `src/input_bindings.rs` is now the Bevy event/capture adapter. Prior binding evidence remains 4/15/1; no native Godot adapter produces or matches input with the portable model.

At `f50dedaa + 6cec7882`, independent verification passes owned-loopback `wire_tests` 1/1, root and Godot `fmt --check`, root `cargo check`, and `cargo check -p game-engine-network` (`/tmp/claude/verify-bindings-{wire,root-fmt,godot-fmt,root-check,godot-network-check}.log`). Root check exits 0 with the two existing unused `InputBindings` imports. The fixture proves only bridge `PlayerInput` decoding, replicated `MovementControl` epochs, and unit removal under owned loopback; it does not prove native Godot input production/matching, readiness, or real-server wire behavior. The evaluator retains exhaustive flat key conversions, original long metadata match tables, and tiny domain helpers; no speculative macro/refactor (`/tmp/claude/verify-bindings-readability-audit.log`). Preserve `45fd1938`: height-grid answers, including inside authored holes, do not establish `WorldGround` support.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Combined shared physics and native motion gate

At `ea5b4a85`, independent verification passes root fmt/check, core physics 7/7, core motion 8/8, native `world::tests` 6/6, root collision 14/14, and root proposal 2/2 (`/tmp/claude/verify-motion-final-summary.md`). Unchanged root `server_movement` source reuses prior 2/2 evidence. Native check is warning-free; root check retains two baseline `InputBindings` warnings.

The initial native fmt failure was only `unit_motion_data` module ordering. `775b14a3` makes that declaration order canonical, and independent208 records native fmt PASS (`/tmp/claude/verify-motion-format-775b14a3.log`). This proves shared-helper/root-adapter/native-host-state behavior only. It does not prove decoded wire control epochs, player-input prediction/production/send, readiness, runtime fixture wire control, or real wire epochs. Agent209 portable bindings are in progress and outside this gate. Preserve `45fd1938`: height-grid answers, including inside authored holes, do not establish `WorldGround` support.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native terrain height versus ground support

Native retained `Root.height_grids` contain only height-grid fields; MCNK low/high-resolution hole masks remain on separate `Root.chunks` used for mesh indices. Thus `GameClient.terrain_height_at` can return a height inside an authored hole. Its nil means no loaded grid covers the query, not the original `WorldGround` `Unloaded` state. The rendered-triangle `StaticBody3D` keeps holes open, but a mesh ray still is not `WorldGround`: it has neither WMO-floor candidates nor the `Unloaded`/`Unsupported` result distinction. Native movement remains unwired; this does not alter the WMO-floor contract or resolve the pending independent physics proof.

Updated [[godot-conversion]] and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Native unit-motion correction and interpolation

`a60e9dea` extracts original authoritative local correction and remote interpolation into `unit_motion_data`, shared by root and `godot/core`; `ea5b4a85` applies it through native `UnitMotion`. Ordinary local snapshots retain the current predicted transform. The initial control epoch is adopted only, a changed epoch snaps, controlled positions interpolate, and optional local yaw is applied only when present. Remote units lerp/slerp; missing remote yaw retains the target, while missing local yaw cannot reuse stale facing. Native processing advances units after account polling and before terrain, lighting, and camera work.

`/tmp/claude/unit-motion-{red,green,server-movement}.log` records supplied core/root development evidence; `/tmp/claude/native-unit-motion-red.log` is the native missing-`UnitMotion` RED; `/tmp/claude/native-world-motion-green.log` is native world tests 6/6 GREEN. These prove pure/helper and host-state behavior, not decoded live-wire `MovementControl` epochs. Actual player-input prediction/production/send, readiness, and the independent combined gate remain open. The preceding `6560b238`/`f3bb1965` physics gate is independently pending. No missing first-main-output path is invented.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Shared original player-physics proposal boundary

`6560b238` extracts the original normalized horizontal proposal plus unloaded/unsupported/supported grounded and gravity/snap transitions into `src/player_physics_data.rs`, then exports the same source through `godot/core`. `f3bb1965` points the two root proposal tests at that shared module. Existing Bevy collision adapters still convert `GroundProbe` and retain the original `GRAVITY` and snap constants.

Agent192 reports development GREEN of seven new core tests, two root proposal tests, and 14 collision tests, following missing-module RED. No supplied log artifact was located, so no path is invented and this is not independent verification. Native Godot movement is not wired; slope, step, swim, and jump input remain original Bevy-only. Independent final gate remains pending; no conversion-completion claim follows.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-26] system | Shared original M2 effect-UV sampler

`8df1ec99` extracts the original M2 effect-material UV sampler into shared code. Effect materials sample sequence 0 at shared application elapsed time; declared global-sequence timing uses modulo, including preserved zero-duration behavior. This timing is independent of bone pause and active animation clips.

Development core5 RED/GREEN evidence is limited to `/tmp/claude/m2-effect-uv-extraction-{red,green}.log`. Native wiring and an independent gate remain pending. The original runtime path did not contain ordinary single-texture colour/opacity animation, so none is claimed. This establishes neither native material/render behavior, visual parity, nor full conversion.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-26] system | Godot MovementControl snapshot retention and camera baseline

`eff94a71d259b020f39fba4b6e7c21229bb57f7f` retains optional copied `MovementControl { epoch, controlled }` in owned `UnitSnapshot`; the native literal fixture initializes it as `None`. `/tmp/claude/godot-snapshot-movement-red.log` is genuine missing-field RED. Targeted network and native fixture commands are GREEN 1/1 at `/tmp/claude/godot-snapshot-movement-green.log` and `/tmp/claude/godot-snapshot-world-fixture-green.log`. This retains snapshot data only, not native correction, prediction, interpolation, input production/send, or decoded-UDP proof.

Agent178 independently baselines `2cacf913`/`aedd9fe6`: native fmt/check, core terrain-height 3/3, core camera-data 7/7, and actual `world_camera_flow` PASS (`/tmp/claude/verify-camera-{native-fmt,native-check,core-terrain-height-data,core-camera-data,world-camera-flow}.log`). `/tmp/claude/verify-camera-artifact-record.log` records native artifact SHA-256 `a83fe827f8edd1f36972732a2963380915facb030c36164bf85c4804261f065b`. Root `cargo check --bin game-engine` passes at `6237af9d` with two baseline `InputBindings` warnings (`/tmp/claude/verify-camera-root-check-6237af9d.log`); later root edits are test-only. Independent190 at `eff94a71` records root camera 6/6, terrain-height 8/8, sky-gradient 4/4, root/native fmt, warning-free native schema check, and snapshot network/world fixture reads 1/1 each (`/tmp/claude/verify-root-snapshot-{summary,root-camera-follow-tests,root-terrain-heightmap-tests,root-sky-gradient-tests,root-fmt-check,godot-fmt-check-corrected,godot-schema-check}.log`). It excludes later native player-physics edits. Full conversion remains open.

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-27] system | Correct Godot `PlayerInput` registration boundary

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md). Source inspection shows `godot/network/src/lib.rs` `run_worker` installs `shared::ProtocolPlugin`; `godot/Cargo.toml` patches `shared` to the local checkout at current HEAD `70dccb0`, where `src/protocol/registration.rs` registers `PlayerInput` client-to-server. Nominal pinned `e25c79d` also registers it in `src/protocol.rs`. Transport registration is therefore not missing; native input production/send invocation and real decoded-UDP `PlayerInput` proof remain absent.

Root adapter import fix `6237af9d` still awaits an independent root gate, so no passing root claim is recorded. Report178's native fmt/check, core 3+7, and actual camera-fixture GREEN are not a finalized independent gate; historical `2cacf913` proof boundaries remain unchanged.

## [2026-09-27] system | Godot terrain collision and world-camera development GREEN

`33c31c8e` creates a `StaticBody3D`/`ConcavePolygonShape3D` per rendered terrain chunk from the same `ArrayMesh` triangles. Authored holes remain absent because the source render mesh omits them; tile-root teardown also removes the colliders. At `f23033a8f1ec2214606cfc99a16cd3d4f5cf27d5`, the real fixture is GREEN for nine rendered tiles, 90 authored holes, terrain-height reset, collision reset, and asynchronous local assets; `/tmp/claude/godot-world-collision-build.log` is warning-free.

`f23033a8` shares the original Bevy 0.19 camera state/follow math through the existing `glam` 0.32 version, rather than introducing a second math implementation. At `2cacf913`, the native world camera consumes that state and uses real physics rays with inherited-hidden and selected-player-descendant filtering; map reset removes it. `/tmp/claude/godot-world-camera-green.log` is GREEN for original orbit, FOV 90, near 0.1/far 1000 projection, fixture-only elevated native-player placement, obstruction and hidden-mesh recovery, self/descendant exclusion, reconnect teardown, and continued `Loading`. `/tmp/claude/godot-world-camera-build.log` is warning-free. Independent collision/camera gates remain pending. No camera input/options, WMO support, readiness, visual parity, or full conversion claim follows.

Independent height verification at exact `40b3ad10635152937d81685681c5d593f50b8046` records native fmt and check GREEN (`/tmp/claude/verify-height-{native-fmt,native-check}.log`) and a real-server nine-tile centroid/reset fixture PASS (`/tmp/claude/verify-height-world-height-flow.log`). The initial `terrain_height_data::tests` core filter and `terrain_heightmap::tests` root filter selected zero tests; future focused filters are `core terrain_height_data_tests` and `bin rendering::terrain_heightmap::tests`. The root check failed only because `sample_chunk_height` ceased to be publicly re-exported; `aa7f00b1` restores the original public path, but its verification is pending. Readability accepts the paired flat bounds guard; inherited opaque sampler name `bxx` is deferred without a functional-bug claim. Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-26] system | Godot lighting verification refresh

At exact Rust semantic revision `2faed51b4ad02f27153ee9f36e3d75e35e392505`, `/tmp/claude/verify-native-fmt-2faed51b.log`, `verify-native-check-2faed51b.log`, and `verify-native-terrain-tests-2faed51b.log` record native fmt/check and `terrain::` 15/15 GREEN. `/tmp/claude/verify-world-lighting-flow-2faed51b.log` independently proves bounded real-fixture authored-lighting reset without fabricated readiness; proof revision is `2faed51b`, run while HEAD was `5a011a9b`.

At root revision `ad9d43f4d10e60ae8a366b13257fe7749d00fb34`, wrappers are `sky_lightdata` 7/7, `retail_light` 6/6, `sky_gradient` 4/4, and cubemap 1/1 GREEN. Root check exits 0 with three warnings. `7592486f` moves `sky_band_at_elevation` into test imports; root recheck remains pending. The two `InputBindings` warnings are unchanged from `master`, so they remain reported rather than fixed. `/tmp/claude/verify-terrain-shadow-pixels-5a011a9b.log` independently verifies all six terrain-shadow GPU assertions at committed `5a011a9b`, after fixture/shader blob preflight. It is bounded material/shadow evidence, not native world-shadow, readiness, parity, or full conversion.

At `b8930da0` (core `e3a78779`), `GameClient.terrain_height_at(x, z)` returns shared authored terrain height or nil when unloaded/reset. Development `/tmp/claude/godot-world-height-red.log` and `godot-world-height-green.log` cover the missing API, nine actual rendered-triangle centroids, and reconnect; `godot-world-height-build.log` is warning-free. No independent gate exists. Holes, WMO ground/collision, movement/camera use, readiness, and parity remain unproven.

CSV-row cyclomatic complexity 27 is declarative field decoding with `?` propagation; splitting for that number alone is rejected. Other root/native length or nesting findings are inherited/shared and already deferred; no broad cleanup task is created. Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-26] system | Godot terrain-shadow fixture development proof

Committed `5a011a9b969404f61d670f2951e31a8e382b92d1` adds `godot/tests/terrain_shadow_pixels.gd` (fixture blob `204984291a0c33a1b1429ece92c7d9e898ee1e6a`). Development run `/tmp/claude/godot-terrain-shadow-pixels.log` exits 0 for six GPU assertions against terrain shader blob `3871251d3adec49f35b49fcb727c59cfcca73b1c`: terrain lit before caster; lit/shadowed StandardMaterial controls; off-center terrain unchanged; ambient survives shadow while direct/specular are removed; direct/specular return when the caster is removed.

Probe 1 failed because its off-center expected value differed from the fixture output; Probe 2 corrected that fixture specification. This is not a product bug. No independent verifier has rerun the committed fixture. It establishes neither native world-shadow configuration, world-rendering parity, readiness, nor conversion completion. Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).


## [2026-09-26] system | Godot native world-lighting producer and independently verified terrain shader

`6a51f613` adds shared volume lookup (five targeted tests); `d70998c9` adds shared sky-gradient/cubemap pixels (three); and `0cfd21ef` makes CSV parsing strict (three), including observed signed ARGB `-3502459`, which must not become black. `2faed51b` has the native worker load the `Light`/`LightData` catalogs, select the original blend at the local player’s WoW coordinates (default noon), create/update a native `DirectionalLight3D`, `Environment`, and 32×32 RGBA16F cubemap, and update actual terrain materials. Map IDs use only the existing limited map-name lookup; this is not all-map coverage.

`/tmp/claude/godot-world-lighting-red.log` is genuine RED for the missing producer. `/tmp/claude/godot-world-lighting-build-green.log` is GREEN. `/tmp/claude/godot-world-lighting-flow.log` is a real-fixture pre-commit GREEN, but the reset assertion was added after that pass, so current fixture verification remains pending. Root `sky_lightdata` tests failed because an old `Dimension` import remained after main removed the parent import; `7e8c5e40` fixes that conversion issue. Do not classify it as pre-existing; the new root/focused gate is still ongoing.

`/tmp/claude/godot-terrain-shader-12case-verify.log` independently records 12/12 GPU cases PASS at exact revision `a0517118384a992cdee773b817333d03a16bd8fc`, shader blob `3871251d3adec49f35b49fcb727c59cfcca73b1c`, and fixture blob `ddcc12700531550d6f389632a3e36762839e8d31`. This supersedes the prior unlogged agent report only for those 12 cases. It does not prove shadows: the rejected `SHADOWS_ONLY` caster probe establishes no shadow parity. Actual world camera/visual terrain capture, camera streaming/collision, WMO/doodads/water, weather/live game-time/material clock/mips, loading readiness/transfers, and full conversion remain open. Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-26] system | Godot shared LightData interpolation and expanded shader fixture boundary

`b267c69a` shares generic `LightDataRow`/`SkyColorSet` interpolation for every authored field, with no fallback in the shared path. Agent140 reports core 5/5; root `cargo test --bin game-engine sky_lightdata::tests` remains running, so no result is recorded. No actual native authored producer exists.

`602b76f7` plus fixture `a391f772` define 12 shader cases: three blends; authored gamma and specular; overbright; MCCV byte 255; linear fog; UV start/offset/repeat; unshadowed direct; and float-cubemap Fresnel. Agent128 reports 12 passes, but no inspectable saved log exists; this is not independent proof. Shadow control is inconclusive even with StandardMaterial. Native map-time, cubemap, sun, camera, fog, UV-clock, and mip inputs remain missing. This establishes neither GPU/world rendering nor parity. Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-26] system | Godot MCCV byte encoding and shader-case boundary

`e65dbb59` corrects authored neutral MCCV storage from byte 127 to byte 255 before `ArrayMesh` construction and reverses that conversion in the shader. `/tmp/claude/godot-terrain-mccv-red.log` is genuine RED for the former encoding; `/tmp/claude/godot-terrain-mccv-build.log` and `/tmp/claude/godot-terrain-mccv-green.log` are GREEN and cover exact formatted pre-commit content. `39e04423` shares Retail-array arithmetic through the API; agent137 reports four targeted tests, but no integrated map-time producer exists. `602b76f7` tracks four shader pixel cases; agent128 reports 4/4, but no inspectable logs were available. Its fixture expansion remains uncommitted and is excluded. Verifier139 only completed an audit: no Cargo proof exists, and the focused terrain check remains pending. This does not establish GPU/world rendering or parity. Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).


## [2026-09-26] system | Godot native terrain-material attachment boundary

`0953ef72` decodes referenced terrain BLPs from local CASC. `9f6000f6` then attaches `ShaderMaterial` mesh children to all nine streamed tiles with native diffuse and MCAL `ImageTexture` inputs. `/tmp/claude/godot-terrain-material-binding-red.log` is genuine RED: parsed terrain had no corresponding native material tiles. `/tmp/claude/godot-terrain-material-binding-green.log` is GREEN: all nine attachments exist, reconnect frees `WorldTerrain`, and the host remains `Loading`. `/tmp/claude/godot-terrain-material-binding-build.log` exits 0.

The build covers source before formatting but does not identify an exact tracked shader revision; agent128's initial GPU cases were uncommitted and remain separately pending. This is material-binding protocol evidence, not GPU/world rendering or parity. Actual Retail map-time, fog, cubemap, sun, camera and animation clock; mip parity; WMO/doodads; water; collision; readiness; and transfer remain open. Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md).

## [2026-09-26] system | Godot terminal terrain-worker error retention

At exact `2ee88127`, `StreamedTerrain` retains a terminal worker panic error, so repeated `poll` calls continue returning that error rather than succeeding after the worker disconnects. Verifier130's focused native check passes (`/tmp/claude/worker-terminal-check-2ee88127.log`). The same revision's formatting check fails (`/tmp/claude/worker-terminal-fmt-2ee88127.log`); main committed formatting-only `01c78a5b`, which is not yet reverified. Earlier `/tmp/claude/cargo-streaming-red.out` and `/tmp/claude/cargo-streaming-green.out` establish the targeted repeated-poll RED/GREEN boundary but lack embedded revision metadata. This does not establish world rendering, readiness, `InWorld`, or conversion parity. Updated [[godot-conversion]] and the [Godot conversion specification](../specs/godot-conversion.md).


## [2026-09-26] system | Godot async native terrain-asset boundary

`cad33614` shares terrain material inputs with Godot core and has five pure tests. This is input-data proof only; no native shader/material parity is claimed.

`27944c68`/`1261766c`/`79267767` add generation-isolated asynchronous native `LoadTerrain` loading. `/tmp/claude/godot-world-terrain-flow-79267767.log` exits 0 after a real local CASC/cache fixture parses nine tiles, then reconnect clears the asset state while the host remains `Loading`. The nine `_obj0` lines on stderr report parsed placement counts and are informational, not errors. `/tmp/claude/godot-world-terrain-integrated-build.log` exits 0 without warnings; the prior five unused-terrain-API warnings are resolved by an async consumer and duplicate WDT tile-field removal. Independent verifier113 at exact `79267767` records `terrain::` 7/7 GREEN (`/tmp/claude/godot-world-terrain-native-tests-79267767.log`), native `fmt --check` GREEN (`/tmp/claude/native-fmt-79267767.log`), and native check exit 0 without warnings (`/tmp/claude/native-check-79267767.log`). The 9-finding readability report has no metrics-limit breach; findings are not automatic tasks.

At docs-only `773dbb91`, main-root `cargo check --bin game-engine` exits 0 with two `InputBindings` unused-import warnings, unchanged from `master` in `src/rendering/ui/target.rs:16` and `src/sound/runtime.rs:11` (`/tmp/claude/godot-shared-source-bevy-check.log`); root tests did not run. No rendered terrain, WMO objects, textures, collision, readiness, `InWorld`, `NewWorld`, or `WorldPortAck` follows. Pending main panic-regression work is outside this proof. Updated [[godot-conversion]].


## [2026-09-26] system | Godot verifier97 and local-CASC dependency boundary

Verifier97 at exact engine `251f3263` plus sibling `9206f9e` records `world::tests` 3/3 GREEN, native `fmt --check` GREEN, and focused native check exit 0 without warnings at `/tmp/claude/godot-{world-tests,native-rust-fmt-check,native-check}-251f3263.log`. This refreshes proof only; prior native fixtures remain bounded. Transform updates and ordinary despawn are unproven; visible models, map, readiness, and `InWorld` remain absent.

`e2821b83`/`1961bd14` add the existing local-CASC resolver dependency through the pinned same-project SDK source. `/tmp/claude/godot-terrain-assets-red.log` is genuine RED for a missing API, not terrain feature GREEN. `c94a6aa6` native character-create fixture is genuine RED for a missing method. Agents100 terrain,101 UI, and102 world refactor remain active; no pending-code claim follows.

## [2026-09-26] system | Godot transport snapshots project WorldUnits

`087d3d74`/`5910ce51` add Godot-owned server-ID-keyed WorldUnits node lifecycle from actual transport snapshots: spawn/update/despawn/reset, direct authoritative position axes, wire-Y yaw, and original spawn defaults. The host selects the local player by exact selected-character name; initial duplicate selection preserves original child creation order rather than sorting server IDs. `account_state` now exposes `world_attached` and `local_player_position`.

Proof remains open. The preceding `b90efecf` `world_units_flow.gd` is genuine RED at `/tmp/claude/godot-world_units_flow-b90efecf.log`, timing out for the selected-character node. `087d3d74`/`5910ce51` are unbuilt and unexecuted. No model/map/readiness/`InWorld`/visual claim follows. `cc324f51` button-margin code remains separately unbuilt and unproven; its genuine pre-fix RED is `/tmp/claude/godot-button-size-red.log` (`actual(256,71)`).

## [2026-09-26] system | Godot viewport EnterWorld reaches Loading

At `b90efecf`, shared original character-select postsetup registry sizing corrects the `dd45d9bc` viewport no-dispatch root cause; it is not a generic layout workaround. Clean native build exits 0 at `/tmp/claude/godot-enter-world-build-b90efecf.log`. `character_select_ui.gd` exits 0 with empty stderr at `/tmp/claude/godot-character_select_ui-b90efecf.log`. `enter_world_flow.gd` exits 0 with empty stderr at `/tmp/claude/godot-enter_world_flow-b90efecf.log`: the real viewport selects `Elara`, activates EnterWorld, receives the actual local-server response, establishes the selected character, and transitions to `LoadingUI`. Authored PNG/shell are visible at 0%; no `InWorld` is fabricated.

This is bounded auth-to-Loading evidence. At `b90efecf`, `world_units_flow.gd` is genuine RED at `/tmp/claude/godot-world_units_flow-b90efecf.log`, timing out waiting for the selected-character unit node. World readiness, unit application, a world scene, visual parity, and full conversion remain open. `cc324f51` subsequently fixes native Button theme content margins that enlarged authored 64px buttons to 71px and adds a size assertion; `/tmp/claude/godot-button-size-red.log` records genuine `actual(256,71)`. That code/test is committed but unbuilt and unproven, so `b90efecf` GREEN evidence does not cover it.

## [2026-09-26] system | Godot loading-model and input-routing boundary

Corrected stale proof through `c3911dff`. The `c3911dff`-inclusive native build exits 0 at `/tmp/claude/godot-loading-shell-build.log`, including `203d2d85` authored three-slice loading projection. `loading_ui.gd` exits 0 with empty stderr at `/tmp/claude/godot-loading-shell-fixture.log`: authored loading PNG artwork, shell, and initial progress. `b7070542` card input is GREEN at `/tmp/claude/character_card_input.gd.green.log`: a viewport left press selects the authored second card. Current `ui_projection.gd` and `character_select_flow.gd` both exit 0 with empty stderr, covering `a89ab4d4` input readability and real auth→roster→Back.

No `GameClient` Loading route or world-readiness logic exists. No rendered visual parity, full workflow, or feature-parity row is closed; full conversion remains open.

## [2026-09-26] system | Godot successful auth to native character select

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md) through `d1641bc9`/`9249929e`. `character_select_flow.gd` is GREEN with empty stderr at `/tmp/claude/godot-character_select_flow-d1641bc9.log`: successful `admin`/`admin` auth against explicitly local UDP `127.0.0.1:5000` creates `CharacterSelectUI` from the original `CharacterSelectModel`, maps protocol roster data, and hides `LoginUI`. `character_select_ui.gd` is GREEN with empty stderr at `/tmp/claude/godot-character-select-ui-corrected.log`: empty authored UI and Back action. The initial EnterWorld-disabled assumption was removed because the original authored button is enabled. `744f3e1b3` mapping tests are pure.

This remains fixture-bounded evidence. Current `1296be4b` native-build and auth→Back-flow logs are GREEN with empty stderr: Back restores LoginUI. `1296be4b` adds host `SelectChar` dispatch, but original cards are Frame `onclick` while native projection supports only Button callbacks, so functional roster selection is unproven. Verifier67 reran `login_flow.gd` and `ui_projection.gd` PASS on an artifact timing-qualified to `d1641bc9` or `12a23693` (host files identical; tint-only difference). Agent65 reports targeted real selected-card tint RED/GREEN evidence only. EnterWorld/create/delete/campsite/menu remain unsupported; no character/world scene, background/appearance, or matched visual proof exists. No parity row closes.

## [2026-09-26] system | Godot native login/UI and parser proof boundary

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md) through `aadb3597`. Latest `/tmp/claude/godot-login_flow-3de5946b.log` and `/tmp/claude/godot-ui_projection-3de5946b.log` identify `3de5946b`, exit 0, and have empty stderr. `50cd3c63`/`33ed3a57` wire native input/action synchronization, connect/reconnect/exit, selected-host routing, and pending status; `1251724a` projects disabled buttons and suppresses callbacks. The login fixture uses existing local UDP port 5000 and wrong `admin` credentials, proving rejection reaches the native button/status path only. No account creation; successful login stops at `screen_requested`; no native character-select exists; realm cycle/menu/create-account explicitly report unsupported.

The UI fixture covers true viewport Unicode/Ctrl-A/backspace/focus editing, gold/resize/removal updates, disabled-state/callback behavior, asset/font/insets. Screenshot `04aa3610` is RealForward+ but predates final colour/focus changes, so it is not an exact visual baseline. A seeded Bevy screenshot hung for ten minutes; parent terminated PIDs 136318/136334 and verified both gone. Its wrapper exit 0 and absent screenshot provide no visual evidence.

Reported `fb54637f` pure-core proof is 233/233 `--lib` tests GREEN with no warnings, including concurrent ADT changes. `3de5946b`/`18666116` retain ADT metadata, expose tile/LOD parsing, and correct fixture coordinate/companion coverage. `1a5c9045`/`aadb3597` move engine-dependent ADT/WMO/M2 tests into Bevy adapters; those legacy adapter tests were not compiled or executed, so they remain an obligation. No parser, login, UI-fixture, or raw-terrain result closes a full parity row.

## [2026-09-26] system | Godot integrated native proof ledger

Updated [[godot-conversion]], the [Godot conversion specification](../specs/godot-conversion.md), and the [detailed parity matrix](../specs/godot-parity-matrix.md) for `ac02b9a0`. The native extension build exits 0 with 27 `game-engine-core` warnings and 2 native-animation dead-code warnings. `client_login.gd` exits 0 with named authored credential controls; `terrain_geometry.gd` exits 0 with 256 raw chunks, authored heights/UVs/colors, holes, and Godot winding; `account_failure.gd` exits 0 when local UDP port 5000 rejects wrong `admin` credentials and native account status receives the feedback. The logged texture-file messages are cwd-only preflight failures: source paths remain retained and the native loader still loads them, so they do not demonstrate missing artwork.

`model_scene.gd` exits 1 because the login canvas covers the model; `5ca2dc8d` hides it only after successful import, and GREEN awaits a rebuilt native binary. Agent39 reports `m2_assets.gd` GREEN after `97d97af1`/`5297d394` on the same `.so` (216 bones, 113 batches, 37,813 vertices, 147,966 indices, torch BLP, no leaks); this remains reported, nonvisual evidence. Agent42's UI projection test is RED for wrong positions/input and requires actual keyboard exercise. Agent58 reports ownership fixes without supplied proof. `CharacterSelectModel` pure tests are GREEN at `a828a028`/`fcd34042`, but no native character-select exists. All feature-parity and full-conversion acceptance gates remain open.

## [2026-09-26] system | Godot account host and raw ADT geometry boundary

Updated [[godot-conversion]] and the [Godot conversion specification](../specs/godot-conversion.md) after account host `7880fc74` and native ADT geometry `2ee292bb`. `GameClient.connect_account`/`account_state` now route typed auth through the headless worker and `Session`; polling emits `screen_requested`, but does not create or route real character-select, creation, loading, or world scenes. `account_failure.gd` is RED (exit 1) against an old binary lacking `connect_account`; real-server GREEN remains pending. `WowTerrainLoader.load_adt_geometry` converts raw ADT chunk geometry (positions, normals, colors, UVs, holes, reversed winding) only; `terrain_geometry.gd` is RED against an old binary and awaits ADT52's core geometry API. No terrain materials, objects, water, collision, or streaming claim follows. Verifier48 reported parser 7/7 GREEN with timing-qualified source evidence; network verification was 4/4 GREEN at `73339584`, before a `WireMessage` re-export. Verifier49 assigned core warnings to ADT52/M254; network readability findings remain unfixed. All conversion acceptance gates remain open.

## [2026-09-26] system | Godot authorized transport/UI boundary

Updated [[godot-conversion]] and the [Godot conversion specification](../specs/godot-conversion.md) after `eef95b16`. Godot is authorized to own rendering, UI, scenes, and gameplay; Bevy remains only as the headless networking transport worker. `godot/network` contains an actual transport bridge, but auth has not passed through it. Sibling UI core `70db707` removes all Bevy dependencies with `DynamicTextureId`, RGBA8 registry data, and atlas `PixelRect` arrays; agent46 reports two targeted model tests GREEN, not independently verified by main. Main `baf769ab` wires login UI startup, while `client_login.gd` remains RED against the old native binary because `LoginUI` is absent. Native compilation is blocked by UI API errors under agent42 work. All parity gates remain open.

## [2026-09-26] system | Godot native M2 preview boundary

Updated [[godot-conversion]] and the [Godot conversion specification](../specs/godot-conversion.md) through `e21f7c40`, `74ccbd8f`, `9c09cfb6`, and workspace integration `a9c8f58b`. The documented capability was a native M2 preview path with skinned batches, limited BLP/type-0 albedo material conversion, sequence attachment, and fixed framing. At that revision, the model-scene and M2-assets scripts were unexecuted because the portable UI-toolkit `WidgetDef` macro-anchor mismatch blocked the integrated build; the later local macro patch fixed that blocker. Headless transport still carries Bevy state and, through `shared`, Bevy camera/mesh dependencies; no runtime, material-fidelity, UI-parity, or migration-completion claim follows.

## [2026-09-26] system | Godot conversion bootstrap

Created [[godot-conversion]] for game-engine `85794170`: verified Godot 4.7.2 archive, `godot` 0.5.5 Rust GDExtension workspace, and RED/GREEN `GameClient` native-`Node3D` headless smoke. Editor import scan `SIGABRT` remains separate and unresolved; all feature parity remains open.

## [2026-09-27] investigation | Release-spirit OOM kill

Created [[release-spirit-oom]]. The client that "terminated" on Stockade spirit release was SIGTERMed by earlyoom at 10.3 GB RSS. A jemalloc profile found per-NPC animation clips (3.8 GB) and per-NPC clones of bone tracks (3.0 GB). Both are now shared per model. In the live run, Stockade RSS went from 12.4–13.0 GB to 5.1 GB, and the release → Stormwind transfer completed.

## [2026-09-26] investigation | Bevy, Godot, and solarityclient bones

Created [[bevy-godot-bone-comparison]], a source-only comparison pinned to game-engine `c3e9be93` (documentation through `a9d9c950`), patched Bevy `20ed24db`, upstream Bevy 0.19.0, Godot 4.7.2 `ed1daf0bf001b61586d9930840f2f1394092c079`, and solarityclient `f5f5f4a81e5c11241f4c80c117e5dfe9b587dcee`. It records representation, dirty/upload, visibility, and skinning-pass boundaries; no runtime winner or flat-array conversion follows. Corrected [[solarityclient-performance-comparison]]: its inspected placement loop recomposes before final camera-frustum rejection; earlier admission gates can still skip work.

## [2026-09-25] collision | WMO floor collision (player ground)

Created [[player-ground]] and the [wmo floor collision](../specs/wmo-floor-collision.md) spec. The client and the server now choose the highest walkable terrain or WMO floor within 1.6 yd above the feet, using `shared::ground`:
- MOPY-collidable faces are reached through the MOBN/MOBR BSP.
- The server loads each tile lazily from the client cache.
- A reposition no longer counts as a fall.

In the live run, set-position into the Stormwind auction house landed at z 98.02, not the terrain at 94.65. In the Goldshire inn it landed at 56.96, over terrain at 56.42. Updated [[collision-system]] and [[stormwind-hilly-plaza]].

## [2026-09-25] performance | ui-toolkit settled-registry gate

Merged ui-toolkit `86d2639` stops reconciling a settled registry into native entities. Profiled `sync_registry` went from 32.5 ms to 0.04 ms per frame. On a distribution master build, FPS rose in all 3 interleaved pairs (22.5→23.8, 13.0→36.0, 11.5→21.7), but two pre-gate runs were GPU-clock-limited. See [[movement-performance]].

## [2026-09-24] performance | Release in-world FPS profile and prepass policy

A distribution build at the Northshire spawn (headless, 1280×685) ran 9–24 FPS. The GPU was 45–98% busy with clocks firmware-limited to 0.6–1.4 GHz, and no CPU thread saturated.
- Shadows take about 44% of GPU frame time.
- `ui_toolkit::native_render::sync_registry` is the largest CPU span, at 10–24 ms per frame.
- `3f0d6ecb` keeps `WowCamera` prepasses, jitter and mip bias only for SSAO/TAA. GPU time per frame fell 22% over five interleaved pairs.
- `6e34121f` gates the world-map model rebuild.

See [[movement-performance]] and [[rendering-pipeline]].

## [2026-09-23] talents ui | Retail trait-tree window

`PlayerSpellsFrame` replaces the legacy 28-talent frame, whose messages the server no longer answers. The client loads the ten class trait trees and the `talents-*` atlas members from the 12.1.0.69933 CSVs (cold 1.49 s, warm 7.4 ms from a 151 KB cache in the dev test profile). It mirrors the server `trait_config` rules to decide which edits it keeps, and sends `CommitTraitConfig` / `SetSpecialization`. The legacy talent status snapshot and the IPC/CLI `talent` commands are removed. 17 targeted tests pass (7 lib, 10 bin) on the real Paladin tree 790. No native capture yet. See [[talents-ui]].

## [2026-09-23] spell catalog | Add client spell catalog

At `77eadcf8`, `SpellCatalog` loads all 414,027 spells from the 12.1.0.69933 CSVs on a background task. It writes a keyed bincode cache under `data/cache/`. Dev-profile load takes 10.7 s cold and 0.89 s warm, with about 114 MB of heap. 16 `spell_catalog` lib tests pass: token fixtures, plus real-CSV rendering for Fireball, Crusader Strike, Shadow Word: Pain and 7 other spells. The app runtime load was not exercised. See [[spell-catalog]].

## [2026-09-23] character creation | Record behavioral scene and name-entry verification boundary

At `24fce090`, independent evidence records 30 passing character-creation scene tests (one ignored), covering mapped-scene replacement/exit lifecycle and authored camera/presentation behavior; its focused 20-logical-pixel Arial Narrow name glyph test also passes. `126ab4aa` differs only in scene module ordering and independently passes `cargo check` plus scoped library formatting. A native runtime dump confirms `Donagh` with cursor position 6 in the unchanged 300×38 name field. This is behavioral/source and native-state evidence, not visual acceptance: revised-lighting GUI inspection, all-three-backdrop scene acceptance, and exact Retail parity remain open. An internally inconsistent visual-helper description was excluded as evidence. See [[character-creation]].

## [2026-09-23] character creation | Record authored creation-backdrop integration boundary

`ChrRaces.CreateScreenFileDataID` maps Alliance/Horde/neutral Pandaren to cached local-CASC M2s `623712`/`623714`/`623716`; race 25/26 reuse Alliance/Horde, while neutral 24 remains loader-supported without an actor or roster change. The cache currently holds three M2s, 12 skins and 101 textures. Snapshot-zero camera framing is normalized from root attachment 0; authored presentation scale and camera-distance offset now apply while existing face-focused controls remain. Type-0 ambient converts to linear RGB with brightness reciprocal to default exposure, retaining its dimensionless multiplier before Bevy BRDF. The extra 8,000-lux directional fill is removed. The procedural environment-map resource was never camera-bound, has been removed, and is not evidence of active IBL or washout. Backdrop-owned standard materials are diffuse (`reflectance = 0`, roughness `1`) to match M2-effect/UI-model references; other materials remain untouched. This is PBR/light-unit approximation, not WoW parity. Scoped RED/GREEN evidence includes neutral loader-only GPU capture (`neutral-diffuse-lighting-gpu.log`); a new GUI lighting capture, three-backdrop visual acceptance and independent final verification remain pending. The pinned Wago NameGen acquisition input is retained separately from untracked `data/NameGen.csv` cache output; no names are fabricated. See [[character-creation]] and [[m2-format]].

## [2026-09-23] formats | Parse first authored M2 camera snapshot

Version-274 MD21 camera offsets are relative to the MD20 payload, not the file; a 116-byte record has base position/target plus first spline offsets and a tracked diagonal FOV. The pure parser rejects missing records/keys and matches cached `623712`, `623714`, and `623716` bytes in three focused tests. Animation and scene integration remain out of scope. See [[m2-format]].

## [2026-09-22] character creation | Apply authored dropdown background slices

Build-pinned `UiTextureAtlasElementSliceData` for `12.1.0.69875` adds element 25590 margins 23/18/23/28. Engine `83490974` applies the asymmetric `NineSlice` after both `Screen::sync` paths; the prior whole-image stretch left the popup background short of its columns. Native nine-part, opaque-center, label-order and idle-dirty tests pass. Runtime and final verification remain pending. See [[character-creation]].

## [2026-09-22] character creation | Match Retail dropdown insets, widths and swatches

Engine `4cfb538f`, `9761008e` and `627cb5d6` implement `MenuStyle2` 3/6/3/7 content insets, content-dependent 144/107/136/70 row widths, dual half/full palette placement, secondary-only colors and selected-outline anchoring. Local Lua/XML source and 31 targeted component tests establish layout/layer behavior. Runtime and independent final checks remain pending; no full Retail visual-parity claim. See [[character-creation]].

## [## [2026-09-22] character creation | Record pending closed-value and hover-ring corrections

Retail `SelectionDetails` is a centered `ResizeLayoutFrame`: the XML's 144-pixel size is initial, while swatch content resolves to 42 pixels for one effective swatch or 54 for dual colors; named text caps at 126. Engine `fa162861` applies the closed-trigger layout without changing its 150-pixel input area. `RingedMaskedButtonMixin:UpdateHighlightTexture` makes checked hover match `CheckedTexture` and unchecked hover match the authored Ring. Engine `c022b454` authors those extents. Native RED evidence exists; toolkit highlight-size projection, runtime capture and independent verification remain pending. See [[character-creation]].

## [2026-09-22] character creation | Correct build-pinned palette atlas provenance

Build-pinned Wago CSV exports for `12.1.0.69875` identify the correct atlas-708 palette crops: normal `[519,471..603,491]`, half `[729,471..813,491]`. Older project CSV metadata selected yellow ornament pixels. Fresh local-CASC extraction of FDID `1253496` is byte-identical to the cache, so cache replacement and the unavailable encrypted local DB2s were not the remedy. Toolkit `78c67e7` corrects only the palette bounds; engine `a55af09f` records native source-pixel RED/GREEN. Reference dropdown insets and dual-swatch layout remain open. See [[character-creation]].

## [2026-09-22] character creation | Record staged full-customization boundaries

The selected local Retail Interface files define reference layout/control behavior. Engine catalog work surfaces 1,147 authored options in 59 categories with order, icon, swatch, requirement and unsupported-effect metadata; automatic cache refresh replaces manual cache repair. Shared and server commits preserve non-core `(option_id, choice_id)` selections through historic-bitcode upgrade, creation, reopen and roster. This records staged data/persistence capability only: generic control integration, effective renderer proof, general eligibility evaluation, circular portrait masking and rendered parity remain open. See [[character-creation]].

## [2026-09-22] ui | Stage additional character-customization selections

Shared `9123a5f` retains the existing six appearance selectors plus `sex` and adds disjoint authored `(option_id, choice_id)` selections. Engine `6ca0a28d` updates barber, export, and IPC consumers to preserve owned appearance values. This is staged representation plumbing only; full character-customization UI, renderer, persistence verification, and acceptance remain open in [character-creation](../../specs/character-creation.md).

## [2026-09-22] ui | Resolve character-creation customization atlas through local CASC

Toolkit `1b48b132` replaces the character-creation atlas's machine-specific source path with FileDataID `1253496` (`Interface/GLUES/CHARACTERCREATE/CharacterCreate.BLP`). Existing arrow and palette names/UVs remain unchanged. Engine `4ee36dd1` compares all eight native crops with decoded local-CASC RGBA source pixels. See [[ui-system]] and [[asset-pipeline]].

## [2026-09-22] ui | Resolve character-creation icons through local CASC

Commit `11267c8d` replaces 22 race and 10 class machine-specific icon paths with their exact community-listfile FileDataIDs. Retained `rsx!` widgets author `texture_fdid`; `GameBlpLoader` resolves through local CASC/cache. The Worgen portrait remains FDID `455993`; no alternate art or directory fallback was added. Source tests cover listfile resolution, decode, and native image content. Current rendered character-creation acceptance is separate. See [[ui-system]] and [[asset-pipeline]].

## [2026-09-12] ui | Restore resolver-backed character-selection atlas artwork

Character-list borders and card backgrounds were already authored through existing atlas names and a panel nine-slice; they rendered as bare text because the toolkit attempted to load `UICharacterSelectGlues.BLP` from a missing machine-specific path. `ui-toolkit` `437b606` replaces that source with explicit `FileDataId(5648070)`. DB2 maps `glues-characterselect-card-*` members through atlas `2726` to the 1024×1024 FDID. Engine `4fffafcf` resolves it through local CASC when absent from `data/textures/`, then the toolkit CPU-decodes and materializes cached card/panel crops. Toolkit RED at `392de1c` reproduces the missing path; nine focused tests pass. A cold-cache native run recreated the exact asset, and final `a2b284bd` capture confirms the dark panel, selected gold card, and authored unselected dark card/rim without obscuring the 3D scene. Exact Retail pixels are not claimed. See [[ui-system]].

## [2026-09-12] rendering | Correct character-select lighting ownership and M2 light clocks

At `4fcc7437`, parser commits `fdfd231f` → `5dc4386f` establish 156-byte modern M2 light records and retain two cauldron plus four-lantern fixtures without changing photometric conversion. Attachment paths retain type-1 lights; standalone local tracks use the authored default-sequence period, player-owned local tracks use their explicit player, and global tracks use shared elapsed time modulo their declared period. Joint-bound equipment lights preserve their bone parent but use linked model-root ownership, so removal and re-equip do not leak lights. `SkySun` positively scopes sky color/time updates; character select has one environment sun, ambient0, and camera IBL300 instead of fabricated campfire/fill lights. Late-created suns initialize at a settled clock. Focused GREEN: 15 binary and 7 library tests; native capture records a non-black terrain/character scene with one environment sun while retained unrelated warnings remain recorded. Independent final verification confirms the fixes, 22 regressions, successful compile/build, and changed-code readability; repository formatting still fails only in 104 unchanged vendor files. No Retail brightness match is claimed. See [[character-select-lighting-overwrite]], [[skybox]], and [[m2-format]].

## [2026-09-12] rendering | Record character-select directional-light overwrite boundary

A read-only headless probe records `OnEnter(CharSelect)` setup at 35,000/12,000 lux, followed by the first sky update setting both directional lights to the same cool 1,000-lux color and rotation; ambient remains brightness 150. Scene-tree output is stored setup metadata rather than live light-component state. Temporary probe-source inclusion was restored. ROI review rejects numerical image EV estimates, and local data does not establish exact Retail settings, a brightness match, or a production fix. See [[character-select-lighting-overwrite]] and [[character-select-waterfall-loading]].

## [2026-09-11] ui | Document nameplate visibility, picking, and offline preview boundary

Source audit through engine `f3dab635` records projection-time exclusion of every local-owner plate part, including late `LocalPlayer` assignment; shared health-body distance fade/hide for names, health, and casts; and visible plate-owner selection before world mesh raycasting after registry UI input precedence. `HudOptions.nameplate_distance` remains the configured policy; no current retail-native cap is asserted. `--screen nameplatedebug` and `--screen nameplate-debug` route to an offline plain-owner preview with looping normal casts and channels, Space pause/resume, and plate selection. The approved half-size calibration and Thick-health/Thin-spellbar defaults remain unchanged; glow work stays deferred. This is source documentation only: no current-cycle test execution or rendered runtime proof is claimed. See [[nameplate-design]], [nameplate spec](../specs/nameplate-style.md), and [nameplate debug spec](../specs/nameplate-debug.md).

## [2026-09-11] ui | Record reference-derived half-scale nameplate art boundary

At engine `e3b65e7c`/`f574d83f` and runtime `d485bdd4`/`bd78d242`, health/cast frames are generated from the supplied reference rather than read directly from WoW frame atlases. `debug/make_nameplate_skins.py` uses reproducible linear unmatting and records source/crop/limitation data in `provenance.json`; frame interiors are transparent so runtime health/cast content remains live. The glyph-free health gradient crop supplies health fill; authored `4505182` remains cast fill/background. No generic pip appears in the reference, so none is rendered. `be3aaf9d` tests the transparent-live-interior boundary. At `NAMEPLATE_SCALE = 0.5`, the effective health width is 188px from a 376px raw interior; health is 20px/10px and cast is 10px/6px for Thick/Thin, with 13px/10px labels. Original alpha cannot be uniquely recovered from the composited screenshot; `debug/compare_nameplates.py` is diagnostic only and GPU pixel-match proof remains pending. See [[nameplate-design]] and [nameplate spec](../specs/nameplate-style.md).


## [2026-09-11] rendering | Verify character-select terrain normal-axis root cause

Same-binary 30-second character-select captures against canonical and retained-worktree data both show the bright `CampsiteGroundPatch` over dark but loaded ADT terrain. Both runs loaded 256 chunks, 13 ground textures, and the skybox, disproving a missing-worktree-assets-only explanation. A standalone height-derived test of all 48 signed MCNR byte permutations on `2703_31_37.adt` identifies `[b0,b2,-b1]` as the supported Bevy mapping (mean geometric alignment `0.997198`), versus production `[b2,b1,-b0]` (`0.089730`). Shader probes show terrain bright before PBR lighting, dark after it, and brighter with an upward lighting normal. `CampsiteGroundPatch` remains a separate 42×42 StandardMaterial workaround: `bbaa3e51` removed it as a bright island and `d335cd0c` re-added it. No parser or shader fix has landed; add parser and rendered regressions before changing production decode. See [[charselect-ground-patch-dark-terrain]].


## [2026-09-11] ui/networking | Calibrate authored nameplates for reference pixel matching

Engine `3160bd2f`, `8934e738`, `0e7041f6`, and `f8cbaab7` move nameplates toward the supplied reference: the shared cache uses health atlas `6704514`, cast fill `4505182`, and important-cast frame `7241122`; health rendering is UI-overlay sprites rather than world-space PBR; names/cast labels are white Friz at 26px/20px. Current calibration is 384px health at 40px Thick / 20px Thin and cast at 20px Thick / 12px Thin. It is working calibration only, not pixel-match proof. The required GPU fixture alignment and verification remain pending; allowed variance is glyph rasterization only. Thick/Thin defaults and cast lifecycle/settings semantics remain unchanged. Windows work is excluded. See [[nameplate-design]], [[networking]], and [nameplate spec](../specs/nameplate-style.md).


## [2026-09-11] ui/networking | Start reference nameplate settings and cast presentation path

Engine `957d068c` makes dynamic Bevy linking the ordinary development default; distribution builds explicitly omit default features and retain `ipc,casc`. Engine `2a0ed210` persists independent nameplate health/spellbar thickness settings with the explicit user defaults Thick health and Thin spellbar. Shared-protocol `d3b0736`, server `211d86a`/`3ba4085`, and engine `6a713af1` provide a bounded player cast-presentation path: validated cast intent attaches named replicated state, then stop/movement/expiry remove it and the worker mirror carries it to render entities. Server targeted tests pass; engine snapshot/style tests are written or in progress without current passing command evidence. No spell effects, NPC cast source, reference-render match, or target-first clutter state machine is claimed. See [[nameplate-design]] and [[networking]].

## [2026-09-10] rendering | Verify visible cloud opacity

`a55e0f5b` centers cloud opacity around the unchanged density threshold instead of stretching the blend to an unreachable texture extreme. Generated-texture GPU coverage now includes visible clouds and clear patches at density0.5; clear/full extremes and recalibrated seam test pass. Standalone screenshot confirms soft cloud contrast without changing exposure or sky colors. Independent check passes; unchanged vendor formatting failures remain. See [[procedural-sky-dome-visibility]] and local `data/diagnostics/cloud-visibility/verification.md`.

## [2026-09-10] rendering | Restore visible mid-density procedural clouds

`a55e0f5b` corrects a rejected RON-only diagnosis: runtime uses the richer LightData 12/noon CSV cache at density 0.5 and threshold 0.62. Periodic noise rarely reached that threshold, making clouds nearly transparent. The final shader preserves original density thresholds and centers its soft edge around the threshold. GPU RED had no mid-density bright pixels; GREEN reports 22.12% bright and 60.16% dark pixels, while clear/full preserve 100% dark/bright. Native capture remains pending. See [[procedural-sky-dome-visibility]].

## [2026-09-10] rendering | Verify cloud tiling correction

At `077599df`, seven generator cases, the real-shader longitude seam regression, and cloud-scroll preservation pass. GPU seam contrast drops from 28 to 3 levels across the fixture's finite angle; standalone capture shows the rectangular blocks absent in the captured area. Build/check pass; 104 unchanged vendor files remain format failures. Evidence and framing limits: [[procedural-sky-dome-visibility]].

## [2026-09-10] rendering | Remove procedural cloud tiling seams

`389e0185` replaces nonperiodic float-offset cloud noise with periodic integer-hashed gradient fBm. The former high-bit ridge seed converted to 45–61M float coordinates, whose ULP 4 collapsed nearby samples into blocks; repeat sampling then exposed nonmatching image edges. `077599df` keeps spherical primary UVs unwrapped until sampling and uses integer 2.0 secondary longitude repeats, eliminating its pre-scale-`fract` seam. RED observes three texture failures and a 28-level actual-GPU longitude discontinuity; GREEN/native proof remains pending. See [[procedural-sky-dome-visibility]].

## [2026-09-10] rendering | Verify procedural sky in standalone screen

At `83cf11ec`, the dedicated `--screen skyboxdebug --light-skybox-id 0` capture shows procedural clouds and a horizon gradient through the shared InWorld dome/material/color path. Concrete Azeroth selection/lifecycle tests and independent verification total 59 distinct passing cases; dev check passes, with unchanged vendor-format failures. No user character/camera changes were needed for this proof. Exact in-world after-view and unrelated authored-M2 artifacts remain explicitly qualified in [[procedural-sky-dome-visibility]].

## [2026-09-10] rendering | Initialize standalone skybox-debug dome colors

`83cf11ec` adds `SkyboxDebug` to the shared sky-color/environment update predicate. The screen already spawned a procedural baseline dome when its selected authored-skybox flags permit it, but its material stayed default white because updates applied only to InWorld and CharSelect. The new registered-system regression proves initial `LightKeyframes` colors and a later game-time refresh. Subsequent standalone rendered proof and its selection/integration limits are recorded in [[procedural-sky-dome-visibility]]. See [[skybox]].

## [2026-09-10] rendering | Correct restored procedural dome visibility

`58d4b12a` follows `21feec27`: the native Azeroth `sky_dome` existed but rendered navy because its triangles faced outward while `SkyMaterial` culls Back faces for interior viewing. It reverses the winding. The same revision makes `update_sky_colors` update newly added material handles when settled game time has not changed, preventing default-white late domes. RED/GREEN covers both boundaries; subsequent build and standalone visible proof are recorded in [[procedural-sky-dome-visibility]]. This does not alter authored M2 rendering; see [[procedural-sky-dome-visibility]] and [[authored-skybox-black-output]].

## [2026-09-10] rendering | Restore ordinary InWorld procedural sky

`21feec27` corrects the live Azeroth sky boundary: at Bevy `[-8977.593, 81.04212, 179.76495]`, map-0 Light row 1 selects clear `LightParamsID 12`; local DB2 decoding gives raw `LightSkyboxID 0`. That explicitly requests the procedural dome, not an authored M2. `ec826ee7` had removed its normal InWorld spawn on April 12, 2026, leaving the dark-navy clear color. The restored lifecycle spawns the existing camera-child dome only for explicit raw-zero rows, follows the active camera, and removes it when visuals disable or InWorld exits. Missing data and authored resolution failures do not fall back. Forced authored `skyboxdebug` black output remains unresolved; current `0x8012`/`0x8016` WGSL branches exist, but their reference-pixel combiner semantics remain unproven. See [[skybox]] and [[authored-skybox-black-output]].

## [2026-09-10] rendering | Add live in-world camera direction control

`f1e377e3`/`fa7d5bee`/`4e1de84d` add `game-engine-cli camera set --pitch-degrees 60` with optional `--yaw-degrees`: values are degrees, pitch bounds are −88° through +88°, omitted axes persist, and the command requires InWorld with exactly one active `WowCamera`. At `4e1de84d`, the dev-built live client accepted four requested directions and visibly changed view; a mixed valid yaw plus `NaN` pitch was rejected. Two library IPC tests, four binary camera behavior tests, and three CLI tests pass. Player-facing/input/collision live behavior remains open. It does not fix authored skybox rendering: the user-positioned upward capture remains uniform dark navy; see [[authored-skybox-black-output]].

## [2026-09-10] ui | Share plugin frame ordering with standalone compatibility

Implemented [[ui-frame-order]] at toolkit `02a3049`, tests through `3e61227`: one explicit preparation and six prepared variants delegate to the same bodies as standalone public systems. `UiRenderSet::Prepare` spans window/layout/button preparation and the final render-gated order producer; named-set same-pass geometry and standalone stale-resource independence pass. Independent verification records 39 tests, toolkit formatting/check/readability and bounded engine compilation. Computational sharing/common-body structure are source-audited, not helper-call tests. Bevy UI migration has not started; no CPU/native claim.

## [2026-09-10] ui | Shared frame-order implementation started

User authorized implementation before the planned Bevy UI migration. Toolkit `752305f` declares `UiRenderSet::{Prepare, Quads, Text, Shadows, Outlines, NineSlices, ThreeSlices}`; no systems use the sets yet and shared preparation/wrappers remain pending. Standalone setup and all rendering behavior remain unchanged at this checkpoint.

## [2026-09-10] design | Shared plugin frame ordering approved for documentation

Added [[ui-frame-order]] and its [contract](../specs/ui-frame-order.md). User selected unchanged standalone setup and accepted named plugin scheduling sets: public systems keep fresh local preparation; private plugin variants share one per-render preparation and common rendering bodies. Function-relative plugin ordering must migrate to named sets. Implementation and feature tests remain pending; no code, builds, native runs, or CPU claims.

## [2026-09-10] ui | Derive player-frame content masks from its artwork

`038b1ecf` replaces rejected circular/inset player-frame fitting with masks derived from the unchanged `396×142` gold/silver shell's connected portrait, health, and mana openings. The player shell remains uniformly reduced to `297×106.5`; portrait and masked resource content fill those openings, while the art overlays fills to preserve its painted edge treatment. Missing absolute class-icon paths resolve through local listfile/CASC cache instead of drawing a white fallback quad. Exact-bounds/state and rendered GPU evidence pass; toolkit normalized texture-coordinate crop support at `af1683e` still has a fixture-only proof correction pending, so cross-repository acceptance remains open. Target geometry remains separate.

## [2026-09-10] ui | Keep world health bars fixed across zoom

Updated [[ui-system]] and [[nameplate-design]] for engine `7b316396`: actor-parented world bars remain depth-tested while a screen-aligned, projected-tangent scale targets 80×8 logical pixels across zoom, viewport, DPI, and FOV changes. Text was already UI-pixel sized; the projected bar edge retains its 4-pixel label gap. Proof: 18 focused health-bar tests, two gap tests, and two GPU tests—including 5/10/20-distance zoom frames with fixed glyph dimensions. No new full native game run, CPU, or whole-nameplate redesign claim.

## [2026-09-10] ui | Avoid stable-sort scratch work in frame ordering

Updated [[ui-system]] for ui-toolkit `1050abb`: the existing total strata/level/raise/ID comparator now uses unstable sorting, and visible-frame effective size is computed once. Same three cases pass before/after; toolkit checks and bounded engine compilation pass with recorded concurrent-engine provenance limits. No shared order cache, allocator measurement, CPU, or native claim.

## [2026-09-10] ui | Borrow unchanged main text during reconciliation

Updated [[ui-system]] for ui-toolkit `66e6d35`: crate-local text properties borrow fontstring, button, and plain EditBox source strings; password masks remain owned with their byte-length asterisk semantics. `Text2d` takes ownership only for spawn or actual content repair. The same five Unicode/password/repair/settled cases pass before and after; toolkit checks and shared engine test compilation are verified in the [final artifact audit](../../data/diagnostics/unchanged-writes-20260909/text-borrow/verification/final-integration-report.md). No allocation benchmark, CPU, or native claim.

## [2026-09-10] ui | Face world health bars and align overlay names

Updated [[ui-system]], [[character-rendering]], and [[npc-motion-validation]] for `b7efbad5`, `5e5b2574`, `045d82d9`, and test-only GPU proof `4475ce04`. Health bars retain their world-mesh depth path while their +Z normal faces the camera under transformed parents; overlay names sit 4 logical pixels above projected bar edges and return to their centered name-only anchor when bars disappear. Twenty-four focused UI tests, a live HumanHD sword/shield regression, and one Health-before-Npc GPU test pass. Independent dev-bin check and scoped formatting pass; whole-tree formatting retains 104 unchanged vendor differences. A one-time approved 59.173-second native run captured names, compact spacing, camera-facing bars, and corrected equipment placement; actor yaw, not camera angle, changed between capture filenames.


## [2026-09-10] ui | Borrow shadow text before owned rendering

Updated [[ui-system]] for ui-toolkit `6802940`: private shadow properties borrow source text during reconciliation and allocate only for spawn or changed `Text2d` ownership. Content, alpha, font, geometry, traversal, and prior component-write behavior remain unchanged. The same three cases pass before and after; toolkit checks and shared engine dev-build integration pass with recorded provenance limits. No allocation benchmark, CPU, or native claim.

## [2026-09-10] ui | Verify primary-window UI integration boundary

Updated [[ui-system]] and [[movement-performance]] for ui-toolkit `924ca23`: actual UiPlugin verification has 5 passing cases plus toolkit format/check/readability for settled state, hover visuals, geometry repair, and thresholded resize behavior. The one test compile has weaker retained provenance. Shared dev-feature engine compilation closed the bounded integration gate, but its intentionally failing equipment assertion exited 101; this is not an engine test/check pass or screenshot-fix completion. No CPU/native claim.

## [2026-09-10] ui | Avoid unchanged three-slice, border, and highlight sprite writes

Updated [[ui-system]] for ui-toolkit `598ded9`: retained three-slice, backdrop-border, CSS-border, and direct button-highlight sprites reuse existing Transform/Sprite comparison. Reconciliation, repair, and lifecycle remain unchanged; highlight proof is direct-system only. The suite first had 4 settled-write failures and 8 passing preservation cases; all 12 current cases pass independently. No CPU/native claim.

## [2026-09-10] ui | Avoid unchanged button-input mutations

Updated [[ui-system]] for ui-toolkit `e25eecc`: hit testing and hover differences remain immutable until a visible button actually changes; press/release enters its mutable path only on left-button edges. Disabled hover and state, pushed-button reset, and real transitions retain their behavior. Five RED/GREEN integration cases plus the adapted unit test and independent toolkit/engine checks pass. No CPU/native claim.

## [2026-09-10] ui | Avoid false clean render-state mutations

Updated [[ui-system]] for ui-toolkit `f041c0e`: quad and tiled reconciliation check `render_dirty` immutably before clearing it. Empty sets no longer falsely mutate `UiState`; nonempty sets still drain at the existing points and reconciliation remains unchanged. Four RED/GREEN cases and independent toolkit/engine checks pass. No CPU/native claim.

## [2026-09-10] ui | Avoid unchanged tiled sprite writes

Updated [[ui-system]] for ui-toolkit `2b3c1f3`: retained tiled sprites reuse the shared quad comparison, so unchanged `Transform` and `Sprite` values are not reinserted. Discovery, real updates, external/missing-component repair, stale cleanup, and registry dirty clearing are unchanged. Four RED/GREEN cases and independent toolkit/engine checks pass. No CPU/native claim.

## [2026-09-10] ui | Avoid unchanged nine-slice sprite writes

Updated [[ui-system]] for ui-toolkit `67413da`: retained nine-slice part entities reuse the shared quad comparison, so settled `Transform` and `Sprite` values are not reinserted. Geometry, color, image and UV updates, external/missing-component repair, spawning and stale removal retain full reconciliation. Four RED/GREEN cases and independent toolkit/engine checks pass. No CPU/native claim.

## [2026-09-10] ui | Avoid unchanged button nine-slice invalidation

Updated [[ui-system]] for ui-toolkit `eebcf28` with test correction `b31324a`: each button builds its complete derived nine-slice through immutable registry access, then compares it before `get_mut`. `NineSlice` and `TextureSource` equality includes all derived arrays, colors, texture variants/handles, per-part textures, and UV rectangles. Settled buttons skip render invalidation; state, hover, resize and external derived-state repair retain full reconciliation. Three corrected RED/GREEN cases and independent toolkit/engine checks pass. No CPU/native claim.

## [2026-09-10] ui | Avoid unchanged shadow text component mutations

Updated [[ui-system]] for ui-toolkit `3e6951d`: existing shadows compare all seven renderer-owned components before mutation, reuse main-text layout/bounds/font helpers, retain full reconciliation and external repair, and preserve unowned font fields. Shadow alpha semantics and outline synchronization are unchanged. Three RED/GREEN cases and independent toolkit/engine checks pass. No CPU/native claim.

## [2026-09-09] ui | Avoid unchanged text-render component mutations

Updated [[ui-system]] for ui-toolkit `c8fa209`/`c3a518a`: full text reconciliation remains, but owned values are compared before `Text2d`, layout, bounds, font face/size, color, transform, or anchor mutation. Unowned `TextFont` fields survive and a missing anchor is restored. Three RED/GREEN behavioral cases plus independent toolkit/engine checks pass. No CPU/native claim.

## [2026-09-09] ui | Combine visibility and alpha descendant propagation

Updated [[ui-system]] for ui-toolkit `ff2acd0`: `set_hidden` now traverses each descendant once, calculating visibility before effective alpha. Conditional writes and stale derived-state repair remain unchanged; `set_alpha` retains alpha-only propagation. The `ecbd655` characterization brings registry coverage to 28 GREEN cases after the prior 6 RED cases. No CPU/native claim.

## [2026-09-09] ui | Avoid unchanged visibility and alpha invalidation

Updated [[ui-system]] for ui-toolkit `2dec7fe`: same-value `set_hidden`/`set_alpha` calls no longer dirty unchanged frames or subtrees. Actual stored or derived visibility/alpha changes still dirty affected frames; descendant propagation still recurses to repair derived values after parent changes. Registry proof: 6 RED and 27 GREEN cases. No CPU/native claim.

## [2026-09-09] systems | Avoid false UI resource changes during clean layout calls

Updated [[ui-system]] for toolkit `a8c846b`: check layout dirtiness before mutable resource access. Two behavioral RED/GREEN tests and independent integration checks pass; no CPU claim.

## [2026-09-09] investigation | Final bounded wolf nameplate and starter-equipment evidence

Updated [[npc-motion-validation]], [[ui-system]], [[character-rendering]], and [[animation]] from `data/diagnostics/wolf-nameplate-equipment-20260909/final-proof.md`. Native capture shows a named Diseased Timber Wolf and Theron's shirt, pants, and boots; physical sword, shirt, pants, boots, and shield records (GUIDs10–14) survived a server restart. The rear capture does not distinguish sword/shield, so their HumanHD bones201/206 attachment proof remains the real spawn regression. Exact weighted wolf selection remains test evidence; the capture does not measure long-run howl distribution. Corrected the stale missing-asset diagnosis: an absolute-path gate skipped existing SKA1 attachments. Both captures contain the same HUD rectangles; no toolkit regression or HUD-health cause is claimed.

## [2026-09-09] character | Load skeleton attachments for absolute model paths

Removed the relative-path gate on SKID attachment loading. Real HumanHD path-equivalence and replicated-player sword/shield bone-parent regressions both fail before and pass after the correction. See [character rendering](systems/character-rendering.md#replicated-player-construction-boundary); no native validation in this slice.

## [2026-09-09] ui | Make clean layout state a no-op

Updated [[ui-system]] and [UI layout invalidation spec](../specs/ui-layout-invalidation.md) for ui-toolkit `0fdcf3f` and game-engine `86bbc945`. Empty `rect_dirty` now performs no layout or render-dirty work; insertion/removal, resize, anchors, flex, auto-sizing, and owned-addon geometry changes explicitly propagate. `get_mut` remains render-dirty only and explicit unanchored cached rectangles are unchanged. Proof: 14 focused plus 41 existing toolkit tests (55), 7 GREEN engine addon tests, and the coordinated engine gate closed at `9d22c7fb` for 62 distinct scoped tests. The saved engine output has no standalone command/exit record. No CPU/native claim.

## [2026-09-09] character | Resolve replicated physical item appearances

Item-only equipment entries now resolve through the existing outfit catalog instead of returning without rendering. Hidden and explicit-display precedence remain unchanged. Actual five-item starter regression fails before the fix and passes afterward; see [character rendering](systems/character-rendering.md#replicated-equipped-item-appearances). Native integration remains pending.

## [2026-09-09] ui | Project world nameplates through the 2D overlay camera

Corrected the `Text2d`/3D camera render-path mismatch in [[ui-system]]. Linked ownership preserves despawn cleanup without world-transform parenting. Six CPU projection/DPI/visibility/fade/lifecycle tests and a real-observer headless GPU RED/GREEN test pass; quest M2 billboarding remains world-space. Native integration remains pending.

## [2026-09-09] networking | Preserve authoritative NPC names in client snapshots

Documented shared `Npc.name` (`c8a06c0`), server SQLite-template propagation (`fc46567`), and engine cloned snapshot delivery (`79a252be`) in [[networking]]. UTF-8 SQLite/spawn and worker/main snapshot fixtures cover data boundaries. This is not renderer or visible-nameplate proof.

## [2026-09-09] animation | Select authored loop variants by weight

Replaced temporal `next_animation` interpretation with `variation_next` candidate lists and signed frequency weights. Actual wolf indices2/9/10/11 receive30445/1092/1170/60 of32767 deterministic rolls; terminal selection returns to the base family. Per-entity random streams, non-looping completion, invalid metadata, elapsed overflow and crossfade regressions pass in107focused animation tests. Replay bounds are retained but nonzero replay scheduling and native validation remain uncredited. See [loop-variation spec](../specs/m2-loop-variations.md) and [[animation]].

## [2026-09-09] investigation | Bound Northshire NPC motion and terrain claims

Added [[npc-motion-validation]] at engine revision `9404235d` and linked animation/UI evidence. The record credits authored facing and 422 observed idle bone changes; 20 landing tests and a 20.33-unit grounded endpoint walk; fog-off/on GPU compilation; two `Camera3d` picking tests preserving `--no-ui` health-bar policy; all-seven WMO bounds; radius-one retention; local-CASC roots/companions; and the initial nine-loaded, zero-failed native state. Cold capture observed two roots and companions, with one tile fully spawned before 30 seconds; the third was not observed and is not called a failure. No locomotion producer, WMO vertical-floor, full cold-ring, or performance claim.

## [2026-09-09] systems | Reconcile Northshire NPC motion, picking, fog, and WMO basis

Audited commits `02a4487`, `cb2d169f`/`a6b2f6fd`, `fbda0693`, `195ab94a`, `3f331032`, and `965f6f9e` against `data/diagnostics/npc-motion-20260909`. Updated [[animation]], [[ui-system]], [[rendering-pipeline]], [[wmo-format]], and [[collision-system]]. Static authored NPC heading, actual HumanMaleHD/sheep idle playback, nested-mesh LOD, world-camera selection, fog-on/off GPU compilation, running-landing completion, and all seven corrected MOHD→MODF bounds have focused proof. NPC locomotion-state production, arbitrary AI heading, WMO/M2 vertical support, and native/final integration checks remain open. `--no-ui` health-bar suppression remains expected.

## [2026-09-09] animation | Attach replicated NPC animation runtime

Updated [[animation]] and [NPC animation LOD spec](../specs/npc-animation-lod.md): exact display skins now use the full animated attachment path, with an animation owner beneath the scaled/facing visual root. LOD sees meshes below grounded descendants. Actual HumanMaleHD and sheep bone-motion fixtures, explicit texture pixels, facing-basis checks, and nested visibility tests pass; native visual proof remains with integration.

## [2026-09-09] rendering | Compile M2 effect fog variants

Updated [[rendering-pipeline]] with the conditional fog-binding and Bevy `apply_fog` argument fixes. Actual-material headless GPU regression reproduces both shader errors and renders fog-off/fog-on variants after correction; no native client or unrelated motion changes included.

## [2026-09-09] systems | Render authored Northshire NPC appearances

Updated [[character-rendering]] and [NPC importer spec](../specs/npc-appearance-importer.md) for game-engine `846a92e7`. Local WDC5-derived coverage supplies every source-present required profile in the current Northshire population; runtime applies declared baked/composited textures, full choice IDs, and display geosets with per-NPC materials. Focused compilation and renderer tests pass. A user-authorized 30-second capture shows clothed, differing background NPCs; the bare foreground model is local unequipped Theron. Full-catalog metadata remains blocked by unavailable local `TextureFileData` material444164 and is not credited.

## [2026-09-09] formats | Validate corrected Northshire wagon terrain

Updated [[terrain]] for `ac9cd925`/`36f5a4d5` and matching server sampler `2cfe233`. The local capture shows `stormwindgypsywagon01.m2` clear of the hillside after authored MCVT axes, water axes, and center-fan sampling were corrected. Its MDDF placement is unchanged; the existing terrain clamp now resolves a lower runtime Y from corrected ground geometry.

## [2026-09-09] systems | Share screen auto-size traversal

Updated [[ui-system]] for ui-toolkit `a118e8c`: collect a screen’s frame IDs once and reuse them across FontString and EditBox sizing. Four characterization tests pass before and after; no cache, change-driven behavior, RED, or CPU claim.

## [2026-09-09] systems | Poll UI attribute hot reload once per second

Updated [[ui-system]] for ui-toolkit `808117a`: debug hot-reload patches poll once per real-time second outside `Screen::sync()`. Normal state/layout updates remain immediate; only existing named-frame attributes reload.

## [2026-09-09] systems | Cache minimap coordinate formatting by raw position

Updated [[ui-system]] for `b0f4a006`: raw X/Z changes drive formatting while cached text repairs replacement frames and external edits. Seven scoped tests pass; no native CPU claim.

## [2026-09-09] formats | Correct MCVT terrain axes and preserve authored heights

Updated [[adt-format]], [[terrain]], and the terrain index summary for game-engine `ac9cd925`; reconciled the matching game-server sampler in `2cfe233`. MCVT rows map to negative Bevy X and columns to positive Bevy Z; the client mesh/sampler and server sampler now use authored four-triangle center fans. Removed erroneous seam averaging and corrected mesh winding. Focused proof: engine 4 tests and server 19 tests GREEN. Water follow-up and runtime cart visual confirmation remain open.


## [2026-09-09] formats | Add bounded NPC authored appearance importer

Updated [[db2-format]] with local WDC5 layouts, material/model resolution, isolated CLI usage, and proposed SQLite output contract. Synthetic tests cover decoding, joins, deterministic output, and explicit failures. Production promotion and renderer acceptance remain outside this importer.

## [2026-09-09] systems | Align startup observation with server pre-entry visibility fix

Updated [[networking]] for server commit `34e7551`. The exact no-UI InWorld command now reaches login success at app 1.896s and InWorld at 6.57s within ten seconds; server verification records 25 focused tests plus fmt/check. Screenshot CLI lacked `LD_LIBRARY_PATH`, so original NPC appearance/cart visual proof remains open.

## [2026-09-09] systems | Record bounded InWorld startup dispatch observation

Updated [[networking]] from `data/diagnostics/northshire-appearance-placement/startup-stacks.txt`: at three seconds, synchronous NPC M2 parsing occupied the main thread before the first main-world network tick. The first 10-second timeout did not dispatch auth. This is one local `--no-ui --screen inworld` observation, not a fix or a conclusion about the historical 18-second delay or all Elwynn sends.

## [2026-09-09] systems | Avoid unchanged character-creation shared state

Updated [[ui-system]] for `14f4a691`: complete character-creation state is reinserted only when it changes, while screen synchronization remains available. Three focused tests pass; no CPU claim.

## [2026-09-09] systems | Avoid equal character-select shared-state inserts

Updated [[ui-system]] for `1262fc58`: character selection, campsite, and delete-confirmation state reinsert only on change while screen synchronization and focus flow remain intact. Three focused tests pass; no CPU claim.

## [2026-09-09] systems | Avoid unchanged login shared-state insertions

Updated [[ui-system]] for `70f14c2a`: unchanged login screen values preserve dependency generations while screen synchronization remains available. Three scoped tests, compiler check, and focused formatting pass; no native CPU claim.

## [2026-09-09] systems | Skip discarded minimap tracking collection

Updated [[ui-system]] for `d319f9cf`: collect tracking points only when the existing redraw guard permits drawing. Thirty-eight scoped tests pass before and after; independent delta verification passes. No changed redraw policy or CPU claim.

## [2026-09-09] systems | Avoid idle action-bar background writes

Updated [[ui-system]] for `a00e88aa`: equal flash backgrounds no longer dirty UI state. Ten scoped tests and independent verification pass; no native CPU claim.

## [2026-09-09] systems | Avoid unchanged target-circle transform writes

Updated [[target-circle-rendering]] for `3ebbd348`: target-circle translation and scale still evaluate every update, but an unchanged complete transform no longer becomes changed. Stationary transform and target-movement behavior have one behavioral RED/GREEN proof; no CPU or rotation-specific claim.

## [2026-09-09] systems | Avoid unchanged minimap coordinate text

Updated [[ui-system]] for `55c8df35`: compare rounded coordinates before dirtying UI state. Thirty-seven scoped tests and independent delta verification pass; no native CPU claim.

## [2026-09-09] systems | Avoid unchanged UI billboard rotations

Updated [[ui-system]] for `56f82791`: nameplates, quest indicators, and health bars still derive their camera-facing rotations each update, but only mutate rotation when it changed. Position and scale remain untouched. Proof: six focused GREEN tests after four behavioral RED failures. No CPU claim.

## [2026-09-09] systems | Compose final nameplate color once

Updated [[ui-system]] for `6637a6d2`: one RGB-plus-distance-alpha writer replaces unordered base-color/fade writes. Seventeen focused tests and independent delta verification pass; no native CPU claim.

## [2026-09-09] systems | Avoid unchanged health-bar, camera, and model-light writes

Updated [[ui-system]], [[rendering-pipeline]], and [[animation]] for `275bd84c`, `536966e8`, and `c3e28125`. Health-bar, camera-effect, and animated-light systems retain evaluation and real updates but avoid mutating equal component or material values. Focused GREEN: health-bar11, camera48, model-light2. No whole-engine CPU claim.

## [2026-09-09] systems | Avoid unchanged nameplate and skybox material writes

Updated [[ui-system]] and [[skybox]] for `0357727b` and `e0aa5809`. Nameplate color and authored skybox UV/transparency evaluation remain current, but unchanged output no longer marks components or assets modified. Proof: three nameplate and 21 skybox focused tests after genuine RED cases. No whole-engine CPU claim.

## [2026-09-09] investigation | Floating Arena Tournament NPCs

Linked the [server-owned event membership investigation](../../../game-server/docs/wiki/systems/world-data.md#event-dependent-creature-spawns) from the index. Exact Northshire GUID96194/96198/96204 belonged to inactive event31; corrected server import/selection, not client coordinates or rendering. Server commits1963dfb/d9784a8/c73a33f,8behavioral tests and independent fmt/check/data proof. Local data/runtime updated; post-fix client view confirmation remains pending.

## [2026-09-09] systems | Suppress weather particles with graphics effects

Updated [graphics effect configuration](../specs/graphics-effects.md), [particle system](../particle-system.md), and [[rendering-pipeline]] for `b0e1f2bf`. `particleEffectsEnabled: false` now suppresses weather particles while retaining weather state, fog, and lighting. Six targeted weather tests pass. No CPU claim.

## [2026-09-09] systems | Persist independent graphics effect configuration

Updated [[rendering-pipeline]], [particle system](../particle-system.md), and [graphics effect configuration](../specs/graphics-effects.md) for `7f86f086`, `24d97a50`, `7b499bb2`, and `cab4207b`. `options_settings.ron` now persists all five controls; invalid SSAO plus MSAA fails explicitly. Disabled particles omit Hanabi and deferred emitters at startup. Proof: persistence6, camera13, particle23. No CPU or native-visual claim.

## [2026-09-09] systems | Gate no-UI observer creation

Updated [[ui-system]] for `711ade3c`: the shared UI gate hides the clock and prevents health-bar/player-nameplate/NPC-nameplate observers from creating UI assets/entities under `--no-ui`.

## [2026-09-09] investigations | Fix empty mesh uploads

Updated [[movement-performance]] for `ebcee198`: skip GPU uploads for meshes that received no allocation. Real GPU empty/populated lifecycle regression passes; native `4ea941c3` InWorld capture has zero allocator errors. Patch provenance lives in [vendor/README.md](../../vendor/README.md#empty-mesh-uploads).

## [2026-09-09] investigations | Record temporary GPU-culling probe

Updated [[movement-performance]] for `802b5014`. `WOO_PERF_GPU_CULLING_AFTER_SECS` is a one-shot diagnostic for a native attribution snapshot, not a supported interface or production setting; it has no CPU/visual result and must be removed after measurement.

## [2026-09-09] systems | Keep FPS visible in no-UI diagnostic

Updated [[ui-system]] and [InWorld scene isolation](../specs/inworld-scene-isolation.md) for `c299491b`. Opt-in `--no-ui` forces numeric FPS visibility despite saved HUD/menu writes, keeps the frame-time graph hidden, and continues to disable toolkit and world-space HUD visuals.

## [2026-09-09] systems | Add full UI-off diagnostic

Updated [[ui-system]] and [InWorld scene isolation](../specs/inworld-scene-isolation.md) for `a60cbc38918ec27c730a2166fb8312223501fb6b`. Opt-in `--no-ui` disables toolkit processing, rendering, text, and world-space HUD visuals; UI plugins/resources and 3D rendering remain. Defaults and existing pre-`Ui` diagnostics are unchanged.

## [2026-09-09] investigations | Record temporary application opt1 CPU evidence

Updated [[compile-latency]] and [[movement-performance]]. A temporary application-only opt1 build retained dynamic dependencies and runtime policy; one matched 20-second InWorld pair observed **253.179033% → 227.135068%** CPU. The reverse pair has clock confounding, initial 1m53s build is not edit-latency evidence, and default configuration/adoption remain pending.

## [2026-09-09] systems | Keep camera collision independent of view culling

Updated [[rendering-pipeline]] and [[collision-system]] for `7d1d8a86`. Camera collision raycasts now honor hierarchy visibility without excluding walls merely because collision places them behind the camera frustum. The real transform/visibility/frustum ordering regression RED recovers through the wall; GREEN retains collision and still permits recovery when the wall's parent is hidden. Original-video camera-motion pixel equivalence remains unproven.

## [2026-09-09] systems | Fix foliage-card depth coverage

Updated [[rendering-pipeline]] for `59018936` and `f39cf99b`. Single-texture M2 blend mode 1 now uses the authored alpha mask rather than alpha-to-coverage. Under 4× MSAA with depth/normal prepasses, the GPU regression changed an alpha-zero foreground sample from black (`[0, 0, 0, 255]`) to its green background (`[0, 254, 0, 255]`), retaining its opaque red sample. Camera-motion flicker from the reported world recording remains unproven.

## [2026-09-09] systems | Avoid unchanged nameplate visibility writes

Updated [[ui-system]] for `6edcdc91`. Nameplate and quest-indicator visibility still synchronizes from HUD toggles every `Update`; matching `Visibility` components are no longer marked changed. Focused tests cover unchanged components and HUD show/hide updates. No performance benefit is claimed.

## [2026-09-09] systems | Avoid unchanged UI quad component writes

Updated [[ui-system]] for `ui-toolkit` `0f5d81c` through `3eb9aa7`: ordinary and backdrop UI quads retain per-frame synchronization, but identical computed `Transform` and `Sprite` values are no longer reinserted. Real geometry, color, texture, clipping, and default-value changes still synchronize; `render_dirty` and rendering cadence are unchanged. CPU improvement remains unproven.

## [2026-09-08] systems | Replace visual doodad solidity with authored M2 triangles

Updated [[terrain]], [[collision-system]], [[scripted-movement]], and its spec for `003e2399` / `9b7e61f2`. Doodad AABBs are broadphase only; authored triangles decide solid hits, absent authored geometry has no solidity fallback, and visual bounds remain independently available to zone interactions. Scoped evidence is 45 targeted passing tests. Native displacement, rendering, final integration, and CPU conclusions remain pending.


## [2026-09-08] systems | Initialize InWorld IBL independently of skybox visuals

Updated [[skybox]] and [InWorld scene isolation](../specs/inworld-scene-isolation.md) with missing camera environment-light initialization, override/stage guards, and registered-system fixtures. Ambient brightness, exposure, shadows, fog, and authored shader combines remain unchanged; native brightness verification is separate.

## [2026-09-08] investigations | Record current CPU attribution limits

Updated [[movement-performance]] with the post-fix1,642-sample capture: diffuse leaf costs, incomplete caller stacks, no demonstrated dominant next fix. Preserved unresolved CPU objective; no speculative renderer or profiler changes.

## [2026-09-08] investigations | Record final-source shared-palette CPU non-result

Updated [[movement-performance]] with the paired final-source (`6e0c4fde`) stationary InWorld captures: unshared **255.935040%** versus shared **264.428382%** process CPU over 20 seconds. Sampled clocks (**2242.9** versus **2149.9 MHz**) and remote populations (unshared **119→120**, shared **119→119**) differ. The evidence demonstrates neither a full-client CPU gain nor a causal regression; CPU objective remains open.

## [2026-09-08] investigations | Separate billboard and terrain behavior proof from CPU gains

Updated [[animation]] and [[movement-performance]]: billboard raw/final double-write regression and staged final pose fix; terrain-time filtering withdrawn after higher native CPU. Authorized terrain application-throughput proxy rose without proven lower per-update cost. CPU objective remains open.

## [2026-09-08] systems | Record constant-curve folding proof

Updated [[animation]] and [[movement-performance]] with constant-track eligibility, catalog construction characterization, and 87-test verification at `50454f89`. Native clocks differed; no additional attributable CPU gain claimed.

## [2026-09-08] systems | Add NPC animation LOD and measure it against the propagation spin

Updated [[animation]] and added [npc-animation-lod](../specs/npc-animation-lod.md). Replicated NPC models now sample Bevy clips every frame within 30 yd, every other frame at 30–60 yd, and not at all beyond 60 yd or off screen. Recorded the A/B in [[movement-performance]]: non-spin CPU fell ~28% but the vendored `propagation_worker` spin absorbed most of it, so total CPU moved only 397% → 378%.

## [2026-09-08] investigations | Record transform invalidation CPU observations

Updated [[movement-performance]] and [event-driven application updates](../specs/event-driven-application-updates.md). `4cfe7bfb` removes a RED-proven constant-pose transform invalidation. Retained post-fix CPU observations are lower than pre-fix observations, but remotes and clocks differ and the repeat was not controlled. CPU goal remains open.

## [2026-09-08] investigations | Resume CPU goal with current baseline

Updated [[movement-performance]] and [event-driven application updates](../specs/event-driven-application-updates.md). Current `206f844f` InWorld captures are **306.86%** and **304.85%** one-core CPU over comparable 20-second intervals; Compute Task Pool workers account for **264.02%** and networking **3.60%** in the first capture. `586b619a` reproduced three transform-change notifications for a constant animated pose; `4cfe7bfb` eliminates that invalidation (**1/1** targeted GREEN). CPU improvement is still unmeasured, so the active CPU goal remains unresolved.

## [2026-09-07] systems | Consolidate execution and animation proof

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md); [[networking]] already records the native reconnect proof. `206f844f` makes deferred M2 binding retirement atomic: strict teardown RED 2 failures → GREEN 3/3, binding 11/11, and offline lifecycle 1/1 with fmt/check. Native `--screen m2debug` rendered `126487.m2`, reported it displayed, retained a screenshot, and recorded 25 changing bone positions. `--screenshot-regression` bypasses the custom animation plugin, so it is excluded. Native forced disconnect and ordinary reconnect are complete behavior proof. Pixel/GPU equivalence and performance were not measured and are not implied.

## [2026-09-07] systems | Reconcile network execution proof boundaries

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md). Worker-permit lifecycle/cadence (**3/3**), active cooldown (**3/3**), and native forced-disconnect → Login are complete proof. Remaining execution proof is native ordinary reconnect, real offline model spawn/despawn/reload, native visual equivalence, and controlled CPU/performance measurement. Historical pending statements remain historical.

## [2026-09-07] systems | Fix interrupted M2 crossfade continuity

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `aaec3864`, `a495893f`, and `5de8e843`. Bevy retains the last blended raw pose in evaluator commit before pivot correction and billboard rotation; an interruption blends that snapshot to the new sequence. The focused Bevy regression proves zero-elapsed and repeated-interruption continuity for translation, rotation, scale, and a nonzero pivot. The historic `a6a5d917` jump remains recorded. Pixel equivalence, GPU deformation, real offline-scene lifecycle, and performance remain open.

## [2026-09-07] systems | Prove native forced-disconnect lifecycle

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `1cce4171`. Authenticated client771292 matched the server netcode connection before an admin kick produced real `Disconnected`, InWorld → Login, visible `LoginRoot`, and zero links/replicas. Hidden semantic scene entries remain; this is not full cleanup or visual proof.

## [2026-09-07] systems | Record worker forced-disconnect handling

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `4d3c7ed6`. After recording `ForcedDisconnect`, the auth receiver requests the worker's actual Netcode client disconnect; its actual-worker RED/GREEN preserves the notice through the published disconnect lifecycle.

## [2026-09-07] systems | Bound interrupted Bevy crossfade proof

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `a6a5d917`. The test preserves legacy two-pose controller timing (`x0→4→16→20`) but does not prove full-pose/outgoing-weight continuity: a 40% A→B blend interrupted by B→C at 30% begins B→C with B weight 70%. Attachment and skinning proof remain open.

## [2026-09-07] systems | Record native replicated-equipment preservation

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `eaeaf9ed`. Native `bevy-animation/native-fixed/{before,head,cleared}.json` proves server-driven Head 1128 set/clear retained `humanmale_hd.m2` and restored the exact baseline appearance data. This is exported lifecycle-state evidence, not pixel-level appearance or animation-equivalence proof. At this point forced-disconnect worker handling was pending; CPU/FPS claims remain open.

## [2026-09-07] investigation | Record Bevy dynamic-link compile measurement

Added [[compile-latency]] for the corrected `dev = ["bevy/dynamic_linking"]` wiring in `8fca26b9` and the command documentation in `50991d70`. Same-literal real-edit samples fell from 21.173815 s default to 6.956593 s, then 4.566569 s after the dynamic cache warmed. Deleted-cache warmups and concurrent compile activity are excluded; the requested under-three-second edit-build target remains unmet. Repository history does not support a Windows rationale for the prior unwired direct `bevy_dylib` dependency.

## [2026-09-07] systems | Replace M2 pose loop with Bevy playback

Updated [[animation]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `6ed7129b`, `780baa1f`, `efe0cb26`, `8056741d`, and `12cced65`. M2 sequence policy, timing, and crossfade state remain in `M2AnimPlayer`; paused Bevy graph clips seek to those times. Bevy now evaluates/blends supported sequence-local raw TRS through custom curves, then commits existing `BonePivot` correction. The old per-model bone-application loop is removed. Graph bindings use existing joints and two nodes per sequence for independent outgoing/current times. Bevy evaluates in `PostUpdate` before transform propagation; this is not a separate 60 Hz animation worker. Focused integrated evidence is 67/67; native equivalence and CPU/FPS claims remain open.

## [2026-09-07] systems | Record literal clean-frame application exits

Updated [[networking]], [[sound]], [[ui-system]], and [event-driven application updates](../specs/event-driven-application-updates.md) for `dc6183f8`, `550b637a`, `9a6b6679`, `1c1d7998`, `ae222f0e`, `e9b81652`, and `fc99128b`. Clean render frames no longer run reconnect/reset lifecycle, sound maintenance, active cooldown advancement, local mount/tag/alive synchronization, addon watcher handling, or the combined spellbook/UI path. Rendering, interpolation, animation, and active presentation remain render-frame-driven. CPU/FPS improvement remains unclaimed pending controlled measurement and user observation.

## [2026-09-07] systems | Remove confirmed idle application frame work

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `1a8c6d58`, `cd47743e`, `c4d936b1`, `12cb981e`, `dc6183f8`, `550b637a`, and `9a6b6679`. Dedicated worker transport remains 60 Hz with negotiated 20 Hz simulation. Main receive/apply/send and reconnect/reset run on `NetworkTick`; equipment remains mutation-driven. UI sync/pointer, automation, addon application, local-player/mount synchronization, sound maintenance, and active cooldown progression now use change, request, relevance, playback, or active-cooldown triggers rather than their prior idle per-frame work. Rendering and remote interpolation remain frame-driven. Controlled CPU/FPS savings are unproved.

## [2026-09-07] systems | Record worker restart and native bridge evidence

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `cf517c34`. `worker-restart-tests.log` records 3/3 actual worker shutdown cases: old worker join, old sender closure, stale queue cleanup, and a second UDP handshake without a main-app update. `migrated-ui-reconnect-fixtures.log` records 11/11, resolving the previous fixture rerun. `worker-native-build.log` succeeded; `worker-native-inworld/` reached InWorld with mirrored player/NPC entities and routed Who result `Theron`, one result. The dark scene/white UI remains pre-existing invalid visual smoke; no clean-render, complete replication/equipment, or CPU claim follows.

## [2026-09-07] systems | Record independent UDP proof and explicit wire identity mappings

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `52508d1d`, `a5f6eab0`, `23ebb85b`, and `2d8b3c0f`. `independent-udp-handshake.log` records 1/1 real UDP handshake after `54411453`, without a main-app update. Target, emote, combat, duel, inspect, and current/default spell entity fields now use explicit main/server identity conversion; numeric spell selectors remain server IDs. Scoped proof now includes worker character-create transport responses 3/3, worker auth 23/23, and wire identity 19/19. `e9e7e034` repairs the binary fixture behind the prior 60/61 result, but its targeted rerun remains pending; no integrated lifecycle, native appearance, or CPU claim follows.

## [2026-09-07] systems | Record dedicated connection-owned network world boundary

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `aa6fda57`, `6eb52d96`, and `54411453`. Main now starts one separate 60 Hz network ECS world per connection; that world owns Lightyear transport, protocol, replication, and typed receive buffers while preserving the 20 Hz simulation. Main holds only connection proxy markers, typed application inboxes, and a server-to-render entity mirror. This is not completion: focused runtime evidence is 19/20 after a UDP replication run exposed Bevy B0002 from querying resources as `EntityRef`; `54411453` excludes those resources but awaits a fresh integrated run. Wire entity-bit boundary conversions, binary fixtures, reconnect, native appearance, CPU, and renderer limits remain open.

## [2026-09-07] systems | Record unintegrated owned-inbox network runtime foundations

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `d4742d87`, `80476abe`, `daa1a7b3`, `cec56837`, and `01cfcade`. Application handlers now use worker-backed `MessageSenders`/`MessageReceivers`; `network_events` dispatches application-owned `Inbox<M>` batches and no longer parks/restores main-world Lightyear receivers. `runtime-tests-transport.log` records worker/module 13/13, including encoded Lightyear loopback while the main app is unupdated; dispatcher RED then GREEN is recorded in `owned-inbox-red-behavior.log` and `owned-inbox-green.log` (8/8). The main binary still does not start the worker or transfer client, transport, replication, lifecycle, or reconnect ownership, so no runnable independent-network or CPU claim follows. Existing native and CPU limits remain unchanged.

## [2026-09-07] investigation | Record event-driven native follow-up

Updated [event-driven application updates](../specs/event-driven-application-updates.md) and [[movement-performance]]. Empty-stage login reached InWorld; its ten-second unfocused sample was 265.779% one-core CPU and 503.746 application updates/s. The earlier 293.279%/422.366 Empty sample used different clocks, so no causal CPU reduction is claimed. A full-world client logged in and completed Who/friends replies, but its visual smoke is invalid from repeated slab-allocator errors and a white/dark screenshot. The same error predates this work in `connected-warm2/client.log:149`; no equipment-event causality is established. Test clients stopped. Render-independent network-world execution remains open.

## [2026-09-07] systems | Record deferred application inbox dispatch

Updated [[networking]] and [event-driven application updates](../specs/event-driven-application-updates.md) for `2a8abacb`, `71d80355`, migrated API handlers, event-driven equipment, and character-create response ownership. Link/transport/message maintenance remains per-frame because transport timers consume frame delta. When no logical 60 Hz application tick is due, typed inboxes move out before `Last` clears them and return in `First`; handlers run only at logical ticks on the main thread. Collection/death and character-create responses now have one network consumer. Focused proof: network 8/8, APIs 55/55, equipment 13 plus 2 appearance tests, IPC FIFO 1/1, character-create 3/3. Connected lifecycle, independent transport/thread execution, native appearance delivery, and CPU improvement remain unproven.

## [2026-09-07] docs | Record event-driven network groundwork and IPC queue guard

Updated [[networking]], [event-driven application updates](../specs/event-driven-application-updates.md), and `index.md` for `1aec1091`, `fc82c5b7`, and `db7e6e7b`. `NetworkTick` is a 60 Hz logical schedule on the main ECS thread, not an independent OS networking thread; it leaves the negotiated 20 Hz simulation unchanged. The central dispatcher now routes auth and profession work over existing typed FIFO inboxes, and IPC skips dispatch parameter acquisition with no pending command. Broad application/API migration, integrated delivery/reconnect and appearance proof, idle-work measurement, and any CPU improvement remain open.

## [2026-09-07] docs | Retire forced strict-Empty pacing claims

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], [[networking]], [[ui-system]], [[replicated-unit-noops]], [[empty-window-baseline]], and `index.md`. `281d291a` removes `4fb2e5c9`'s forced 100 ms / 10 FPS Empty limiter: only the configured global frame-rate limit applies. Historical near-10-FPS readings, including 11.199% at approximately 9.994 updates/s, are capped data—not an uncapped baseline or a 97% CPU gain. `35e15d27` is build-backed successful startup after omitting Empty PBR/light/debug/material paths and target visuals; Empty is startup-only. Uncapped Green is pending.

## [2026-09-07] docs | Correct strict-Empty LightPlugin boundary

Updated [[rendering-pipeline]], [[empty-window-baseline]], [[movement-performance]], [[inworld-scene-isolation]], and `index.md` for `be9a1ff7`. The August 11, 2026 LightPlugin-absent Empty claim remains historical: native RED showed PBR requires `PointLightShadowMap` and scattering-medium assets even with no light entities. Current documentation records LightPlugin as a required provider and retains only the gizmo disables. Native GREEN is still running; no pass is claimed until `update-schedule-isolation/strict-empty-fixed/` exists and the main session confirms it.

## [2026-09-07] diagnostic | Record coarse idle-CPU isolation and repaired caller stacks

Updated [[empty-window-baseline]] and [[movement-performance]] with destructive reduced-scene schedule/render-set cutoffs. `Update`, `PostUpdate`, and the combined main schedules are material contributors, while `Prepare`'s apparent CPU drop is invalid because it halves update cadence; `Specialize` is negative and `Queue` is pending. A literal Empty-stage launch failed before measurement from missing `PointLightShadowMap` and scattering-medium initialization. Recovered Deep-DWARF ancestry now covers all 2,138 archived reduced-game samples in `flamegraphs/reduced-game-dwarf.svg`; the prior raw flamegraph's corrupt caller frames remain historical only. Bevy 0.19 scheduling/material hazards and upstream #24448 are candidates or external context, not local causal findings.

## [2026-09-07] retirement | Remove unproven task-submission batching

Retired `079f0cbf`'s temporary Bevy task-submission patch and restored stock `bevy_ecs`/`bevy_tasks` sources and the root lockfile. Four same-binary, 10-second blank-renderer samples kept CPU high: baseline 207.679% and 215.581% one core; batched 220.981% and 203.752%. Nearby update rates and clock/host conditions varied, so the lower batched CPU/update estimates do not establish a causal gain. No deployment or retirement rebuild occurred. The [spec](../specs/task-submission-batching.md) is historical. The active question is why disabled subsystem registrations, including sky-material paths, still execute and cost CPU; this result does not make their callback volume normal or explain bulk CPU.

## [2026-09-07] experiment | Batch independent Bevy system-task registration

Documented the user-approved, reversible `bc827e0f`/`2a818246`/`9b68de16`/`876fa5db` task-submission experiment. Unchanged Bevy 0.19.0 `bevy_ecs` and `bevy_tasks` are locally patched without package-version or root-feature changes. `Scope::spawn_many` retains each future's existing panic/result handling while bulk-registering independent futures. The ECS executor buffers at most 32 ready Send-system indexes only after its existing access and condition checks; it retains per-system completion, and leaves non-Send/exclusive execution unchanged. `BEVY_ECS_BATCH_TASK_SUBMISSIONS=1` enables batching; unset/`0` retain baseline and invalid values fail explicitly, with an executor-local override. This changes registration calls, not system count or body parallelism, and targets active-task registration locking rather than all task/memory/render cost. Scoped-task tests pass 3/3; ECS behavioral tests and bounded engine CPU/update/throughput comparison remain pending. No build, runtime, focus, capture, or profiler test occurred for this documentation update.

## [2026-09-07] diagnostic | Resolve libc memory operations in preserved native profile

Updated [[empty-window-baseline]] with matching-library offline attribution. libc covers 1,413,726,253 sampled cycle period (12.1680% of the original total); Arch debuginfod symbols resolve `memcpy` at 2.4163%, `memset` at 1.1201%, `_int_free_chunk` at 1.7458%, and `_int_malloc` at 1.3708%. These are sampled-period shares, not CPU percentage points or caller attribution. Allocation/copy work cannot be assigned to task lifecycle or a render callback without runtime ancestry. The 11 MB libc debug artifact is retained under diagnostics; independent verification remains pending. No build, capture, runtime, focus, or profiler test occurred.

## [2026-09-07] diagnostic | Attribute decoded continuous-renderer instructions

Updated [[empty-window-baseline]] and linked [[movement-performance]] to offline attribution from preserved matching binaries. Blank continuous rendering resolves 78.61% of sampled cycle period across 1,089 executable addresses: `bevy_ecs` 19.6184%, `concurrent-queue` 15.7290%, `async-executor` 6.7951%, and `async-task` 5.5316%. Hottest sites are queue empty probing, executor locking, completion publication, and mutex spin. This identifies task lifecycle/synchronization operations, not upstream callbacks, waste, or a safe removal. Full-scene proportions use a different sample-count denominator and a reduced `no-npcs-ui`/terrain-isolated workload, so no causal comparison follows. No profiler test, build, capture, runtime, focus, or process action occurred.

## [2026-09-07] diagnostic | Bound local profiler entry overhead

Updated [[empty-window-baseline]] from the retained single-thread CPU benchmark. Across three alternating disabled/enabled rounds of 50,000 precreated equal-work span entries, median added CPU was 2.202289 µs/entry at 154 names and 2.355397 µs/entry at 1,031 names. `93cef709` relocated the test-only benchmark without behavioral change to `src/cpu_system_profile/overhead_benchmark.rs`; independent verification passed formatting and both changed Rust files' readability audit without rerunning the benchmark. This is not a whole-app bound, subtraction, or scheduling-causation result.

## [2026-09-07] investigation | Clarify update-rate normalization

Updated [[movement-performance]] with source-backed metric provenance: game IPC/overlay FPS and blank-renderer logging both measure app-update cadence, with different smoothing. Neither proves presentation cadence. Compare exact per-capture update counts before interpreting CPU-per-update ratios.

## [2026-09-07] diagnostic | Isolate blank-renderer dirty-tree callback

Updated [[empty-window-baseline]] and [[movement-performance]] for the single-selector continuous-window extension and native before/after experiment. The removed callback and its worker spans disappear, but bulk CPU remains. Clock variation prevents an efficiency claim; the source investigation remains open.

## [2026-09-07] diagnostic | Add dirty-tree worker CPU spans

Updated [[empty-window-baseline]] for `5712ae71` and `55a7abe9`. The first behavioral test proved the existing Bevy `producer_mark_dirty`, `consumer_mark_dirty`, and `par_traversal_mark_dirty` spans were excluded; the retained selector now exports them with positive CPU. This records worker-span entries, including future polls and final drop, independently, does not charge them to `mark_dirty_trees` or count distinct tasks. Bevy creates the consumer/traversal workers before scanning changed transforms, a source candidate only. `d37278bb` enables the omitted transform tracing feature; native worker output is now recorded in the investigation, without a bulk root-cause claim.

## [2026-09-07] diagnostic | Blank renderer named-system profiler output

Updated [[empty-window-baseline]] after `29c3251d` and `23bdab77`. The pre-integration `7aa4` RED logged updates for 46 seconds with `WOO_CPU_PROFILE_OUTPUT` but exported no profile. The native GREEN exported 1,910 positive named system/thread spans. A retained ten-second, 11/11-focused pair on the same `5a36…` feature binary/default pools/1280×989 window measured 236.080% unprofiled versus 241.581% profiled process CPU and 1404.792 versus 1067.050 logged main updates/s. Clock differences and 263-second versus eight-second launch age mean it is not an isolated overhead estimate or an FPS/optimization comparison; a 0/13-focused pair is discarded. The five-second selected-span report has 12.487799 observed CPU-seconds (not whole-process CPU), 6.708256 selected self CPU-seconds, 3.055642 named-system self CPU-seconds, and 5.779543 seconds outside selected spans. `submit_pending_command_buffers` is largest at 390.551 ms/5,411 calls; no dominant callback or bulk root cause follows. Independent audit 268 confirmed the arithmetic and rejected a controlled-overhead interpretation.

## [2026-09-07] measurement | Continuous blank renderer is the first major CPU rise

Updated [[empty-window-baseline]], the staged service spec, and the index for source `76076850` and renderer binary `d847c8c5568a549b976972f8cdffc8e2cf8e1d99ec1cbfccb7a499bfe9cfa56d`. Twelve-second, 13/13-focused 1280×1198 samples measured native 0 ticks, Bevy core 0.16664% one core, reactive blank GPU 0.08331%, and continuous blank GPU 216.31478%. Reactive telemetry was 0.2 updates/s; continuous mean was 1212.906 updates/s (1071.767–1306.683 logged range), not presented FPS. `continuous` changes only Winit policy; renderer, camera, Mailbox, default pools, pipelining and no FPS limiter remain fixed. This locates the first major rise in continuous framework updates before project services, not a full-game root cause or an optimization. Independent runtime audit 248 passes this bounded attribution. Raw GPU metrics captured average graphics activity 0–1 reactive versus 80–86 continuous and average GPU clocks 618–682 versus 1945–2326 MHz; there is still no presented FPS, frame pacing, render-thread/per-thread CPU attribution, or hardware-counter proof. Continuous PID 1217890/window 331 remains open.

## [2026-09-07] diagnostic | Blank renderer smoke requires Bevy accessibility resource

Updated [[empty-window-baseline]], its staged service spec, and the index for `0669eac5` and `76076850`. `--service-window <core|render|continuous>` has no implicit stage: `render` is a reactive blank `Camera2d` GPU layer and `continuous` changes only the Winit update policy. The initial render smoke initialized the Vulkan adapter but failed before window creation because Winit required `AccessibilityRequested`; `AccessibilityPlugin` supplies that resource. Input and mesh remain required Bevy dependencies for Winit focus processing and render mesh extraction. No CPU, update-rate, or throughput result is claimed from the failed smoke; rebuild and visible-window proof precede measurement.

## [2026-09-07] measurement | Bevy core services have no bulk idle CPU source

Updated [[empty-window-baseline]] and its service spec with the source `217b4de8` + `6f6e10e3` focused comparison. Same binary SHA-256 `822ebfde5fa13dd13c9489a8bd519e41be91fb746954eea5a19c894b4c45fd25` and 1280×1198 native loop: core services measured **2 ticks/12 seconds** (**0.16664% of one core**), 25 threads and 13/13 focused; native measured 0 ticks, one thread and 13/13 focused. Host 5.407% versus 5.799% is not attributable to the single process. An unfocused closed-lid native capture is excluded. No renderer, continuous-frame FPS comparison, or full-game CPU conclusion follows. Source verification passed fmt/check/readability and 6/6 targeted tests; routing RED was valid, but the core module test has no demonstrated pre-implementation RED. Runtime data audit remains pending.

## [2026-09-07] documentation | Record additive Bevy-core service-window stage

Updated [[empty-window-baseline]], its specs, and the index for `217b4de8` and `6f6e10e3`. `--service-window` reuses the native softbuffer `Wait` loop, adds `MinimalPlugins` without `ScheduleRunnerPlugin`, and calls `App::update()` at `about_to_wait`. It excludes renderer, assets, game services, networking, IPC, sound, UI, timers, and continuous redraws. Native zero-work proof remains canonical on [[empty-window-baseline]]; service-stage runtime CPU measurement is pending.

## [2026-09-07] measurement | Empty window waits with no measurable idle CPU

Verified [[empty-window-baseline]]: two 12-second idle samples recorded zero CPU ticks, one sleeping thread in `do_epoll_wait`, and no game IPC/GPU resources. Each sample is below approximately 0.0833% of one core at accounting resolution. Native resizing and closing worked; the window was reopened for observation. Routing tests, formatting, checking, readability, dependency-version review, and independent native-data audit passed. This establishes the requested zero-work baseline, not a full-game optimization.

## [2026-09-07] documentation | Record empty native Wayland baseline

Added [[empty-window-baseline]] and linked it from the index. `c3568e59` routes the exclusive `--empty-window` mode before normal startup, so it creates no Bevy app, plugins, assets, networking, IPC, renderer plugins, or game task pools. Winit waits for OS events; softbuffer paints a flat background only for requested redraws and resize-triggered redraws. Linux painting is Wayland-only. `softbuffer` is the sole new dependency; existing locked versions remain unchanged. Native results are recorded in the subsequent measurement entry; the zero-work baseline cannot establish an FPS-comparable game optimization.

## [2026-09-07] measurement | Material group negative; native ownership mapped

Updated [[movement-performance]] with cumulative visibility/tree/material-preparation measurements and the explicitly approved 25-callback material group. With 11 removals retained, the group changed CPU 332.55→340.89% and FPS 231.63→240.90; no CPU benefit. Corrected native CU attribution maps 514/2,134 user-mode samples to application units, mostly generated/helper symbols rather than directly named game functions. ELF file offsets require PT_LOAD conversion; the unadjusted result is invalid. Independent audits passed. Bulk CPU cause remains unresolved; compiler/worker/FPS policy unchanged.

## [2026-09-06] measurement | Cumulative exclusions retain bulk CPU

Updated [[movement-performance]] with sender-only and cumulative results. Sender removal retained connection state but did not reduce CPU. All six retained exclusions still measured 330.30% CPU / 214.74 FPS in a focused six-second tail, then 331.80% / 234.68 FPS in a separately recreated, fully focused 12-second state. Mixed-focus and firmware-clamped phases are excluded from savings claims. Independent audits passed; diagnostic clients stopped.

## [2026-09-06] diagnostic | Add exact application-message sender isolation

Documented `dc922265` in [[movement-performance]]. `--freeze-message-send-after <SECONDS>` resolves only the uniquely named PostUpdate `MessagePlugin::send` callback and its one-member implicit set; receive, transport, rendering, and other send-group work remain active. The five tests cover exact removal, queued-message retention, unrelated work, ambiguity, defaults, and argument parsing. A worker stack proves the sender loop executes, not that it sent a message or explains CPU. Connected-idle runtime validation is still required; no CPU/FPS result or optimization claim follows.

## [2026-09-06] documentation | Canonicalize timed callback removal

Updated [[movement-performance]], the InWorld isolation spec, and `index.md` for `52e44eff`. The current interface is repeatable `--remove-system-after <main:SCHEDULE|render:SCHEDULE> <EXACT_SYSTEM_NAME> <SECONDS>`; it resolves one exact schedule/name and a one-member implicit set, removes with `RemoveSystemsOnly`, uses Extract as the safe barrier, and delays `render:ExtractSchedule` removal to Render cleanup. Added a six-row migration table. The retired `--freeze-*-after` flags have no aliases; dated entries retain their former syntax as historical evidence.

## [2026-09-06] measurement | Mesh-collection removal lacks comparable-throughput benefit

Updated [[movement-performance]] with the main-owned `78b3a63c` pair. The exact callback was removed at 30.004 seconds: all-focused windows measured **330.55% CPU / 200.17 FPS** before and **326.55% / 179.20 FPS** after, with a post-removal 600 MHz GPU limit. A later **310.70% / 103.89 FPS** window is excluded for collapsed throughput. The earlier agent-created pair corroborates but does not replace this evidence. `9271c885` behavioral proof passed 7/7; `78b3a63c` retained the established log prefix and removed test-only schedule synthesis. No CPU benefit.

## [2026-09-06] measurement | Camera-follow removal leaves bulk CPU unchanged

Updated [[movement-performance]] with the verified `a9de6b17` same-process pair: 338.80→338.39% CPU and 210.41→214.54 FPS after removal at 30.004 seconds. Focus and reported camera/player positions remained unchanged; rotation was not exposed. Five behavioral tests, formatting, locked checking, and readability passed. No material CPU reduction.

## [2026-09-06] diagnostic | Add timed camera-follow isolation

Documented `a9de6b17` in [[movement-performance]]. `--freeze-camera-follow-after <SECONDS>` removes only the registered `camera_follow` callback from `Update` at its `Last`-schedule cutoff, retaining the current camera transform while leaving camera input, player movement, graphics synchronization, rendering, normal pipelining, and unrelated systems active. Behavioral proof passed 5/5. This is stationary-only attribution infrastructure; no CPU/FPS result or optimization claim exists yet.

## [2026-09-06] clarification | Bound CPU-profile coverage claims

Clarified [[movement-performance]] and JSON metadata: observed CPU is bracketed by each thread's first/last captured selected-span clocks, not exact five-second CPU. Residuals apply only inside those intervals; unobserved threads and boundary time are absent. The crossing counter records late exits, not spans already active at capture start. Accounting arithmetic is unchanged.

## [2026-09-06] measurement | Trace upload exclusions with thread CPU clocks

Updated [[movement-performance]] with three `5f3fb679` feature-build captures: both upload writers active, indirect removed, then batched also removed. Exact callback spans disappear, but worker residual remains 5.828 CPU-seconds with both absent. Differing frame counts and instrumentation prevent a normal-build optimization claim. Recovered native flat leaf samples using `perf script -G` and the matching saved binary; full ancestry remains unresolved. Default build restored; profiling clients stopped.

## [2026-09-06] measurement | Record first named-span thread-CPU capture

Updated [[movement-performance]] from feature-build `36d1d994` PID `3611636`. Within the five-second capture, first-to-last selected-span observations bracketed **18.122 s** thread CPU and **11.061 s** summed self CPU while overlapping focused telemetry measured **358.55%** process CPU and **161.64 FPS**. The **7.061 s** residual includes uninstrumented worker-task work and profiler overhead; it is not native CPU ownership. Nine late exits were counted, none on Compute Task Pool workers; spans already active at capture start are excluded without being counted. `5f3fb679` subsequently shares span labels and adds concurrent-same-span and blocked-sleep tests, so future captures have lower label-allocation overhead and are distinct. No root cause, normal-build comparison, or optimization claim follows.

## [2026-09-06] diagnostic | Add named-span thread-CPU profiler

Documented `36d1d994` in [[movement-performance]] and the InWorld isolation spec. The diagnostic-only `cpu-system-profile` feature installs its tracing layer only when `WOO_CPU_PROFILE_OUTPUT` is set. It captures selected named system/schedule/executor spans from seconds 10–15 and exports aggregate per-thread CPU-clock JSON after a one-second drain at approximately second 16. It reports inclusive/self CPU, first-to-last selected-observation thread CPU, and late-exit counts; see [[movement-performance]] for boundary and coverage limits. `bevy/trace` enables optional tracing-related transitive dependencies only in this feature build. First runtime results are recorded in the later capture entry; no cause or optimization claim follows.

## [2026-09-06] measurement | Isolate pipelined-rendering CPU contribution

Updated [[movement-performance]] with sequential upload removals, the `09e77aa1` same-extraction schedule-registry repair, and the pipelining comparison. Upload removals did not eliminate the bulk CPU load. Omitting only `PipelinedRenderingPlugin` reduced process CPU **320.72→208.15%** while FPS fell **194.86→146.81**; render frames and readable FPS remained. Earlier upload exclusions were restored individually, leaving ordinary uploads active. This is a measured CPU/throughput trade-off, not a default optimization or resolution of firmware clamps. Final source/data audit passed formatting, locked checks for both binaries, readability, and reused 5/5 upload plus 2/2 pipeline tests. A later all-uploads-active observation measured219.23%CPU/142.54FPS with recoveredCPUlimits, so the lower-CPU diagnostic mode does not require frozen upload data. Details and limitations remain in [[movement-performance]].

## [2026-09-06] diagnostic | Record GPU-cluster preparation isolation

Updated [[movement-performance]] and the InWorld isolation spec for `7d8baeb9`. A roughly five-second instrumented trace reported about **128 ms** across roughly **552** `prepare_clusters_for_gpu_clustering` calls; spans are overlapping traced wall time, not exclusive CPU cost or callback attribution. The new stationary-only cutoff resolves the exact private callback to one implicit system set and asserts removal of exactly one system. Same-process PID `3445983` logged removal at **60.006 s**. `before/` and `before-second/` were both pre-removal (**328.89% / 193.37 FPS**, **336.47% / 214.99 FPS**). Post-removal `after/` (**326.38% / 137.43 FPS**) and `after-later/` (**316.04% / 54.87 FPS**) reached 600 MHz CPU/GPU limits. This is not a comparable-FPS result and establishes no CPU reduction or causality. Artifacts: `settled-low-fps/cpu-system-isolation/gpu-clusters/`.

## [2026-09-06] measurement | Remove indirect-parameter uploads

Updated [[movement-performance]] from the first actual CPU-system removal. Same-process PID `2428304` at `c3ad0ccf` removed only Bevy Render's `write_indirect_parameters_buffers` after 20 seconds while preserving position/view and all 13 focused samples per side. Process CPU **increased** from **321.63%** to **330.97%**; this target did not reduce CPU usage. FPS changed **134.47→197.57**, but CPU/GPU limit ranges differed, so no pure FPS conclusion follows. Before/after screenshots preserve the empty view and readable overlay. The stationary-scene control can affect downstream render behavior; it is diagnostic, not an optimization. The next distinct target, `write_batched_instance_buffers<MeshPipeline>`, remains pending separate proof.

## [2026-09-06] diagnostic | Add timed indirect-parameter upload isolation

Updated [[movement-performance]] for `c3ad0ccf`. The next CPU baseline recorded **314.38%** process CPU across 13 focused samples: named Compute Task Pool workers accounted for **234.14%**, and game-engine threads **80.08%**; terrain, water, and M2-effect materials were all absent. Opt-in `--freeze-indirect-parameters-after <SECONDS>` removes only Bevy Render's typed `write_indirect_parameters_buffers` system at a `Time<Real>` deadline using `RemoveSystemsOnly`; allocated buffers and other render callbacks remain. The rejected implicit-type-set approach was abandoned. Corrected behavioral RED proved the target still ran 3 rather than 2 times; GREEN, build, runtime, and independent verification remain pending. Stationary-scene only: frozen indirect metadata can affect downstream render work, so no CPU-drop or efficiency claim is made.

## [2026-09-06] measurement | Preserve directional-shadow FPS collapse

Updated [[movement-performance]] from the interrupted `ba6b756a` directional-shadow run. The planned ON/OFF series stopped after 60 shadows-on samples and 27 shadows-off samples when the user observed foreground FPS fall from 170–200 to about 40. This is not a completed A/B and provides no shadow benefit claim. The retained shadow-off PID `2004379` immediately measured 12 focused samples at **43.92–58.24 FPS** with both enforced CPU/GPU limits at **600 MHz** in every row, **309.04%** process CPU, **43.5%** mean GPU activity, and +11,006 raw core/GFX thermal-residency counters (units unspecified). A 192.33–202.57 FPS CPU profile ran after recovery and is not slow-state attribution. User clarified FPS settles in under 15 seconds; future relaunches use 10-second settling, distinct from measurement. The cancelled detach capture's missing log is not application-stall evidence.

## [2026-09-06] diagnostic | Add directional-shadow isolation control

Updated [[movement-performance]] for `ba6b756a`. Opt-in `--no-directional-shadows` inserts a startup resource consumed only by InWorld `spawn_world_environment`, which sets the spawned directional light's `shadow_maps_enabled` false and logs the override. The light, illuminance, transform, ambient lighting, cascade configuration, 4096-pixel shadow-map resource, camera effects, and general lighting remain; standalone/other scene setup paths are unchanged. Saved focused RED/GREEN is **3/3**. No build, relaunch, measurement, or independent final verification has occurred, so no FPS benefit is claimed.

## [2026-09-06] repair | Synchronize UI and world camera MSAA

Updated [[movement-performance]] for `256bd37d`. The original `--no-msaa` path left the non-clearing UI camera at Bevy-default 4× MSAA while the world camera became single-sample, splitting their intermediate targets and accumulating dynamic FPS glyphs. `sync_ui_camera_msaa` now runs after 3D graphics synchronization and copies the actual active 3D sample count to the UI camera without changing its clear/order behavior or adding UI effects. RED `514259b9` reproduced UI `Sample4` where `Off` was required; saved GREEN passed **11/11** and the game-engine build exited 0, both with the existing `binrw v0.15.1` future-incompatibility notice. Fixed live `--no-msaa` PID `1875826` produced three readable changing FPS values in both Niri-native and IPC captures, ten seconds apart. This repairs the display regression; it does not turn earlier mixed-camera MSAA timing into pure sampling-cost evidence.

## [2026-09-06] measurement | Record repeated MSAA ON/OFF comparison

Updated [[movement-performance]] and `index.md` from `settled-low-fps/no-msaa-repeat/`. The closed-lid, 1280×1198 series used fixed position/camera/options and four 60-sample phases. Excluding the first MSAA-on phase because only 29 samples were focused, the focused sequence was MSAA-off **196.19 FPS**, MSAA-on **173.45 FPS**, and MSAA-off **204.75 FPS**. The adjacent ON/OFF phases had near-equal CPU use (**339.29%/339.55%**) and similar mean GPU clocks (**1,947.13/1,920.32 MHz**), with lower MSAA-off GPU activity (**74.07%/65.65%**). This records a repeated diagnostic-selector throughput difference, not a pure MSAA sampling cost: the later glyph investigation found the selector also changed world/UI intermediate-target composition. It does not show CPU reduction, collapse remediation, or an exact causal/attributable percentage. Independent saved-data audit confirmed the phase 3/4 +18.0% FPS and 5.818→4.951 ms direction, but only 26 minimum-count rows overlap across six joint-limit bins and actual graphics-clock deltas span -431.6 to +279.0 MHz. Screenshot rendering is visually inconsistent across modes; no graph-state inference is made. The artifact base retains its September 5 directory name; measurements are September 6. Source remains `56258e5f`.

## [2026-09-06] measurement | Record corrected UI/world MSAA repeat

Updated [[movement-performance]] from `settled-low-fps/corrected-msaa-repeat/` after `256bd37d` aligned UI and world camera sample counts. All four 60-sample phases were focused with matching saved view, 1280×1198 geometry, display configuration, options, and closed-lid state: MSAA-on/off/on/off means were **133.88 / 166.28 / 113.85 / 184.68 FPS**, while process CPU stayed **325.96% / 327.67% / 326.29% / 326.12%**. The corrected series removes the stale-UI target from the comparison, but both on phases reached CPU/GPU 600 MHz limits and three coarse joint-limit bins have only 29 minimum-count overlaps with differing actual clock distributions. Verifier 137 independently passed artifact integrity, matching conditions, fixed-overlay screenshots, and repeated MSAA-off direction. It supports that direction, not an exact MSAA percentage, CPU-work-per-frame conclusion, or collapse fix; earlier `no-msaa-repeat` remains non-pure due to its world/UI sampling mismatch.

## [2026-09-06] investigation | Record MSAA-off FPS-overlay corruption hypothesis

Updated [[movement-performance]] from compositor-native and IPC captures of live MSAA-off PID `1553387`: changing numeric FPS glyphs accumulate while `FPS:` remains clean and graph bars update. The defect is displayed-surface rendering, not screenshot encoding. Source shows `--no-msaa` changes only the world `Camera3d`; the later non-clearing UI camera remains Bevy-default 4× MSAA, yielding separately keyed intermediate targets. This is a stale UI-composition hypothesis, not a confirmed root cause or authorized fix. The minimal proposed UI-camera MSAA synchronization remains pending approval and behavioral/live proof.

## [2026-09-06] audit | Record terrain-off, frame-graph, and MSAA isolation through `56258e5f`

Updated [[movement-performance]] and `index.md` from saved `settled-low-fps` artifacts. Recorded terrain-rendering-off as a logical terrain/height-state control, its 120-second focused mean (**98.25 FPS**, **311.67%** CPU), and its two 600 MHz-limited lows; no CPU reduction or terrain-mesh attribution is claimed. Recorded the lid-open matched graph pair (**180.47** versus **179.87 FPS**; **333.60%** versus **338.89%** CPU), which shows no material graph-cost improvement. Recorded the MSAA-only pair (**106.14** versus **180.68 FPS**) as inconclusive because the samples ran under different firmware-limit regimes and MSAA-off CPU was higher (**337.33%** versus **317.76%**). Graph was restored for both MSAA commands. Verifier 115 saved `cargo fmt --check` and locked binary check exits 0 for `56258e5f`; `binrw v0.15.1` retains its existing future-incompatibility notice, and live component readback was not independently verified. The original tile hitch remains unresolved; nameplate implementation remains queued.

## [2026-09-05] investigation | Foreground FPS collapse matches firmware frequency clamps

Updated [[movement-performance]] with the flat-material diagnostic, persistent slow periods, verified GPU execution, and a foreground transition to 600 MHz CPU/GPU firmware limits with advancing thermal-throttle counters. Cooling/platform-policy cause and original tile hitch remain unresolved; no hardware controls or optimization changed.

## [2026-09-05] experiment | Reduced-scene material freeze does not sustain gain

Updated [[movement-performance]] with conflicting unchanged-code freeze results: 47.53 mean FPS before, one80.95 sample then21–27 after, mean31.81. Same position/assets/view; CPU and GPU busy counters increased. Normal updates restored, later48–55FPS. Earlier material-freeze gain is not a general resolution; frequency/thread/render-state attribution remains open.

## [2026-09-05] diagnostic | Water and skybox visual isolation

Updated [[movement-performance]] with opt-in streamed-water and skybox-visual exclusion, zero water assets in the new client, and 46.65 mean FPS. Recorded retained lighting/camera paths and position/focus differences that prevent a controlled speed comparison. Terrain-material updates remain the stronger prior measured lead; no optimization applied.

## [2026-09-05] measurement | Terrain-material freeze and object-free isolation

Updated [[movement-performance]] with the main-controlled 256-material freeze (19.87→67.80 mean FPS), normal-update restoration, and requested object-free terrain run (user ~60 FPS; IPC mean54.41 with mixed focus). Recorded zero object colliders, reduced asset counts, close-up-view limitations, and unresolved attribution. No production optimization or tile-hitch resolution claimed.

## [2026-09-05] diagnostic | Add timed terrain-material freeze selector

Updated [[movement-performance]] for `4ab3deb6` (`Add timed terrain material freeze diagnostic`). Optional `--freeze-terrain-materials-after <SECONDS>` keeps normal startup updates, then gates terrain animation-time and environment-map synchronization together using `Time<Real>` while retaining loaded terrain material values and rendering. This isolates `Assets::iter_mut` modification churn; it intentionally freezes animated terrain and later environment-map changes. Focused tests **3/3** and build passed; no runtime comparison or performance claim yet.

## [2026-09-05] diagnostic | Add exact NPC and UI isolation selector

Updated [[movement-performance]] for `3d364dc8` (`Add NPC and UI isolation without changing scene lighting`). `--inworld-stage no-npcs-ui` preserves terrain, lighting, particles, local-character policy, networking, and FPS overlay while excluding remote visuals plus game UI/nameplates; selector and nameplate-observer behavior have focused coverage. PID `3510050` did not survive its Pyrun launch, so no runtime FPS result is claimed.

## [2026-09-05] investigation | Separate sustained-FPS report from tile stall

Updated [[movement-performance]] with the user-reported approximately 10-FPS settled condition, the non-reproducing 30–45 FPS full-scene IPC samples, and the confounded 56–76 FPS terrain-stage isolation. The terrain selector also excludes lighting and particles, so it is not subsystem attribution. No runtime selector result, optimization, or tile-stall closure is claimed.

## [2026-09-05] experiment | Target BLP residency does not reproduce the stall

Updated [[movement-performance]] with a per-file cache-advice control, verified residency changes, and bracketing CPU/fault observations. Target BLP residency alone did not reproduce the original delay. The original stall remains open in `PLAN.md`; no optimisation, global kernel-setting change, or asset-content change was made.

## [2026-09-05] measurement | Split repeat tile application into stages

Updated [[movement-performance]] with gated timing output from `7d9e7370`: 316.465 ms total application, including 159.696 ms doodads, 132.490 ms WMOs, and 22.125 ms terrain water. Nested BLP timings record 110.015 ms alpha normalization across 156 loads. This identifies repeat CPU subcosts, not the cause of the original additional delay; no optimization was implemented.

## [2026-09-05] diagnostic | Correct cache characterization and split tile timing

Corrected [[movement-performance]]: FDID-named root `777827.adt` predated the experiment; the resolver only selects existing local roots. Earlier cold-cache/CASC-supply wording was unsupported. Repeat application intervals were approximately 350 ms, but the original 2.96-second application gap remains valid. Added gated BLP and tile-stage diagnostics (`d4eaf8cf`, `7d9e7370`) for caller-independent timing; no optimization or collision change.

## [2026-09-05] measurement | Reproduce tile-crossing stall

Updated [[movement-performance]] with a normal timed segment crossing `(32,48)` to `(31,48)`. The initial cache characterization was later corrected: the FDID-named root already existed; see the diagnostic entry and investigation. The old tile unloaded at +0.94 s, background parsing finished at +5.67 s, and spawn statistics appeared at +8.73 s; one IPC request waited 3,124 ms across the main-thread tile-application interval. This establishes a boundary-associated stall, not a fix or per-subsystem timing attribution. Client remained connected with the new tile loaded; LOD swaps remain unmeasured.

## [2026-09-05] measurement | Verify autonomous route and separate movement cost

Added [[movement-performance]] and reconciled [[scripted-movement]], its spec, and the earlier investigation. Ten timed segments completed 140 yards of travel with normal collision; matched focused samples averaged 26.58 FPS idle and 26.97 moving. A separate 1,379-sample profile identified transform parent propagation at 20.45% self cost. Documented the canopy bounding-box trap and startup cache recovery separately. Streaming/LOD boundaries remain unmeasured; no general world-performance fix claimed.

## [2026-09-05] feature | Record scripted movement IPC controls

Updated [[scripted-movement]] and `docs/specs/scripted-movement.md` for `51456222` (`Add scripted movement IPC controls`). The CLI now exposes `movement forward --seconds N [--yaw-degrees D]` and `movement stop`; IPC carries `ScriptedMovementForward { duration_secs, heading_degrees }` and `ScriptedMovementStop`. Focused proof: CLI **3 passed**, IPC **4 passed**, playback validation **3 passed**, and camera integration **6 passed**. Connected-runtime displacement and a valid moving-frame comparison remain open.

## [2026-09-05] docs | Record bounded scripted movement

Added [[scripted-movement]], updated [[procedural-cloud-regeneration]], and `index.md` for `15928f4e`, `d3f525ac`, and `6c9d82a3`. Timed forward segments validate duration/heading, clip their final frame, and run through ordinary player movement/collision/networking rather than teleporting. Waypoint attempts at the current spawn did not move the player, so no valid moving-frame comparison exists; connected IPC/CLI displacement proof remains required.

## [2026-09-05] fix | Record replicated M2 cache reuse

Updated [[procedural-cloud-regeneration]] and `index.md` for `484586ac` (`Reuse parsed models for replicated spawns`) and `dfb29983` (`Test cached NPC model spawning`). Shared replicated spawn helpers now reuse the existing model cache keyed by path, skin FileDataIDs, and zero-opacity mode while preserving filtering and joint binding. A concrete model/skin/skeleton regression removes the M2 after the first spawn and proves the second independent-root spawn retains identical vertices and indices; RED `/tmp/claude/npc-cache-red-corrected.log`, GREEN `/tmp/claude/npc-cache-green.log` (**1 passed**). No startup, FPS, connection-stability, or movement-performance effect is claimed before a fresh runtime measurement.

## [2026-09-05] investigation | Record startup parser blocker before movement measurement

Updated [[procedural-cloud-regeneration]] and `index.md`. A current-source client could not reach a controlled movement workload: synchronous replicated-NPC visual spawning used `load_m2_uncached` on the main thread. A five-second profile captured 241 samples with zero lost; `read_i16` held 16.60% self cost. A live stack traced `spawn_replicated_npc` through uncached M2 skeleton/animation parsing. The FPS overlay was enabled but retained its empty numeric span (`FPS:`) while IPC timed out. This records a startup blocker only—no movement root cause, fix, or performance claim.

## [2026-08-11] docs | Record movement performance probe

Updated [[procedural-cloud-regeneration]] for `53a9f66a` (`Add gated movement performance probe`). Recorded `WOO_PERF_MOVEMENT` activation, once-per-second movement/pathing/collision timing and collider-count output, and the absence of any performance conclusion before controlled runtime measurement.

## [2026-08-11] docs | Record Bevy 0.19 strict-Empty LightPlugin boundary

Updated [[rendering-pipeline]], [[inworld-scene-isolation]], and `index.md` for `17c2bdc6` (`Preserve strict Empty gizmo isolation on Bevy 0.19`). Bevy 0.19 `LightPlugin` registers nested `LightGizmoPlugin` resources, so exact configured `InWorldSceneStage::Empty` now disables `LightPlugin` as well as `GizmoPlugin` and `GizmoRenderPlugin`; unconfigured/default, `Character+`, debug, and screenshot/default paths retain the normal LightPlugin/gizmo boundary. The in-world isolation spec and implementation inventory now record this requirement.

## [2026-08-10] docs | Record Bevy 0.19 / Lightyear 0.28 migration boundary

Updated [[networking]], [[rendering-pipeline]], `AGENTS.md`, `docs/network-integration.md`, and `docs/character-generation.md` for game-engine commit `4bc50a22` (`Upgrade engine to Bevy 0.19 and Lightyear 0.28`). Recorded the current engine boundary as Bevy 0.19, `bevy_hanabi` 0.19, Lightyear 0.28, and Rust 1.95; renamed current client receive-marker references from deprecated `Replicated` to `Remote`. Historical Bevy 0.18 bug findings and the game-server's separate dependency state remain unchanged.

## [2026-08-10] proof | Record stable rebuilt reconnect runtime

Updated [[networking]] and [[procedural-cloud-regeneration]] with post-build proof for `0d215316` at engine docs commit `6b034959`. The old strict-Empty client PID `2846177`/SHA `84f6ecc...` was terminated before fixed PID `3715288`/start ticks `192167263`/SHA `aef6f08d318a21063d698816b8202ed21f6ca70b7cf4d015a2520a5f107619c3` launched; exactly one current socket remained. Ten-second stability held the same PID/start/client ID with `InWorld`, `connected=true`, `connected_links=1`, `local_players=1`, ping `pong`, and **10.21 FPS / 97.98 ms**, `focused=false`. Scene output had 78 undisplayed NPC entries, zero camera/terrain/WMO/doodad/particle terms, and zero terrain/cache counts. Logs showed one expected initial Connecting marker, one connect/login/InWorld path, and zero InWorld disconnects/reconnect loop/panic/OOM/device-loss. Cargo-watch server PID `3123827` remained unchanged, target-matched, admin-responsive, and at server HEAD `4aca4d3` containing the Who fix. This completes the pending Who live health gate without another server restart; the real reconnect ordering is covered by the App RED/GREEN proof.

## [2026-08-10] fix | Ignore initial reconnect disconnect marker

Updated [[networking]], [[procedural-cloud-regeneration]], and `index.md` for `0d215316` (`Ignore initial reconnect disconnect marker`). Recorded root cause: Lightyear `NetcodeClient` requires initial `Disconnected { reason: None }`; while `GameState::InWorld` and `ReconnectPhase::PendingConnect`, the observer misclassified it as a real loss, queued another reset, and replaced client entity/ID about every 100 ms before handshake. The fix ignores only reasonless/no-forced-notice initial markers during `PendingConnect`; reasoned pending failures, connected disconnects, forced disconnects, auth/token/selection/world-reset behavior, and retry behavior remain. RED `/tmp/claude/game-engine/reconnect-initial-marker-red.log`; GREEN `/tmp/claude/game-engine/reconnect-initial-marker-green.log` (**14 passed**); fmt `/tmp/claude/game-engine/reconnect-initial-marker-fmt.log`; readability `/tmp/claude/game-engine/reconnect-initial-marker-readability.json` and `reconnect-initial-marker-readability-metrics/`. Post-build proof `/tmp/claude/game-engine/reconnect-fixed-runtime.json` and `/tmp/claude/game-engine/reconnect-fixed-live-current.log` was recorded at engine docs commit `6b034959`: old PID `2846177`/SHA `84f6ecc...` was terminated before fixed PID `3715288`/start ticks `192167263`/SHA `aef6f08d318a21063d698816b8202ed21f6ca70b7cf4d015a2520a5f107619c3` launched. One unchanged strict-Empty client ID remained `InWorld` with `connected_links=1`, `local_players=1`, ping `pong`, **10.21 FPS / 97.98 ms**, and `focused=false` for ten seconds; 78 NPCs were all undisplayed, camera/terrain/WMO/doodad/particle terms and terrain/cache counts were zero, and logs showed one expected initial Connecting marker, one connect/login/InWorld path, and zero disconnects/reconnect loop/panic/OOM/device-loss. Server PID `3123827` stayed cargo-watch managed, matched its target, admin ping was `pong`, and server HEAD `4aca4d3` contained the Who fix. This completes the pending Who live health gate without another server restart; reconnect ordering remains covered by the App RED/GREEN proof.

Chronological record of wiki operations.

## [2026-08-10] fix | Accept strict Empty gizmo A/B result

Updated [[rendering-pipeline]], [[procedural-cloud-regeneration]], and `index.md` for `1503cd1c` (`Disable gizmos in strict Empty`). Explicit fixed `InWorldSceneStage::Empty` disables both Bevy `GizmoPlugin` and `GizmoRenderPlugin`; unconfigured/default, `Character+`, debug, and screenshot/default paths retain gizmos. Baseline PID `2655273` measured **8/12** passing windows, **9.97% mean**, **10.90% maximum**, and a 60-second profile with **2,153 samples** where `GizmoBuffer<LightGizmoConfigGroup>::queue` contributed **1.51% sampled CPU**. Candidate PID `2846177`, SHA `84f6ecc9590979a2fba498b8206549d33ebff7fdc27878a16331acccc19811da`, measured **10/12**, **9.47% mean**, **10.60% maximum**; its 60-second profile collected **1,936 samples**, **0 lost**, and no `Gizmo` symbol. The targeted path disappeared and passive mean improved **0.50 percentage points**, so the commit is retained, but the final every-window `<=10.0%` gate remains open. Readiness proved one client/server, IPC/admin ping, approximately **10.09 FPS**, zero displayed NPCs/cameras/terrain, no panic/device-loss/OOM, and no Character advancement. One startup missing-despawn warning was recorded without blocking readiness.

## [2026-08-10] fix | Record idle Who runtime change-tick correction

Updated [[networking]], [[procedural-cloud-regeneration]], and `index.md` for `2c265ffa` (`Avoid dirtying idle Who runtime`). `send_pending_queries` previously called `pop_front()` on an empty queue every `Update`, advancing `WhoRuntimeState` change ticks without a query, send, reply, or snapshot transition. A read-only empty guard now preserves FIFO query sending, unavailable replies, inbound receive handling, reset cleanup, and later-stage behavior. RED evidence: `/tmp/claude/game-engine-perf/who-idle-change-tick-red.log`; GREEN evidence: `/tmp/claude/game-engine-perf/who-idle-change-tick-green.log`; `cargo fmt` passed, and Rust-readability evidence is under `/tmp/claude/game-engine-perf/who-idle-change-tick-readability/`. No CPU-savings or Character-readiness claim is made before runtime measurement.

## [2026-08-10] fix | Record audio backend Empty boundary

Updated [[sound]], [[procedural-cloud-regeneration]], and `index.md` for `463e9e47` (`Disable audio backend without sound flag`). No-sound mode now omits Bevy `AudioPlugin`, while `--sound` retains Bevy audio plus project `SoundPlugin` exactly. Outside `src/sound/`, optional `AudioSink` status and `SoundSettings` do not require `AudioPlugin`. Recorded RED/GREEN, formatting, and readability artifacts.

Post-fix PID `2468254` remained focused, connected, and visually Empty. It had no audio backend threads or PipeWire/CPAL/ALSA profile symbols. Twelve passive windows measured **9.61% mean**, **9.55% median**, and **10.60% maximum** CPU; **8/12** met `<=10.0%`. Audio removal materially lowered the mean, but the strict all-window gate and Character advancement remain blocked.

## [2026-08-10] fix | Record network reset due-gate boundary

Updated [[procedural-cloud-regeneration]], [[networking]], and `index.md` for `ce0ce2d0` (`Gate network reset flush until due`). The due predicate skips no-pending/not-due frames while preserving earliest-target selection, one-frame deferral, exactly-once reset, and existing reset content. Recorded RED `/tmp/claude/game-engine-perf/network-reset-due-gate-red.log`, GREEN `network-reset-due-gate-green.log`, formatting, and readability artifacts.

Post-fix PID `2402583` remained focused, `InWorld`, connected with one link/player, and visually Empty. Its steady profile contained neither the flush wrapper nor due predicate. Twelve passive windows measured **12.05% mean**, **11.75% median**, and **14.00% maximum** CPU; **0/12** met `<=10.0%`. The due gate is verified, but the performance hypothesis is rejected and Character remains blocked.

## [2026-08-10] fix | Record final Empty M2 asset-store boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], and `index.md` for the `0a1a1bfb` → `49304144` → `e7f98704` correction. Exact Empty now keeps lightweight `Assets<M2EffectMaterial>` for active consumers while omitting Bevy `EntitiesNeedingSpecialization` and material/render schedules; Character+, unconfigured, and debug runs retain the full plugin. PID `2339740` scene-setup and PID `2365242` `sync_equipment` panics remain failure evidence; their stale sockets were removed only after identity-safe verification. Corrective RED, GREEN, formatting, and readability artifacts use the `empty-m2-asset-specialization-*` prefix.

Final PID `2390217` matched `e7f98704`, stayed `InWorld` and connected with one link/player, retained FPS text, and had zero terrain, `Camera3d`, or displayed NPCs. Its profile contained no Hanabi, `M2EffectMaterial`, or M2 specialization symbol. After a 30-second warm-up, twelve passive windows measured **10.24% mean**, **10.20% median**, and **10.60% maximum** CPU; only **2/12** met `<=10.0%`. The M2 boundary is verified, but the overall Empty CPU and Character gates remain open.

## [2026-08-10] fix | Record Particle/Hanabi Empty render boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], `docs/particle-system.md`, and `index.md` for `beead231` (`Register particles only at Particles stage`). Recorded that exact Empty through Lighting no longer registers ParticlePlugin/Hanabi, while Particles, Ui, and unconfigured normal runs retain it. The pre-fix PID `2176863` profile sampled `bevy_hanabi::render::VfxSimulateNode::run` at 2.18% self CPU despite stage-gated emitter systems. Recorded RED `/tmp/claude/game-engine-perf/empty-particle-plugin-red.log`, GREEN `empty-particle-plugin-green.log`, and formatting/readability artifacts. Rebuilt PID `2297374` stayed connected with one link/player, FPS text, and zero world content; its post-fix profile contained no Hanabi symbol. Three passive samples measured 12.50%, 12.70%, and 9.90% of one core, so the `<=10%` result is not stable and no causal CPU reduction is claimed. Temporary 10 FPS pacing and the blocked Character gate remain.

## [2026-08-10] fix | Record strict Empty FPS graph boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], [[ui-system]], and `index.md` for `8cac2b03` (`Disable Empty FPS frame-time graph`) and follow-up `cc5780a8` (`Preserve Empty FPS graph disablement`). Corrected the startup-only failure: runtime option writers restored the graph, visibly producing the solid red block in `empty-fps-graph-2246158.webp`. `cc5780a8` makes every writer stage-aware. Rebuilt PID `2283621` kept FPS text with no graph (`empty-fps-graph-options-2283621.webp`), stayed connected with one link/player and zero world content, reported 9.95 FPS / 100.45 ms, and measured 9.80% of one core over 10 seconds. This meets the numerical threshold under temporary 10 FPS pacing; it is not accepted as the final Empty design, and Character remains blocked.

## [2026-08-10] fix | Record Character-stage camera/input guard

Updated [[procedural-cloud-regeneration]], [[replicated-unit-noops]], [[rendering-pipeline]], and `index.md` for `c446d81c` (`Gate camera updates at Character stage`). Recorded that strict Empty skips `sync_camera_options`, `camera_input`, `cursor_grab`, `player_movement`, and `camera_follow`; Character and later stages retain them. Recorded Empty-only collision/pathing collection and raycast setup, RED/GREEN/fmt/readability evidence, and protected `camera.rs` instrumentation preservation through partial staging. Rebuilt PID `2176863` remained connected at 10.01 FPS / 99.89 ms with zero world camera/terrain/displayed NPCs and measured 11.10% of one core. The prior paced result was 11.20%, but remote entities changed from 70 to 75, so no measurable improvement is accepted. The 10 FPS pacing remains temporary and Character remains blocked.

## [2026-08-10] fix | Record strict Empty diagnostic pacing

Updated [[procedural-cloud-regeneration]], [[networking]], [[rendering-pipeline]], and `index.md` for `4fb2e5c9` (`Pace strict Empty stage at 10 FPS`). Recorded the exact gate: only `GameState::InWorld` with exact `InWorldSceneStage::Empty` uses the existing limiter's 100 ms interval; Character/later stages and other states retain persisted/global frame limiting and PresentMode, while FPS overlay, networking, and IPC remain registered. Framed pacing as diagnostic frame-cadence control, not render/application-work removal. Recorded RED `/tmp/claude/game-engine-perf/empty-frame-interval-red.log`, valid GREEN `/tmp/claude/game-engine-perf/empty-frame-interval-green-2.log`, module GREEN `/tmp/claude/game-engine-perf/empty-frame-client-options-green-2.log`, formatting/readability evidence, and runtime measurements: pre-pacing PID `2093844` at 490.62 FPS / 369.05% one-core CPU; paced PID `2130439` at 9.98 FPS / 100.23 ms and 11.20% one-core CPU, connected with one link/player and zero world camera/terrain/displayed NPCs. The `<=10%` gate failed. Alessio chose to keep 10 FPS temporarily for investigation, not as the final fix; Character remains blocked.

## [2026-08-10] fix | Record demand-driven IPC status refresh

Updated [[procedural-cloud-regeneration]] and [[networking]] for `abf68fd9` (`Refresh IPC status snapshots on demand`). Recorded the `Receive → RefreshStatus → Dispatch` ordering, explicit request-to-snapshot dependency matrix, FIFO command dispatch with coalesced refresh flags, no-command idle behavior, and removal of duplicate map synchronization. Recorded RED evidence in `/tmp/claude/game-engine-perf/status-demand-red.log` and `status-refresh-matrix-red.log`, GREEN evidence in the corresponding `*-green-2.log` files, formatting checks with empty stderr, and readability artifacts under `status-demand-readability/`. No CPU improvement or `<=10%` Empty-stage claim is made until a rebuilt live measurement. Protected `src/rendering/camera/camera.rs` and all source/tests/PLAN/Cargo files were left untouched.

## [2026-08-09] fix | Record event-driven NPC visibility

Updated [[networking]], [[replicated-unit-noops]], and `index.md` for `e745d35e` (`Make NPC visibility event driven`). Recorded the prior per-`Update` scan history (`1a1a8179`, extended by `6ffa6ce0`, with `3c77d346` guarding writes only), then the new trigger boundaries: added/changed `Npc` entity-scoped updates, semantic `LocalAliveState` changes for `DeadOnly` policies, dawn/dusk phase changes for scheduled policies, and one full reconciliation on state/NPC-stage activation. Recorded focused RED/GREEN behavioral proof without claiming measured CPU improvement.

## [2026-08-09] fix | Record Npcs-gated remote interpolation

Updated [[networking]], [[replicated-unit-noops]], [[procedural-cloud-regeneration]], and [[rendering-pipeline]] for `538e8329` (`Gate remote interpolation at Npcs`). Connection/auth, replication receive, and replicated target synchronization remain active before `Npcs`; `interpolate_remote_entities` now runs only from cumulative `Npcs`, so `Empty`, `Character`, `Skybox`, and `Terrain` no longer mutate remote visual `Transform`. Recorded the RED failure and GREEN pass in `/tmp/claude/game-engine-perf/strict-empty-interpolation-red.log` and `strict-empty-interpolation-green.log`. The live replacement (`538e83290769c70a6980ec0903db74bf2981c0fb`, PID `2960624`, start ticks `182917558`, socket `/tmp/game-engine-2960624.sock`) compared cautiously with the pre-gate client (`96e1308a31940ed5c03046b574ca0b52fe15d8e2`, PID `2592665`, start ticks `182782909`, socket `/tmp/game-engine-2592665.sock`): both had no world camera, remote counts `133` versus `134`, aggregate CPU `325.49% → 288.12%`, compute CPU `229.17% → 197.62%`, and client gfx `6.93% → 5.60%`. FPS/frame direction is not acceptance evidence because runqueue and live conditions drifted. The subsequent strict-Empty eu-stack capture contains no interpolation stack, but about `2.9` cores remain, so the root-cause loop continues.

## [2026-08-09] fix | Record strict Empty world-camera boundary

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], and [[replicated-unit-noops]] for `96e1308a` (`Skip world camera at Empty stage`). `Empty` no longer spawns the world `WowCamera`/`Camera3d`; `Character` and later cumulative stages still retain one. The standalone performance panel/UI camera remains active, and the permanent `Empty`–`Npcs` post-process gate from `3b144afc` is unchanged. Commit tests cover zero world cameras in `Empty` and one world camera across `Character` re-entry. This audit changed documentation only; no live relaunch or performance claim was made.

## [2026-08-09] fix | Record permanent Empty-to-Npcs camera gate

Updated [[procedural-cloud-regeneration]], [[rendering-pipeline]], and `index.md` for permanent behavior commit `3b144afc`. Recorded removal of the WowCamera TAA/SSAO/depth/normal/motion-prepass bundle plus `TemporalJitter`/`MipBias` through cumulative `Empty`–`Npcs`; `Lighting` onward and unconfigured/default stages retain graphics-option-driven behavior, including TAA restoration and configured MSAA depth/normal prepasses. Common bloom/render-scale/CAS/DoF, camera identity, tonemapping, shadow filtering, spatial audio, UI/network/IPC, and FPS overlay remain unchanged. The temporary selector was removed in `e9d3d470`. Alessio accepted the diagnostic cause. The historical PID `3367453` diagnostic client later exited at `2026-08-09T06:05:43Z` with `WindowCloseRequested` followed by `AppExit Success`; its socket is gone, with no coredump, OOM kill, crash, or agent lifecycle action. It was not permanent-build verification, and the permanent Empty replacement human gate remains pending; do not relaunch or advance it without Alessio's explicit permission.

## [2026-08-09] investigation | Confirm Empty-stage camera bundle cause

Updated [[procedural-cloud-regeneration]] and [[rendering-pipeline]] for diagnostic commit `b6468868` and live proof `/tmp/claude/game-engine-perf/empty-camera-post-process-live.json`. At capture time, PID `3367453` was the only running client on `/tmp/game-engine-3367453.sock`, connected to `InWorld` with one link, one local player, 134 remote entities after sampling, zero terrain tiles, empty game UI, and zero UI/font/panic/GPU-error evidence. The performance panel and non-targeted camera behavior remained active. Alessio judged performance improved and accepted the camera bundle as the Empty-stage cause; six post-warmup samples are supporting only. The client later exited at `2026-08-09T06:05:43Z` with `WindowCloseRequested` followed by `AppExit Success`; no coredump, OOM kill, crash, or agent lifecycle action was observed. Commit `e9d3d470` removed the temporary selector from source before permanent implementation. No permanent fix is claimed yet; do not relaunch or advance the human gate without Alessio's explicit permission.

## [2026-08-08] fix | Record pre-UI game/UI-toolkit scheduling boundary

Updated [[ui-system]], [[procedural-cloud-regeneration]], [[replicated-unit-noops]], [[networking]], and `index.md` for `game-engine` `508891a6` and `ui-toolkit` `50e4a17`. Recorded the empty-stage `UIActionBar.BLP` flood (**853,196 lines**, **75.9 MB**) and root cause: `UiRenderEnabled(false)` gated only the inner renderer while game-UI builders, sync/input work, texture-related frame processing, and observers continued. Recorded `UiProcessingEnabled` around the complete toolkit UI update chain, cumulative pre-`Ui` game-UI gates, independent FPS overlay tests, and machine-side relaunch proof from `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json`: connected `InWorld`, one link, one local player, 133 remote entities, empty toolkit UI tree and `MainActionBar` filter, zero `[UI]`/`UIActionBar.BLP` lines, no font panic/GPU OOM/device-loss/panic, and responsive ping/performance. The client was left running for Alessio at that time; human visual approval remained pending and no next stage launched. The three performance samples are not comparative evidence. Preserved existing M2 and replicated-unit NOOP facts.

## [2026-08-08] investigation/fix | Record empty-stage replicated-unit NOOPs and committed suppression

Added `investigations/replicated-unit-noops.md`; updated `systems/networking.md`, `investigations/procedural-cloud-regeneration.md`, and `index.md`. Recorded the preserved empty-stage boundary: `remote_entities=133` includes one local player (132 NPCs plus one local player), client per-frame unchanged `Transform`/`Visibility` writes, Lightyear receiver equality suppression versus server-side same-value movement/gravity serialization, real wander movement, and nearby movement-type-2 NPCs without waypoint rows. Recorded tests and fixes from `3c77d346`, `2927382`, and `ae81c65`. No runtime FPS improvement is claimed before corrected-binary relaunch.

## [2026-08-08] feature | Document World Builder diagnostic sidebar

Added `specs/world-builder.md` and `systems/world-builder.md`; updated `systems/ui-system.md`, `reference/keybindings.md`, and `index.md`. Recorded opt-in lifecycle, scene inventory, reversible render/processing overrides, bounded property editing, fixed F9 input, and measurement constraints.

## [2026-08-08] investigation | Record preliminary M2 UV comparison

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md`. The initial direct pair used the same binary/source/options/server/token/environment, ten readiness polls, one local player, stable recorded world/material invariants, 125-second holds, and six unprofiled samples per condition. Enabled measured **31.767 FPS / 54.177 ms**; disabled measured **36.843 FPS / 54.482 ms**. The first sample in each condition immediately followed an expensive `dump-scene` request and inherited its long diagnostic frame (**188.06 ms** enabled after **215 ms** scene latency; **210.88 ms** disabled after **266 ms** scene latency). Recorded readiness workloads also differed (**135** versus **133** remote entities). The result is therefore **preliminary/inconclusive pending a clean repeat** with a prospective performance warm-up; it supports no M2 performance conclusion or fix. An earlier startup attempt ended at a Friz parse failure; the pre-overwrite bytes were not preserved, while the current Friz/Arial bytes pass the exact Bevy parser. Selector/tests/flag were removed in `58e2f9c2`, then restored temporarily in `a7784e70` for the repeat.

## [2026-08-08] investigation | Record stabilized UI/render performance evidence

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md`. Recorded the enabled control (**12.332 FPS / 81.157 ms**, 71 remote entities, `game-engine` `00e7b3b0` / `ui-toolkit` `5ead575`) and all-text-disabled diagnostic (**37.415 FPS / 26.785 ms**, 76 remote entities, `game-engine` `6806717c` / `ui-toolkit` `43a2784`). The runs used different revisions and exact workloads; the delta strongly implicates UI-text-associated rendering with moderate confidence, not proof. Recorded the rejected/reverted equality-guard experiment (**11.373 FPS / 90.230 ms**, commits `33fa74d`/`36d4692`), engine CPU/Compute Task Pool load, adapter-wide shared GPU-busy measurement, wgpu buffer-transition/unmap attribution, low text-extraction self-cost, and unresolved downstream causality. Distinguished unprofiled CLI performance evidence from profiler attribution. Recorded transient reconnect failures later cleared by an unchanged logged launch. Shadow-only diagnostic is implemented with a GREEN behavioral test; live measurement is pending. No production fix or FPS improvement claim is made.

## [2026-08-08] fix | Record SSAO anti-aliasing compatibility

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md` for commit `cff4ad46` (`Keep SSAO compatible with anti-aliasing`). The real `WowCamera` now removes SSAO under default MSAA4x; switching to TAA restores `Msaa::Off`, `TemporalAntiAliasing`, and SSAO. The RED test reproduced SSAO with `Msaa::Sample4`; the exact GREEN compatibility test passes, removing the per-frame Bevy incompatibility error path. No runtime FPS improvement is claimed until the restarted engine is measured.

## [2026-08-08] correction | Record Mailbox presentation evidence

Updated `investigations/procedural-cloud-regeneration.md`, `systems/rendering-pipeline.md`, and `index.md` for commit `89f58874` (`Use mailbox presentation for VSync`). Corrected prior performance evidence: screenshots and the stale 15.68 FPS overlay are visual artifacts, not baselines; five seconds without CLI requests produced zero completed SSAO extraction frames; `ping`, `status`, and `performance` waited approximately one second; main-thread stacks waited in `SubApps::update`; and the render worker blocked in Vulkan `Queue::present` through Wayland `wl_display_dispatch_queue`/`ppoll` with events every approximately 0.96–0.97 seconds. The Vulkan surface supports Mailbox and FIFO. Existing `vsyncEnabled=false` selected Mailbox and removed the stall; production now maps VSync-enabled mode to Mailbox while VSync-disabled remains `AutoNoVsync`. Fully visible unfocused Mailbox evidence: 142 frames/5.009 seconds (28.35 FPS), CLI six-sample mean 29.32 FPS / 34.37 ms, request mean 38.7 ms, CPU 226.57% of one core, process GPU gfx busy 58.01%, system GPU busy mean 60.33%, network InWorld/connected. The cloud simplex hotspot disappeared, but its earlier FPS attribution is invalidated by the presentation stall. SSAO/MSAA remains unresolved.

## [2026-08-07] update | Record final procedural-cloud post-fix evidence

Updated `investigations/procedural-cloud-regeneration.md` with provisional post-fix evidence later superseded by the presentation-stall investigation: screenshot-derived 27.98/27.89 FPS values and the single 28.14 CLI result were not valid baselines. The later correction records the Mailbox presentation fix and valid CLI evidence. The cloud simplex hotspot remains removed; SSAO/MSAA remains separate and unresolved.

## [2026-08-07] update | Document IPC performance diagnostics

Updated `AGENTS.md` and `investigations/procedural-cloud-regeneration.md` for commit `9003b421`: `game-engine-cli performance` reports `fps`, `frame_time_ms`, and `focused`; screenshot `FPS: 1.00` overlays are capture-frame artifacts, not timing evidence.

## [2026-08-07] investigation | Remove synchronous procedural cloud regeneration

Updated `systems/rendering-pipeline.md` and added `investigations/procedural-cloud-regeneration.md` for commit `b2b07e5b`: 512×1024 six-octave cloud textures regenerated synchronously every five seconds, with profiler self samples placing about 60% of sampled CPU in simplex cloud functions. Shader UV/time scrolling already animates clouds, so runtime regeneration was removed while preserving the three startup textures and visual settings. The earlier screenshot/CLI FPS values are now marked invalid because a separate presentation stall affected the measurement. IPC screenshots can show a transient 1.00 FPS overlay and are not valid FPS evidence; capture does not leave screenshot entities in the scene.

## [2026-04-30] update | Add Scenemachine M2 loading reference

Updated `reference/open-source-wow-clients.md` with Scenemachine as a C# reference for loading M2 scene/model data.

## [2026-04-09] ingest | Initial bulk ingest of 32 existing docs

Ingested all existing documentation from `docs/` into the wiki structure. Created pages across systems/, formats/, investigations/, design/, and reference/ categories.

## [2026-04-11] update | Document authored skybox black-output repro

Added `investigations/authored-skybox-black-output.md`, updated `systems/skybox.md`, and recorded the current `skyboxdebug` repro showing effectively black output for both default authored lookup and forced `LightSkyboxID 653`.

## [2026-04-21] update | Document LightParams sky-affecting flag composition

Updated `systems/skybox.md` with the implemented `LightParams::Flags` contract (`DontInheritSkybox`, `HideSun`, `HideMoon`, `HideStars`, `HideCelestialObject`, `OverrideCelestialSphere`, `HeightFogAbovePlane`) and how those flags now alter `skyboxdebug` procedural baseline/fog composition.

## [2026-04-21] update | Trace modern authored skybox shader/effect path

Updated `investigations/authored-skybox-black-output.md` with a detailed trace for `11xp_cloudsky01.m2` modern shader batches (`0x4014`, `0x8012`, `0x8016`), including stage binding, combine-mode routing, UV mode mapping, and the current WGSL combine-coverage gap for `0x8012`/`0x8016`.
## [2026-05-01] update | Document direct DB2 CASC access

Updated [[db2-format]] and [[asset-pipeline]] to record that DB2 bytes can be read directly from CASC via `AssetResolver::resolve_bytes`, with `ensure_db2_path` as a cache/debug path. Added `Frostshake/WDBx` as external verifier/export tooling rather than a runtime dependency.

## [2026-09-09] systems | Share terrain and water animation clock

Updated [[terrain]] and [[rendering-pipeline]] for `6ea29dba`, `1b5efc68`, and `e5a76829`/`8c2a89d8`: shader UV animation now reads Bevy shared virtual time instead of modifying terrain/water materials each frame. Focused asset-event tests prove clock-only advancement does not invalidate materials; a bounded Vulkan fixture completed in 0.71 seconds, changed terrain/water pixels from time 0 to 1, and observed no material `Modified` events. The shared clock wraps after one hour, so UV phase restarts; no full-client equivalence or CPU claim is made.

- 2026-09-09: Reproduced premature player appearance during Loading: first child preceded meshes/default equipment, yet dedup marked the snapshot applied. Final model metadata now gates application. Real asset lifecycle/body pixel tests pass; missing weapon attachments 0/1 remain explicit. See [character rendering](systems/character-rendering.md#replicated-player-construction-boundary).

## [2026-09-12] rendering | Record waterfall mist emitter boundary

Updated [[character-select-waterfall-loading]] and [split-shadow spec](../specs/split-adt-shadows.md) for `5206d9b4`/`030090f3`: parsed emitters and skeleton joints now reach the existing particle path only for the existing waterfall/ripple backdrop selection, preserving unrelated terrain props' prior behavior and the graphics particle-effects gate. The census identifies six `1028937` mist and three `2904370` misty-ripple placements, each with one emitter; primary cascade sheets have none. `62e36331` records zero attached emitters where one was expected; focused GREEN passes. `29819999`/`b1663674` record generated-WGSL sprite-index RED/GREEN. Both selected mist models decode zero gravity; the prior NaN shader came from an unrelated activated prop. Local CASC supplied missing `2904679`; automatic particle-texture extraction remains open. Native cascade/mist acceptance remains open.

## [2026-09-10] character | Correct starter weapon attachment semantics

Updated [[character-rendering]] and [[npc-motion-validation]] for engine `496a057b`. Local WMVx reference code identifies right palm 1, left palm 2, and shield left wrist 0; HumanMaleHD maps them to bones 206, 211, and 201. The prior 201/206 root-parent proof did not prove the correct semantic mount points. The new live sword/shield test was RED against the former mapping and passes at `5e5b2574`; replacement native visual proof remains pending. No item-local rotation correction is claimed.

## [2026-09-23] ui | Combat feedback wiring

Updated [[ui-system]] (Combat Feedback). The player cast bar now follows the replicated local `CastState`. FCT labels are drawn through the nameplate projection. `CombatLogEvent` becomes the only FCT producer once it arrives. `UIErrorsFrame` is fed by `CastFailed`. Focused bin (24) and lib (63) tests pass. No native run.

## [2026-09-23] ui | Spellbook and action bars from server data

Updated [[ui-system]] (Spellbook and action bars) and [[spell-catalog]] (passive flag, tab rule). Known spells, spec, action bar slots, cooldowns and charges come from the server. The spellbook is mounted in production behind `P`. Action buttons are `ActionButton<bar>_<button>` with icons, cooldown wipe/text, charges and range/power tints. Drag and drop sends `SetActionButton`. Casts target the server entity. Focused tests only; no native run.

## [2026-09-23] ui | Window manager, movable windows, HUD edit mode

Updated [[ui-system]] (Windows and HUD Edit Mode) and added the [window manager](../specs/window-manager.md) and [HUD edit mode](../specs/hud-edit-mode.md) specs. `WindowManager` replaces per-scene open flags and `InWorldEscapeStack`. Window positions (per character) and edit-mode layouts (account-wide, active per character) persist in `ui_layout.ron`. No native run.

## [2026-09-25] ui | Professions

Created [[professions-ui]]. Retail ClassTrainerFrame, ProfessionsBook and ProfessionsFrame on the new trainer/profession protocol; the AzerothCore-era professions placeholder and the client-only gather cast are gone.

## [2026-09-25] investigation | Stormwind dark render

Created [[stormwind-dark-render]]. Stormwind district WMOs take the unified MapObj path (MOHD `0x02`), where MOCV is additive light. The engine drew them unlit as texture×MOCV, so near-zero MOCV turned them black. `WmoUnifiedMaterial` adds MOCV to daylight or MOHD ambient. Updated [[wmo-format]].

## [2026-09-25] ui | Loot and flight masters

Created [[loot-and-flight]]: corpses stay replicated, `Lootable` sparkle/cursor, right-click loot with auto-loot XOR Shift, Retail LootFrame; Retail FlightMapFrame on the UiMap continent art; `MovementControl` epoch snap and controlled follow for flights and server teleports. Removed the client-only `taxi.rs` preview and `loot_data.rs`.


## [2026-09-25] investigation | Terrain blend steps

Created [[terrain-blend-steps]]. The alpha map is now linear `Rgba8Unorm`. WDT MPHD flags select MCAL storage and the terrain blend (layered, weighted, or Retail height-weighted with MTXP defaults). The diffuse alpha is no longer used as height. MCAL layers are bounded by the next offset, and the edge fix applies to every format. Updated [[adt-format]] and [[terrain]].

## [2026-09-25] investigation | WMO Retail lighting

Created [[wmo-retail-lighting]]. WMO lighting, fixup, two-layer shaders, alpha test, metal and placement now follow WebWowViewerCpp's Retail shaders, replacing the 3.3.5 (solarityclient) semantics. Updated [[wmo-format]] and [[stormwind-dark-render]].

## [2026-09-25] investigation | Washed-out sky

Created [[washed-out-sky]]. LightData colours decode as `0x00RRGGBB` sRGB bytes; the procedural dome uses the client ring profile (SkyTop/Middle above 16°, SkyFogColor below the horizon); world fog uses FogEnd/36 yards and SkyFogColor; sky and fog blend the global and local Light rows around the local player. Updated [[skybox]].

## [2026-09-25] system | Retail lighting

Created [[retail-lighting]]. RetailSceneLight (ambient/horizon/ground ambient, direct, WWV sun direction, fog) is one GPU buffer that terrain, M2 and M2 effect shaders read; they apply WebWowViewerCpp calcLight and fog in authored space. PBR light calibration, camera IBL and TonyMcMapface are gone for world cameras. Updated [[washed-out-sky]] and [[skybox]].

## [2026-09-25] investigation | Stormwind hilly plaza

Created [[stormwind-hilly-plaza]]. The hilly cobblestone in the Trade District was the authored ADT terrain under a district WMO that portal culling fully hid. Antiportal AABB occlusion and bbox-only camera-group detection caused it. Portal culling now draws every exterior group from outside, enters an interior only when a floor of that group is below the camera, and no longer uses antiportals as occluders. Updated [[wmo-format]].

## [2026-09-25] investigation | Stormwind building textures

Updated [[stormwind-hilly-plaza]] and [[wmo-format]]. MOBA flag 0x2 selects the u16 material id; without it every `sw_tradedistrict` batch took MOMT 0, one wall texture, and the roofs never drew. MOMT texture_2 is at 0x18; the parser was reading diffColor there.

## [2026-09-25] design | Nameplate style

Updated [[nameplate-design]] and the [nameplate style](../specs/nameplate-style.md) spec. `NameplateStyle` drives plate sizes, reaction/cast colours, border and fonts; Options > Nameplates edits it and the Thin/Thick selectors are presets. Fills are tinted by the owner's FactionTemplate reaction via `shared::faction_reaction` (Defias Thug, template 7, is neutral yellow). Names are centred 2px above the plate; plate offsets now apply in overlay units, which fixes the name drifting left of the bar under the scaled in-world UI camera. Targeted bin (153) and lib (76) tests pass; headless captures in `data/diagnostics/nameplate-20260925/`.

## [2026-09-26] create | Trade and mail

[[trade-and-mail]]: Retail TradeFrame and MailFrame on the server trade and mail protocols; the fake local mail store, the doodad mailbox and the mail keybind removed.

## [2026-09-26] investigation | UI rounding seams

Created [[ui-rounding-seams]]. Auction house tab seams came from taffy 0.10.1 rounding a node's location parent-relative; game-engine now patches taffy with upstream's cumulative rounding via bevy-patches.

## [2026-09-26] create | Unit tooltip

Created [[unit-tooltip]]: hovered-unit resolution, CreatureTooltip cache, account appearance collection marks, default tooltip anchor, IPC hover for headless proof.

## [2026-09-26] create | Cursor item

Created [[cursor-item]]: CursorItem click rules, StackSplitFrame, DELETE_ITEM / DELETE_GOOD_ITEM, ItemSparse catalog and item tooltips; merchant-frame updated for drag buy/sell and Sell All Junk.

## [2026-09-26] investigation | Bevy and Godot directional shadows

Created [[bevy-godot-shadow-comparison]]. Revision-pinned source review records shared CPU scan/cascade and batching paths, Godot's extra directional silhouette-plane culling plus shadow mesh/LOD path, and Bevy's conditional gathered-cascade GPU preprocessing. No runtime comparison or performance winner is claimed.

## [2026-09-26] update | Map transfers and WMO-only maps

[[terrain]]: maps named by Map.db2 Directory; WDT MPHD 0x1 global WMO maps (Stockade) spawn one WMO placed from the world origin, no tiles; WMO floors are the only ground and the camera is not terrain-clamped there. Spec [instances](../specs/instances.md) (NewWorld/WorldPortAck, TransferAborted, CONFIRM_SUMMON after loading).

## [2026-09-26] system | Godot full-contract parity inventory

Created [Godot feature parity matrix](../specs/godot-parity-matrix.md) and linked it from [[godot-conversion]] and the conversion specification. It inventories every existing feature specification by concrete capability, without duplicating those contracts. Every conversion row remains Missing: current parser/core, preview, portable-UI-model, account-host, and headless transport results do not establish Godot runtime parity. The authorized Bevy boundary remains transport-only; the integrated full-client conversion gate is unchanged and open.

## [2026-09-26] investigation | Stockade entrance

Created [[stockade-entrance]]: MODD name_offset indexes MODI (WMO doodads of MODI-only roots now spawn, including the Stockade instance portal); camera collides with portal-culled WMO groups, smoothed pose ray-checked, portal visibility clips the polygon and opens within 2.25 yd. Updated [[collision-system]], [[stormwind-hilly-plaza]], [[wmo-format]].

## [2026-09-26] update | Doodad particles, player walls, antiportal flag, interior camera terrain

[[stockade-entrance]]: all doodads spawn M2 emitters; player movement ignores visibility for WMO walls; antiportals from the MOGP flag; no camera terrain clamp inside WMO interiors. [[collision-system]] updated.

## [2026-09-26] investigation | Stockade floor fall

Created [[stockade-floor-fall]]: the failing teleport points lie outside WMO 108631 (no face crosses them); server ground matches brute force on all 7,650 floor samples; sky is the exterior-only portal view. Open: TrinityCore fall-to-void kill.

## [2026-09-27] change | Launcher caches Godot

The launcher resolves Godot as `GODOT_BIN`, else `${XDG_CACHE_HOME:-~/.cache}/game-engine/godot/4.7.2/`, downloading the official 4.7.2 zip and verifying its pinned SHA-512 on first use. `data/tools/godot` is no longer read. See [[godot-conversion]].

## [2026-09-28] change | Launcher imports fresh checkouts

After the native build, the launcher runs `godot --headless --import --path godot` once when `godot/.godot/extension_list.cfg` is missing; failure keeps its exit status and prevents launch. `GAME_ENGINE_ROOT` overrides the checkout root (used by launcher tests). See [[godot-conversion]].

## [2026-09-28] feature | World map

Shared `UiMap` catalog and view model drive a Retail-style world map in Godot and Bevy: `M` opens the player's zone, right-click zooms to continent and world, arrow follows the player. Local CASC lacks many map tiles. See [[world-map]].

## [2026-09-29] update | Native WorldMap placement boundary

At `ac44cc2e`, `WorldMapFrame` is the only native managed window: title drag clamps and persists per selected server character in canonical `ui_layout.ron`; reopen and a second authenticated process restore it. `/tmp/claude/world-map-owned-fixture-ac44cc2e.log` proves selected-character reset retention, default-slot reopen, and fresh-process read. The direct open-map reset has state-test coverage only: simultaneous Options-plus-map UI is keyboard-unreachable because Escape closes the map. Other windows and generic native window-manager parity remain unconverted. See [[world-map]] and [window-manager spec](../specs/window-manager.md).

## [2026-09-28] fix | Godot M2 blend modes and batch colour

Godot M2 blend 3/5/6/7 now follow WebWowViewerCpp's GL factors and batch colour tracks (meshColor) multiply every batch, animated with transparency and colour alpha; the Stockade portal sheet turns blue. WMO doodads and particles remain absent in Godot. See [[stockade-entrance]].

## [2026-09-28] fix | Godot reaches and enters the Stockade

Placed WMO floors are ground once their tile is parsed (not once the object queue spawns the WMO); lighting takes the map ID from Map.db2; WMO-only maps request no ADT tiles, spawn their global WMO and run the camera. A live walk from the room floor into area trigger 101 reaches InWorld on stormwindjail. See [[stockade-entrance]].

## [2026-09-28] port | Godot WMO: all retail MOMT shaders

Godot maps every MOMT id 0-23 to WebWowViewerCpp's vertex/pixel shader pair. It binds the nine material textures, four MOTV sets, MOCV2 and MOC2, and ports every caclWMOFragMat case, including MapObjParallax and the MapObjDFShader height blend. A batch that cannot be built no longer drops its WMO. See [[wmo-retail-lighting]].

## [2026-09-28] fix | Campsite fog and WMO selection

Godot fogs with WebWowViewerCpp's legacy exponential fog from FogScaler/FogDensity, so FogEnd-0 campsites (Freywold Spring, Gallagio Grand Gallery) no longer render as flat fog. Campsite WMOs are selected by MODF extents, so Cultists' Quay shows its delve WMO. See [[campsite-fog-and-wmo-selection]].

## [2026-09-28] fix | Weighted zero-duration M2 variations

Both clients share `VariationFamily`. A weighted zero-duration variation plays for no time, as in WebWowViewerCpp, so the Freywold Spring redbird (FDID 588287) keeps looping Stand. Before, Godot logged an error every frame and Bevy panicked. See [[animation]].

## [2026-09-28] fix | Light selection with ZoneLight polygons

Both clients now select LightParams in WebWowViewerCpp's order: the map default, then ZoneLight polygons, then local lights (strongest first). ZoneLight and ZoneLightPoint are exported from local CASC. Stormwind and Elwynn gain LightParams 6080. Cultists' Quay keeps LightParams 12, because no Light or ZoneLight row covers map 2837. Its Retail blue comes from the WMO's MFOG fog, which is not ported. See [[retail-lighting]] and [[campsite-fog-and-wmo-selection]].

## [2026-09-28] feature | Godot NPC locomotion from CreatureMotion

Replicated NPCs in the Godot client now play Walk 4, Run 5 and Stand 0 from the server's `CreatureMotion`, as the Bevy client does, instead of always standing. See [animation](systems/animation.md#native-godot-replicated-npc-locomotion).

## [2026-09-28] perf | Doodad animation LOD

Doodads now take the NPC animation LOD in both clients (user decision; [npc-animation-lod](../specs/npc-animation-lod.md)). Godot's in-world doodad cull advances doodad bone and material animation only on sampled frames, with the time owed, and M2 skeletons use manual modifier processing, which removes 7,997 per-frame `Skeleton3D` internal processes. Stormwind indoor A/B: 25–28.5 → 31–38 FPS. See [[godot-stormwind-fps]] and [[animation]].

## [2026-09-28] feature | Godot doodad scenery fade

Godot ADT doodads fade over the retail 5/10/15/20/50 yd band before their far radius instead of popping, as solarityclient's `SceneryDistance::opacity`. Opaque batches take a blended shader variant only while fading. Spec: [doodad-scenery-distance](../specs/doodad-scenery-distance.md); see [[godot-stormwind-fps]].

## [2026-09-28] port | Godot WMO doodads

Godot spawns WMO MODD doodads (set 0 plus the MODF doodad set, MODI/MODN models) under their WMO node within the object budget, for ADT and WDT global WMOs, with the retail scenery-distance cull. The Stockade portal now stands in the Jail01 doorway. See [[godot-conversion]], [[wmo-format]], [[stockade-entrance]].

## [2026-09-28] perf | Godot doodad animation culling

Hidden doodads stop their material (UV/colour) animation with their bone animation, and models whose bone or material tracks are all constant never process. In Stormwind, processing animation nodes fall from 1,729 + 631 to 172 + 135. See [[godot-conversion]].

## [2026-09-28] feature | Godot entrance difficulty bar

Near a dungeon entrance the Godot client shows Plumber's difficulty bar: `JournalInstanceEntrance`/`JournalInstance` exported from local CASC (enUS copy for the locale-split table), `MapDifficulty` choices with `DungeonEncounter` lock counts, Plumber's own art; clicking a difficulty sends `SetDungeonDifficulty`. No new protocol. See [[entrance-difficulty-bar]].

## [2026-09-28] feature | Godot NPC poses and gear

Godot replicated creatures hold their `UnitPose` (Sit 97, Sleep 100, Emotes.AnimID such as Ready1H 26 / ReadyRifle 48) while still and play Walk/Run while moving, crossfaded; virtual items attach drawn in hand or at their `Item.SheatheType` sheath and move on a sheath change; the display's `NPCModelItemSlotDisplayInfo` armor switches body geosets and attaches its item models. The pose/gear data (`npc_gear_data.rs`) is now engine-free and shared with Bevy. See [npc-stance-gear](investigations/npc-stance-gear.md).

## [2026-09-28] feature | Godot server-driven breath bar

The Godot client shows the server's `MirrorTimerStart`/`Pause`/`Stop` (fatigue, breath, feign death) instead of a GDScript-started bar, caps swim vertical rate at `SWIM_SPEED` × aura as the server's movement bank does, and takes `SWIM_DEPTH`/`is_swimming`/`swim_top` from shared-protocol. Live on the dev server: breath bar under water at the server's drain rate, gone after surfacing; server height follows the swimmer. See [[swimming]].

## [2026-09-28] feature | Godot spellbook, action bar and casting

The Godot client shows the Retail 12.x spellbook (known spells plus later-level spells greyed "Level N"), the main action bar with keys 1..=, sends `SpellCastIntent` at the target, and shows cooldown/GCD sweeps, `CastFailed` errors, the casting bar and floating combat text. The spell catalog is now engine-free and shared. game-server `spellsbylevel` gates spec spells by SpellLevel and pushes learned spells to the bar. See [[spellbook-action-bar]].

## [2026-09-28] feature | Godot merchant frame

The Godot client interacts with NPCs: right-click targets and sends `InteractNpc` in range, the hover cursor follows `NpcFlags` (Buy on vendors). The server's vendor list opens the shared MerchantFrame, backpack and StackSplitFrame; buy, sell, buyback, Repair All and vendor split work live at Brother Danil; Escape, the close button and walking away close it. Shared data files drop their Bevy derives under `cfg(godot_host)`. See [[merchant-frame]], [[godot-conversion]].

## [2026-09-29] feature | Godot remote player locomotion

Other players in the Godot client walk, run, backpedal, strafe, swim and jump: the server replicates `PlayerMotion` (Retail `MovementFlags`) and `WorldUnits` drives each remote model every frame through the local player's selector and jump sequence. Live proof with two headless clients on a private server. See [animation](systems/animation.md#native-godot-remote-player-locomotion).

## [2026-09-28] feature | Godot TargetFrame at the Retail preset, UIParent HUD scale

The Godot TargetFrame sits where Retail's Modern Edit Mode preset puts it (BOTTOMLEFT at UIParent BOTTOM 300, 250), and the in-world HUD layers lay out on the 768-unit UIParent canvas scaled by viewport height / 768, so frame and text sizes match Retail at 1280×720 and 1920×1080. World map, entrance bar, nameplates and glue screens are not converted yet. See [[godot-conversion]].

## [2026-09-29] feature | Confirmed native CastStart

Shared normalized-phase legacy PCM and cast-ID observation drive an owned Godot spatial emitter only on the local player's replicated `CastState` transition; the owned spell-click UDP fixture proves request quiet, active/repeated/inactive/retriggered/muted/removal boundaries. Final bounded verification at `0e0726a3` passes (`/tmp/claude/verify-native-caststart-final.md`); its fixture-only readability refactor preserves the nine-marker runtime sequence. See [[sound]].

## [2026-09-29] test | Rendered UI-scale gate

Final gate PASS (`/tmp/claude/verify-ui-scale-all-owners-final.md`): inspected owned-UDP Wayland/Vulkan root-viewport captures prove tooltip 1280/800 edge bounds and Merchant Options 0.75/1.25 physical-pointer input. PNGs are valid, non-empty sRGB with distinct pixels. Retained `b63c64ec` native/fixture is bounded by a range diff showing only later M2/combat Rust through `fcff316a`; no current-whole-engine artifact claim. Login/reset-headless proof remains; entrance stays source-only. See [ui-system](systems/ui-system.md#godot-native-ui-scale-bounded).

## [2026-09-29] investigation | DXT1 punch-through alpha

Elwynn bush 189700's leaf texture 189937 (DXT1, alpha depth 1) drew black squares in the Godot client: Godot uploads `FORMAT_DXT1` as BC1 RGB, so punch-through texels were opaque black. `fe422318` decodes DXT1 with alpha bits to RGBA8 with every mip level. See [[godot-dxt1-punch-through]].

## [2026-09-29] investigation | DXT1 punch-through alpha on master

On master `56a134a6`, Northshire captures show no remaining black foliage cards. The trees and plants use DXT5 leaf textures. No local DXT1 BLP without alpha bits has punch-through texels. Black cards reported after `fe422318` come from pre-fix builds. See [[godot-dxt1-punch-through]].

## [2026-09-29] feature | Unit frames at the Modern preset, retail class bar

Player and target frames now sit at Retail's Modern Edit Mode preset. The target health bar uses the `CheckClassification` anchor (`TargetFrame.lua:419`), not the XML default, which had placed the target 12 px high. The class bar is anchored like `PlayerFrameBottomManagedFramesContainer`, and bars are gated by the Retail `spec` KeyValue. Arcane Charges draw the full `ArcaneChargeTemplate` art and animations. Live proof: `godot/tests/player_class_bar.gd`, with captures in `data/diagnostics/resourceorbs-2026-09-29/`. See [ui-system](systems/ui-system.md#unit-frames).

## [2026-09-29] feature | Auras on the Godot HUD

The Godot client shows the player BuffFrame/DebuffFrame and the Retail TargetFrame aura container ([buff frame spec](../specs/buff-frame.md)). The player's own auras are large (21 px). On a hostile NPC, other players' debuffs are hidden. Timed icons get the reverse cooldown swipe with its edge. Countdowns run on wall time. The Retail PlayerFrame draws no aura icons. The shared `aura_display_data` keeps the replicated slot order that BuffFrame uses. Before this, the Godot client drew no auras at all, and the server marked Polymorph on the neutral Blackrock Spy as a buff (game-server `70551a9`, effect positivity). Live proof: `godot/tests/auras_live.gd`, with captures in `data/diagnostics/auras-2026-09-29/`.

## [2026-09-30] investigation | Torch billboards and M2 point lights

The particledebug torch's golden halo is a quad on spherical billboard bone 1 and its flame emitter sits under billboard bone 2; the Godot client had no bone billboarding, so the halo was edge-on. M2 point lights were never rendered. Both now follow WebWowViewerCpp/solarityclient; retail forces torch attenuation to 1.667-5.267 yd. Startup also stopped failing `add_child` on the root viewport (NativeTaa/bloom/RCAS never attached on normal launches). See [[godot-torch-rendering]].


## [2026-09-30] feature | Spell assets load off the main thread

A spell's first use no longer extracts, parses or decodes its kit models, textures and sounds on the main thread. Before, a first Flash of Light spent 117-283 ms per frame on spell visuals, and the CASC resolver init (1.5-1.7 s) could land on the first cast. Now a worker loader with prefetch priorities does this work. A late asset joins its kit's timeline at arrival, and the local player's known spells are prefetched. A first-use Flash of Light played every sound on time, with at most 30.4 ms of spell-visual time per frame and a 1.493-1.573 s precast for a 1.5 s cast. See [spell-visuals](systems/spell-visuals.md#asset-loading-godotrustsrcspell_assetsrs-godotcoresrcasset_loaderrs).

## [2026-09-30] fix | Retail ADT water

Northshire streams used the procedural placeholder water shader and lost MH2O LVF 0 depths. Godot water now ports WebWowViewerCpp `liquidWaterMat` with LiquidType/LiquidObject/LiquidTypeXTexture DB2 inputs and LightData/LightParams colours. See [northshire-pale-water](investigations/northshire-pale-water.md).

## [2026-09-30] audit | Native loot integration, proof pending

Recorded shared original loot state/cards/placement/actions, one ordered LootChannel relay, server-owned Auto Loot XOR Shift, actual authored LootFrame and per-looter sparkle/cursor; existing authenticated inventory/gold flow retained. Actual native fixture remains RED (no LootUnit after corpse right-click, case 1); main build and agent1299 portable export pending. Both matrix rows remain Missing; no completion checkbox changed. See [native loot boundary](systems/godot-conversion.md#native-loot--implemented-proof-pending).

## [2026-09-30] documentation | Native Options and loot money overflow; final pending

Updated existing [[godot-conversion]], [[loot-and-flight]], index and loot spec. Saved rendered RED: HUD last row 12 px past panel; money native 51 px versus authored 38 px (font size 12, glyph height 15, default gaps 3). Records content-driven Options height `b50a139f` and fixed multiline gap fitting `d6f39c45`, without smaller fonts/truncation/clipping; main rendering pending, no GREEN claim. Replaces stale export/build/agent-pending wording with shared exports/tests 8 + 1, relay wire test 1 at `c6ae14bf`, root compile and runtime `af03660f` all four cases/inventory/error/cursor through LOOT_DONE. Post-DONE RenderingServer-null exit 101 unresolved/deferred; independent final boxes pending, source unfrozen. Docs only; no builds/tests/delegation or source/server/protocol/data/PLAN changes. Existing log entries preserved.

## [2026-09-30] evidence | Native loot/caption final main reconciliation

Updated loot spec, Partial matrix rows, [[godot-conversion]] and index for `292a2fb2`/Depot `tt4c247nl1`, latest `/tmp/claude/native-ui-caption-run.log`: 42 Options records/no overflow, three visible money lines/contained shadow and main-inspected Items/stack 2/Poor captions. Four Auto Loot cases, InventoryFull reject/retry, authoritative bags 11/money 32756, matching removals/closure, duplicate chat once and empty-corpse target-only reach LOOT_DONE. Full exit 101 after DONE is fixture timeout; prior af03660f RenderingServer-null retained separately. Caption-2 RED corrected by width caps on all fixed axes, height caps only on spacing-fitted explicit multilines; no glyph clipping/font shrink. Verifier 1314 report absent at reconciliation, no PASS credited. Exact range/living-NPC runtime, corpse-pose parity, clean acceptance/full conversion open; shutdown explicitly deferred, source unfrozen. Supersedes older pending build/export/main-rendering entries; preserves AA/shutdown/cache evidence. Docs only; no tests/build/delegation or source/data/PLAN changes.

## [2026-09-30] fix | Showcase client bugs

Floating combat text starts at per-number camera-plane offsets from the retail WorldText CVars; the target's nameplate takes `nameplateSelectedAlpha`; robes select skirt/sleeve geosets and paste over shirt and pants in `CCharacterComponent` priority; player weapons sheathe at `Item.SheatheType`; creature poses follow `AnimationData.Fallback` (Dead → Death held). `.anim` out-of-bounds reads trace to stale cached `.skel` files. See [showcase-client-bugs](investigations/showcase-client-bugs.md).

## 2026-09-30 — SettingsReload reconciliation

Linked [accepted bounded two-process proof](systems/godot-conversion.md#native-settingsreload--bounded-two-process-proof) from fixture workflow, loot/conversion specs, matrix and index. Accepted `/tmp/claude/verify-native-settings-reload.md`: bounded saved-artifact functional reload and scoped Rust formatting PASS at `fc303c77`. Byte equality only at post-spawn/post-load observation boundaries; both children deliberately SIGKILL/reap/join, not normal shutdown. Saved build provenance is caller-supplied; parent exit 0 lacks a log footer. Partial/full conversion and shutdown gaps remain open; inherited 42 Options records do not upgrade all-options/geometry acceptance.

## [2026-09-30] fix | Audio/visual parity gaps (avfix)

M2 point lights fall off as retail's squared linear ramp; melee sounds apply the reverse-engineered 1.12 rules (miss whoosh, exertion and injury chances, chest armour, Material flags; dagger size 8 and the Pierce columns stay unknown); WMO group liquids draw with their LiquidType materials; scene fog is the full `makeFog2` (height, artistic, end and sun fog). See [godot-torch-rendering](investigations/godot-torch-rendering.md), [spell-visuals](systems/spell-visuals.md#melee-sounds), [northshire-pale-water](investigations/northshire-pale-water.md#wmo-liquids-mliq), [retail-lighting](systems/retail-lighting.md#scene-fog-godot). Water specular power stays 1.0: no retail source.

- 2026-09-30: Docs-only standalone bag tooltip checkpoint: `7e7b70af` saved build/actual hover RED, committed `ac7c16d7`, test-only `1c00b32a`; extraction/integration compile/GREEN/independent gate pending. [Owned evidence](systems/godot-conversion.md#standalone-bag-item-tooltip--red-integration-proof-pending); accepted `ddc318d8` foreign gate and exclusions preserved. No builds/tests/operations.

- 2026-09-30: Docs-only followup to `f35996cd`: shared original extraction `0e41239a` committed alongside native `ac7c16d7`/test `1c00b32a`; actual source audit names root/ui-model exports and Bevy record/anchor adapter. [Owned checkpoint](systems/godot-conversion.md#standalone-bag-item-tooltip--red-integration-proof-pending). Main Depot build/runtime/pure proof, walkthrough and independent verifier1415 pending; no accepted tooltip GREEN. Prior foreign accepted PASS, inherited findings/exclusions and whole goal open preserved. No tests/builds/delegation/operations.

- 2026-09-30: Reconciled cursor formatter/unit-tooltip authored-screen source paths after extraction walkthrough became available. MAIN reports Depot `ksb906c316` native compile0, existing WMO warning only; runtime/CPU tests/verifier1415 pending, no accepted tooltip GREEN. [Owned checkpoint](systems/godot-conversion.md#standalone-bag-item-tooltip--red-integration-proof-pending); prior accepted evidence/exclusions unchanged.

- 2026-09-30: Docs-only reconciliation records MAIN-observed bounded tooltip GREEN for native `ac7c16d7` + shared `0e41239a`, tests `1c00b32a` + oracle correction `88b0505f`: actual corrected runtime, CPU native5/shared4, registry2, same-build drag/actions/cursor and MAIN-opened Linen/Poor captures. [Owned evidence](systems/godot-conversion.md#standalone-bag-item-tooltip--accepted-bounded-pass) retains authentic RED101, labels first GREEN attempt101 false oracle (not production failure), exact exclusions/inherited12 findings and intentional kills (not shutdown). Verifier1415 accepted bounded PASS after MAIN read the full report; original pure gear proof is not native gear hover. No source/PLAN/data edits, tests, builds, delegation or runtime operations.

- 2026-10-01: Docs-only saved-proof reconciliation updates [auction house](systems/auction-house-ui.md#saved-native-runtime-proof-2026-10-01) and [native receiving mail](systems/trade-and-mail.md#saved-native-receiving-proof-2026-10-01), linked specs and index. Inspected root CLI receipts, seller run 3, buyer, large browse and mail run 6 logs plus fixture assertions: bounded trading, 311/87 global/category groups, disjoint 50/50 pages, actual mailbox 197134/display 1907/model 199999, authoritative claims and quiet reopen. Records faction normalization `dc638c8e` RED 0-versus-1/nine-model-test GREEN and terrain/replica lifetime fix `90dc2ccf`, followed by camera-position correction in the owned fixture. Main inspected large-market captures; this update did not repeat that visual review. Retains LiquidObject 42/local-CASC/icon errors, two unavailable icons and unproved broader rendering/performance/shutdown/full conversion. No code, data, operations, tests, builds, delegation or push.

- 2026-10-01: [Native player mail](systems/trade-and-mail.md#native-player-mail-2026-10-01): full native MailFrame (Send Mail, C.O.D., Return/Delete, Reply, Open All, minimap indicator) and the live two-client private-server proof including a server restart; `PendingMail` now survives Loading and map changes.

- 2026-10-01: [Native player mail proof](systems/trade-and-mail.md#native-player-mail-proof-2026-10-01) rerun after merging master 3f0779e6 (uiown bags/Escape): Reply and Open All now live; Reply edit-box fix `ad8553f3`.

- 2026-10-01: Bounded [native JS reconciliation](systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail): independent1542 accepts Login/frame-deadline continuation and four readability fixes; root compilation PASS, overall FAIL on transferred quest rootfmt spacing. MAIN offline/world runtime0 covers ten variants combined, independent1547/1548 pending. Linked spec/matrix retain semantic, omitted-water, shutdown and full-conversion gaps; no source/tests/operations.

## 2026-10-01 — Native JS bounded acceptance reconciliation

[JS evidence SSOT](systems/godot-conversion.md#native-js-automation--bounded-login-green-overall-gate-fail) reconciles accepted1550 offline source-equivalent cleanup and1557 current world source/order/runtime17/17 readability. Ten combined variant examples only; historical failures, rootfmt FAIL, omitted-water errors/warnings and deferred shutdown retained. Broader goal OPEN; no new page/index change.

## 2026-10-01 — Race/sex item files, collections, sheath links, emotes (charequip)

[Race and sex item files](systems/character-rendering.md#race-and-sex-item-files-2026-10-01): Component*FileData texture/model selection, ChrModel body chain, wowdev geoset group table, native sheath links, skinned collections with both model columns, player social emotes; named-character and 62-way race fixtures.


## 2026-10-01 — Player stand state (standstate)

[Animation](systems/animation.md): players hold the replicated `PlayerStandState` pose (SitGround 97, Sleep 100, SitChairLow/Med/High 102-104, KneelLoop 115) with down/up clips 96/98, 99/101, 114/116 on a change; X (`SITORSTAND`) and /sit, /sleep, /kneel send `StandStateIntent`; sit/sleep/kneel no longer play from `EmoteEvent`. Chairs (`GAMEOBJECT_TYPE_CHAIR`) render and right-click seats. Live: `godot/tests/player_stand_state_live.gd` (sitter, observer, `STAND_FOOD=1` bread phase); evidence `data/diagnostics/standstate2-2026-10-01/`.
## 2026-10-01 — LiquidObject IDs without DB2 rows (liquidobj)

[Resolved](investigations/northshire-pale-water.md#liquidobject-ids-without-db2-rows--resolved): IDs 42 and 13134/13136–13139 have no LiquidObject row in the build or its hotfixes; 42 is the ocean object (4.5M layers, 458 maps). Row-less objects take their MH2O liquid_type (WebWowViewerCpp `getLiquidObjectData`), and LiquidType 2 Ocean object layers are LVF 2 depth-only. A world-wide scan of 52,882 root ADTs leaves 0 omitted layers. Live Adventurer's Rest: 133 errors → 0.

## 2026-10-01 — Grounded character/clothing pixels (appearpix)

[Grounded appearance pixels](systems/character-rendering.md#grounded-appearance-pixels-2026-10-01): independent DB2/texture oracle and close-up pixel test over the player loader; fixes to item alpha, PasteScale, mipmaps, eye layers and slots, translucent canvases, full HD body canvas, group-0/ears/face geosets, Eyesight for every class.

## 2026-10-01 — Native IPC request coverage and semantic ExportScene (tooling)

[Native IPC request coverage](systems/godot-conversion.md#native-ipc-request-coverage): semantic ExportScene per screen, map target/waypoint auto-walk, group/emote/spell, quests, items, presence, character stats, trade (deferred replies), combat log; per-request not-ported reasons; index-based terrain chunk lookup fixing the zone-0/no-height gap.

## 2026-10-01 — Character select and Enter World loads off the main thread (stalls)

[Character select and Enter World loads](investigations/world-entry-stalls.md#character-select-and-enter-world-loads--2026-10-01): character model on a worker, no listfile for item models, sound tables on a thread, minimap tiles on a worker, loader drops no longer join workers, mip chains on the worker. Remaining: uncached main-thread UI texture decode.

## 2026-10-01 — Godot Wayland exit hang (investigation)

[godot-wayland-exit-hang](investigations/godot-wayland-exit-hang.md): quit() hangs root-caused to Godot 4.7.2 WaylandThread::destroy() roundtrip race; upstream PR #123946 removes it (0/180 vs 14/190).

## 2026-10-01 — Zaralda appearance and rigid-waist evidence (task-date update)

Updated [model/gear investigation](investigations/npc-stance-gear.md#zaralda-rigid-waist-binding-verified-task-date-2026-10-01), fixture reference and index. Independent appearance report preserves 113,120 rows; unavailable encrypted geosets exclude completeness. Recorded matched protocol/InWorld, actual binding failure, fix 016b0fce/207aea4e and supplied Depot z7mpr71q0v CPU 27/27 only. Commit timestamps are 2026-10-01 -0500; task evidence date retained, not a new verification timestamp. New native build/window gate pending main; overall goal open. Docs only; no tests/builds/commits.

## 2026-10-01 — Zaralda derived-friendly/native boundary (evidence date)

Recorded supplied retained binding PASS, 116-mesh/frustum/posed-torso native evidence and body-point/cold-setup fixture revisions 17d77fcb/d2084de7. Linked server faction SSOT; merchant-window acceptance remains pending main. Evidence dated October 1, not a new runtime verification timestamp. Docs-only update; no code/data/PLAN changes, commits, builds, tests or service actions.

## 2026-10-01 — Zaralda final observed-proof reconciliation (task evidence)

Updated existing Zaralda SSOT and linked specs/reference/index: native-friendly exit0 / 529.10s now observes actual merchant pick/title/tooltip/backpack/close. Retained historical RED attempts, independent gate114 source/import proof and renderer/cache limits; gate116 artifact audit gives scoped PASS for the saved native flow and three current AH receipts. Server SSOT separates current three AH receipts from pinned old-binary performance. Prior uncommitted docs preserved; no tests, builds, commits, data/source/PLAN edits or service operations.

## 2026-10-02 — Godot in-world frame time (frameperf)

Added [godot-inworld-frame-time](investigations/godot-inworld-frame-time.md): benchmark tooling (on-CPU, GPU, renderer areas, symbolized perf) and the material-animation uniform-rebuild fix `9cf5cc0b` with before/after per scene.

## 2026-10-02 — Shader compilation ahead of need (stalls)

Added "Shader compilation ahead of need" to [world-entry-stalls](investigations/world-entry-stalls.md): used M2/WMO/terrain/liquid shaders recorded in `user://used_shaders.txt` and compiled during startup, login and loading; campsite catalog off the main thread. Idle character select max frame 136 ms cold, 54-79 ms warm.

## 2026-10-02 — Shadow and depth passes

Updated [godot-inworld-frame-time](investigations/godot-inworld-frame-time.md): interior WMO groups/doodads cast no shadows, two cascades (retail `shadowNumCascades 2`), depth pre-pass off. Stormwind idle p50 45.9 -> 34.9 ms, shadow draws 2.46k -> 1.40k.

## 2026-10-02 — Bevy client retired

User decision: the Godot client is the only client. The root Bevy package `game-engine` (src/, Bevy tests, shaders, release Dockerfile/deploy.sh) is deleted; the 312 root files godot crates compiled through `#[path]` moved into their owning `godot/<crate>/` (shared ones in `game-engine-core`), `game-engine-cli` moved to `godot/cli` (`depot-build.py --cli`), and `png_to_ktx2` plus the cache importers the client reads moved to root `tools/` (`game-engine-tools`). Wiki pages citing `src/` paths describe the pre-retirement tree. Deleted (git history keeps them): zone_name, sound_music_zone and particle_color cache importers, `lightdata_convert`, `blp_to_pam`, `blp_to_ppm`, `debug_blp`, `benches/parser_benches.rs`, `examples/ui_demo.rs`, `scripts/run_screenshot_regression.sh`, `scripts/run_skybox_screenshot_regression.sh`, `scripts/capture_skybox_validation.sh`. Root `deploy.sh`, `Dockerfile` and `scripts/windows-dev.ps1` are kept pending a user decision; they build the deleted Bevy package and are broken until then. See [godot-conversion spec](../specs/godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

## 2026-10-02 — Elastic tree prototype source audit

[[elastic-trees]] records `0290ca95`, `7cf63c9b`, `32e9726b`: manual Barrens FDID 201394 annotation, two-branch limit, loader/render ownership, native swept flight contact and damped recovery. Spec inventory and fixture assertions updated; pivots/regions provisional pending screenshot inspection. Mount/terrain/collision cross-links reconcile annotated capsule exception only. No tests run or native acceptance claimed; main owns integration proofs.

## 2026-10-03 — Godot client deploy

Root `deploy.sh` now ships the Godot client instead of the Bevy binary: `depot-build.py --release` builds the optimized extension, and the Linux x86_64 bundle holds the pinned patched Godot runtime, project files with a pre-imported `.godot/` cache, and an allowlisted `data/` (38 GB). It publishes through the live S3-backed file server. The removed Bevy files are `Dockerfile`, `scripts/windows-dev.ps1` and `docs/windows-development.md`. A cold `--import` that hot-loads the extension aborts at exit, so the bundle registers the extension first. See [deploy](../deploy.md).

## 2026-10-03 — Desktop/local build trial contract

[[build-hosts]]: replaced Depot-only guidance with approved desktop SSH/WSL and local Docker selection, saved-default/error contract, and retained snapshot/cache boundaries. Linked conversion spec and architecture; real builds/tests and desktop server/manual GPU acceptance remain pending. Historical Depot evidence preserved.

## 2026-10-04 — XP HUD

[[xp-bar]]: Retail experience bar screen for both presets, mounted from the owner-only `PlayerXpUpdate`. Forever geometry measured from the reference screenshot (596×17 gamepad container at the top centre); no protocol change. CPU tests only, no live capture.

## 2026-10-04 — Turned UI textures

[[ui-rounding-seams]]: a texture turned about its centre keeps its exact rect through layout; fixes the 1 px step between the Forever XP bar's border edges and corners. Live capture `data/diagnostics/xpborder-2026-10-04/`.

## 2026-10-06 — Party presentation settings

[[party-edit-mode-settings]]: exhaustive local Retail Party inventory, both-skin Options HUD controls and per-character Edit Mode persistence. Compact stays default. At `6df0d0d3`, 1 native + 10 UI-model tests and six real-engine cases per skin pass; all eight final captures inspected. Corrected drawable-border syntax, compact self-pet lookup and health-only pet bars. Global shutdown warnings remain unclassified. Big-defensive classification is absent from existing runtime data; no full completion claim or protocol/server changes.

## 2026-10-05 — In-world launcher

[[launcher]]: added user's centred search grid, shared micro actions/icons, Toggle Launcher binding and minimap opener. Micro menu retained. Targeted tests and live proof pending; Support remains a placeholder.

## 2026-10-06 — Private live-run and warm-slot documentation

[Private headless live client](../headless-live-run.md) generalises saved private-server/Weston/Dozen runs, account and token isolation, capture entry points and owned-process cleanup. [Warm-slot rule](../remote-builds.md#warm-slot-rule) records path-keyed caches, fixed-slot branch reuse and global build serialisation; [[build-hosts]] links current GC config instead of retaining stale copied limits. Source/record review only; no builds or live rerun.

## 2026-10-06 — Forever preset wiring

[[forever-preset]] records current master skin/layout wiring, set-1 atlas selection, explicit product-file boundary, character-scoped persistence and FlareUI reference restrictions. Current active skin remains process-wide; pending `skinctx` is not described as integrated. HUD measurements remain in the contract and party-sheet provenance in [[portrait-party-frames]].
## 2026-10-05 — Guild rank settings model

Added portable authoritative rank controls and Officer chat parsing/classification. [Guild ranks spec](../specs/guild-ranks.md) records pending native transport, settings widgets in both skins and member menus. No native guild-settings rendering or live proof claimed. Five targeted model tests pass at `61e129e4`; Godot/UI-model/network test compilation succeeds. The additional hidden-tab RED reproduced loss of independent deposit/stack settings; GREEN preserves them.
## Live quest rerun — questrun

[[quest-ui]]: reconciled the stale tracker-only Godot description with the native dialogue/log host. Recorded private Modern/Forever gameplay proof, prior Jasperlode ground fixes, living-walk/death recovery, and six successful Milly interest returns without claiming the historical mirror cause. Documented real slow-frame input regressions and opaque post-draw error capture. Evidence: `data/diagnostics/questrun-2026-10-05/`; long-text overflow remains outside this repair.

## Sparse portrait FoV — 2026-10-06

[[portrait-party-frames]]: real Human female HD camera data reproduces the first-slot-only parser substituting one radian despite authored 0.785396576 keys in slot 2. Static snapshots skip empty slots; no race-specific zoom, clipping or light changes. Real/synthetic RED confirmed; production `e112494a` plus fixture `9f0e0719` passes eight core camera tests, eight authored head orientations and real Human/Blood Elf male/female skull framing/brightness. Jaina height improves 0.452195→0.585480; skin/hair textures unchanged. Both skins' five roster/lifecycle tests and inspected offline recaptures pass. Existing global shutdown leaks remain. Rejected hairstyle/typing oracles retained. Evidence: `data/diagnostics/jainaportrait-2026-10-06/`.
## 2026-10-06 — Quest overflow

[[quest-ui]] records the missing log scroll ancestor and dialog estimate/native-height mismatch, Retail scroll/QuestInfo sources, measured scroll-child feedback and native capture/input regressions. Evidence and current acceptance: `data/diagnostics/questoverflow-2026-10-06/`. Recorded quest trees came only from `capture_base_trees`; Options/menu suffix unchanged.
