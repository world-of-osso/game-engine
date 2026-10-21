# No-install certification — 2026-10-21

**Status: in progress; P4 remains open.** Scope remains all zones, races and items. Representative runtime visits cannot certify the complete catalog, every customization, or a pristine manifest-only player bundle.

## Evidence and isolation

Private worktree: `/home/osso/.worktrees/game-engine-noinstall`, branch `noinstall-cert`. Evidence: its `data/diagnostics/noinstall-cert-2026-10-21/`. Canonical asset publication belongs to the lead; this worktree stages payloads and provenance only.

Initial runtime source `4d266a933`; `build.log` records extension/CLI build success, `executables.json` their hashes. Each case records exact argv/environment, complete native output, observations, screenshot and summary. Native Vulkan runs through `agent-run` and `headless-client.sh` on a private cage. Bubblewrap mounts the host read-only except owned runtime paths, hides the configured install/CASC/resolver-cache paths, and isolates networking for offline cases. This is configured-path isolation, not an exhaustive successful-access/syscall trace or immutable manifest-only bundle.

The real `WowAssetLoader.asset_runtime_status()` is recorded per observation: every observed count is zero and mode `ExtractedOnly`. Early fatal startup originally prevented a counter readback for `debugcharacter`; do not infer zero from missing observations. Initial login setup failures are retained separately in `login-setup-invalid/`; they do not enter the asset inventory. Complete private data links fixed those harness-only misses.

## Initial runtime coverage

| Case | Status | Exact boundary |
|---|---|---|
| Login | Bounded PASS | Native screen captured, no emitted engine errors, tripwire zero. No authentication in this case. |
| Character creation | GAP | All 31 catalog race entries × two bodies selected correctly; 62 captures/observations, many absent models and required receipt errors. Defaults only, not all classes/customization combinations. |
| Character select | Pending | Requires owned authenticated server/character. |
| M2 debug | Bounded PASS | Reference wolf visible, three mesh descendants, no emitted errors, tripwire zero; interactions not certified by this sweep. |
| Selection debug | Asset-I/O subset only | Screen visible, no emitted errors/tripwire hits; zero descendant model meshes, so not a model-rendering certificate. |
| Particle debug | Bounded PASS | Visible authored emitters/models, three mesh descendants, no emitted errors, tripwire zero; all toggles/emitters not exercised. |
| Debug character | GAP | Startup stops on required equipped-character texture receipts; no positive counter/model proof yet. |
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
- Round1: all 192 legacy payloads match authenticated Retail70448 content keys, 74,918,722 bytes. Lead reports re-extraction and no-clobber canonical publication with `model-asset-index-noinstall-cert-round1.json`. A subsequent run exposes further texture layers; this was not a fixed point.
- All 62 numeric skins match owner SFID relationships and authenticated parent/child product roots. Private staging retains source alias and per-file provenance; 65 scoped receipts cover 62 Forever70450 identities plus three identical Retail70448 identities. Lead publication/recheck pending.
- Round2 preparation selects a conservative all-choice material-texture superset from the current versioned customization cache, excluding existing indexed Retail textures, plus the observed debug-character miss. This does not certify all model/item descendants. Staging/publication/runtime recheck remain pending.

## Remaining acceptance

Owned protocol-matched authenticated server/client runs, receipt/skin publication rechecks, all-class/item/spell descendants, full-catalog closure/provenance reconciliation and pristine player-bundle/download acceptance remain unqualified. Historical graph counts and metadata-only `36 profiles / 0 certified` evidence are documented in [offline asset closure](../wiki/systems/offline-asset-closure.md); they are not replaced by this bounded sweep.
