# Asset Pipeline

WoW assets are stored in CASC (Content Addressable Storage Container), a path-free archive system where every file is identified by a FileDataID (FDID). The engine extracts assets to local disk via a three-stage lookup chain, then caches them by FDID.

## CASC Lookup Chain

```
FDID (integer)
  → root.bin:     FDID → ContentKey (MD5 of raw file)
  → encoding.bin: ContentKey → EncodingKey (MD5 of BLTE blob)
  → .idx files:   EncodingKey[0..9] → (archive_id, offset, size)
  → .data.XXX:    seek, read BLTE blob, decompress
```

Cached tables: `data/casc/root.bin` + `data/casc/encoding.bin` (~250MB total). **Never delete** — expensive to regenerate. Refresh with `cargo run --bin casc_refresh` when they drift from the local WoW install.

## Local Extraction

```bash
# Extract by FDID to data/ subdirectory
cargo run --bin casc-local -- <fdid> [fdid2 ...] -o data/textures/
cargo run --bin casc-local -- <fdid> -o data/models/
```

Files are named `{fdid}.{ext}` (extension derived from the community listfile). Always extract from local CASC; never use Blizzard CDN.

## Asset Naming

- `data/textures/{fdid}.blp` — BLP textures
- `data/models/{fdid}.m2` — M2 models and `.skin` files
- `data/terrain/{fdid}.adt` — ADT terrain files
- `data/community-listfile.csv` — 136MB FDID→path map (from wowdev/wow-listfile)

## Community Listfile

CASC is path-free; the community listfile is a crowdsourced FDID→virtual-path map. It is load-bearing for two directions:

**Path → FDID** (`lookup_path`): resolving WMO group files, ADT companion files (`_tex0`/`_obj0`), terrain tiles by map coordinate, character models referenced by path, particle textures.

**FDID → Path** (`lookup_fdid`): deriving file extension, specular→diffuse texture swap (`_s.blp` suffix), debug labels.

Only ~7.8% of root.bin records have Jenkins96 name hashes populated (legacy mechanism, no longer maintained by Blizzard for modern files). The listfile is the only reliable path-based lookup.

## FDID Chunks in Modern Formats

Modern M2/WMO/ADT files embed FDID references in dedicated chunks (TXID, SFID, GFID, MODI, MDID, MHID) — these supersede legacy path strings. Note: WMO group loading currently ignores the parsed GFID chunk and does a listfile roundtrip instead; switching to GFID-first would eliminate the listfile dependency for WMO groups.

## Encrypted Files (TACT Keys)

Some BLTE chunks are encrypted. Keys come from `wowdev/TACTKeys` (not from WoWDBDefs, which is for DB2 schema only). Keys are loaded from `data/tactkeys/WoW.txt`. The `LightSkybox.db2` key (`0xD1055199767FB373`) was required for the skybox lookup chain. See [casc-db2-keys.md](../casc-db2-keys.md).

## DB2 Schema vs. TACT Keys

| Source | Purpose |
|--------|---------|
| WoWDBDefs | DB2 field layouts, layout hashes, type info |
| TACTKeys / `data/tactkeys/WoW.txt` | BLTE chunk decryption keys |

Both are needed for encrypted DB2 tables, but they solve different problems.

## Untextured Item Gotcha

Some item-driven textures come from `ItemDisplayInfo.ModelMaterialResourcesID_*` via `TextureFileData` — not from the M2's own TXID chunk. Auto-extraction is not fully reliable for these paths. If a model shows untextured, verify the FDID exists under `data/textures/` and extract it manually.

## Sources

- [casc-architecture.md](../casc-architecture.md) — lookup chain, listfile, FDID chunks, encrypted files
- [casc-extraction.md](../casc-extraction.md) — casc-local tool, refresh procedure, cascette-rs library
- [casc-db2-keys.md](../casc-db2-keys.md) — WoWDBDefs vs TACTKeys distinction
- AGENTS.md — Data Assets section

## See Also

- [[rendering-pipeline]] — consumes extracted assets at runtime
- [[terrain]] — ADT extraction and companion file lookup
- [[character-rendering]] — texture compositing from CASC-extracted BLP files
- [[skybox]] — LightSkybox.db2 decryption required for authored lookup
