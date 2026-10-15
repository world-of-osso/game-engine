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

Cached tables live under `~/.cache/asset-resolver/casc/<product>/<build-key>/schema-2/` (~250MB total). The Godot client uses that default too: its resolvers set no cache root (`assets::creature::local_resolver`), so every run shares one resolution cache whatever its `XDG_DATA_HOME`. Measured 2026-09-30 (`godot/tests/startup_login.gd`, load about 20): two runs with different `XDG_DATA_HOME` each loaded the 1,931,507-entry cache in 4.1 and 6.5 s, with no rebuild. With `ASSET_RESOLVER_CACHE_DIR` pointed at an empty directory, the rebuild took 48.1 s. The login reply was handled at 1.2 s, while the build was still running; character select showed at 63.1 s. While CASC starts, the client keeps polling its session and holds back the screen and unit events until startup finishes. They are generated cache data and can be rebuilt with `cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc_refresh` when they drift from the local WoW install.

## Installed-build following — verified 2026-10-13

Retail patched from12.1.0.69933/dcfc90ff to12.1.5.70340/eba6b262; runtime
requested the former receipt key and keyring lookup failed after its row vanished.
Local extraction now selects the product's unique active `.build.info` row,
including when the row changes within one process. Resolution state/tables are
build-qualified, so historical or unqualified tables cannot masquerade as current.
Published historical files remain read-only hits; misses return a path under the
installed build. New offline receipts use installed identity, while DB2 closure
metadata stays12.1.0.69933. Extracted-only/frozen authenticated publication is
unchanged. [Contract](../../specs/product-isolated-model-assets.md#installed-build-extraction-decision-2026-10-13)
and [lead-only regeneration](../../casc-extraction.md#lead-only-canonical-regeneration).

Proof: real fixture IDX/archive changes active row in-process, extracts changed
FDID120191 bytes, creates two separate resolution caches and preserves historical
published bytes/unqualified decoy. Real install, Syncthing idle/needBytes0/errors0:
FDIDs120191,1100087,6653697,7508739 pass root MD5 and have SHA256 receipts;
1927944-entry private resolution cache. Native private UDP5781 fb_casc_follow /
Cascfollow enters rendered Elwynn world with the active resolver initialized.
SoundKit1237434 has unknown encrypted chunks; its zero-filled diagnostic output
is explicitly nonauthentic, not accepted asset proof. Evidence:
`/home/osso/.worktrees/casc-follow-proof/` (SHA receipts, logs, scene, screenshot).
Canonical assets/tables/install were not mutated; canonical regeneration remains
lead-owned. This is extraction/startup proof, not a DB2 re-export or full closure.

## DB2 and live asset-index following — implementation 2026-10-13

### Scalar versus array pallets — 2026-10-14

Installed PlayerCondition layout `1D743D99` interleaves a storage-4 MovementFlags descriptor before storage-3 WeaponSubclassMask/ContentTuningID descriptors. Raw CASC bytes put scalar pallets first, then array pallets, not descriptor order across both types. The former decoder incorrectly advanced those scalar offsets by MovementFlags' 44 bytes, yielding 1048866/958 instead of authored 0/0 for ID923. `read_fields` now tracks separate scalar/array cursors. A compressed unchanged installed payload and pinned DBD in `scripts/tests/fixtures/playercondition-12.1.5.70391/` reproduce the CSV boundary; their receipt records hashes and the 15 unreadable encrypted rows, none synthesized. Canonical re-publication remains lead-owned.

[Contract and lead command](../../specs/db2-follow-install.md) own current publication policy. The private refresh follows active Retail70391, exports365 consumed tables against exact pinned DBD layouts, regenerates eight caches/catalogs and native legacy CSV translations, and compares302 tables with former69933 exports. Census:276020 added,1447 removed,159495 changed and201 identity changes; zero newly missing TDB join IDs. Layout-label/width corrections are not automatically gameplay patch changes. TDB1210 is not imported or rewritten.

A new model receipt updates its owning base/fragment index; older staged receipts cannot reverse the refresh. Runtime `ModelAssetIndex` already resolves the receipt's qualified path and its reload test proves the new namespace. Player assembly selects indexed assets/aliases instead of copying historical product directories. The lead migration authenticates the installed root, reuses identical content through new namespace hard links, extracts changed content locally and removes absent current-root mappings. Private metadata census:191253 identical content keys,808 changed,12 absent;1297 non-Retail receipts preserved. This census is not changed-payload extraction or canonical publication proof.

Native acceptance: authenticated private UDP5821 fb_db2_follow/Dbtwofollow entered Elwynn in Modern and Forever. Both inspected1920×1080 captures draw all twelve7508739 hotkeys plus wildbattlepet/left-right rotation samples through normal named-atlas projection. All30 native regions exactly match current CSV coordinates; sheet SHA256 receipts and visible-pixel counts are recorded. PNGs: `/syncthing/AgentShared/2026-10-13/db2-follow-install/`; private proof: `/home/osso/.worktrees/db2-follow-proof/captures/atlas-region-proof.json`. No client fatal errors; a QuestPOIBlob519073 point-count warning remains, not claimed as pristine startup.

First native RED exposed `LightSkybox.csv`'s new `Name` column: fixed-position parsing prevented world attachment. Named-column parsing handles quoted names and preserves Flags/FDID; focused RED/GREEN and successful native reruns cover the boundary. The fixture now observes the production `Parts/Part0` texture, not the outer frame. Parallel all-table translation import was killed by global OOM at7.5GiB RSS; one CSV per native process finished78 current inputs/678 resources. Legacy translation lookup is tested natively. Current DB2 manifest dry proof covers1509 files/2links, with TDB/auth excluded. These are private preparation proofs; canonical publication remains lead-only.

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

## Equipped-item material closure — verified 2026-10-15

M2 `TXID` and `ItemDisplayInfo.ModelMaterialResourcesID_*` do not exhaust an equipped model's textures. `ItemDisplayInfoModelMatRes` supplies additional `(display, model column, TextureType, MaterialResourcesID)` overrides; resolve each through `TextureFileData` and the same authored race/sex selection as the runtime. Sfkgeared's head/shoulders require types3/4 textures6256534/6153788/6310348/6153800, omitted by the earlier M2-chain audit. Native preview accepted6256534, then exposed6153788; no combat or clear was proved.

A fresh private inventory audit identified ten equipped items. Active70391 source joins and Human-male/shoulder-side selection produce43 assets:4M2,13skins,26BLP. Every staged file matches its installed334a36cf5f3c32a5f7a167405487fa14 CASC content key and SHA256 receipt. The earlier41 included two female-only body variants; the actual male selection uses3946811/3946819 instead of3946810/3946818. Private validation covers43/43 bytes and metadata, duplicate0. Canonical has24/43, missing19 runtime receipts/products; publication remains lead-owned. These are equipped-item graph proofs, not rendered-character, full-instance or native-clear acceptance.

Evidence: `data/diagnostics/encounter-coverage-2026-10-12/native-shadowfang-20261015/equipment-closure-{manifest,proof,private-validation}.json`; reproducible private `prepare-equipment-closure.py`, actual `equipment-current.log`, and `client-{b,c}.log`. Sources: [material resolver](../../../godot/core/src/game/equipment/outfit_catalog.rs), [typed material rows](../../../godot/core/src/game/equipment/item_model_material_data.rs), [component selection](../../../godot/core/src/game/equipment/component_file_data.rs). [Product receipt contract](../../specs/product-isolated-model-assets.md) remains unchanged.

## Asset Naming

- `data/textures/{fdid}.blp` — BLP textures
- `data/models/{fdid}.m2` — M2 models and `.skin` files
- `data/terrain/{fdid}.adt` — ADT terrain files
- `data/community-listfile.csv` — 136MB FDID→path map (from wowdev/wow-listfile)

## Forever overlay

[[forever-data]] records the pinned local-CASC importer, separate Forever CSV overlay, encrypted-row drops, content-key mismatch and AFID/BFID validation limits. [[terrain]] owns Zephras closure inventory; neither importer validation nor available files imply complete native acceptance.

## UI consumers

WoW UI metadata stores authored FileDataIDs. A virtual path is only listfile lookup input; it is never a runtime filesystem path or a fallback export directory. Character-creation race/class icons emit `texture_fdid`; customization arrows and palette regions use an atlas FileDataID. `GameBlpLoader` resolves both through local CASC/cache. See [[ui-system]].

### Native unavailable icons

The native Godot `ui::assets` loader reads `data/textures/<fdid>.blp`. If read/decode fails for a FileDataID whose local listfile path belongs to `Interface/Icons`, it reports the original FDID/error through `frame_error::report_once` and loads the existing `bank_art::UNKNOWN_ICON` (134400). This policy covers asynchronous rectangular UI images and synchronous BLP decoding used by circular masks. Loading remains asynchronous where it already was; valid images and unavailable non-icon panel/model art keep their existing behavior. The listfile path classifies the asset, never names a filesystem export. No runtime CDN fetch or new local-CASC extraction path is added.

Texture successes/failures remain process-cached; installing a previously missing BLP requires a fresh client for this proof. See [auction icon diagnosis](auction-house-ui.md#missing-result-icons-2026-10-08), [extraction](../../casc-extraction.md#ossobuild-local-store-2026-10-08), and the [auction contract](../../specs/auction-house-ui.md#item-icons).

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

- [DB2/current-index contract](../../specs/db2-follow-install.md) — exact-layout exports, consumer identity and guarded lead migration

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
