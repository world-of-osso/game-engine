# CASC Asset Extraction

For agent slots, complete [shared data setup](remote-builds.md#shared-worktree-data) before extracting to the `data/` paths below.

## Local WoW Install

Full WoW installation synced from Windows via Syncthing:

```
/syncthing/World of Warcraft/
├── Data/              # CASC storage (archives, indices, config)
│   ├── config/        # Build configs (root/encoding key references)
│   ├── data/          # .idx index files + .data archive files
│   ├── indices/       # CDN index cache
│   ├── wow/           # retail CASC data
│   ├── wow_beta/
│   └── wow_classic/
├── _retail_/          # Game client (exe, Interface, WTF, Fonts)
├── _classic_/
├── _classic_era_/
└── _beta_/
```

**Always extract from local CASC storage. Never use Blizzard CDN.**

## OssoBuild local store (2026-10-08)

Verified: 2026-10-08. `/syncthing/World of Warcraft/` is absent on this host, but `/mnt/c/World of Warcraft/Data/data` holds the actual local indices/archives. The existing `casc-local` discovers this WSL location; `WOW_INSTALL_PATH=/mnt/c/World of Warcraft` selects it explicitly. The successful auction-icon extraction used Retail 12.1.0.69933, build key `dcfc90fffd79ba00406ae46f5f657592`, with 1,926,810 cached resolution entries. See [auction evidence](wiki/systems/auction-house-ui.md#missing-result-icons-2026-10-08).

`data/casc/root.bin`, `encoding.bin`, and the build-key directories under `data/casc/wow/` are resolution metadata (root/encoding/SQLite), not archive storage. They cannot supply file payloads alone. Do not mistake their presence for a usable install, or delete them to repair missing texture files.

## Local Refresh

When the cached `root.bin` and `encoding.bin` under
`~/.cache/asset-resolver/casc/<product>/<build-key>/` drift out of sync with the
synced WoW install, local extraction starts failing with errors like:

```text
Content key not found in local indices
```

Refresh the cache from the local WoW archives, not CDN.

### Refresh From Local CASC

```bash
cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc_refresh
```

This binary:

1. Reads the active retail build from `/syncthing/World of Warcraft/.build.info`
2. Opens the matching local build config under `Data/config/`
3. Reads `encoding.bin` from local CASC archives by the build config's encoding key
4. Resolves the build config's root content key through that fresh encoding file
5. Reads `root.bin` from local CASC archives by the resolved encoding key
6. Writes both files back to `~/.cache/asset-resolver/casc/<product>/<build-key>/`

### Verify

```bash
# Known-good spot check
cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- 145513 4219004 4239595 4226685 -o data/textures

# Optional: verify runtime extraction path too (client running, then capture)
cargo run -- --screen charselect
target/debug/game-engine-cli screenshot data/charselect-check.webp
```

If refresh worked, `casc-local` should extract files instead of failing with
content-key lookup errors.

## casc-local (Primary Tool)

Binary in asset-resolver that reads directly from local CASC archives.

```bash
# Extract by FileDataID (saves as {fdid}.{ext} based on listfile)
cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- <fdid> [fdid2 ...] -o data/models/
cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- <fdid> -o data/terrain/
cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- <fdid> -o data/textures/
```

### How It Works

1. Opens local CASC at `/syncthing/World of Warcraft/Data`
2. Loads `.idx` index files (2.1M entries across 197 archives)
3. Loads cached `~/.cache/asset-resolver/casc/<product>/<build-key>/root.bin` + `encoding.bin`
4. Resolution chain: FDID → ContentKey (root) → EncodingKey (encoding) → archive location (.idx)
5. Reads + BLTE-decompresses from local `.data` archives
6. Files named by FDID: `{fdid}.m2`, `{fdid}.blp`, etc.

### Prerequisites

Cached `root.bin` and `encoding.bin` must match the local WoW
build. If they do not, run `cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc_refresh`.

## CASC Lookup Chain

```
FDID (e.g. 189929)
  → Root file: FDID → ContentKey (16 bytes)
  → Encoding file: ContentKey → EncodingKey (16 bytes)
  → Local .idx: EncodingKey (9 bytes truncated) → archive_id + offset + size
  → Local .data: raw BLTE blob → decompress → file bytes
```

Key detail: local `.idx` files use encoding keys truncated to 9 bytes, not content keys. The `Installation::read_file_by_encoding_key()` method handles this correctly.

This matters in code too: `read_file_by_fdid()` is not the correct local
archive path for reliable extraction in this project. The working path is:

```text
FDID -> content key -> encoding key -> read_file_by_encoding_key()
```

## Asset Naming

- Assets stored by FileDataID (FDID): `data/textures/{fdid}.blp`, `data/models/{fdid}.m2`
- FDID↔path mapping: `data/community-listfile.csv` (136MB, semicolon-separated: `FDID;path`)
- Split ADT files share a base FDID: root=778027, _obj0=778028, _tex0=778030 (for azeroth_32_48)

## Libraries

- cascette-rs: `~/Repos/cascette-rs` — Rust CASC implementation
  - `cascette-client-storage` (feature `local-install`): `Installation::open()` + `read_file_by_encoding_key()`
  - `cascette-crypto`: ContentKey, EncodingKey types
  - `cascette-formats`: BLTE decompression
- CASCLib: https://github.com/ladislav-zezula/CascLib — C reference implementation

## Item-model material declarations

Equipment requires `data/ItemDisplayInfoModelMatRes.csv`, not only the two legacy
`ItemDisplayInfo.ModelMaterialResourcesID` columns. Extract FDID **4050937** from local
CASC, then export layout **52510D63**:

```sh
casc-local 4050937 -o data/dbfilesclient
python3 scripts/export_db2_csv.py ItemDisplayInfoModelMatRes data/dbfilesclient/4050937.db2 data/ItemDisplayInfoModelMatRes.csv
```

The catalog loads `(ItemDisplayInfoID, ModelIndex, TextureType, MaterialResourcesID)`
and selects each texture through `TextureFileData`/race-sex component ownership. Native
items cache these BLPs from local CASC and bind each declared M2 type on every sampled
slot. `GAME_ENGINE_EQUIPMENT_TEXTURE_DIAGNOSTICS=1` logs concrete FDIDs per item batch.
Missing declarations/files fail explicitly rather than leaving shader-white uniforms.
The 2026-10-06 export contains 141,252 rows; 129 records remain encrypted/unavailable.
See [Everforged shoulder diagnosis](wiki/investigations/npc-stance-gear.md#everforged-shoulder-materials-2026-10-06).

## Related Notes

- `docs/casc-db2-keys.md` — difference between WoWDBDefs schema metadata and TACT key sources for encrypted DB2 extraction
- `docs/skybox-authored-lookup.md` — current `Light.csv -> LightParams -> LightSkybox -> FileDataID -> .m2` lookup chain and fallback behavior

## WoW Forever atlas data

`python3 scripts/import_forever_atlas.py` imports Forever's `UiTextureAtlas*` tables into
`data/db2/1.60.1.69913/` and extracts every `UiTextureAtlasSetID` 1 texture (the `c60`
re-skins of Retail atlas names) with `WOW_PRODUCT=wow_classic_beta casc-local`.
`scripts/forever-atlas-listfile.csv` names those textures; none is in the community listfile.

Verified 2026-10-03: the installed `wow_classic_beta` build 1.60.1.70205 (build key
`842b2e5d11f8d6fe257a5b73bd5cf6c6`) has no root in local archives (encoding key
`fcae3917977c7fdf9f3864ed5bf96521` is in no `Data/data/*.idx`), and the 69977 keys of the
atlas DB2s and `c60` textures are gone too. So the tables are the Wago 69913 CSVs from
wow-ui-sim (sha256-checked), and 17 of the 211 set-1 textures came from wow-ui-sim's
`~/.cache/wow-ui-sim/casc-extract`, each byte-identical (MD5 = content key) to the 69977 root.
The other 194 stay missing until the Forever data is back in local CASC; rerun the script then.
