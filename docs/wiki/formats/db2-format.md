# DB2 Format

DB2 (and its predecessor DBC) is Blizzard's binary table format for game data — item stats, character customization options, light parameters, display info, and more. DB2 files are extracted from CASC by FDID and parsed using schemas from `wowdev/WoWDBDefs`. Some DB2 files are BLTE-encrypted and require TACT keys separate from WoWDBDefs.

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

## Sources

- [docs/casc-db2-keys.md](../casc-db2-keys.md) — WoWDBDefs vs TACTKeys distinction, practical extraction model
- [docs/skybox-authored-lookup.md](../skybox-authored-lookup.md) — Light→LightParams→LightSkybox lookup chain, fallback behavior
- [docs/helmet-geoset-extra-field-investigation-2026-03-28.md](../helmet-geoset-extra-field-investigation-2026-03-28.md) — HelmetGeosetData extra field observation
- [docs/hd-skeleton-status.md](../hd-skeleton-status.md) — ChrCustomization chain traced for Human Male HD, CharComponentTextureSections layout

## See Also

- [[casc-format]] — DB2 files extracted from CASC; TACT keys for encrypted blobs
- [[m2-format]] — M2 texture type system driven by DB2 customization chains
- [[geosets]] — Geoset groups and how ItemDisplayInfo slot values map to them
