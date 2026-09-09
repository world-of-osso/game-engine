# DB2 Format

DB2 (and its predecessor DBC) is Blizzard's binary table format for game data — item stats, character customization options, light parameters, display info, and more. DB2 files can be read directly from CASC by FDID through the project `AssetResolver`, or extracted to `data/dbfilesclient/` as a local cache/debug artifact. Schemas come from `wowdev/WoWDBDefs`; some DB2 files are BLTE-encrypted and require TACT keys separate from WoWDBDefs.

## CASC Access Model

The engine does not need WDBx to get DB2 bytes out of CASC. The existing asset resolver exposes `resolve_bytes(fdid)`, and the DB2 parsers already operate on `&[u8]`. Current runtime loaders often use `ensure_db2_path(fdid, path)`, which extracts/caches the DB2 to disk and then reads it back; that is a convenience path, not a hard requirement of the format parser.

Preferred implementation direction:

- Use direct CASC bytes for runtime DB2 loading when a table is identified by FDID.
- Keep `ensure_db2_path` for debug visibility, reproducible cache files, and tooling that wants paths.
- Use `casc-local` for manual extraction when inspecting a table or sharing a fixture.

`Frostshake/WDBx` is useful as external reference/tooling. It opens DBC/DB2 from CASC, MPQ, or native files; exports CSV/JSON/SQL; and is powered by WDBReader plus WoWDBDefs. It is a C++/Qt/vcpkg desktop tool, so it is better treated as a verifier/exporter than as a runtime dependency for this Rust engine.

## Schema Source vs. Decryption Source

These are two separate concerns:

| Need | Source |
|------|--------|
| Field layouts, type info, layout hashes | `wowdev/WoWDBDefs` |
| BLTE decryption keys for encrypted DB2 blobs | `wowdev/TACTKeys` / `data/tactkeys/WoW.txt` |

`LightSkybox.db2` was blocked until its TACT key (`0xD1055199767FB373`) was sourced from TACTKeys — WoWDBDefs does not carry keys.

## Key DB2 Tables

### Skybox / Lighting

| Table | Purpose |
|-------|---------|
| `Light.csv` | World-space light volumes; maps position → `LightParamsID` |
| `LightParams.db2` | Per-params record → `LightSkyboxID` + fog/color settings |
| `LightSkybox.db2` | `LightSkyboxID` → `SkyboxFileDataID` (M2 skybox model FDID) |

Lookup chain: scene position → Light.csv row → LightParamsID → LightParams.db2 → LightSkyboxID → LightSkybox.db2 → SkyboxFileDataID → M2 model.

Some LightParamsID values used by warband scenes are absent from the local modern `LightParams.db2`, causing fallback to the default `costalislandskybox.m2`.

### Character Customization

| Table | Purpose |
|-------|---------|
| `ChrCustomizationOption` | Named options per model (Skin Color, Face, Hair Style…) |
| `ChrCustomizationChoice` | Choices per option; links to `ChrCustomizationElement` |
| `ChrCustomizationElement` | Points to `ChrCustomizationMaterial` |
| `ChrCustomizationMaterial` | Points to `TextureFileData` → FDID per texture region |
| `CharHairGeosets` | Hair style → geoset ID mapping |
| `CharComponentTextureSections` | Body texture atlas region coordinates (X, Y, W, H per section) |

Example chain for Human Male HD face texture: ChrModelID=1 → Option 10 (Face) → Choice 20 → Element → Material → TextureFileData → FDID 1027494.

### Equipment / Helmet Display

| Table | Purpose |
|-------|---------|
| `ItemDisplayInfo` | Per-display-ID: `GeosetGroup_0`/`_1` (helmet geoset selectors), `ModelMaterialResourcesID` |
| `TextureFileData` | Resolves model material resource → BLP FDID |
| `HelmetGeosetData` | Per `HelmetGeosetVisDataID`: `RaceID`, `HideGeosetGroup` list |
| `HelmetGeosetVisData` | Links a vis data ID to its set of `HelmetGeosetData` rows |

`ItemDisplayInfo.GeosetGroup_0` maps to character geoset group 27 (helmet variant), `GeosetGroup_1` maps to group 21 (head show/hide). Raw values are not applied directly — slot-aware translation is required.

`HelmetGeosetData` has an undocumented extra field (`Field_10_0_0_46047_003`) with values `32` or `-1` that correlates with basic vs. specialized hide groups; meaning is not yet confirmed.

### Texture Resolution

