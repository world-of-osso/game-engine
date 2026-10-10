# Offline asset closure audit

`scripts/asset_closure.py` reads extracted files and local metadata only. It never discovers an install, reads CASC, launches the client/server, or extracts assets. Its manifest is an audit, not a certified release asset set. Product scope is all zones/races/items; Northshire remains the initial proof slice.

## Usage

From a checkout, use `agent-run` for agent audit/test/extraction processes; no build lock for these Python jobs (the lock is release-only):

```text
python3 scripts/asset_closure.py --data /path/to/extracted/data --world-db /path/to/world.db --output data/diagnostics/closure-slice-2026-10-09/manifest.json --estimate-output data/diagnostics/closure-slice-2026-10-09/full-catalog-estimate.json
```

Default config is `scripts/closure-northshire.json`: Azeroth tiles (32,48), (33,48), (32,49), (33,49), encompassing the Abbey, lake and vineyard vicinity; male level-1 Human Warrior with first player-selectable choices in option order (ReqType player bit, race/class masks and no account unlocks). Starting gear is world.db's Purpose-9 CharacterLoadout plus race/class item overrides, **not** the obsolete CharStartOutfit kit. NPCs are the imported `content_creature` rows inside those tile bounds; all template display alternatives and explicit spawn model overrides are included. This over-enumerates the slice rather than relying on a playthrough.

Spellbook seeds include auto-learned class/default-skill spells and future Warrior entries displayed in the book, plus Initial-spec spells. Spell IDs and chosen customization IDs are recorded. Spell icons come from pinned `SpellMisc` DifficultyID=0 rows. No spell visuals/audio are part of this bounded request.

`table_paths` explicitly names four top-level legacy CSV inputs absent from the pinned directory; each remains a reported unverified metadata build. This is source configuration, not an alternate-path fallback. Unknown/default-locked customization requirements are not selectable; NPC-only options are excluded from the bounded player.

`--config scripts/closure-full-catalog.json` changes roots to all Map directories/tiles, modeled race/sex pairs/all customization choices, all world.db template displays, all content_item rows and all spell_name rows. This is a conservative catalog superset. Nonplayable placeholders, unsupported source products, customization requirements and pruned server spell reachability are unresolved policies, not silently inferred support.

## Manifest and boundaries

- Assets are sorted by FDID/type; incoming edges and local paths are sorted. Fixed-point work queue handles cycles and late owner aliases: a skeleton initially missing by its own FDID is expanded if a later model edge supplies an existing owner cache path. Modern chunk tags follow existing core parsers.
- Named paths reproduce asset-resolver's SQLite import (`INSERT OR REPLACE`, unique FDID **and** lower_path), including a repeated FDID displacing a previously bound path. `--path-resolution-cache <absolute snapshot path under data/>` uses a read-only SQLite snapshot of `data/local-listfile-cache.sqlite` before community bindings, matching native resolvers' shared data root. The snapshot is hashed as an input; absence of a binding stays unmapped, not guessed. Full Map roots use the CSV's WdtFileDataID; TXID/MDID override stale filenames even for zero/short slots; those slots never select a filename fallback. `resolved` rows explain formerly ambiguous selections; the original listfile candidate census remains a superset.
- M2 SFID skin and SKID skeleton FDIDs retain their identity and record the existing owner's runtime cache aliases. Different bytes at aliases are a conflict, not an accepted source. WMO groups include all GFID LODs; all doodad sets/component item variants are included.
- Every present file has size/SHA-256. Metadata files and tool/config bytes are hashed. world.db is read in one read transaction; selected content seeds have a canonical JSON fingerprint (not an incorrect hash of a live SQLite main file ignoring WAL).
- `requested_product` and `metadata_build` describe the requested source. `source_product` and `actual_build` remain null for unreceipted legacy bytes. Existing filenames, CSV directory labels and cache mtimes are never promoted into authenticated provenance.
- `closure_terrain.py` inspects all repeated MCNK chunks (root 128-byte header versus headerless split companions), joins MCLY effects through GroundEffectTexture/Doodad and MH2O instances through LiquidObject/Type/XTexture, and includes liquid shader-global textures. MDID/MHID are FDIDs; MTXF is flags. `resolved` records preserve per-file inline-only/empty-liquid evidence; absent required metadata and unknown nested chunks remain unresolved. Optional ground-effect rows missing from a present hashed CSV are recorded not-needed because the current scatter/model-FDID collection skips them before requesting bytes. MPTX/legacy MCLQ are recorded unused by current core loaders, not proven free of Retail-format references. WDT MAID already supplies all eight split/LOD/map/minimap identities; map WDL is a separate named seed.
- Missing expandable files, unresolved joins/paths, malformed chunks, conflicting aliases, emitter auxiliary edges and unsupported customization effects are reported explicitly. Exit 1 means an incomplete/unverified audit, while still writing the manifest. Exit 0 is reserved for zero missing/unresolved/unverified identities; current legacy data cannot meet it.

The full-catalog census counts listfile candidates by extension and actual extracted disk files/bytes. Additional extraction file counts and bytes are **estimates**, using numeric FDID overlap and the biased local mean size per class. Mixed-product listfile entries, owner-named skin aliases and absent receipts prevent treating these as verified release coverage. No size is guessed when there is no local sample. Unnamed FDIDs can be absent from the listfile, especially skeletons, so the census is neither an upper nor a lower bound on the required full catalog. Compression/download size is unknown.

