# DB2 follows the installed build

## Contract

- Export the consumed Retail tables from the active local install, never remote CSV services or CDN data.
- Select column order, signedness, arrays and localized strings from the payload's exact layout in WoWDBDefs at a pinned commit. Unknown layouts fail before output.
- Record actual install version/build key, binary and DBD SHA-256, schema commit, readable row count and unreadable records in build-keyed receipts.
- Regenerate client caches and server CSV inputs in an isolated output tree. Preserve TDB1210 world.db; report its broken joins rather than rewriting server content.
- Canonical publication is lead-only: one command with a fingerprint comparison and backup.
- Acceptance includes a row/ID census against former exports and native private hotkey captures for atlas7508739 in both skins.

## Current implementation

`export_db2_csv.py TABLE INPUT OUTPUT --definitions PINNED_DEFINITIONS` selects the payload layout. Sparse tables use `--server-scripts GAME_SERVER/scripts`, the existing server WDC5 reader. Definition acquisition and source receipts belong to the refresh command.

## Tests

`scripts/tests/test_db2_install_export.py`: reordered new-layout fields and signed arrays; unknown layout refuses output.
