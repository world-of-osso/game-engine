# Offline asset closure audit

`scripts/asset_closure.py` reads extracted files and local metadata only. It never discovers an install, reads CASC, launches the client/server, or extracts assets. The CLI constructs `ReceiptClosure`; extraction producers explicitly use `RawByteGraph`,which is byte discovery rather than an audit. Neither graph alone is a certified release asset set. Product scope is all zones/races/items; Northshire remains the initial proof slice.

## Usage

From a checkout, use `agent-run` for agent audit/test/extraction processes; no build lock for these Python jobs (the lock is release-only):

```text
python3 scripts/asset_closure.py --data /path/to/extracted/data --world-db /path/to/world.db --output data/diagnostics/closure-slice-2026-10-09/manifest.json --estimate-output data/diagnostics/closure-slice-2026-10-09/full-catalog-estimate.json
```

Default config is `scripts/closure-northshire.json`: Azeroth tiles (32,48), (33,48), (32,49), (33,49), encompassing the Abbey, lake and vineyard vicinity; male level-1 Human Warrior with first player-selectable choices in option order (ReqType player bit, race/class masks and no account unlocks). Starting gear is world.db's Purpose-9 CharacterLoadout plus race/class item overrides, **not** the obsolete CharStartOutfit kit. NPCs are the imported `content_creature` rows inside those tile bounds; all template display alternatives and explicit spawn model overrides are included. Dynamic game objects use the explicit `game_objects` selector: `tiles` selects placed template IDs within those same bounds; `all` includes every template, including unspawned/script-summoned entries. Template display IDs join `GameObjectDisplayInfo.FileDataID`; native type15 map transports seed WMO, other native model requests seed M2. Selected template/type/display rows enter the world-selection fingerprint. This over-enumerates the slice rather than relying on a playthrough.

Selected map IDs also seed authored skies through `Light.ContinentID` → every nonzero `LightParamsID_*` → `LightParams.LightSkyboxID` → `LightSkybox.{SkyboxFileDataID,CelestialSkyboxFileDataID}`. This includes global maps without ADT tiles and all weather slots, not just current weather. Positive missing rows stay unresolved; zero references issue no file request. M2 traversal exposes skybox skins and other descendants; the observed Skyborne7345733→7345740 gap motivated this root fix.

Spellbook seeds include auto-learned class/default-skill spells and future Warrior entries displayed in the book, plus Initial-spec spells. Spell IDs and chosen customization IDs are recorded. Spell icons come from pinned `SpellMisc` DifficultyID=0 rows. No spell visuals/audio are part of this bounded request.

`table_paths` explicitly names four top-level legacy CSV inputs absent from the pinned directory; each remains a reported unverified metadata build. This is source configuration, not an alternate-path fallback. Unknown/default-locked customization requirements are not selectable; NPC-only options are excluded from the bounded player.

`customization_cache` in the seed config explicitly selects the SQLite source for both player and NPC customization joins. The maintained configs use `cache/customization-v5.sqlite`, matching native `customization_query_data::customization_cache_file`; the obsolete unversioned cache is not an alternate source. Imported cache contents still require separate provenance authentication.

`--config scripts/closure-full-catalog.json` changes roots to all Map directories/tiles, modeled race/sex pairs/all customization choices, all world.db creature and game-object template displays, all content_item rows and all spell_name rows. This is a conservative catalog superset. Nonplayable placeholders, unsupported source products, customization requirements and pruned server spell reachability are unresolved policies, not silently inferred support.

## Manifest and boundaries

