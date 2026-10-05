# Forever data overlay

Forever `1.60.1.70205` supplies Skyborne character data alongside Retail, not as a replacement. Verified evidence: 2026-10-04; native Skyborne preview acceptance remains in progress.

## Import and consumers

[`import_forever_skyborne.py`](../../../scripts/import_forever_skyborne.py) decodes local `wow_classic_beta` WDC5 tables using matching WoWDBDefs layout hashes and records FDID, content/encoding keys, source/definition hashes, dropped rows and absent columns. Build key: `842b2e5d11f8d6fe257a5b73bd5cf6c6`. CSVs and importer provenance/assets manifests live in `data/db2/1.60.1.70205/`; extraction staging is `data/cache/forever-skyborne-extract`, with runtime assets cached under `data/models/` and `data/textures/`. No CDN acquisition.

`player_model_data` substitutes the Forever DB2 chain only for races 95/96: Retail contains placeholders for those IDs. Both races share ChrModel 218/219 → body FDIDs 7478487/7478494. Customization catalogs overlay those models and layouts 201/202 while retaining Retail consumers. The UI roster supplies the grounded names, factions, class defaults/lists and atlas 8200220; see the [contract](../../specs/character-creation.md#skyborne-forever-160170205).

At `2d82554a`, the collection query interprets only Forever skinned-model rows referenced by options on models 218/219. Eight unrelated rows have GeosetID values outside u16; raw source/cache values are retained, unrelated rows are not interpreted, and a selected invalid row still errors. This is not general support for those wide representations.

## Source limitations

Importer provenance records **17 CreatureDisplayInfo and 8 Map records dropped** from zero-filled encrypted sections. CDI's extracted MD5 differs from its CASC content key because local CASC zero-filled unknown encrypted chunks, not because the content key identifies the decoded zero-filled bytes. Skyborne CDI rows are in the readable key-0 section. Drops are explicit; encrypted records are not reconstructed.

Observed AFID assets are untagged timestamp/keyframe streams: importer acceptance requires nonempty bytes and exact CASC content-key MD5. BFID payloads begin with version 1 followed by BIDA/BOMT chunks rather than a conventional leading magic. ChrModel skeleton fields 4690403/4690402 resolved to MD21 files, not SKEL; metadata verification retains that limitation. Importer validation does not prove native animation/rendering acceptance.

## Recovery and remaining acquisition gap

The initial root encoding key `fcae3917977c7fdf9f3864ed5bf96521` was absent from local indexes. On **2026-10-04**, Battle.net's `wow_classic_beta` update completed at 18:55 UTC; scoped Syncthing sender rescans corrected stale advertised archive sizes for data.207/209/210. Root initialization then succeeded (1,436,183 entries), allowing the tables and body/skin/icon assets to be recovered.

The [Zephras inventory](terrain.md#forever-zephras-map-2991) retains 169 archive failures, with 48/72 roots available: **24 root tiles remain unavailable**. User-reported current blocker: desktop C: full, Syncthing stopped, with 21 other files unavailable locally. The disk/Syncthing state and that 21-file subset are not established by the supplied ledgers/metadata; they must not be treated as a replacement for the measured 169-failure closure or as proof of complete dependencies below missing parents.

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

Native humanoid support remains **incomplete**, not merely unverified: `NpcGearData` and the outfit consumer are still Retail-only; 876 of 884 referenced armor display IDs are absent from Retail. Five referenced item texture resources are absent from the decoded Forever TextureFileData. Existing customization overlays remain Skyborne-scoped. No two-display native fixture or screenshot was attempted: rendering unprofiled bare bodies would bypass the required-appearance contract. Source recovery alone does not finish these remaining consumers.

Verification: importer tests 38 passed/3 skipped; desktop core 2/2 (synthetic collision/cache-removal and real exports with every Retail cache row compared); Ruff and touched Rust formatting pass. The core test build retains the pre-existing unused `blend_pixel`/`scaled_section` warning. No line-coverage or rendered-pixel claim.

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
