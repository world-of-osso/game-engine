# Shipped assets

## P1 runtime contract

Select `GAME_ENGINE_ASSET_MODE=extracted-only` in the launcher/player environment.
Unset means `local-casc`, preserving development behavior. Unknown values fail
startup. Selection precedes every asset reader and is immutable for the process.
All files needed by a shipped game are extracted ahead of time; runtime must not
require a WoW install or CASC.

`asset-resolver::AssetRuntimeMode::{LocalCasc, ExtractedOnly}` and
`AssetResolverConfig::with_runtime_mode` are shared with model/appearance isolation.
Extracted-only skips the startup CASC worker. Resolver initialization is a no-op;
local cache hits load normally, without loading a listfile for extraction.
The single `guard_casc_access` policy check protects low-level CASC entry and
install discovery, including direct resolver construction and terrain raw DB2
requests. No archive, root, encoding, keyring or listfile-driven extraction occurs.

Required misses return errors containing mode, FDID (and authored product/build
when present), and the exact expected path. Checked APIs propagate these errors.
Legacy optional cache APIs return `Result<Option<PathBuf>, String>`: extracted-only
misses return the same error as the checked API, never panic or silently return
None. Local-CASC extraction failures retain their logged `Ok(None)` behavior.
Detached AssetLoader requests propagate required texture/animation errors to an
`Err` completion; polling marks the key Failed (not Loading), delivers the error
once for caller logging, and does not retry that key. `forbidden_casc_access_count()` records forbidden low-level entries;
a test hook can panic immediately, and cold-process tests assert zero even if a
caller catches or logs an error. An ordinary extracted-file miss is not a CASC
entry and does not increment this count.

## P4 no-install certification run

This section defines the P4 certification run. The run is a bounded native
acceptance gate for `extracted-only`; it does not by itself complete P4.

### Environment

- `GAME_ENGINE_ASSET_MODE=extracted-only` for the whole process.
- `WOW_INSTALL_PATH` points at an empty directory that only has `Data/data` so
  install discovery succeeds. The directory contains no CASC archives; the run
  proves nothing was read from an install by asserting
  `forbidden_casc_accesses == 0` and by emitting no CASC-forbidden error.
- One native Godot process per pose, pinned editor/runtime build, headless cage,
  1280x720, fresh output directory per run.
- Each pose is one process. A process covers one zone; poses are not reused.

### Zones and poses

The pose is the Godot world position `(x, y, z)` of the requested viewpoint.
`y` is height; the streamer selects the tile from `x` and `z`.

| Zone | Map directory | Pose | Mode |
|---|---|---|---|
| Northshire | `azeroth` | `(-8914.57, 80.5, 133.909)` | terrain |
| Stormwind | `azeroth` | `(-8841.06, 109.6, -489.656)` | terrain |
| Dun Morogh | `azeroth` | `(-6240.0, 382.8, -331.0)` | terrain |
| Orgrimmar | `kalimdor` | `(1798.27, 102.5, 4363.27)` | terrain |
| Mulgore | `kalimdor` | `(-2915.55, 59.45, 257.347)` | terrain |
| Durotar | `kalimdor` | `(-618.52, 38.7, 4251.67)` | terrain |
| Stockades | `StormwindJail` | `(56.68, -19.27, -0.62)` | global WMO |
| Zephras Isle | `2991` | authored tile centre | terrain |
| City of Dalaran | `2959` | authored tile centre | terrain |
| Dalaran City | `2980` | authored tile centre | terrain |
| Nightmare Grove | `2832` | authored tile centre | terrain |

Forever poses use the authored-tile centre chosen by the run; the capture
records the resolved pose.

### Pass criteria per pose

A pose passes only when every criterion holds:

1. Process exits normally (code 0, no crash) and the runtime status reports
   mode `ExtractedOnly` with `forbidden_casc_accesses == 0`.
2. No emitted asset error: no `missing extracted-only asset FDID`, no
   `No verified asset receipt`, no wrong-product or CASC-forbidden line.
3. No asset fallback: no substitution of another product's bytes, no legacy
   flat path used where the source contract requires a scoped receipt, no
   placeholder asset. Documented renderer/material fallbacks that do not
   substitute asset bytes are recorded separately and listed in the report:
   the Material130 LiquidType5 water fallback and the dummy audio driver.
4. Terrain poses: at least one authored tile parsed, zero terrain failures,
   `spawned > 0`, `pending == 0`, `failures == 0` after the queue quiet window.
5. Global-WMO poses: `global_wmo_fdid > 0`, zero terrain failures, at least
   one visible mesh, `pending == 0` and `failures == 0` after the queue quiet
   window.

### Miss classification

Every requested FDID with an asset error is recorded by zone, product, kind and
class:

- `locally_fixable`: the FDID exists in the pinned local CASC resolution for
  that product/build. Extract or receipt it with the existing closure scripts,
  then rerun the same pose.
- `absent_from_build`: the FDID is absent from that product's current CASC root.
  This includes the recorded 2,126 Retail root-absent identities and the nine
  Forever WMO-doodad FDIDs absent from the Forever build
  (`192262,194853,243074,243154,243158,243166,243169,243172,603672`); the exact
  run records the subset it observes.
- `engine_bug`: the requested FDID, path or product does not match the authored
  placement or metadata row (a wrong-ID request). One commit per fixed bug.

A pose that still reports one of the first two classes and no engine bug is a
reported gap, not a passing pose. Only the three Forever poses named in the
report carry captures; captures do not substitute for the pass criteria.

## Later phases (not P1)

Round 3 offline dataset evidence (2026-10-10): **970,356 present /2,302 missing /5,024 unresolved**,188,100 new paths /4,967,995,591 bytes. [Final closure counts, receipts and residuals](../wiki/systems/offline-asset-closure.md#round-3-final-dataset--verified-2026-10-10) own the proof; this does not complete P3/P4 or pristine no-install acceptance.

- **P2 (implemented):** Authenticated product/build identities, receipts and
  manifest-qualified model bytes; same-FDID chains coexist without legacy-path
  substitution. [Model/product isolation contract](product-isolated-model-assets.md)
  owns requirements and bounded native acceptance; later closure is separate.
- **P3:** Offline fixed-point closure for one map and complete character/NPC/item/
  spell/audio/UI slice; pristine no-install bundle and deterministic missing-file
  failure.
- **P4:** Extend deterministic seeds to all supported content; zero unresolved
  closure, scenario recordings contained in the manifest, valid receipts.
- **P5:** Package only verified manifest members; remove deployment's install
  requirement, compile out CASC in release, sandbox startup/scenarios and validate
  download hashes. No deployment changes are part of P1.
