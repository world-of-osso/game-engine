# No-install certification — 2026-10-21

**Status: in progress; P4 remains open.** Scope remains all zones, races and items. Representative runtime visits cannot certify the complete catalog, every customization, or a pristine manifest-only player bundle.

## Evidence and isolation

Private worktree: `/home/osso/.worktrees/game-engine-noinstall`, branch `noinstall-cert`. Evidence: its `data/diagnostics/noinstall-cert-2026-10-21/`. Canonical asset publication belongs to the lead; this worktree stages payloads and provenance only.

Initial runtime source `4d266a933`; `build.log` records extension/CLI build success, `executables.json` their hashes. Each case records exact argv/environment, complete native output, observations, screenshot and summary. Native Vulkan runs through `agent-run` and `headless-client.sh` on a private cage. Bubblewrap mounts the host read-only except owned runtime paths, hides the configured install/CASC/resolver-cache paths, and isolates networking for offline cases. This is configured-path isolation, not an exhaustive successful-access/syscall trace or immutable manifest-only bundle.

The real `WowAssetLoader.asset_runtime_status()` is recorded per observation: every observed count is zero and mode `ExtractedOnly`. Early fatal startup originally prevented a counter readback for `debugcharacter`; the updated diagnostic records the root's exit signal and observes zero on its round3 fatal startup. Do not infer zero for earlier missing observations. Initial login setup failures are retained separately in `login-setup-invalid/`; they do not enter the asset inventory. Complete private data links fixed those harness-only misses.

## Initial runtime coverage

| Case | Status | Exact boundary |
|---|---|---|
| Login | Bounded PASS | Native screen captured, no emitted engine errors, tripwire zero. No authentication in this case. |
| Character creation | GAP | All 31 catalog race entries × two bodies selected correctly; 62 captures/observations, many absent models and required receipt errors. Production race/body changes randomize appearances; not all classes/customization combinations. |
| Character select | Pending | Requires owned authenticated server/character. |
| M2 debug | Bounded PASS | Reference wolf visible, three mesh descendants, no emitted errors, tripwire zero; interactions not certified by this sweep. |
| Selection debug | Asset-I/O subset only | Screen visible, no emitted errors/tripwire hits; zero descendant model meshes, so not a model-rendering certificate. |
| Particle debug | Bounded PASS | Visible authored emitters/models, three mesh descendants, no emitted errors, tripwire zero; all toggles/emitters not exercised. |
| Debug character | GAP | Startup stops on required equipped-character texture receipts; round3 early-exit counter is zero, but no rendered-model proof. |
| Loading | Bounded PASS | Offline loading screen captured, no emitted errors, tripwire zero; no authenticated loading transition claimed. |

## Offline zone coverage

All cases below exercise production map identity, terrain, object and lighting readers through `preview_world_map`, not an authenticated local player/NPC population. Each ran in a separate process. Terrain presence is not healthy gameplay or complete zone coverage.

| Zone | Terrain/object status | Proof |
|---|---|---|
| Northshire | Bounded streaming PASS | Nine parsed tiles, zero terrain failures/pending; 23,021 meshes, 6,487 spawned objects; no emitted errors, tripwire zero. |
| Dun Morogh | Bounded streaming PASS | Nine parsed tiles, zero terrain failures/pending; 10,888 meshes, 3,040 objects; no emitted errors, tripwire zero. |
| Orgrimmar | Bounded streaming PASS | Nine parsed tiles, zero terrain failures/pending; 12,148 meshes, 1,888 objects; no emitted errors, tripwire zero. Coordinates preserve the historical representative visit, not a valid player-grounding claim. |
| Mulgore | Bounded streaming PASS | Nine parsed tiles, zero terrain failures/pending; 7,064 meshes, 2,587 objects; no emitted errors, tripwire zero. |
| Skyborne/Map2991 | GAP | Nine parsed tiles, zero terrain failures/pending; 3,814 meshes, 584 objects; missing numeric skins and four `Vertex 101 out of bounds` errors. Tripwire zero. |
| Stockades/Map34 | Bounded global-WMO PASS | Global-WMO interior, 1,105 meshes, no terrain tiles as expected; no emitted errors, tripwire zero. No dungeon lifecycle/NPC certificate. |
| Stormwind | Bounded streaming PASS | Nine parsed tiles, zero terrain failures/pending; 18,217 meshes, 4,198 objects; no emitted errors, tripwire zero. |

## Recovery and producer fixes

`misses-round1.json` preserves every emitted asset error from these cases, source line and last scenario marker. It contains 254 identities: 192 missing verified Retail texture receipts with existing legacy bytes, and 62 absent numeric skin paths. Four non-file vertex errors remain separate. Markers are best-effort attribution; asynchronous completions can belong to an earlier selection, and failed owners hide descendants.

