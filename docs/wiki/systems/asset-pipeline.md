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

Cached tables live under `~/.cache/asset-resolver/casc/<product>/<build-key>/` (~250MB total). The Godot client uses that default too: its resolvers set no cache root (`assets::creature::local_resolver`), so every run shares one resolution cache whatever its `XDG_DATA_HOME`. Measured 2026-09-30 (`godot/tests/startup_login.gd`, load about 20): two runs with different `XDG_DATA_HOME` each loaded the 1,931,507-entry cache in 4.1 and 6.5 s, with no rebuild. With `ASSET_RESOLVER_CACHE_DIR` pointed at an empty directory, the rebuild took 48.1 s. The login reply was handled at 1.2 s, while the build was still running; character select showed at 63.1 s. While CASC starts, the client keeps polling its session and holds back the screen and unit events until startup finishes. They are generated cache data and can be rebuilt with `cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc_refresh` when they drift from the local WoW install.

## Local Extraction

```bash
# Extract by FDID to data/ subdirectory
cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- <fdid> [fdid2 ...] -o data/textures/
cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- <fdid> -o data/models/
```

Files are named `{fdid}.{ext}` (extension derived from the community listfile). Always extract from local CASC; never use Blizzard CDN.

The shared file cache previously treated an empty persisted `.missing` file as permanent absence, bypassing local extraction even when CASC held the asset. Asset-resolver `c24d035` removes that check and marker writes; existing markers remain untouched and are ignored. Positive-cache hits, local extraction, failure diagnostics and public initialization are unchanged. The contract lives in the sibling [asset-cache spec](../../../../asset-resolver/docs/specs/asset-cache.md).

Main accepts standalone `25debb1` **bounded PASS** in `/tmp/claude/verify-shared-negative-cache-final.md`: fresh processes persist a marker, preserve positive-cache bytes/inode/timestamps, assert unavailable-FDID failure context, then recover FDID `1244035` as exact valid local-CASC BLP2 bytes. Saved regression exits 0 (1/1); format, offline locked all-target check and focused readability pass. Existing `binrw` future-incompatibility warning remains. This proves the standalone cache boundary, not native cold-marker recovery.

Main accepts `/tmp/claude/verify-negative-cache-native-integration.md` **bounded saved-artifact PASS**: Depot `s1q4qhb120` build2 exit 0 compiles the native consumer and fixture; extension load and `data/diagnostics/negative-cache-native/runtime.log` exit 0 cover eight Options helper PASS markers plus Menu/owned UDP/Exit. Native cold-marker acquisition is not verified; standalone proof above supplies that separate process boundary. Logs do not independently attest compiled binary identity. Existing native `fdid` warning and runtime warnings/notices remain disclosed in the report. No full-conversion, deployment or current unfrozen whole-tree acceptance; shutdown remains paused.

Extraction to disk is not the only access path. The project `AssetResolver` also exposes `resolve_bytes(fdid)`, which can read file contents directly from local CASC. Runtime DB2 loading can use direct bytes because the DB2 parsers accept `&[u8]`; path-based helpers such as `ensure_db2_path` are mainly useful for debug artifacts, cache inspection, and tools that require filesystem paths.

`771c1f5f` uses the existing local `CascListfileResolver` cache path for a native creature-model helper: model FDID → `.m2`, primary M2 SFID → adjacent `00.skin`, optional SKID → adjacent `.skel`, then parsed render-batch/explicit creature-slot texture FDIDs → `.blp`. The helper is not yet attached to native world-unit spawning.

## Asset Naming

- `data/textures/{fdid}.blp` — BLP textures
- `data/models/{fdid}.m2` — M2 models and `.skin` files
- `data/terrain/{fdid}.adt` — ADT terrain files
- `data/community-listfile.csv` — 136MB FDID→path map (from wowdev/wow-listfile)

## Forever overlay

[[forever-data]] records the pinned local-CASC importer, separate Forever CSV overlay, encrypted-row drops, content-key mismatch and AFID/BFID validation limits. [[terrain]] owns Zephras closure inventory; neither importer validation nor available files imply complete native acceptance.

## UI consumers

WoW UI metadata stores authored FileDataIDs. A virtual path is only listfile lookup input; it is never a runtime filesystem path or a fallback export directory. Character-creation race/class icons emit `texture_fdid`; customization arrows and palette regions use an atlas FileDataID. `GameBlpLoader` resolves both through local CASC/cache. See [[ui-system]].

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

External tools such as `Frostshake/WDBx` can open DBC/DB2 from CASC and export CSV/JSON/SQL using WDBReader and WoWDBDefs. Use them to inspect or verify schema behavior, not as a required runtime dependency.

## Untextured Item Gotcha

Some item-driven textures come from `ItemDisplayInfo.ModelMaterialResourcesID_*` via `TextureFileData` — not from the M2's own TXID chunk. Auto-extraction is not fully reliable for these paths. If a model shows untextured, verify the FDID exists under `data/textures/` and extract it manually.

## Sources

- [casc-architecture.md](../casc-architecture.md) — lookup chain, listfile, FDID chunks, encrypted files
- [casc-extraction.md](../casc-extraction.md) — casc-local tool, refresh procedure, cascette-rs library
- [casc-db2-keys.md](../casc-db2-keys.md) — WoWDBDefs vs TACTKeys distinction
- AGENTS.md — Data Assets section
- [character-creation icon metadata](../../../src/scenes/char_create/data.rs) — typed UI FileDataID consumer

## See Also

- [[forever-data]] — pinned Skyborne tables/assets and acquisition limits

- [[rendering-pipeline]] — consumes extracted assets at runtime
- [[terrain]] — ADT extraction and companion file lookup
- [[character-rendering]] — texture compositing from CASC-extracted BLP files
- [[skybox]] — LightSkybox.db2 decryption required for authored lookup
- [[ui-system]] — registry UI consumers of local CASC-backed FileDataIDs
