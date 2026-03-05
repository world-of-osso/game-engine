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
3. Loads cached root+encoding files from `~/.cache/casc-extract/wow-{build}/`
4. Resolution chain: FDID → ContentKey (root) → EncodingKey (encoding) → archive location (.idx)
5. Reads + BLTE-decompresses from local `.data` archives
6. Files named by FDID: `{fdid}.m2`, `{fdid}.blp`, etc.

### Prerequisites

One-time CDN init to get root+encoding mapping files:
```bash
cd ../casc-extract && cargo run -- init
```
These cached files (`root.bin`, `encoding.bin`) map FDIDs to encoding keys. The actual asset data comes from local CASC.

## casc-extract (CDN fallback)

Located at `../casc-extract/`. Downloads from Blizzard CDN. **Avoid** — only use for `init`.

```bash
cargo run -- init                              # One-time: cache root+encoding
cargo run -- search "azeroth_32_48"            # Search listfile
```

## CASC Lookup Chain

```
FDID (e.g. 189929)
  → Root file: FDID → ContentKey (16 bytes)
  → Encoding file: ContentKey → EncodingKey (16 bytes)
  → Local .idx: EncodingKey (9 bytes truncated) → archive_id + offset + size
  → Local .data: raw BLTE blob → decompress → file bytes
```

Key detail: local `.idx` files use encoding keys truncated to 9 bytes, not content keys. The `Installation::read_file_by_encoding_key()` method handles this correctly.

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
