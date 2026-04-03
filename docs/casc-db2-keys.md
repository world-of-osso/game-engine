# CASC DB2 Keys And WoWDBDefs

This note documents the difference between DB2 schema sources and CASC decryption key sources, because they solve different parts of the extraction pipeline.

## Short Version

- `wowdev/WoWDBDefs` is the primary reference for DB2/DBC schemas, layout hashes, and `DBMeta`-derived field layouts.
- `wowdev/WoWDBDefs` is **not** where TACT/CASC decryption keys live.
- Encrypted DB2 payloads require TACT keys from a separate source, typically `wowdev/TACTKeys` or keys derived from a local client cache.

## What WoWDBDefs Does

Repository:

- <https://github.com/wowdev/WoWDBDefs>

WoWDBDefs is for understanding how to parse DB2 files once you have the bytes:

- table names and field definitions
- layout hashes
- type information
- metadata generated from WoW client `DBMeta` / `DB2Meta`

Its update flow is based on extracting metadata from the WoW executable, not decrypting CASC archives. See:

- <https://github.com/wowdev/WoWDBDefs/blob/master/UPDATING.md>

That makes it useful for tables like `LightParams.db2` and `LightSkybox.db2`, but only after the DB2 file has already been extracted and decrypted.

## Where The Keys Come From

Repository:

- <https://github.com/wowdev/TACTKeys>

The missing `LightSkybox.db2` decryption key for this project was:

```text
0xD1055199767FB373
```

That key exists in `TACTKeys`, not in `WoWDBDefs`.

`TACTKeys` also documents the usual upstream sources for these keys:

- `TactKey.db2`
- `TactKeyLookup.db2`
- hotfix pushes cached in `DBCache.bin`
- BroadcastText additional data cached in `DBCache.bin`

## Practical Extraction Model

For encrypted DB2 files, the working model is:

1. Use local CASC metadata (`data/casc/root.bin`, `data/casc/encoding.bin`) to resolve the file.
2. Use TACT keys to decrypt encrypted BLTE payloads when needed.
3. Use WoWDBDefs to interpret the DB2 structure correctly.

In other words:

```text
WoWDBDefs = schema/layout knowledge
TACT keys = decryption material
```

Both are needed for some modern DB2 tables, but they solve different problems.

## Project-Specific Outcome

For `game-engine`, the authored skybox lookup work was blocked on decrypting `LightSkybox.db2` until the missing TACT key was supplied.

The local extractor paths now support optional external key loading from:

```text
data/tactkeys/WoW.txt
```

Relevant code:

- [casc_resolver.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/asset/casc_resolver.rs)
- [casc_local.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/bin/casc_local.rs)

That removes the decryption blocker for `LightSkybox.db2` when the key file is present.

## Remaining Limitation

After decryption was unblocked, the next issue was not key-related. The remaining work is accurate row/field resolution across current client DB2 data:

- mapping `Light.csv` / `LightParamsID` correctly into the local `LightParams.db2`
- resolving `LightSkyboxID`
- resolving the authored `SkyboxFileDataID`

So the current blocker is DB2 parsing and row mapping, not missing TACT keys.
