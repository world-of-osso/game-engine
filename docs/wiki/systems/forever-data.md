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
