# CASC Architecture

How Blizzard's CASC (Content Addressable Storage Container) system works and how this project interacts with it.

## Core Concept

CASC is a content-addressable storage system. Files have no names or paths inside the archives — they are blobs identified by hashes. Game data (DB2 tables, M2 models, ADT placements) references files by **FileDataID** (FDID), an integer.

## Lookup Chain

Extracting a file from CASC requires three lookup tables, each resolving one layer:

```
FDID (e.g. 189929)
  → root.bin:     FDID → ContentKey  (MD5 of the raw, uncompressed file)
  → encoding.bin: ContentKey → EncodingKey  (MD5 of the BLTE-encoded blob)
  → .idx files:   EncodingKey[0..9] → (archive_id, offset, size)
  → .data.XXX:    seek to offset, read BLTE blob, decompress
```

Each layer only knows about its own mapping:
- **root.bin** maps FDIDs to content keys. It knows nothing about archives.
- **encoding.bin** maps content keys to encoding keys. It knows nothing about FDIDs.
- **.idx files** map truncated encoding keys (9 bytes, enough to be unique across ~2M entries) to physical archive locations. They know nothing about content keys or FDIDs.

## Bootstrap

`root.bin` and `encoding.bin` are themselves CASC blobs stored in the same `.data.XXX` archives. The bootstrap sequence to find them:

1. **`.build.info`** — plaintext file at the WoW install root, names the active build config
2. **Build config** — plaintext file under `Data/config/`, contains the content keys for `encoding` and `root`
3. **encoding.bin** — special case: its content key can be looked up directly in `.idx` files
4. **root.bin** — resolved normally: content key → encoding key (via the now-available encoding.bin) → `.idx` lookup

Once both files are cached locally (`data/casc/root.bin`, `data/casc/encoding.bin`), the full chain works. `cargo run --bin casc_refresh` repeats this bootstrap from the local WoW install when the cache drifts out of sync.

## Archive Layout

```
/syncthing/World of Warcraft/Data/
├── config/          # Build configs (plaintext, reference encoding/root keys)
├── data/
│   ├── *.idx        # Index files: encoding_key[0..9] → (archive, offset, size)
│   └── data.XXX     # Archive files: raw BLTE blobs at known offsets
├── indices/         # CDN index cache
└── wow/             # Retail CASC data
```

The `.idx` files are hash tables written by Blizzard's patcher. The patcher controls the physical layout of `.data` archives, so only it knows where each blob ends up. Clients (and our code) read the index to find locations.

Each local archive entry has a 30-byte header followed by a BLTE container. BLTE (Binary Large Transfer Encoding) is Blizzard's chunked compression/encryption format — a file may be split into independently compressed (and optionally encrypted) chunks.

## Encrypted Files

Some BLTE chunks are encrypted with TACT keys. These keys are not part of the CASC structure itself — they come from:

- `TactKey.db2` / `TactKeyLookup.db2` (client DB2 tables)
- `DBCache.bin` (hotfix cache)
- Community-maintained key lists (`wowdev/TACTKeys`)

This project loads external keys from `data/tactkeys/WoW.txt`. See `docs/casc-db2-keys.md` for details on DB2-specific decryption.

## How the Client Resolves Paths

CASC is entirely path-free — no filenames are stored in the archives. Most game data references files by FDID directly. However, two path-based resolution mechanisms exist:

### ManifestInterfaceData.db2

The addon/UI API still accepts path strings (e.g. `SetTexture("Interface\\Icons\\INV_Misc_QuestionMark.blp")`). The client resolves these via **`ManifestInterfaceData.db2`**, a DB2 table shipped with the client that maps interface file paths to FDIDs. This covers `Interface/Icons/*`, `Interface/Addons/*`, and other addon-visible paths. It is a narrow bridge — only the addon API surface uses it.

### Internal Format FDID Chunks

Modern M2, WMO, and ADT files ship FDID chunks that supersede legacy path strings:

| Format | Chunk | Purpose |
|--------|-------|---------|
| M2 | TXID | Texture FDIDs |
| M2 | SFID | Skin file FDIDs |
| WMO | GFID | Group file FDIDs |
| WMO | MODI | Doodad model FDIDs |
| ADT | MDID | Diffuse texture FDIDs |
| ADT | MHID | Height texture FDIDs |

These chunks contain the authoritative FDID references — the client uses them directly, not path strings. Legacy path fields (`particle_model_filename`, `child_emitters_model_filename` in M2 particle emitters) still exist in the binary format but are dead weight in modern files. Verified: a retail WMO root (FDID 108238) has a fully populated GFID chunk with 46 non-zero group FDIDs.

**Note:** This project's `resolve_wmo_group_fdids` currently ignores the parsed `group_file_data_ids` (GFID) and instead does a listfile roundtrip (root FDID → path → construct group path pattern → lookup_path per group). The FDID data is already parsed but unused — switching to GFID-first resolution would eliminate the listfile dependency for WMO group loading.

## The Community Listfile

The **community listfile** (`data/community-listfile.csv`, ~136MB, from `wowdev/wow-listfile`) is a crowdsourced reverse-engineering effort that maps FDIDs to virtual paths:

```csv
145513;textures/loginscreen/northrend/northrendloginscreen.blp
189929;world/maps/azeroth/azeroth_32_48.adt
```

### Root.bin Name Hashes (Not Viable)

Root.bin records can optionally include a Jenkins96 hash of the file's virtual path (uppercase, forward-slash-normalized). In theory this allows path → content key resolution without the listfile. In practice, **modern WoW has largely stopped populating name hashes** — only ~7.8% of root records (248K out of 3.2M) have them, and common paths (terrain, models, textures, WMOs) all miss. This is a legacy mechanism that Blizzard no longer maintains for new files.

### How the Listfile Is Built

It is built by:
- **Brute-forcing** known path patterns (e.g. `world/maps/{map}/{map}_{x}_{y}.adt`)
- **Extracting** path references embedded in other files (DB2 records, M2 models, Lua scripts)
- **Diffing** patches to discover new FDIDs and correlating with known patterns
- **Manual identification** by modders and dataminers

Coverage is incomplete — some FDIDs have no known path. These files are still extractable by FDID; they just can't be found by name.

### How This Project Uses It

The listfile is not cosmetic — it is load-bearing for resolving between path-based and FDID-based references:

**Path → FDID** (`lookup_path`): When WoW data references files by virtual path rather than FDID:
- WMO group files (root references groups by path pattern: `wmo_000.wmo`, `wmo_001.wmo`)
- ADT companion files (finding `_tex0`/`_obj0` siblings from the root ADT path)
- Terrain tiles (`world/maps/azeroth/azeroth_32_48.adt` → FDID)
- Character models (some DB2 entries reference models by path)
- Creature skin resolution (named model path matching)
- Particle textures (referenced by path in M2 data)

**FDID → Path** (`lookup_fdid`): Deriving context from a known FDID:
- File extension for `casc-local` output naming
- Specular → diffuse texture resolution (swap `_s.blp` suffix)
- WMO group FDID discovery from root FDID
- Debug labels and logging

Files not in the listfile can only be reached through DB2 table chains (e.g. `ItemDisplayInfo → TextureFileData → FDID`), not path lookup. This is the source of the "untextured item" gotcha documented in AGENTS.md.

## Related Docs

- `docs/casc-extraction.md` — local extraction workflow, `casc-local` usage, refresh procedure
- `docs/casc-db2-keys.md` — TACT key sources vs DB2 schema sources