- `9323d70c0`: legacy receipt authentication requires explicit product/build pins instead of obsolete hardcoded versions; only requested products are authenticated. Current-build synthetic root/publication and integrity tests: RED → GREEN18. No identity checks bypassed.
- `abc2de31b`: player and NPC closure seeds use the explicitly configured native customization cache, not obsolete `customization.sqlite`. Concrete stale-cache FDID99 versus current FDID8: RED → GREEN14. Maintained configs select `cache/customization-v5.sqlite`; its metadata origin remains separately unverified.
- Round1: all 192 legacy payloads match authenticated Retail70448 content keys, 74,918,722 bytes. Lead reports re-extraction and no-clobber canonical publication with `model-asset-index-noinstall-cert-round1.json`. Subsequent randomized sweeps expose additional material choices/layers; this was not a fixed point or an identical-profile comparison.
- All 62 numeric skins match owner SFID relationships and authenticated parent/child product roots. Private staging retains source alias and per-file provenance; 65 scoped receipts cover 62 Forever70450 identities plus three identical Retail70448 identities. Lead reports per-product CASC re-extraction/verification and publication of scoped plus numeric paths.
- Round2 publishes a conservative all-choice material-texture superset from the current versioned customization cache, excluding existing indexed Retail textures, plus the observed debug-character miss. Its 9,294 receipts comprise 9,229 Retail textures and 65 skins, 3,538,326,632 bytes. All 64 other selected Retail texture FDIDs have authenticated roots/local index entries but absent legacy bytes; local decoding remains pending. This is not all model/item closure.

## Post-round2 runtime recheck

Matching engine base `52e47cd5d` was merged into this branch and rebuilt against frozen protocol `03cebdb`; `build-current.log` succeeds without compiler warnings and `executables-current.json` records binary hashes. A private SQLite backup isolates writable listfile bindings from canonical data; prior read-only binding-persistence messages were sandbox setup limits, not missing payloads or CASC accesses.

- Creation: 62 valid race/body selections, **59 nonzero model observations**, five emitted receipt misses: Retail M2 `7760206` and Forever textures `7485094`, `7484364`, `7485028`, `7485156`. Four labeled sheets cover all 62 captures and were inspected. Production `apply_race_change_with_seed` randomizes appearances, so sweep counts are not equivalent-profile comparisons or full customization/class acceptance. Last-marker attribution remains asynchronous.
- Skyborne: numeric-skin misses disappear; objects **584→988**, meshes **3,814→4,840**. Four existing `Vertex 101 out of bounds` errors remain separate from file closure; no new asset miss emitted. Tripwire zero.
- Debug character: startup now reaches a further required Retail texture `2373869`; early-exit tripwire zero, model rendering still GAP.
- Own server source `0cc7b3866` builds successfully against frozen protocol `03cebdb`; inherited dead-code warning `followers/pull_history.rs::has_uncleared_fight` remains visible. Own admin build also succeeds. Private server first-tick/admin readiness and disposable five-character setup succeed; authenticated client proof remains pending. Private read-only world snapshot is 883,261,440 bytes; no live player database was copied.

## Post-round3 continuation

Lead reports round3 publication: 237 scoped assets plus 29 aliases re-extracted/verified, and 127 new/110 identical numeric paths, zero refused. All 64 previously absent Retail customization textures are included. The bounded debug-item graph expands after those model roots arrive: 757 assets, zero structural unresolved records, with 408 newly discovered absent children; this is only the twelve debug displays plus the observed collection model, not every item.

`debugcharacter-round4` now renders both equipped characters (57 mesh descendants), inspected native capture, no emitted engine error, normal shutdown and tripwire zero. Character creation still emits five Forever material receipt misses because later selections randomize appearances. Its product-local Skyborne cache contributes 1,086 texture IDs for models218/219; 873 remain unreceipted after round3, plus ten collection/dependency identities. Round4's explicit frontier is 426 Retail gear children and 883 Forever identities; local decoding/publication/recheck pending.

Private server setup initially stopped before any client when a PvP live window was observed. The corrected pair preflight reads the lease file even when capacity's ADD summary hides the owner; no authenticated client pass is inferred from account setup. The server is stopped between batches, with a 300-second watchdog and confirmed unbound private UDP5961.

## Published rounds4–5 and current bounded coverage

Lead published round4 after removing26 equal-identity overlaps:1,283 assets/408 aliases,1,707 new files. Producer commits `485814fb6` and `a2e407c56` now dedupe every canonical fragment before staged payload writes, reject conflicts/remaining duplicate assets and preserve existing legacy alias metadata (GREEN7). Numeric raw repairs remain separate from index ownership.

`creation-round5` reaches **62/62 valid race/body selections with nonzero models**, zero emitted engine errors and tripwire0; all four contact sheets inspected. Randomized appearance selection still prevents equivalent-profile or full class/customization claims. `debugcharacter-round4` remains the inspected57-mesh equipped-character proof.

