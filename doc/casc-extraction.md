# CASC Asset Extraction

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

## Local Refresh

When `data/casc/root.bin` and `data/casc/encoding.bin` drift out of sync with the
synced WoW install, local extraction starts failing with errors like:

```text
Content key not found in local indices
```

Refresh the cache from the local WoW archives, not CDN.

### Backup First

```bash
ts=$(date +%Y%m%d-%H%M%S)
mkdir -p data/casc/backups/$ts
cp data/casc/root.bin data/casc/backups/$ts/root.bin
cp data/casc/encoding.bin data/casc/backups/$ts/encoding.bin
```

### Refresh From Local CASC

```bash
cargo run --bin casc_refresh
```

This binary:

1. Reads the active retail build from `/syncthing/World of Warcraft/.build.info`
2. Opens the matching local build config under `Data/config/`
3. Reads `encoding.bin` from local CASC archives by the build config's encoding key
4. Resolves the build config's root content key through that fresh encoding file
5. Reads `root.bin` from local CASC archives by the resolved encoding key
6. Writes both files back to `data/casc/`

### Verify

```bash
# Known-good spot check
cargo run --bin casc-local -- 145513 4219004 4239595 4226685 -o data/textures

# Optional: verify runtime extraction path too
cargo run --bin game-engine -- screenshot data/charselect-check.webp --screen charselect
```

If refresh worked, `casc-local` should extract files instead of failing with
content-key lookup errors.

## casc-local (Primary Tool)

Binary in game-engine that reads directly from local CASC archives.

```bash
# Extract by FileDataID (saves as {fdid}.{ext} based on listfile)
cargo run --bin casc-local -- <fdid> [fdid2 ...] -o data/models/
cargo run --bin casc-local -- <fdid> -o data/terrain/
cargo run --bin casc-local -- <fdid> -o data/textures/
```

### How It Works

1. Opens local CASC at `/syncthing/World of Warcraft/Data`
2. Loads `.idx` index files (2.1M entries across 197 archives)
3. Loads cached `data/casc/root.bin` + `data/casc/encoding.bin`
4. Resolution chain: FDID → ContentKey (root) → EncodingKey (encoding) → archive location (.idx)
5. Reads + BLTE-decompresses from local `.data` archives
6. Files named by FDID: `{fdid}.m2`, `{fdid}.blp`, etc.

### Prerequisites

`data/casc/root.bin` and `data/casc/encoding.bin` must match the local WoW
build. If they do not, run `cargo run --bin casc_refresh`.

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
