# Forever data overlay

Forever `1.60.1.70205` supplies Skyborne character data alongside Retail, not as a replacement. Verified evidence: 2026-10-04; native Skyborne preview acceptance remains in progress.

## Import and consumers

[`import_forever_skyborne.py`](../../../scripts/import_forever_skyborne.py) decodes local `wow_classic_beta` WDC5 tables using matching WoWDBDefs layout hashes and records FDID, content/encoding keys, source/definition hashes, dropped rows and absent columns. Build key: `842b2e5d11f8d6fe257a5b73bd5cf6c6`. CSVs and importer provenance/assets manifests live in `data/db2/1.60.1.70205/`; extraction staging is `data/cache/forever-skyborne-extract`, with runtime assets cached under `data/models/` and `data/textures/`. No CDN acquisition.

`player_model_data` substitutes the Forever DB2 chain only for races 95/96: Retail contains placeholders for those IDs. Both races share ChrModel 218/219 → body FDIDs 7478487/7478494. Customization catalogs overlay those models and layouts 201/202 while retaining Retail consumers. The UI roster supplies the grounded names, factions, class defaults/lists and atlas 8200220; see the [contract](../../specs/character-creation.md#skyborne-forever-160170205).

At `2d82554a`, the collection query interprets only Forever skinned-model rows referenced by options on models 218/219. Eight unrelated rows have GeosetID values outside u16; raw source/cache values are retained, unrelated rows are not interpreted, and a selected invalid row still errors. This is not general support for those wide representations.

## Liquid exports

The importer exports hash-matched LiquidType, LiquidMaterial, LiquidObject and LiquidTypeXTexture. Only Forever LiquidType expands to all38 DBD-named Float columns; Retail inputs/other headers stay unchanged. **Material130 drawn with LiquidType5 legacy inputs; PBR parity unsupported.** This is an explicitly borrowed fallback, not client-authored PBR data; source floats/foam textures remain intact. [Terrain: Forever liquid catalogs](terrain.md#forever-liquid-catalogs--bounded-cpu-proof) owns isolation/closure; [near-white water cause and captures](../investigations/northshire-pale-water.md#zephras-material130--explicit-borrowed-legacy-fallback) owns the rendering boundary.

## Source limitations

Importer provenance records **17 CreatureDisplayInfo and 8 Map records dropped** from zero-filled encrypted sections. CDI's extracted MD5 differs from its CASC content key because local CASC zero-filled unknown encrypted chunks, not because the content key identifies the decoded zero-filled bytes. Skyborne CDI rows are in the readable key-0 section. Drops are explicit; encrypted records are not reconstructed.

Observed AFID assets are untagged timestamp/keyframe streams: importer acceptance requires nonempty bytes and exact CASC content-key MD5. BFID payloads begin with version 1 followed by BIDA/BOMT chunks rather than a conventional leading magic. ChrModel skeleton fields 4690403/4690402 resolved to MD21 files, not SKEL; metadata verification retains that limitation. Importer validation does not prove native animation/rendering acceptance.

## Recovery and remaining acquisition gap

The initial root encoding key `fcae3917977c7fdf9f3864ed5bf96521` was absent from local indexes. On **2026-10-04**, Battle.net's `wow_classic_beta` update completed at 18:55 UTC; scoped Syncthing sender rescans corrected stale advertised archive sizes for data.207/209/210. Root initialization then succeeded (1,436,183 entries), allowing the tables and body/skin/icon assets to be recovered.

The historical initial [Zephras inventory](terrain.md#forever-zephras-map-2991) lacked24 roots and recorded169 archive failures. [Post-sync recovery](../../../target/forever-resync/ledger.md) supersedes that availability gap; the [current liquid closure](terrain.md#forever-liquid-catalogs--bounded-cpu-proof) includes its six new textures. NPC metadata limitations below remain separate from terrain asset availability.

## Proof boundary

[Skyborne proof ledger](../../../target/skyborne-proof-ledger.md): targeted core 4/4 at `2d82554a`, retained real body-chain 1/1, UI roster 9/9 and creation/layout 3/3 (17 distinct passing tests). Earlier compile blockers and imported-catalog failure are historical, superseded by the scoped query fix. Native rendered preview, create/save/reload and world entry remain unproved. Zephras rendering evidence is separate and does not establish Skyborne character acceptance.

## Zephras NPC displays — bounded, incomplete

`creature_display_cache` imports Forever CDI rows absent from the base catalog using the Forever CMD table, independently of colliding ModelIDs. Both CSV pairs participate in cache freshness; removing the overlay removes its cached rows. Native `VisualCatalogs` still reads `data/cache/creature_display.sqlite`; it does not rebuild it. The importer adds selected Forever-only rows to that cache without replacing existing rows.

[`forever_npc_displays.py`](../../../scripts/forever_npc_displays.py) follows display variation textures, readable Extra/option/geoset relationships, customization materials/collections and armor ItemDisplayInfo resources. The hash-matched table exports retain provenance and cleartext encrypted-ID metadata. Run the existing importer with `--display-ids <id-file> --spawn-report <server-report>`; grounded-spawn dependency closure precedes deferred displays. New ordinary coverage is published, but required profiles with missing metadata are withheld. Declared missing material mappings and cached bytes differing from the current CASC root are errors; failed assets are not copied into new companion aliases.

Bounded proof at `31f44743` ([ledger](../../../target/zephras-npc-display-proof/ledger.md), [per-display audit](../../../target/zephras-npc-display-proof/per-display.tsv)):

| Boundary | Evidence | Still missing |
|---|---|---|
| Display → model | 235/235 requested, 174/174 grounded; Ailee 136968 and Ventaari 139694 → 7478494 | Native visual acceptance |
| Retail preservation | All 118,499 pre-existing display rows and all pre-existing appearance/choice/geoset/preferred-skin rows unchanged | General multi-product asset isolation |
| Required profiles | 62 new ordinary coverage rows; 0 required profiles added | Extra FDID 1264997 and geoset FDID 1720141 unavailable; 162 displays blocked (125 grounded) |
| Known assets | 3,354 current-root-matched/format-accepted, 32 mismatched cached files retained, 319 unavailable | Dependencies below unavailable/mismatched parents and unknown baked materials are not complete |

The single full import at `189b32b1` exited 1 (`/tmp/zephras-npc-display-import.log`). Its earlier magic-only manifest counted 3,385 assets available and 320 failed; the separate current-policy audit above supersedes that availability count, not the import's provenance. The corrected importer was not rerun against unavailable archives. Twenty of 224 Forever-only displays have no known asset failure (15 of 171 grounded); this is not appearance readiness. All 235 requested IDs are readable CDI rows: none overlap the 17 encrypted IDs recorded in `npc-displays.json`/`provenance.json`.

At `31f44743`, native gear consumers were Retail-only: 876 of 884 referenced armor display IDs were absent there. The gear consumer gap is addressed below; complete humanoid appearance and native acceptance remain open. Existing customization overlays remain Skyborne-scoped. No two-display native fixture or screenshot was attempted in that display-import cycle.

Verification: importer tests 38 passed/3 skipped; desktop core 2/2 (synthetic collision/cache-removal and real exports with every Retail cache row compared); Ruff and touched Rust formatting pass. The core test build retains the pre-existing unused `blend_pixel`/`scaled_section` warning. No line-coverage or rendered-pixel claim.

## Gear overlay — bounded CPU proof (2026-10-05)

At `10f802aa`, `NpcGearData::load` keeps Retail display IDs (including ordinary displays with Extra 0) and joins Forever-only displays to a separate Forever Extra/armor map. `OutfitData` imports absent ItemDisplayInfo IDs and whole absent model/material resource groups into its existing cache. Component ownership is overlaid by FDID; helmet rules by visibility group. Retail collisions retain every Retail row/candidate. Cache source tracking detects overlay addition/removal. Declared missing model/texture mappings remain errors, including body materials that the old importer would have dropped.

Hash-matched exports include ItemDisplayInfo, ItemDisplayInfoMaterialRes, ModelFileData, TextureFileData, both component tables and HelmetGeosetData; the latter is FDID 2821752, layout 103B3B37, 16,909 readable rows, exact content-key MD5. The existing importer acquires them through `forever_npc_displays.TABLES`; no separate decoding path or CDN.

[Gear ledger](../../../target/forever-npc-gear-proof/ledger.md) and [coverage snapshot](../../../target/forever-npc-gear-proof/gear-coverage.json):

| Boundary | Evidence | Remaining gap |
|---|---|---|
| Gear IDs | 884/884 rows resolve: 8 Retail, 876 Forever | Not full appearance readiness |
| Required resources | 856 fully mapped; 849 have validated known root assets | 28 items reference five missing texture resources; 7 other items have root-asset failures; companions are separate |
| Concrete resolution | Display 136967 → Extra 162977 → shoulder 734891 → models 7579617/7579618, texture 7731197 | That NPC's other gear references missing resource 1102747 |
| Preservation | Every Retail row and whole candidate group in four outfit cache tables unchanged; removal restores baseline | General product-isolated asset storage |

The single gear import ran at `a87685d1`, before the concurrent asset-refresh policy `a3d37affe`: `/tmp/forever-npc-gear-import.log`, exit 1, 482 extracted and 3,712 accepted of 3,725 known assets; 13 stale cached assets rejected. Extra and geoset sources recovered; seven required profiles were published, 155 displays still had metadata errors. At that revision, missing body paths in the local listfile and five missing texture resources blocked the requested Skyborne NPCs. The [FDID profile repair](#unnamed-skyborne-npc-bodies--fdid-profile-repair) supersedes the body-path gap. Later shared manifests may be replaced by other import runs; the log/ledger retain this run's boundary.

Verification: integrated Python 42 passed/3 skipped; desktop core outfit 7/7 and helmet 2/2; native product-scoped Extra/collision test 1/1. Scoped rustfmt and Ruff pass. Workspace `cargo fmt --check` is blocked by missing local sibling `ui-toolkit/core`; desktop tests compile the affected crates. Existing core-test unused-import warning retained. No screenshot, full asset-closure, line-coverage or native visual claim.

## Unnamed Skyborne NPC bodies — FDID profile repair

`5f612d83` selects the authored HD bake for Forever body FDIDs 7478487/7478494 without inventing listfile paths. These are the pinned ChrModel 218/219 → CDI → CMD bodies. Shared appearance joining now consumes selected material IDs; the Retail importer retains its existing named-model SD/HD classification. Ailee 136968 and Ventaari 139694 both author female body 7478494; Ventaari is not a male-body fixture.

The single import at this revision (`/tmp/skyborne-npc-profiles-import.log`, exit 1) publishes **134/162 required profiles**, up from 7; metadata blocks fall **155 → 28**. Known reachable assets are **4411/4411** current-root accepted (152 newly extracted). Remaining blocks are absent TextureFileData resources: 1102747 affects five displays, 1102759 six, 1102772 three, 1102870 six and 1102900 eight. Asset availability is not native acceptance.

Fresh local-CASC re-extraction of TextureFileData FDID 982459 is byte-identical to its current content key `c0e65f258ed349d83d24f9289561cb22`: 58,607 readable rows, no encrypted omissions and no resource 1102747. ItemDisplayInfoMaterialRes FDID 1280614 repeats the prior decoded bytes and 13 references to that resource; its three unknown-key records remain explicit omissions. The missing TextureFileData mapping, not synchronization or the encrypted item-material records, withholds Ailee's profile. No substitute texture/body/profile was published. Ventaari's profile resolves race 96, sex 1, bake 7487478.

Bounded CPU proof: importer Python 42 passed/3 unavailable fixtures skipped at `5f612d83`; desktop `forever_npc_profiles` 2/2 at `4241a4b9` proves Ventaari's selected customization, decoded bake, composed type-6 texture and gear, plus Ailee's explicit missing coverage/material error. It does **not** prove full Ailee resolution. The two-spawn native fixture passed at script `a8c1bcb2` with the shared local build `c015b79b`: both displays replicate; Ventaari gets a prepared native visual, Ailee emits the expected out-of-coverage error and gets none. Inspected `screenshots/ventaari-139694-{front,back}.png` in the [profile ledger directory](../../../target/skyborne-npc-profiles-proof/ledger.md) show a blue-gray female with pointed ears, swept-back hair, blue eyes, a dark full-length green/gold-trimmed robe and silver upper-arm bands. This is bounded Ventaari visual proof, not complete Ailee or all-profile acceptance. Final runtime exits 0 with one expected Ailee error, no leak lines and no surviving owned processes. Receipt/script/capture hashes are in `native-artifacts.json` beside the ledger.

The first isolated-World3D capture produced blank PNGs despite successful native allocation. Capturing the original World3D without reparenting the visual fixed the observed boundary. The fixture now rejects blank frames (negative sample count 0; final two views 269 changed samples each); no shader/body fallback or production rendering change was added. The captured shared executable predates the `47c5a844` log-parser readability refactor. A separately authorized local fixture build at `06d2d192` compiles the current helper and exits 0; it retains one existing unused-WMO-field warning. No extra native run was requested, so the earlier captures are not relabeled as runtime proof for that later build. Both build receipts and the full compile log are retained in the profile ledger.

## Sources

- [Recovery ledger](../../../target/skyborn-ledger.md) — root/archive recovery and grounded race/model identities.
- [Skyborne proof ledger](../../../target/skyborne-proof-ledger.md), [Zephras proof ledger](../../../target/zephras-proof-ledger.md) — bounded results and exclusions.
- [Importer](../../../scripts/import_forever_skyborne.py), `data/db2/1.60.1.70205/provenance.json` — decoding, encrypted drops and content-key mismatch reason.
- `data/forever-1.60.1.70205/metadata/{ChrRaces-Skyborn,CharBaseInfo-Skyborn,ChrModel-Skyborn,Skyborn-graph.provenance,Skyborn-graph.verification,Map-readable-identities,Zephras-closure}.json` — local source artifacts; untracked data, not bundled documentation.
- Commits `04a12a2e`, `1737e089`, `e4abfa89`, `9dd52bc8`, `2d82554a` — importer/formats, consumer overlay, roster and scoped query.

## See Also

- [[character-creation]] — catalog/UI and acceptance boundaries
- [[asset-pipeline]] — local CASC resolution
- [[terrain]] — Zephras provisioning and rendering limits