Round5 authenticated33 assets/6 aliases/8 raw repairs locally, zero source/alias gaps. Lead re-extracted and published all39 fragment files and8 raw repairs without clobber, retaining212,774 unique canonical asset keys. Game-object skin505852 and skybox skin7345740 errors disappear in post-publication runs. `e9794c6c7` fixes dynamic game-object roots (GREEN5); `60262b0c8` fixes selected-map lighting roots through every LightParams slot, skybox and celestial FDIDs, including tile-free maps (entrypoint RED→GREEN3; full-root regression3).

### Authenticated representative visits

Engine base `52e47cd5d`, protocol `03cebdb`, own server `0cc7b3866`; per-case argv, fixture SHA, observations, complete logs and captures are retained. All cases below verify the requested roster character, positive player/NPC geometry and HP, mode `ExtractedOnly`, tripwire0 and normal shutdown. Owned UDP5961 server stops after each pair. Counts are observations, not whole-zone/catalog completeness.

| Case | Player/NPC meshes; HP | Current result |
|---|---|---|
| `auth-human-round2` / Northshire |14/93;292|No emitted engine error;9tiles.|
| `auth-orc-round1` / Durotar |15/49;293|No emitted engine error;9tiles.|
| `auth-stormwind-round2` |14/46;292|No emitted engine error;9tiles; landing/final error0.000015m.|
| `auth-dunmorogh-round1` |14/58;292|No emitted engine error;9tiles; landing/final error0.000489m.|
| `auth-orgrimmar-round1` |15/72;293|No emitted engine error;9tiles; landing/final error0.000130m.|
| `auth-tauren-round2` / Mulgore |15/54;295|No missing-file error;9tiles; spell81502/kit507 owner Unit4294897458 attachment34 lookup fails (two reported lines).|
| `auth-skyone-round2` / Skyborne95 |17/51;292|No missing-file error;9tiles; four existing Vertex101 errors.|
| `auth-skytwo-round4` / Skyborne96 |17/54;292|No missing-file error;9tiles; four existing Vertex101 errors.|
| `auth-stockades-round3` |14/96;292|No missing-file error; global WMO108631;5,048 zero-Basis errors; landing/final error0.000025m at authored entrance.|

Latest Mulgore/Skyborne captures inspected. They close the observed missing-file boundary, not overall rendering/gameplay acceptance. Mulgore rig samples contain attachment34 on every sampled model; the reported effect-owner ID never appears in those samples or visual-request logs. This does **not** establish that an owner rig lacks the attachment; lifecycle/visibility/lookup cause remains under investigation.

### Stockades repro classification

Earlier `auth-stockades-round2` requested engine `[82.3245,-26.5229,10.6106]`, but entered at `[56.68209,-19.26912,-0.62376]` (28.92m away). Generic cause: server `maps/login.rs::LoginMaps::resolve` replaces a saved dungeon position with `MapStore::entrance_location` when the instance copy is newly created; that selects the lowest-ID incoming trigger. This is not a missing asset or failed admin save. `auth-stockades-round3` expects the authored entrance `[56.6821,-19.2691,-0.62376]` and measures0.000025m error.

Zero-Basis errors reproduce on that valid landing with populated NPCs, not offline global-WMO streaming. First round2 error follows NPC display2992 visual completion and reports `core/math/basis.cpp:717 get_quaternion`; correlation does not identify the responsible model. Round3's sampled unit-root and model global determinants are all nonzero, falsifying a simple root-scale-zero explanation. Exact repro inputs are retained in `auth-stockades-round3/{argv.json,fixture.gd,observations.json,rig-samples.jsonl,client.log}` and `server/auth-stockades-round3-{argv,teleport}.json`. Engine rendering cause stays outside no-install fixes unless closure is demonstrated.

Invalid attempts remain explicit: early Stormwind used incorrect teleport axes; Skyborne96 round1 entered Human/Stockades and round2 failed selection. Neither counts as Skyborne96 proof. Corrected fixture uses keyboard roster selection and verifies the exact selected name before entry.

`proof-ledger.json` retains historical development-test scope/log hashes rather than claiming current-HEAD broad verification. Current bounded visits expose no further missing-file request; pristine manifest-only bundle, all-catalog fixed point and comprehensive classes/customizations/items remain unproven.

## Remaining acceptance

Owned protocol-matched authenticated server/client runs, receipt/skin publication rechecks, all-class/item/spell descendants, full-catalog closure/provenance reconciliation and pristine player-bundle/download acceptance remain unqualified. Historical graph counts and metadata-only `36 profiles / 0 certified` evidence are documented in [offline asset closure](../wiki/systems/offline-asset-closure.md); they are not replaced by this bounded sweep.
