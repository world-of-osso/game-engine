# Player bundle staging

Linux and Windows players share one staged project/data bundle. Publisher: `deploy.sh`; assembler/verifier: `scripts/player_bundle.py`.

## What it must do

- [x] Map shipped paths to source files without copying the full data set. Generate the project import and native files separately. Use SQLite online backups for mutable database files; hash, pack, and upload the backups.
- [x] Ship patched release-template Godot runtimes on Linux and Windows. Use editor builds only to import the project, and verify runtime checksums.
- [x] Generate the download manifest from source bytes. Detect source changes after hashing and reject changed bytes during pack construction.
- [x] Stream each changed file to a temporary object, verify the streamed size and SHA256 against the manifest, and promote only matching bytes. Missing or changed files abort publication with path-specific errors.
- [x] A dry run generates both manifests and packs without accessing the live bucket.
- [x] Compare old and new file paths, sizes, and SHA256 except `godot/.godot/uid_cache.bin` and SQLite backups. Compare the UID cache's parsed UID-to-path pairs; two cold imports of the same old bundle produced different cache byte order. Do not rewrite the cache to make it deterministic. For each SQLite backup, compare schema and rows with a quiesced copy of its source; online backup bytes can differ from a raw file copy. Pack locations may differ; verify every referenced pack, range, and reconstructed file hash.
- [x] Preserve the runtime allowlist, product receipt selection and credential/extraction-state exclusions. Abort if model indexes change while selecting or hashing their assets.
- [x] Include the top-level client JSON files `battle-pets.json` and `battle-pet-model-scenes.json`; do not include unrelated diagnostic JSON or credentials.
- [ ] Derive requestable map BLPs from active DB2: System-0 world-map types that have a world assignment and art, their art-backed parent maps, and System-2 Housing maps reached from `NeighborhoodMap` records in `cache/housing.sqlite`. Include phase-zero layer-zero tiles that fit the style-layer bounds and map highlights. Do not include `WorldMapOverlayTile`. Keep the existing active atlas and Forever set-1 (`1.60.1.69913`) selection unchanged. Exclude only the two known-unavailable atlas IDs listed below. Extract every other missing BLP from local CASC into staged `data/textures`, and fail if any other selected texture is absent or invalid.
- [x] `model-asset-index*.json` entries must stay within `products/<product>/<build_key>/`. UI audio ships directly through `sounds/` (including `sounds/ui/`); keep its verification receipts outside the model-index filename family. Do not produce model-asset-index fragments for flat UI sound paths.

## Template proof — 2026-10-27

- Linux: the release template ran the raw imported project under private Xvfb; X11 reported `World of Osso`, native IPC returned `Pong`, and the lead inspected the login screenshot.
- Windows: the template loaded godot-rust and reported `World of Osso` through `ssh desktop`. The one GUI capture was blank because the approved October 11 data fixture lacked current DB2 files; login remains unproved. The lead owns the real-player proof after deployment.

## How it works

- [Deploy workflow](../deploy.md#data).

## Tests asserting this spec

- `scripts/tests/test_player_bundle.py` discovers literal client data-root file reads and verifies bundle selection, including requestable System-0 maps and their parents, Housing map assignments from `cache/housing.sqlite`, layer-zero tile bounds, overlay exclusion, the unchanged active/Forever atlas selection, `battle-pets.json` and staged-manifest sources. `godot/tests/player_bundle_map_assets.gd` checks all Forever Dazar'alor and Founder's Point map tiles load in the renderer. `test_launcher_packs.py`, `test_mapped_deploy.py`, and `test_verified_upload.py` cover source drift, removed files, pack output, and verified upload promotion. Existing allowlist, product namespace and native-file cases remain required.

## Known gaps (current cycle)

- [ ] Active DB2 build `12.1.5.70448` references atlas FDID `3307666`, and Forever set-1 references FDID `6227333`; local CASC build `12.1.5.70783` has neither. The bundle omits these two IDs. Do not substitute another texture; add them only when matching assets are available.

## Out of scope

- Publication and production access remain prohibited by the security hold.
- Atomic snapshots across independently written catalogs and full offline asset closure.
