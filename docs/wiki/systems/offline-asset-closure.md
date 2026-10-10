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

## Resolver continuation — tooling verified 2026-10-10

`closure-unresolved2` adds sourced resolvers; extraction is still running, so the prior final counts above remain the last completed dataset milestone. Tooling `1f9ff541d` has 66 isolated Python fixtures passing with no warnings. This is not no-install gameplay acceptance.

| Boundary | Traversal now handles | Remaining evidence boundary |
|---|---|---|
| Spell kits | WoWDBDefs effect discriminants, model attachments, emission/barrage models, recursive beam textures/sounds, texture blends, decals, screen effects and conditional unit-voice kit superset | Client-scene script assets and missing rows remain explicit; numeric-only effects have current-native no-file-IO evidence, not complete Retail rendering support |
| Emitters | Core 272/274 legacy filename arrays, 0x1ec particle stride, packed 5-bit multitexture indices, 176-byte ribbon texture arrays and recursive M2 work-queue cycles | Malformed arrays and unmapped filenames remain errors; no alternate Legion-header guess |
| Extended displays | Extra's model-path-selected HD/SD bake, raw element material/collection/conditional-model/display choices, NPC item-display resources including ModelMatRes | Zero HD never substitutes SD; unknown voice semantics, missing metadata and unauthenticated legacy caches remain boundaries |
| WMO liquids | Owning MOHD flags carried to GFID/named groups; core MOGP liquid translation; shared and late roots; LiquidType/Material/XTexture and existing renderer globals | MLIQ's material index is MOMT, not a liquid ID; hidden tiles still load the material before geometry; orphan roots are unresolved |
| Auxiliary identities | Actual unlisted sound headers, BFID bone/PFID physics satellites, raw elements behind legacy unsupported-choice flags | Unknown sound bytes remain unknown; satellite collection does not implement runtime physics/poses |

Local DB2s are exported with exact Retail layout hashes, not inferred field offsets. Empty WDC5 tables can legitimately lack storage descriptors. Layout definitions are preserved and hashed under `data/diagnostics/closure-unresolved2-2026-10-10/sources/`; exported CSVs and immutable phase tooling snapshots are separate evidence. `TextureFileData` now uses the pinned local export in full-catalog mode. Partial encrypted tables report dropped records; readable rows do not prove complete metadata coverage.

Each class runs from an immutable scripts snapshot under `agents.slice`, then uses the existing authenticated local-CASC batch publisher and fixed-point driver. Failure records carry forward without inventing identities or retrying known unavailable bytes. No install is used by the traversal itself. Manifest fingerprints include every resolver module; hashes and separate receipt proofs do not automatically authenticate legacy bytes or caches.

Evidence root: `data/diagnostics/closure-unresolved2-2026-10-10/`. Final per-class counts, extraction bytes and receipt ranges must come from completed phase summaries, not this implementation inventory. Six existing model parse boundaries have inspected non-M2 headers: one reverse-MVER WMO and five M3DT models; do not misreport them as proven corrupt M2 files or silently invent format support.

## Sources

- [Contract](../../specs/offline-asset-closure.md).
- `scripts/asset_closure.py`, `scripts/closure_seeds.py`, `scripts/tests/test_asset_closure.py`.
- `scripts/closure_{kit_effects,emitters,appearance,wmo_liquid}.py`, `closure_db2_layouts.py`, `export_db2_csv.py` and their concrete `scripts/tests/test_closure_*.py` fixtures.
- WoWDBDefs `meta/enums/SpellVisualKitEffectType.dbde` and exact-build `definitions/*.dbd`; source hashes in the evidence root. NPC bake selection follows `scripts/import_npc_appearance.py`; WMO liquid identity/request ordering follows `godot/core/src/wmo_liquid.rs` and `godot/rust/src/terrain/wmo_liquid.rs`.
- wowdev/pywowlib `m2_file.py` / `file_formats/m2_format.py` — BFID `.bone` arrays and PFID `.phys` identity, collected without claiming runtime satellite support.
- `godot/core/src/asset/{adt,m2,wmo}_format/` — current binary layouts and flags; WMO material shader 19/20 texture slots reference WebWowViewerCpp in parser_types.rs.
- Sibling `game-server/crates/server/src/{player_create_info.rs,class_progression.rs,spell_info/class_data.rs}` — imported loadout and auto-learn/default-skill rules; source comments cite TrinityCore ObjectMgr/Player.

## See Also

- [[m2-product-shadowing]] — FDID presence is not authenticated source/build identity.
- [[forever-data]] — product-selected metadata and provenance gaps.
