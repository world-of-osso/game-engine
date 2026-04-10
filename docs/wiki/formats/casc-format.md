# CASC Format

CASC (Content Addressable Storage Container) is Blizzard's archive system used since WoW 6.0. All game files — models, textures, DB2 tables, terrain — are stored as anonymous blobs identified by integer FileDataIDs (FDIDs). There are no filenames inside the archives.

## Lookup Chain

Extracting any file requires three successive lookups:

```
FDID
  → root.bin      FDID → ContentKey (MD5 of raw file)
  → encoding.bin  ContentKey → EncodingKey (MD5 of BLTE-encoded blob)
  → .idx files    EncodingKey[0..9] → (archive_id, offset, size)
  → .data.XXX     seek + BLTE decompress → file bytes
```

Each layer is opaque to the others: root knows only FDIDs, encoding knows only content keys, index files know only truncated encoding keys.

## Bootstrap

`root.bin` and `encoding.bin` are themselves CASC blobs. To obtain them:

1. `.build.info` (WoW install root) → active build config name
2. Build config (under `Data/config/`) → content key for `encoding` and `root`
3. `encoding.bin` looked up directly in `.idx` files by its content key
4. `root.bin` resolved through the now-loaded `encoding.bin`

Cached locally at `data/casc/root.bin` and `data/casc/encoding.bin`. Refresh with `cargo run --bin casc_refresh` when the local WoW install updates.

## BLTE Encryption (TACT Keys)

Some BLTE chunks are encrypted. Keys come from:
- `wowdev/TACTKeys` community list
- `TactKey.db2` / `TactKeyLookup.db2` (client DB2 tables)
- Hotfix cache `DBCache.bin`

Loaded from `data/tactkeys/WoW.txt`. Separate from WoWDBDefs — see [[db2-format]].

## FDID Resolution

Most in-engine references use FDIDs directly. Modern format chunks that embed FDIDs:

| Format | Chunk | Purpose |
|--------|-------|---------|
| M2 | `TXID` | Texture FDIDs |
| M2 | `SFID` | Skin file FDIDs |
| WMO | `GFID` | Group file FDIDs |
| WMO | `MODI` | Doodad model FDIDs |
| ADT | `MDID` | Diffuse texture FDIDs |
| ADT | `MHID` | Height texture FDIDs |

Legacy path strings in format headers are dead weight in modern files.

## Community Listfile

`data/community-listfile.csv` (~136MB, from `wowdev/wow-listfile`) maps FDIDs to virtual paths. Used for:
- **Path → FDID** (`lookup_path`): finding `_tex0`/`_obj0` ADT companions, WMO group paths, terrain tile paths
- **FDID → Path** (`lookup_fdid`): output naming, specular→diffuse suffix swap, debug labels

Only ~7.8% of root.bin records have Jenkins96 name hashes; path-based lookup relies entirely on the community listfile.

## Local Extraction Tool

```bash
cargo run --bin casc-local -- <fdid> [fdid2 ...] -o data/models/
```

Reads directly from `/syncthing/World of Warcraft/Data`. Never uses CDN.

## Sources

- [docs/casc-architecture.md](../casc-architecture.md) — full lookup chain, bootstrap, FDID resolution, listfile usage
- [docs/casc-extraction.md](../casc-extraction.md) — local extraction workflow, casc-local tool, refresh procedure
- [docs/casc-db2-keys.md](../casc-db2-keys.md) — TACT key sources vs WoWDBDefs schema sources

## See Also

- [[db2-format]] — DB2 files extracted from CASC; WoWDBDefs for schema, TACTKeys for decryption
- [[m2-format]] — M2 TXID/SFID/SKID chunks reference FDIDs resolved via CASC
- [[adt-format]] — ADT companion files and MDID/MHID FDIDs
- [[blp-format]] — BLP textures extracted by FDID
