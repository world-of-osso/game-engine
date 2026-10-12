# Legacy global-WMO WDTs

Verified: 2026-10-12. The M2 full-rebake inventory's 193 `missing global WMO 0` failures were legacy WDT name-resolution failures, not missing WMO payloads. All authored roots, GFID groups and MODI models were already locally extracted.

## Format and correction

MPHD bit0 marks a global-WMO map. MODF flag0x8 marks a direct FileDataID. Without that flag, MODF's name ID indexes the WDT's null-separated MWMO names; normalize the authored path to lowercase/slashes and resolve it through the local community listfile. Preserve placement position, rotation, scale and active doodad set. Never rewrite WDT assets or invent a replacement FDID.

Example: WDT952203/map1212 carries `world\\wmo\\draenor\\human\\6hu_garrison_blacksmithing_hub.wmo`, MODF name0/flags0, resolving to root948620 and GFID948622. The baker formerly treated name0 as FDID0. The native core formerly parsed MODF with empty name tables, leaving neither FDID nor path. Native acquisition now resolves the preserved path and publishes the acquired root FDID in map diagnostics.

The standalone server baker retains its small chunk parser; native loading uses game-engine-core and its existing placement/listfile resolver. Sharing a resolver would introduce cross-workspace dependency coupling; both use the same authored MWMO-index and direct-FDID semantics.

## Terrain-less legacy WDTs

Thirteen non-global WDTs lack MAID but have complete 32768-byte MAIN chunks with zero active tiles: maps1232,1233,1236–1245,1471. These are terrain-less, not global WMO maps. The baker emits empty tile selections/report-only success. Missing MAID with active MAIN tiles or malformed/missing MAIN remains an explicit gap.

## Scoped evidence

Durable evidence: `data/diagnostics/global-wmo-assets-2026-10-12/`. Original316-map inventory:110 missing WDTs,193 global-WMO failures,13 no-MAID failures. All316 were invoked with `--all --m2-only --no-eyes` against a private slot view and SQLite snapshot. After:206 successful maps,110 missing WDTs,zero other failures/asset warnings.128 maps emit498 M2 tiles totaling6,983,272bytes;65 global maps and13 terrain-less maps have no authored admitted M2 floor tiles. Empty output is not invented collision.

The package contains704 floor/report files totaling7,058,879bytes, no new raw assets. `apply_cache_package.py` validates source/input/before hashes and relevant Map WDT bindings by default; only lead-owned `--apply` publishes. Five temporary-data behavioral tests cover validation-only, publication/backup/independent copies, source corruption, before-hash drift and input drift.

Local `casc-local` attempted105 unique missing WDT FDIDs covering110 maps using Retail build key `dcfc90fffd79ba00406ae46f5f657592`; none is available in its root. Identity comes from the imported Map WdtFileDataID and community listfile, not a guessed filename. No CDN/Wago/Wowhead or canonical raw-asset/world.db writes.

Real-WDT RED→GREEN tests cover legacy blacksmith geometry, real garrison M2 hulls, empty MAIN, active-MAIN refusal and the native client loader. Fresh private Godot clients render maps1213/1214/1258 in Modern and Forever at1280×720 with extracted-only mode and production WMO/doodad/lighting paths. PNGs: `/syncthing/AgentShared/2026-10-12/global-wmo-assets/`; JSON/logs stay in diagnostics. Captures establish asset/render loading, not authenticated instance gameplay, HUD parity, walking or full no-install certification. Map1214's overlapping authored building variants also appear in a fresh client; they are not proven cross-map carryover.

Offline previews do not enter Loading, which normally attaches global WMOs. The existing offline lighting hook now invokes that same attachment/adoption path. The capture fixture selects the production art layout directly without authenticating or changing saved account layouts.

## Sources

- [Baker WDT parser](../../../../game-server/crates/los/src/bake/adt.rs), [real regressions](../../../../game-server/crates/los/tests/legacy_wdt.rs).
- [Native WDT parser](../../../godot/core/src/asset/wdt.rs), [native map loader](../../../godot/rust/src/terrain/assets.rs), [capture fixture](../../../godot/tests/legacy_global_wmo_capture.gd).
- Local WebWowViewerCpp `wowViewerLib/src/engine/persistance/{wdtFile.cpp,header/wdtFileHeader.h}`: MPHD/global bit, MAIN active bit, MWMO/MODF chunks; actual WDT952203 supplies the legacy name reference.
- [M2 ground contract](../../specs/m2-floor-collision.md), [local CASC extraction](../../casc-extraction.md).

## See Also

- [Offline asset closure](../systems/offline-asset-closure.md) — broader no-install certification remains separate.
- [Player ground](../systems/player-ground.md) — shared authored floor companion/runtime behavior.
