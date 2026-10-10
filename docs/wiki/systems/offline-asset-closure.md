# Offline asset closure audit

`scripts/asset_closure.py` reads extracted files and local metadata only. It never discovers an install, reads CASC, launches the client/server, or extracts assets. Its manifest is an audit, not a certified release asset set. Product scope is all zones/races/items; Northshire remains the initial proof slice.

## Usage

From a checkout, run under the agent build/test lock and agent-run when used as acceptance proof:

```text
python3 scripts/asset_closure.py --data /path/to/extracted/data --world-db /path/to/world.db --output data/diagnostics/closure-slice-2026-10-09/manifest.json --estimate-output data/diagnostics/closure-slice-2026-10-09/full-catalog-estimate.json
```

Default config is `scripts/closure-northshire.json`: Azeroth tiles (32,48), (33,48), (32,49), (33,49), encompassing the Abbey, lake and vineyard vicinity; male level-1 Human Warrior with first unrestricted choices in option order. Starting gear is world.db's Purpose-9 CharacterLoadout plus race/class item overrides, **not** the obsolete CharStartOutfit kit. NPCs are the imported `content_creature` rows inside those tile bounds; all template display alternatives and explicit spawn model overrides are included. This over-enumerates the slice rather than relying on a playthrough.

Spellbook seeds include auto-learned class/default-skill spells and future Warrior entries displayed in the book, plus Initial-spec spells. Spell IDs and chosen customization IDs are recorded. Spell icons come from pinned `SpellMisc` DifficultyID=0 rows. No spell visuals/audio are part of this bounded request.

`--config scripts/closure-full-catalog.json` changes roots to all Map directories/tiles, modeled race/sex pairs/all customization choices, all world.db template displays, all content_item rows and all spell_name rows. This is a conservative catalog superset. Nonplayable placeholders, unsupported source products, customization requirements and pruned server spell reachability are unresolved policies, not silently inferred support.

## Manifest and boundaries

- Assets are sorted by FDID/type; incoming edges and local paths are sorted. Fixed-point work queue handles cycles. Modern chunk tags follow existing core parsers; old named references resolve through the local listfile.
- M2 SFID skin and SKID skeleton FDIDs retain their identity and record the existing owner's runtime cache aliases. Different bytes at aliases are a conflict, not an accepted source. WMO groups include all GFID LODs; all doodad sets/component item variants are included.
- Every present file has size/SHA-256. Metadata files and tool/config bytes are hashed. world.db is read in one read transaction; selected content seeds have a canonical JSON fingerprint (not an incorrect hash of a live SQLite main file ignoring WAL).
- `requested_product` and `metadata_build` describe the requested source. `source_product` and `actual_build` remain null for unreceipted legacy bytes. Existing filenames, CSV directory labels and cache mtimes are never promoted into authenticated provenance.
- Missing expandable files, unresolved joins/paths, malformed chunks, conflicting aliases, liquid/ground-effect metadata, emitter auxiliary edges and unsupported customization effects are reported explicitly. Exit 1 means an incomplete/unverified audit, while still writing the manifest. Exit 0 is reserved for zero missing/unresolved/unverified identities; current legacy data cannot meet it.

The full-catalog census counts listfile candidates by extension and actual extracted disk files/bytes. Additional extraction file counts and bytes are **estimates**, using numeric FDID overlap and the biased local mean size per class. Mixed-product listfile entries, owner-named skin aliases and absent receipts prevent treating these as verified release coverage. No size is guessed when there is no local sample. Compression/download size is unknown.

## Sources

- [Contract](../../specs/offline-asset-closure.md).
- `scripts/asset_closure.py`, `scripts/closure_seeds.py`, `scripts/tests/test_asset_closure.py`.
- `godot/core/src/asset/{adt,m2,wmo}_format/` — current binary layouts and flags; WMO material shader 19/20 texture slots reference WebWowViewerCpp in parser_types.rs.
- Sibling `game-server/crates/server/src/{player_create_info.rs,class_progression.rs,spell_info/class_data.rs}` — imported loadout and auto-learn/default-skill rules; source comments cite TrinityCore ObjectMgr/Player.

## See Also

- [[m2-product-shadowing]] — FDID presence is not authenticated source/build identity.
- [[forever-data]] — product-selected metadata and provenance gaps.
