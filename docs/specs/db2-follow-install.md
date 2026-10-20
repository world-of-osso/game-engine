# DB2 follows the installed build

## Contract

- Export the consumed Retail tables from the active local install, never remote CSV services or CDN data.
- Select column order, signedness, arrays and localized strings from the payload's exact layout in WoWDBDefs at a pinned commit. Unknown layouts fail before output.
- Record actual install version/build key, binary and DBD SHA-256, schema commit, readable row count and unreadable records in build-keyed receipts.
- Regenerate client caches and server CSV inputs in an isolated output tree. Preserve TDB1210 world.db; report its broken joins rather than rewriting server content.
- Trading Post outfit defaults read `db2/active/UiModelScene.csv`, including when no historical snapshot directory exists.
- Catalog consumers validate one declared source identity, not a compiled build number. Housing catalog keys follow their imported receipt; the property-storage namespace is unchanged. Renown reads `data/db2/active/renown.json` at server startup (`GAME_SERVER_RENOWN_CATALOG` selects an explicit input).
- Model-asset mappings follow the installed Retail build. A new-build receipt supersedes the prior receipt for the same product, FDID and kind, including unchanged bytes. Changed bytes use the new namespace; base and fragment indexes retain one mapping, not competing receipts.
- Runtime and player bundles select the current asset namespace, not a compiled historical build key. Build-test asset manifests select receipt-backed files by `model:<product>:<FDID>:<kind>`; missing or duplicate receipts fail instead of selecting historical files.
- Canonical DB2 and model-asset migration publication is lead-only: one runtime-build command with a fingerprint comparison and backup.
- Acceptance includes a row/ID census against former exports and native private hotkey captures for atlas7508739 in both skins.

## Current implementation

`export_db2_csv.py TABLE INPUT OUTPUT --definitions PINNED_DEFINITIONS` selects the payload layout. Fixed and sparse records both decode through the game-server checkout's `scripts/db2_casc.py` (`scripts/server_db2.py`: the engine's sibling `game-server`, or `DEPOT_SIBLING_GAME_SERVER`); a missing checkout fails explicitly, and the refresh command's `--server` must be that checkout. Definition acquisition and source receipts belong to the refresh command.

## Private refresh

Build `db2_cache_import` through `scripts/depot-build.py --root CHECKOUT --fixture db2_cache_import` before invoking `scripts/refresh_db2_data.py`. Supply the built executable as `--cache-importer`, plus the private output, exact definitions checkout, resolver/cache, baseline, read-only world, and canonical repository paths. The refresh reads canonical Forever inputs but never writes them; failed housing regeneration leaves the previous catalog intact. Prepared server SQL is reviewable output only, never automatically imported into TDB.

## Lead publication

Stop canonical clients/importers first. Run `scripts/apply_db2_install.py --manifest PRIVATE/apply-manifest.json --backup PRIVATE/canonical-backup --asset-data CANONICAL/data --asset-resolver CASC_LOCAL --asset-resolver-cache PRIVATE/resolver`. The command reads and authenticates the installed Retail identity at run time and refuses a reviewed DB2 manifest from another build. It stages all model indexes and companion aliases in the current namespace, reuses identical bytes through hard links, and extracts changed bytes from local CASC only. Absent current-root FDIDs are reported and removed from the live index; historical files remain unreferenced. Old chain metadata remains in the migration report, not relabeled as current provenance. Publication backs up every changed existing destination and records the combined DB2/assets fingerprint diff before any live write. Bytes/documents are staged together, indexes publish last, and a publication failure restores the previous complete state. Already-current fingerprints are no-op, including hardlinks; importer failures retain staging/recovery data. The new asset namespace and staging tree must share a filesystem. A completed backup is never reused.

## Tests

`scripts/tests/test_db2_install_export.py`: reordered new-layout fields and signed arrays; unknown layout refuses output.
