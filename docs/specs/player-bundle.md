# Player bundle staging

Linux and Windows players share one staged project/data bundle. Publisher: `deploy.sh`; assembler/verifier: `scripts/player_bundle.py`.

## What it must do

- [x] Map shipped paths to source files without copying the full data set. Generate the project import and native files separately. Use SQLite online backups for mutable database files; hash, pack, and upload the backups.
- [x] Generate the download manifest from source bytes. Detect source changes after hashing and reject changed bytes during pack construction.
- [x] Stream each changed file to a temporary object, verify the streamed size and SHA256 against the manifest, and promote only matching bytes. Missing or changed files abort publication with path-specific errors.
- [x] A dry run generates both manifests and packs without accessing the live bucket.
- [x] Compare old and new file paths, sizes, and SHA256 except `godot/.godot/uid_cache.bin` and SQLite backups. Compare the UID cache's parsed UID-to-path pairs; two cold imports of the same old bundle produced different cache byte order. Do not rewrite the cache to make it deterministic. For each SQLite backup, compare schema and rows with a quiesced copy of its source; online backup bytes can differ from a raw file copy. Pack locations may differ; verify every referenced pack, range, and reconstructed file hash.
- [x] Preserve the runtime allowlist, product receipt selection and credential/extraction-state exclusions. Abort if model indexes change while selecting or hashing their assets.
- [x] `model-asset-index*.json` entries must stay within `products/<product>/<build_key>/`. UI audio ships directly through `sounds/` (including `sounds/ui/`); keep its verification receipts outside the model-index filename family. Do not produce model-asset-index fragments for flat UI sound paths.

## How it works

- [Deploy workflow](../deploy.md#data).

## Tests asserting this spec

- `scripts/tests/test_player_bundle.py`, `test_launcher_packs.py`, `test_mapped_deploy.py`, and `test_verified_upload.py` cover selection, source drift, removed files, pack output, and verified upload promotion. Existing allowlist, product namespace and native-file cases remain required.

## Out of scope

- Publication and production access remain prohibited by the security hold.
- Atomic snapshots across independently written catalogs and full offline asset closure.