Item-driven textures (worn equipment) come from `ItemDisplayInfo.ModelMaterialResourcesID_*` → `TextureFileData` → FDID. This chain is distinct from the M2 TXID chunk textures. Auto-extraction is not fully reliable for all paths; manual `casc-local` extraction may be needed.

## NPC authored appearance import

`python3 scripts/import_npc_appearance.py` uses stdlib only and reads existing local files; it does not extract assets or alter the renderer. [Importer contract](../../specs/npc-appearance-importer.md).

```sh
python3 scripts/import_npc_appearance.py \
  --db2-dir /syncthing/Sync/Projects/world-of-osso/game-engine/data/diagnostics/northshire-appearance-placement \
  --data-dir /syncthing/Sync/Projects/world-of-osso/game-engine/data \
  --model-cache /syncthing/Sync/Projects/world-of-osso/game-engine/data/cache/creature_display.sqlite \
  --output /syncthing/Sync/Projects/world-of-osso/game-engine/data/diagnostics/northshire-appearance-placement/npc-appearance-fixtures.sqlite \
  --display-id 13035 --display-id 13036 --display-id 130617
```

Omit `--display-id` to import every CSV display. All three DB2s are fully decoded even for a selected output subset. The output must not exist. Production destination, when main approves promotion, is shared-data `cache/npc_appearance.sqlite`; no default output or automatic promotion exists. JSON stdout reports decoded/output counts and requested fixture rows (default report IDs: 13035, 13036, 130617).

Supported fixed-record layouts from local `/home/osso/Repos/wowless/vendor/dbdefs/definitions/`:

| File | Layout | Fields / relation |
|---|---|---|
| `1264997.db2` / CreatureDisplayInfoExtra | `4D9FE25C` | Inline ID, race, sex, class, flags, SD bake material, HD bake material |
| `3692043.db2` / CreatureDisplayInfoOption | `2F331C33` | Noninline ID; option, choice; relation to Extra |
| `1720141.db2` / CreatureDisplayInfoGeosetData | `5E539080` | Noninline ID; geoset index/value; relation to display |

Integer storage 0 (direct), 1 (bitpacked), 2 (common default/ID override), 3 (palette), and 5 (signed bitpacked) are supported. Copies inherit record bytes and relationships; inline IDs and common overrides use the destination ID. Sparse records, array palettes, unknown layouts/storage, missing relations, and absent/all-zero encrypted section payloads fail explicitly. A nonzero TACT key hash alone does not imply missing data: already decrypted section payloads are decoded normally.

`CreatureDisplayInfo.csv.ExtendedDisplayInfoID` links to Extra. The existing `creature_displays` cache supplies the actual model FDID; exact local listfile lookup supplies its path. An `_hd.m2` filename selects HD bake, other `.m2` paths select SD. Missing paths never infer resolution from race or alternate models. TextureFileData uses `UsageType=0`; unresolved or conflicting selected material mappings fail. `--outfit-cache PATH` explicitly uses its read-only `material_to_texture(material_resource_id, texture_fdid)` table instead of CSV.

Displays without Extra retain no appearance/choice row; authored geosets remain independently display-keyed. Child references outside the selected CSV displays do not fabricate parents. Runtime interpretation remains main's responsibility.

Targeted tests: `python3 -m unittest discover -s scripts -p test_import_npc_appearance.py`.

## Sources

- [NPC importer](../../../scripts/import_npc_appearance.py) and [behavioral fixtures](../../../scripts/test_import_npc_appearance.py)

- [docs/casc-db2-keys.md](../casc-db2-keys.md) — WoWDBDefs vs TACTKeys distinction, practical extraction model
- [docs/wiki/systems/asset-pipeline.md](../systems/asset-pipeline.md) — CASC resolver/cache model and direct byte access
- [docs/skybox-authored-lookup.md](../skybox-authored-lookup.md) — Light→LightParams→LightSkybox lookup chain, fallback behavior
- [docs/helmet-geoset-extra-field-investigation-2026-03-28.md](../helmet-geoset-extra-field-investigation-2026-03-28.md) — HelmetGeosetData extra field observation
- [docs/hd-skeleton-status.md](../hd-skeleton-status.md) — ChrCustomization chain traced for Human Male HD, CharComponentTextureSections layout
- [Frostshake/WDBx](https://github.com/Frostshake/WDBx) — external DB2 viewer/exporter using WDBReader and WoWDBDefs

## See Also

- [[casc-format]] — DB2 files extracted from CASC; TACT keys for encrypted blobs
- [[m2-format]] — M2 texture type system driven by DB2 customization chains
- [[geosets]] — Geoset groups and how ItemDisplayInfo slot values map to them