- Assets are sorted by FDID/type; incoming edges and local paths are sorted. Fixed-point work queue handles cycles and late owner aliases: a skeleton initially missing by its own FDID is expanded if a later model edge supplies an existing owner cache path. Modern chunk tags follow existing core parsers.
- Named paths reproduce asset-resolver's SQLite import (`INSERT OR REPLACE`, unique FDID **and** lower_path), including a repeated FDID displacing a previously bound path. `--path-resolution-cache <absolute snapshot path under data/>` uses a read-only SQLite snapshot of `data/local-listfile-cache.sqlite` before community bindings, matching native resolvers' shared data root. The snapshot is hashed as an input; absence of a binding stays unmapped, not guessed. Full Map roots use the CSV's WdtFileDataID; TXID/MDID override stale filenames even for zero/short slots; those slots never select a filename fallback. `resolved` rows explain formerly ambiguous selections; the original listfile candidate census remains a superset.
- M2 SFID skin and SKID skeleton FDIDs retain their identity and record the existing owner's runtime cache aliases. Different bytes at aliases are a conflict, not an accepted source. WMO groups include all GFID LODs; all doodad sets/component item variants are included.
- Every present file has size/SHA-256. Metadata files and tool/config bytes are hashed. world.db is read in one read transaction; selected content seeds have a canonical JSON fingerprint (not an incorrect hash of a live SQLite main file ignoring WAL).
- `ReceiptClosure` requires matching product/FDID/kind receipts for model roots and validates their size,SHA256 and content-key MD5. Model-owned SFID/SKID/animation/textures use the owning model's namespace; authenticated alias bytes must match their same-product unique owner. Missing receipts never select flat or other-product bytes. Product,namespace,FDID and kind distinguish graph nodes,including explicit mixed-product roots. Schema2 adds product/build-qualified source edges; payload changes invalidate a previously verified identity.
- `requested_product` and `metadata_build` describe the configured source. Receipt-validated scoped nodes record `source_product`, `actual_build` and `identity_status=verified_receipt`. Native flat WMO/ADT material paths never switch to another scoped current copy; their actual flat texture bytes can qualify only through matching generic extraction receipts. RawByteGraph retains raw unverified identity fields. Existing filenames, CSV directory labels and cache mtimes never authenticate origin. [Metadata-owned roots](../../handoffs/noinstall-cert-2026-10-21.md#metadata-owned-auditor-roots) now follow native MapCatalog precedence,creature-display sources and independent NPC model/material resource sources. Owned selectors require an explicit ItemDefinitionSource; unsourced IDs remain individually unresolved rather than inheriting race/global product. The [explicit player overlay](../../handoffs/noinstall-cert-2026-10-21.md#explicit-skyborne-player-source-overlay) separately follows the native Skyborne-only95/96 body/customization source chain,not the merged NPC display cache. Cache source rows still do not authenticate import provenance; supported playable/customization policy remains open.
- `closure_terrain.py` inspects all repeated MCNK chunks (root 128-byte header versus headerless split companions), joins MCLY effects through GroundEffectTexture/Doodad and MH2O instances through LiquidObject/Type/XTexture, and includes liquid shader-global textures. MDID/MHID are FDIDs; MTXF is flags. `resolved` records preserve per-file inline-only/empty-liquid evidence; absent required metadata and unknown nested chunks remain unresolved. Optional ground-effect rows missing from a present hashed CSV are recorded not-needed because the current scatter/model-FDID collection skips them before requesting bytes. MPTX/legacy MCLQ are recorded unused by current core loaders, not proven free of Retail-format references. WDT MAID already supplies all eight split/LOD/map/minimap identities; map WDL is a separate named seed.
- Missing expandable files, unresolved joins/paths, malformed chunks, conflicting aliases, emitter auxiliary edges and unsupported customization effects are reported explicitly. Exit 1 means an incomplete/unverified audit, while still writing the manifest. Exit 0 is reserved for zero missing/unresolved/unverified identities; current legacy data cannot meet it.

The full-catalog census counts listfile candidates by extension and actual extracted disk files/bytes. Additional extraction file counts and bytes are **estimates**, using numeric FDID overlap and the biased local mean size per class. Mixed-product listfile entries, owner-named skin aliases and absent receipts prevent treating these as verified release coverage. No size is guessed when there is no local sample. Unnamed FDIDs can be absent from the listfile, especially skeletons, so the census is neither an upper nor a lower bound on the required full catalog. Compression/download size is unknown.

## Model-owner recovery boundary

[Owner/companion source projection](../../handoffs/noinstall-cert-2026-10-21.md#model-owner-and-companion-source-projection--no-publication-yet) distinguishes authenticated generic source bytes from native-owned namespaces. Native selects a unique product/FDID/kind owner before constructing its resolver; recovery requires explicit immutable owners and authenticated companion aliases,not flat-byte fallback. Historical source identity stays historical even when identical bytes require an alias under a newer model namespace. Missing source receipts,global-owner-present unresolved parent namespaces and untraversed descendants remain separate gaps. A globally known texture owner does not override its requested parent namespace.

## Canonical raw DB2 source mismatch

The [canonical raw census](../../handoffs/noinstall-cert-2026-10-21.md#canonical-raw-db2-mismatch-census--report-only) records unversioned raw caches that do not match the pinned70448 raw receipt hashes,including both HelmetGeosetData aliases. A WDC5 `WOWSTATIC` schema label is not an authenticated extraction-build receipt. Preserve the mismatch and unknown origin as named gaps; do not use canonical raw filenames to invalidate separately authenticated frozen CSV/private restored-source proofs,or silently replace their bytes. Investigation is read-only.

## Non-model receipt boundary

[Texture/sound and DB2 qualification](../../handoffs/noinstall-cert-2026-10-21.md#non-model-texturesound-and-db2-receipt-qualification) uses existing generic extraction JSONL records and per-directory table export receipts,not new model-index kinds. Actual flat bytes retain their paths; product/path/content mismatches remain gaps and historical matching builds are not relabelled. Hash-only UI inventories do not authenticate origin. Selected DB2 tables require matching authenticated product/build/CSV hash; partial readables retain exact unreadable counts. The [corrected r5 full graph](../../handoffs/noinstall-cert-2026-10-21.md#corrected-full-graph-r5--complete-traversal-explicit-gaps) completes traversal and artifact reconciliation with explicit gaps,not a P4 certificate. Table-receipt refusals block downstream spell/game-object/player roots and Forever maps; exact flat ADT/WMO bytes now use distinct generic receipt families with unchanged product/path/size/SHA256/content-MD5 checks. Their historical build/build-key provenance is preserved,not relabeled to metadata identity; source qualification does not certify parser acceptance. WDT/WDL/physics and model companions gain no eligibility. Cache-import provenance remains unqualified. Exact counts and evidence live in the handoff rather than being duplicated here.

### Named encrypted-section qualification

`closure_db2_policy.py` implements [`DB2_ENCRYPTED_SECTION_KEY_UNAVAILABLE`](../../specs/offline-asset-closure.md#encrypted-db2-section-policy). It checks the retained WDC5 hash/layout and section record spans,requires matching missing-key extraction evidence,then checks a hashed build-info/keyring/external-key census. Available keys require a decrypting re-export,not exclusion. Unexplained unreadable rows refuse the whole table.

The source receipt keeps `content_authenticated=false`; accepted CSV inputs instead record `authenticated_readable_sections`. `db2_policy_exclusions` lists each section/key/base-record/copy-record count and its disclosed key-census build. Summary fields `db2_policy_excluded_records`, `db2_policy_excluded_copy_records` and `db2_policy_excluded_rows` keep base records,declared copy rows and their total separate from unresolved diagnostics. Encrypted-chunk evidence must cover the copy-table tail too. Changing a proof input invalidates qualification. The existing decoder/WoWDBDefs pipeline still owns row export; this verifier reads encryption metadata only. [Current measurement and bounded implementation](../../handoffs/noinstall-cert-2026-10-21.md#encrypted-db2-receipt-policy--bounded-implementation) does not certify newly reachable descendants.

### Restoring a pinned partial DB2 source

[Archived-source recovery](../../handoffs/noinstall-cert-2026-10-21.md#pinned-db2-source-recovery--effectname-restored-other-payloads-blocked) authenticates cached pinned-build root/encoding metadata before looking up retained local archive envelopes. A recovered source must match its full16-byte EKey and encoded-chunk checksums; active-build data is not substituted. Re-export must reproduce pinned raw/CSV/schema hashes and row counts before qualification.

A decoded partial file has already lost unavailable ciphertext. Newly available keys cannot recover those bytes without the original encrypted envelope. Known-key sections therefore require decrypting source recovery; they cannot enter the unavailable-key bucket. Reconstructed raw-container sentinel ranges are never emitted as zero-valued rows. Missing source envelopes remain restoration gaps,not unreachable content. Host libgcrypt supplies the private restoration's Salsa20 primitive; no new package or runtime dependency is introduced.

## Bounded proof (2026-10-09)

Code `5520e82ff`: twelve concrete fixtures pass under the build lock; two real manifests are byte-identical (SHA256 `98b1fd059d2c70a82d85764e21a4c86cdfb4cae55af833fc09d318cb4700c393`). The four-tile audit selects 753 NPC spawns/116 NPC display alternatives, nine starting items, 24 spellbook spells and twelve player choices. It finds 2,585 identities: 2,472 present (482,511,719 bytes), 113 absent (112 skins and Abbey bell M2 FDID189599), 156 unresolved records and 2,585 unverified identities. Evidence is under `data/diagnostics/closure-slice-2026-10-09/`; audit exit1 is expected. No extraction or pristine no-install gameplay proof.

## Closure continuation — verified 2026-10-10

On `closure-unresolved`, terrain auxiliary warnings **184,670 → 0**, named ambiguities **47,249 → 0**. Terrain extraction reached fixed point in four rounds (5,398 runtime paths /445,790,591 bytes); runtime-binding/declared-Map-WDT extraction took six rounds (6,256 paths /2,404,843,862 bytes), including the WDTs' newly discovered ADT/WMO/model descendants. Total **11,654 paths /2,850,634,453 bytes**. Final graph: 862,509 present /2,275 missing /86,217 unresolved, 864,784 identities. World selection fingerprint unchanged; optional emitter/spell-kit joins remain open, not no-install gameplay/P4 certification.

Evidence: `data/diagnostics/closure-unresolved-2026-10-10/{summary.md,summary.json,terrain/,paths/}`; immutable native local-listfile snapshot has 141,188 rows. New receipt ranges pass authenticated root/native keys, decoded size, MD5 and SHA-256; 50 concrete Python fixtures pass, including memory-limited graph replacement. Jobs stayed below a 14GiB limit with swap disabled (observed peaks about12.1GiB). Independent model verifier unavailable (expired OAuth), no independent PASS claimed.

Publication uses an independent same-directory temporary, flush/fsync, atomic no-clobber **link**, then temporary-name removal—not an in-place write or a hardlink to mutable extraction staging. Native CASC writes only private batch staging in this workflow. All 938,530 receipt rows in the lead-requested size audit matched; the remaining appended rows are covered by content/key proof. Existing files are never overwritten. Separate receipt proof does not authenticate every unreceipted legacy file or populate the graph's still-unverified identity fields.

## Resolver continuation — tooling verified 2026-10-10

`closure-unresolved2` completed round 3 extraction and owned receipt proof; [final dataset](#round-3-final-dataset--verified-2026-10-10) supersedes the prior dataset milestone. Tooling `20787e177` retains 69 isolated Python fixtures passing with no warnings; no tracked resolver changes invalidate that proof. This is not no-install gameplay acceptance.

| Boundary | Traversal now handles | Remaining evidence boundary |
|---|---|---|
| Spell kits | WoWDBDefs effect discriminants, model attachments, emission/barrage models, recursive beam textures/sounds, texture blends, decals, screen effects and conditional unit-voice kit superset | Client-scene script assets and missing rows remain explicit; numeric-only effects have current-native no-file-IO evidence, not complete Retail rendering support |
| Emitters | Core 272/274 legacy filename arrays, 0x1ec particle stride, packed 5-bit multitexture indices, 176-byte ribbon texture arrays and recursive M2 work-queue cycles | Malformed arrays and unmapped filenames remain errors; no alternate Legion-header guess |
| Extended displays | Extra's model-path-selected HD/SD bake, raw element material/collection/conditional-model/display choices, NPC item-display resources including ModelMatRes | Zero HD never substitutes SD; unknown voice semantics, missing metadata and unauthenticated legacy caches remain boundaries |
| WMO liquids | Owning MOHD flags carried to GFID/named groups; core MOGP liquid translation; shared and late roots; LiquidType/Material/XTexture and existing renderer globals | MLIQ's material index is MOMT, not a liquid ID; hidden tiles still load the material before geometry; orphan roots are unresolved |
| Auxiliary identities | Actual unlisted sound headers, BFID bone/PFID physics satellites, raw elements behind legacy unsupported-choice flags | Unknown sound bytes remain unknown; satellite collection does not implement runtime physics/poses |

Local DB2s are exported with exact Retail layout hashes, not inferred field offsets. Empty WDC5 tables can legitimately lack storage descriptors. Layout definitions are preserved and hashed under `data/diagnostics/closure-unresolved2-2026-10-10/sources/`; exported CSVs and immutable phase tooling snapshots are separate evidence. `TextureFileData` now uses the pinned local export in full-catalog mode. Partial encrypted tables report dropped records; readable rows do not prove complete metadata coverage.

Each class runs from an immutable scripts snapshot under `agents.slice`, then uses the existing authenticated local-CASC batch publisher and fixed-point driver. Failure records carry forward without inventing identities or retrying known unavailable bytes. `be1a2e37c` counts only retryable local-index identities when testing the extraction fixed point; recorded failures still remain missing in the manifest. This prevents known encrypted/indexed failures from forcing redundant extraction rounds. Later `687839b78`/`20787e177` preserve a fresh missing frontier exposed by peer publication, even when own extraction created zero files and the missing count is unchanged. Required owner aliases count as work even when the numeric skeleton already exists; newly classified root-absent descendants persist to failure records. No install is used by the traversal itself. Manifest fingerprints include every resolver module; hashes and separate receipt proofs do not automatically authenticate legacy bytes or caches.

Evidence root: `data/diagnostics/closure-unresolved2-2026-10-10/`. Completed phase summaries, `summary.{json,md}`, `final-fingerprints.json`, `final-source-drift.json`, `final-residuals.json` and `receipt-reconciliation.json` own the exact final counts, hashes, boundaries and receipt ranges. Six existing model parse boundaries have inspected non-M2 headers: one reverse-MVER WMO and five M3DT models; do not misreport them as proven corrupt M2 files or silently invent format support.

### Round 3 final dataset — verified 2026-10-10

| Graph metric | Completed round-2 baseline | Final round 3 |
|---|---:|---:|
| Present identities | 862,509 | 970,356 |
| Missing identities | 2,275 | 2,302 |
| Unresolved records | 86,217 | 5,024 |
| Total identities | 864,784 | 972,658 |
| Present identity bytes | 139,754,981,627 | 143,732,207,363 |

The first post-resolver spell audit had 862,700 present /65,529 missing /51,968 unresolved: discovery enlarged the graph before extraction. Final missing totals therefore are not a fixed-scope failure-rate comparison.

| Resolver warning class | Baseline | Final |
|---|---:|---:|
| Spell-kit effects | 35,781 | 0 |
| M2 emitters | 20,026 | 0 |
| Extended appearances | 18,716 | 0 |
| Unknown sound types | 4,476 | 22 |
| WMO liquids | 1,259 | 0 |
| Unsupported customization effects | 678 | 0 |
| Unsupported model edges | 112 | 0 |

Emitter warnings reached20,032 after spell descendants; extended-display resolution later covered18,722 records. Unknown sound warnings reached5,974 before header classification. Complete before/after **all** warning classes are in evidence `summary.md`; zero warning counts do not certify complete format/rendering support.

| Extraction phase | New runtime paths | Bytes | Fixed-point rounds |
|---|---:|---:|---:|
| Spells | 125,999 | 1,795,450,372 | 3 |
| Emitters | 203 | 38,407,872 | 1 |
| Appearance | 58,028 | 2,915,819,214 | 3 |
| Sounds | 2,329 | 76,053,234 | 1 |
| WMO / satellites | 1,506 | 125,827,230 | 2 |

Runtime total **188,065 paths /4,951,557,922 bytes**. Two raw-DB2 publication batches add35 paths /16,437,669 bytes: **188,100 paths /4,967,995,591 bytes** overall. Phase totals include descendants, aliases and earlier pending frontiers—not exclusively class-owned bytes. Emitter parsing introduced no new identities; its203 paths completed a prior shared-data frontier. Graph bytes count identities, whereas publication bytes include required runtime aliases; dependency-only extractor build seeding is excluded.

Final manifest: evidence `wmo/round-02/manifest.json`, SHA-256 **`517829f9704c57920fbbe86cfc3566a104f9dff7d72f4bf9c557013d188a8c2a`**. Resolver/config/input hashes are preserved in `final-fingerprints.json`.

Source drift is explicit: world-selection SHA-256 `5549a27c4874a268eb1e443dfda7e7d5bc22cda92c785e955a91eeb33152dc2c` → `fa0ad2f2a1282570da12f8e54c948bdb9c5265ed4b156607dee937247c1cf0e5`, **27 added display roots**, no removed displays or changed item/spawn/spell roots. Exact IDs are in `final-source-drift.json`. Compared with the completed baseline,36 hashed input paths were added and legacy `TextureFileData.csv` removed; shared input paths have no changed hashes. The later appearance traversal additionally began reading the existing pinned `ChrCustomizationDisplayInfo.csv`: an input-set addition, not changed bytes. Peer additions and evolving roots prevent a pristine/immutable-dataset claim.

| Owned receipt byte range | Authenticated rows | Foreign-schema rows excluded |
|---|---:|---:|
| 368708685..418094009 | 126,132 | 0 |
| 418094009..418398492 | 93 | 670 |
| 418398492..428783024 | 26,675 | 0 |
| 428783024..444535058 | 40,149 | 0 |

**193,049 owned rows, zero errors**:188,100 extracted plus4,949 verified-existing. Authenticated root/native content and encoding keys, size, MD5 and SHA-256 pass; extracted path/byte totals exactly reconcile every phase. Only previously unproved suffixes were newly checked. The670 foreign `version`/`disposition`-schema rows remain explicitly excluded; this is not whole-ledger or legacy-identity authentication. `final-owned-receipt-proof.{json,log}` and `receipt-reconciliation.json` preserve proof.

Residual boundaries, with every record in `final-residuals.json`:

- **3,643 metadata gaps**:3,382 SpellMisc DifficultyID=0,247 CreatureDisplayInfo, eight NPC item-resource joins, four SoundKitEntry kit joins and two SpellVisualEffectName rows.
- **1,297 unexpanded dependency records** overlap the2,302 missing identities:2,127 absent from the authenticated local root and175 indexed encrypted records rejected for unsupported8-byte IVs. No unavailable bytes were fabricated. Raw metadata also retains key-rejected FDID4050937; CameraEffect dropped one encrypted row and the existing ItemDisplayInfoModelMatRes export dropped129 rows.
- **19 client-scene scripts /22 unknown sound types** remain. All35,878 pinned customization-element voice values are zero; nonzero voice joins remain unsupported, not certified by that dataset.
- **27 alias conflicts /six non-M2 headers /four unmapped paths** remain. FDID4928485 is reverse-MVER WMO;6655655/7267179/7267181/7267182/7267183 are M3DT, not proven corrupt M2s. Two unauthenticated caches, two legacy metadata sources and two catalog-reachability policies also remain.
- **972,658 manifest identities remain unverified** despite the separate new-receipt proof. No native/game/CI/pristine/no-install P3/P4 acceptance or independent verifier PASS is claimed; independent model verification remained OAuth-unavailable.

### Extractor artifact lifetime

The shared `asset-resolver/target/debug/casc-local` disappeared after the spell phase, stopping emitter extraction before publication. Recovery uses the native helper and an explicit dependency-only warm seed (455,378,969 bytes); this filesystem rejected reflinks, so no full-target clone/cold bulk-cache rebuild was used. The rebuilt extractor is retained under the evidence root's `tools/` with its source revision and SHA-256; resumed phases verify that private artifact instead of depending on mutable shared build output. No packages, releases, protected host services or asset-download sources changed.

Appearance's original driver later exceeded its14GiB cgroup: the receipt/allocator heap remained alive while a separate traverser built another graph. Raising only the soft limit did not fix that overlap. Task-only `staged-recovery.py` resumed from retained round-1 publication/progress, rerunning only the failed traversal, then separated extraction, traversal and analysis into sequential processes at unchanged14GiB hard/zero-swap limits. The standalone recovery traversal completed at13GiB peak. A256MiB controller also rejected an oversized decoded source-fingerprint report; completed analysis was retained and large fingerprints separated from the small frontier/count report, with the cap unchanged. Failed logs/journals are preserved. This operational wrapper is not a fix or acceptance claim for the tracked combined-process driver; retire it when that driver has process-isolated stages. The512MiB short-test mitigation was not lifted.

### Bounded-test incident

A new RED fixture falsely returned `files=1` for unchanged data, so its round ledger grew indefinitely and the short-test process was OOM-killed (36,423,376,896-byte slice peak). The corrected fixture returns zero after its one available leaf is present; finite RED/GREEN proves retryable-count termination without marking the unavailable leaf present. Continuation short tests now use an explicit 512MiB hard maximum and zero swap on their own slice; a soft `MemoryHigh` alone did not bound the runaway. Heavy extraction units are separate. Other-process impact was not audited. Evidence: `test-fixture-oom-incident.txt`, `retryable-finite-red.log`, `retryable-green.log` under the continuation evidence root.
## Recorded-gap recovery — verified 2026-10-10

`recheck_local_asset_gaps.py` is separate from the offline graph audit. Feed a product-keyed JSON inventory of FDID/type/locations, an explicit read-only install, canonical data root, existing extractor and private output directory. It authenticates current metadata, retries indexed payloads, then uses the existing no-clobber publication path. Unknown-key output stays unpublished; server imports are outside this tool.

Evidence `data/diagnostics/forever70338-keys-2026-10-10/` records stable archive/config/IDX mtimes across180s and idle Syncthing with zero needed files. Forever1.60.1.70338 now has a readable root, unlike70334. Seven recorded Skyborne bakes and Retail vehicle model7476985 are published. All204 original IV8 failures decode with the existing blteiv extractor; three old IDX misses and one old size mismatch also resolve. This is asset recovery, not renderer or complete-closure acceptance; original root-absent identities and alias conflicts remain recorded individually.

Sibling asset-resolver's `scripts/import_dbcache_keys.py` joins16-byte TactKey payloads with8-byte TactKeyLookup names by record ID, supplementing hotfix lookups with a local lookup DB2. Retail cache yields48 names, all already present in the external store; no new names added. Reports never include key bytes. The Classic-beta cache is absent locally.

Reexports preserve existing CSVs. Retail candidates under `data/db2/12.1.0.69933/recheck-forever70338/` expose Vehicle+6, VehicleSeat+8 and GlobalStrings+65 IDs relative to the recorded/current CSVs. Creature's43-row gain over its older handoff is already present in canonical CSV. Forever reexports use distinct1.60.1.70338 paths. No world.db import or live acceptance performed.

## Round 4 local fixed point — verified 2026-10-10

Evidence: `data/diagnostics/closure-round4-2026-10-10/`. Three standalone full-catalog traversals completed; extraction and graph heaps never overlapped. The final authenticated local-root recheck has **zero retryable identities**. This means locally available bytes reached a fixed point, not that the no-install target is complete.

| Graph metric | Round 3 | Round 4 |
|---|---:|---:|
| Present identities | 970,356 | 970,584 |
| Missing identities | 2,302 | 2,127 |
| Unresolved records | 5,024 | 4,993 |
| Total identities | 972,658 | 972,711 |
| Present identity bytes | 143,732,207,363 | 143,760,566,463 |

| Boundary | Before → after | Proof / remaining reason |
|---|---|---|
| Unexpanded dependencies | 1,297 → 1,290 | Seven IV8 ADTs recovered and expanded. All remaining records reference FDIDs absent from the authenticated Retail root:1,164 ADT,105 WDT,21 M2. |
| Metadata gaps | 3,643 → 3,640 | Two SpellVisualEffectName rows and ModelResourcesID85100 recovered. Refreshed ModelFileData exposes11 M2 roots; their41 skins/one texture expanded/extracted. |
| Unknown sounds | 22 → 0 | Authentic bytes identify Ogg; required Ogg aliases published. |
| Alias conflicts / client-scene scripts | 27 /19 → unchanged | Conflicting bytes preserved; script semantics not implemented. Six non-M2 headers/four unmapped paths also unchanged. |
| Provenance / policy | Not certified | 972,711 legacy graph identities unverified; two cache/three metadata markers and two reachability policies remain. Diagnostic CSV overrides add one metadata-build marker; receipts do not suppress it. |

Remaining missing identities are **2,126 root-absent positive FDIDs plus invalid FDID−1**, not encrypted payloads. All175 formerly rejected IV8 identities now have authenticated bytes. Metadata classification preserves every record:3,382 SpellMisc spell joins absent from readable raw records (417,635 parent joins exactly match pinned CSV;55 encrypted rows withheld),247 missing readable CreatureDisplayInfo rows (10 withheld), four SoundKitEntry kit joins (2,537 withheld), and seven item-resource joins absent from complete authenticated ModelFileData/TextureFileData. Membership in an encrypted remainder is explicitly unproven, not asserted absent. `metadata-classification.json` and `final-residuals.json` own the per-record reasons.

Separate CSVs are selected by evidence `closure-config.json`; existing metadata is not overwritten. ModelFileData's pinned `2AE4E788` exporter retains inline FDIDs after the six-float bounding box, resource/flags/LOD values and rejects other layouts. `48fb38b10` has concrete synthetic CLI RED2/GREEN2. Readable sections of key-rejected DB2 staging may supply CSV rows but are never published as authenticated complete DB2s.

Publication creates **330 legacy/runtime-alias/raw-DB2 paths /44,418,564 bytes**, excluding scoped-index publication. Root/key/size proofs for each phase pass;789 requested readable paths have matching MD5/SHA-256 receipts. This is bounded publication proof, not whole-ledger/legacy-origin authentication. `summary.json`, `publication-proof.json`, `frontier{-2,}-proof.json` and `receipt-proof.json` preserve exact scope.

Runtime-miss recovery uses the [product-isolated importer](../../specs/product-isolated-model-assets.md): derived index **3,386 →3,774 entries**,319 aliases retained,477 requested entries verified in actual-build product namespaces. Backup: `data/cache/model-asset-index.json.pre-closure4-20261010`. `ac34ad95e` changes asset publication to atomic no-clobber links: equal bytes preserve their inode; differing bytes fail explicitly. RED2/GREEN7 and full Python **280 run /277 pass /3 skipped, exit0** at that revision; inherited ResourceWarnings remain visible. No extension build or independent verifier PASS claimed.

Three preserved legacy M2 conflicts (`119369`, `119376`, `1000764`) have no closure receipts and match neither Retail69933 nor authenticated cached/current Forever70291/70338 roots. Their original writer/build is unknown, not guessed Forever. Authenticated Retail copies coexist in scoped paths. Runtime `200010.m2` recovered; `8203433.blp` remains root-absent. Display145204/145205 exists in pinned CSV and readable raw CDI but not the runtime cache; no display-cache rewrite. Scoped index publication covers recorded requests, **not complete model chains or runtime acceptance**.

Final manifest `manifest-3.json` SHA-256 **`8349cd5eb377bcf911269ef6d1a22aabc38a25c9e7b52ed5a2a7f292fa4448ac`**. World-selection fingerprint remains `fa0ad2f2a1282570da12f8e54c948bdb9c5265ed4b156607dee937247c1cf0e5`; two explicit CSV inputs replace their previous paths. Shared `cache/npc_appearance.sqlite` changed concurrently; exact hashes in `source-drift.json`. No immutable/pristine dataset, master merge, world.db write, download, runtime/P3/P4 or no-install certificate.

## No-install receipt recovery — verified 2026-10-10

[Legacy authentication contract](../../specs/offline-asset-closure.md#recorded-legacy-byte-authentication)
owns the publication rules. Reproduce the recorded inventory with:

```text
python3 scripts/verify_legacy_product_assets.py --misses data/diagnostics/noinstall-2026-10-10/misses.json --install "/syncthing/World of Warcraft" --data data --output data/diagnostics/noinstall-receipts-2026-10-10/legacy-authentication --build wow=12.1.0.69933 --build wow_classic_beta=1.60.1.70338
```

Private staging uses `model_asset_publication.deduplicate_staged_fragment` against the canonical base index and every receipt fragment before producing a new fragment. Identical product/FDID/kind bytes retain their existing owner even under another build directory; conflicting payloads fail. Aliases dedupe by product/owner-relative path and payload, not child FDID. `ModelAssetIndex` uniquely indexes only `assets`; existing repeated alias provenance is preserved, including older rows without a content key. `validate_staged_fragment` rejects remaining asset-key and newly repeated alias duplicates before publication; raw path repairs do not create a second receipt owner.

Publication also validates the [native asset-kind contract](../../specs/offline-asset-closure.md#recorded-legacy-byte-authentication) before any writes. A byte-authenticated opaque `blob` node is valid audit evidence but not a native index member: one such row rejects the entire index, not just that asset. The [2026-10-22 failure and correction](../../handoffs/noinstall-cert-2026-10-21.md#unsupported-opaque-node-publication-failure) preserve the native reproduction, failed fragment and RED→GREEN producer checks. Do not expand native kind support merely to import an opaque graph node.

The legacy-byte receipt producer requires explicit `--build PRODUCT=VERSION` pins matching exactly the inventory's requested products. It authenticates those local product roots only; historical hardcoded build defaults are removed. A pin mismatch still fails before publication, and bytes must still match authenticated root content keys. Current install builds must be named explicitly rather than silently replacing the historical pins above.

473 legacy candidates match authenticated requested-product roots; the same3
M2 conflicts above remain rejected and untouched. All476 original runtime
receipt identities have matching authenticated scoped files, including the14
Forever70338 textures already recovered in round4. No new extraction was needed.
This is not whole-catalog provenance. `8203433.blp` is absent from the **Retail**
root; its Forever-root presence does not authorize substitution.

The tracked creature-display importer rebuilt the cache from pinned
`data/db2/12.1.0.69933/{CreatureDisplayInfo,CreatureModelData}.csv` using private
input staging, preserving the older root CSVs. Backup beside the cache:
`creature_display.sqlite.pre-noinstall-receipts-20261010`. Rows145204/145205 now
resolve to Retail models968705/986648.

Evidence: `data/diagnostics/noinstall-receipts-2026-10-10/`. Authentication tests
RED5/GREEN7; full Python289run/286pass/3skip,exit0 at7eeda7277. Native extension/CLI
rebuilt at655e6cea1d after merging origin/master for protocol compatibility.
Private read-only prebuilt server, UDP54212, own `fb_noinstall_receipts` account;
extracted-only bubblewrap hides the configured install/CASC paths. Character
select authentication,62 representative race/body selections and both-skin
Northshire/Skyborne/mount scenarios completed. Glue screens stay Modern by
contract; no alternate skin forced. All139 original PNGs were inspected in
labeled sheets, with mount/roster detail crops. Textured horse+rider renders in
both skins after own458 cast/go (bounded one-mount PASS). The roster preview and
many creation/NPC chains remain missing/incomplete; no general rendering PASS.

**Certification still fails.** The480 earlier broad-sweep identities become170
emitted identities in this narrower2-zone recheck:130 newly exposed receipt
misses (126 legacy candidates present),39 qualified skin-companion/alias misses
(all39 legacy candidates present), and Retail8203433. None of the original476
receipt errors recur. Four new absent texture candidates:7484430,7485185,
7484392,7961159. Failed owners still hide further descendants; counts are not
full-catalog or equivalent-scenario coverage. `misses.json`, `receipt-proof.json`,
`world-sweep.json`, `proof-ledger.json` and `summary.md` retain exact boundaries.

## 2026-10-21 certification continuation

[Current certification coverage and evidence](../../handoffs/noinstall-cert-2026-10-21.md) owns the extracted-only native sweep,source fixes,publication readbacks and frozen current-metadata full-config diagnostics. The [fresh Retail](../../handoffs/noinstall-cert-2026-10-21.md#fresh-full-catalog-retail-diagnostic-on-frozen70448-metadata) and [Forever-runtime-metadata](../../handoffs/noinstall-cert-2026-10-21.md#sequential-forever-runtime-metadata-diagnostic) graphs predate receipt-selected auditor IO and retain their legacy-flat/single-requested-product scope. Already-valid scoped files appear missing and hide descendants in those preserved diagnostics. The [bounded receipt-IO qualification](../../handoffs/noinstall-cert-2026-10-21.md#offline-auditor-receipt-selected-model-io) changes the auditor,not those manifests; no full-graph replay or native metadata-owned mixed-product certification follows. These diagnostics are not extraction frontiers or a P4 certificate; historical graph counts remain historical.

## Sources

- [Contract](../../specs/offline-asset-closure.md).
- `scripts/asset_closure.py`, `scripts/closure_seeds.py`, `scripts/tests/test_asset_closure.py`.
- `scripts/closure_{kit_effects,emitters,appearance,wmo_liquid}.py`, `closure_db2_layouts.py`, `export_db2_csv.py` and their concrete `scripts/tests/test_closure_*.py` fixtures.
- WoWDBDefs `meta/enums/SpellVisualKitEffectType.dbde` and exact-build `definitions/*.dbd`; source hashes in the evidence root. NPC bake selection follows `scripts/import_npc_appearance.py`; WMO liquid identity/request ordering follows `godot/core/src/wmo_liquid.rs` and `godot/rust/src/terrain/wmo_liquid.rs`.
- wowdev/pywowlib `m2_file.py` / `file_formats/m2_format.py` — BFID `.bone` arrays and PFID `.phys` identity, collected without claiming runtime satellite support.
- `scripts/recheck_local_asset_gaps.py`, `scripts/tests/test_recheck_local_asset_gaps.py`; recovery evidence above includes per-FDID results, native logs and CSV candidates.
- `scripts/tests/test_model_file_data_export.py`, `scripts/import_model_asset_chains.py`, `scripts/tests/test_model_asset_provenance.py`; round-4 inline-ID and scoped no-clobber behavioral proofs.
- `godot/core/src/asset/{adt,m2,wmo}_format/` — current binary layouts and flags; WMO material shader 19/20 texture slots reference WebWowViewerCpp in parser_types.rs.
- Sibling `game-server/crates/server/src/{player_create_info.rs,class_progression.rs,spell_info/class_data.rs}` — imported loadout and auto-learn/default-skill rules; source comments cite TrinityCore ObjectMgr/Player.

## See Also

- [[m2-product-shadowing]] — FDID presence is not authenticated source/build identity.
- [[forever-data]] — product-selected metadata and provenance gaps.

The 44 `manifest*.json` files under `data/diagnostics/closure-*` (61.5 GiB) were deleted on 2026-10-13 at the user's request. The SHA-256 and size of each are kept in `data/diagnostics/lead-deleted-closure-manifests-2026-10-13.sha256`. Hashes cited above still identify those manifests, but the files are gone; rerun `scripts/asset_closure.py` to regenerate one.
