# Player bundle staging

Linux and Windows players share one staged project/data bundle. Publisher: `deploy.sh`; assembler/verifier: `scripts/player_bundle.py`.

## What it must do

- [x] Copy mutable catalogs independently of their sources: `cache`, `db2`, `dbfilesclient`, `gametables`, `reference`, and SQLite/CSV/RON/JSON/text files elsewhere. Later source writes must not change staged bytes. Immutable extracted payloads may share storage.
- [x] Generate the download manifest by hashing staged files, not live source files.
- [x] Before uploading any bundle bytes, compare every manifest member's staged size and SHA256 with the generated manifest. Missing or changed files abort publication with path-specific errors.
- [x] Preserve the runtime allowlist, product receipt selection and credential/extraction-state exclusions.
- [x] `model-asset-index*.json` entries must stay within `products/<product>/<build_key>/`. UI audio ships directly through `sounds/` (including `sounds/ui/`); keep its verification receipts outside the model-index filename family. Do not produce model-asset-index fragments for flat UI sound paths.

## How it works

- [Deploy workflow](../deploy.md#data).

## Tests asserting this spec

- `scripts/tests/test_player_bundle.py`: real SQLite source mutations cannot change staged catalogs; actual verifier CLI accepts matching bytes and rejects same-size mutation, incorrect size and missing files. Existing allowlist, product namespace and native-file cases remain required.

## Out of scope

- Publication and production access remain prohibited by the security hold.
- Atomic snapshots across independently written catalogs and full offline asset closure.