## Bounded proof (2026-10-09)

Code `5520e82ff`: twelve concrete fixtures pass under the build lock; two real manifests are byte-identical (SHA256 `98b1fd059d2c70a82d85764e21a4c86cdfb4cae55af833fc09d318cb4700c393`). The four-tile audit selects 753 NPC spawns/116 NPC display alternatives, nine starting items, 24 spellbook spells and twelve player choices. It finds 2,585 identities: 2,472 present (482,511,719 bytes), 113 absent (112 skins and Abbey bell M2 FDID189599), 156 unresolved records and 2,585 unverified identities. Evidence is under `data/diagnostics/closure-slice-2026-10-09/`; audit exit1 is expected. No extraction or pristine no-install gameplay proof.

## Closure continuation — verified 2026-10-10

On `closure-unresolved`, terrain auxiliary warnings **184,670 → 0**, named ambiguities **47,249 → 0**. Terrain extraction reached fixed point in four rounds (5,398 runtime paths /445,790,591 bytes); runtime-binding/declared-Map-WDT extraction took six rounds (6,256 paths /2,404,843,862 bytes), including the WDTs' newly discovered ADT/WMO/model descendants. Total **11,654 paths /2,850,634,453 bytes**. Final graph: 862,509 present /2,275 missing /86,217 unresolved, 864,784 identities. World selection fingerprint unchanged; optional emitter/spell-kit joins remain open, not no-install gameplay/P4 certification.

Evidence: `data/diagnostics/closure-unresolved-2026-10-10/{summary.md,summary.json,terrain/,paths/}`; immutable native local-listfile snapshot has 141,188 rows. New receipt ranges pass authenticated root/native keys, decoded size, MD5 and SHA-256; 50 concrete Python fixtures pass, including memory-limited graph replacement. Jobs stayed below a 14GiB limit with swap disabled (observed peaks about12.1GiB). Independent model verifier unavailable (expired OAuth), no independent PASS claimed.

Publication uses an independent same-directory temporary, flush/fsync, atomic no-clobber **link**, then temporary-name removal—not an in-place write or a hardlink to mutable extraction staging. Native CASC writes only private batch staging in this workflow. All 938,530 receipt rows in the lead-requested size audit matched; the remaining appended rows are covered by content/key proof. Existing files are never overwritten. Separate receipt proof does not authenticate every unreceipted legacy file or populate the graph's still-unverified identity fields.

## Recorded-gap recovery — verified 2026-10-10

`recheck_local_asset_gaps.py` is separate from the offline graph audit. Feed a product-keyed JSON inventory of FDID/type/locations, an explicit read-only install, canonical data root, existing extractor and private output directory. It authenticates current metadata, retries indexed payloads, then uses the existing no-clobber publication path. Unknown-key output stays unpublished; server imports are outside this tool.

Evidence `data/diagnostics/forever70338-keys-2026-10-10/` records stable archive/config/IDX mtimes across180s and idle Syncthing with zero needed files. Forever1.60.1.70338 now has a readable root, unlike70334. Seven recorded Skyborne bakes and Retail vehicle model7476985 are published. All204 original IV8 failures decode with the existing blteiv extractor; three old IDX misses and one old size mismatch also resolve. This is asset recovery, not renderer or complete-closure acceptance; original root-absent identities and alias conflicts remain recorded individually.

Sibling asset-resolver's `scripts/import_dbcache_keys.py` joins16-byte TactKey payloads with8-byte TactKeyLookup names by record ID, supplementing hotfix lookups with a local lookup DB2. Retail cache yields48 names, all already present in the external store; no new names added. Reports never include key bytes. The Classic-beta cache is absent locally.

Reexports preserve existing CSVs. Retail candidates under `data/db2/12.1.0.69933/recheck-forever70338/` expose Vehicle+6, VehicleSeat+8 and GlobalStrings+65 IDs relative to the recorded/current CSVs. Creature's43-row gain over its older handoff is already present in canonical CSV. Forever reexports use distinct1.60.1.70338 paths. No world.db import or live acceptance performed.

## Sources

- [Contract](../../specs/offline-asset-closure.md).
- `scripts/asset_closure.py`, `scripts/closure_seeds.py`, `scripts/tests/test_asset_closure.py`.
- `scripts/recheck_local_asset_gaps.py`, `scripts/tests/test_recheck_local_asset_gaps.py`; recovery evidence above includes per-FDID results, native logs and CSV candidates.
- `godot/core/src/asset/{adt,m2,wmo}_format/` — current binary layouts and flags; WMO material shader 19/20 texture slots reference WebWowViewerCpp in parser_types.rs.
- Sibling `game-server/crates/server/src/{player_create_info.rs,class_progression.rs,spell_info/class_data.rs}` — imported loadout and auto-learn/default-skill rules; source comments cite TrinityCore ObjectMgr/Player.

## See Also

- [[m2-product-shadowing]] — FDID presence is not authenticated source/build identity.
- [[forever-data]] — product-selected metadata and provenance gaps.
