# Northshire pale water

Northshire's streams rendered as flat, pale white-blue sheets. The Godot water shader was the Bevy procedural placeholder. It used a generated normal map, constant base and sky colours, a hard-coded light direction and a fixed 0.08–0.85 alpha. It also never received MH2O vertex depths. Water now ports the retail water material from WebWowViewerCpp `liquidWaterMat.slang` (`calcWaterLiquidMat`), with inputs from DB2 and LightData.

## Causes and fixes

1. **Vertex depths were dropped.** Northshire's layers are LiquidType 5 "Slow Water" through LiquidObject 427. Its LiquidMaterial 1 uses LVF 0: all heights first, then all depth bytes (wowdev MH2O; WebWowViewerCpp `LiquidDataGetters.h` `getLiquidDepth`). The parser read only the heights, so every vertex got depth 1.0. `adt_tex_water.rs` now also reads the depths. Tile 32_48 has depths from 0 to 80, and depth 0 is authored on dry cells.
2. **No LiquidType material.** Colours and normals were constants. `scripts/export_db2_csv.py` now exports LiquidType, LiquidObject, LiquidMaterial and LiquidTypeXTexture from local CASC 12.1.0.69933. `game_engine_core::liquid_data::LiquidCatalog` resolves each layer following `LiquidMaterialManager.cpp`: the object gives the type, the type gives the material and LVF, and the texture slots are filled by `FrameCountTexture`. For type 5 that is slot 2 bump `newoceanbump.blp` (463849), slot 3 foam `waterchop7bw.blp` (317230), and River colours (procedural texture type 1). The shader uses Float[0..17], Coefficient[0..3] and the LiquidObject flow values. Slot 0's 30 `lake_a` frames are not read by the modern water shader.
3. **Wrong colour source.** The shader now uses the LightData RiverClose/Far (or OceanClose/Far) colours at the camera, with LightParams Water/Ocean Shallow/Deep alphas blended across the same LightParams weights (`DayNightLightHolder.cpp:928-937`, `MapSceneRenderer.cpp:194-203`). The specular colour is SunColor. Underwater fog comes from the Light's underwater LightParams slot and is inert when that slot has no keyframes (`MapSceneRenderer.cpp:297-315`).
4. **Wrong blend.** The water now reads Godot's depth and screen textures. The scene behind the water is refracted and underwater-fogged by the water's thickness, then mixed with the lit close/far colour using the depth-cubic alpha. Shore alpha and foam fade over the first yards of thickness, and depth-0 vertices are invisible.

## Zephras Material130 — explicit borrowed legacy fallback

The first product-isolated Zephras catalog resolved1251/1279, but rendered a near-white sheet. The problem was the **material contract**, not shifted columns or lighting units. DBD D1ECEEC9/WDC5 independently place Float[38] at field18,Int[4] at19,Coefficient[4] at20. The old CSV header truncated20 floats; later columns remained correctly name-mapped. Forever exports/native metadata now preserve all38 source floats.

Material130 PBR values Float2=7.85,Float6=60,Float8=26 were fed to legacy Water. Its six named textures are foam high/mid/low/rim/depth/river: foam_low was sampled as a normal, foam_rim as generic foam. Real LightParams7588 has Ocean alpha.75→0, River.5→1; all12 LightData7588 rows have black Ocean/River RGB. Native uniforms match raw rows. The legacy expression extrapolated alpha to.75−.75×7.85=−5.1375, amplifying the refracted scene. A controlled GPU reproduction measured **6.27×** backing gain.

No grounded Material130 shader/FFT contract was found in the audited references (WebWowViewerCpp1a8cccbe, wow.exportc2fd7bde generic preview, noggitb5edce3b legacy water); no PBR math was invented. Main explicitly authorized a **borrowed fallback**: same-product LiquidType5 supplies known legacy params, normal463849 and foam317230. Original source type/material/LVF/flow remain; source38-float metadata is unchanged. Required borrowed rows/slots/files error if absent. Code comment and one diagnostic per type say this is borrowed, not authored PBR. Retire the substitution when a grounded PBR renderer is implemented. Legacy opacity is clamped[0,1].

Proof at4ef4bad0: core3 product/fallback +13 Retail water tests pass; all199 Retail type results unchanged. Importer27/27 plus closure5/5 pass. GPU RED→GREEN backing gain **6.27×→.98×**, with correct borrowed normal pixels and Float2=1. Own local extension79773870…/fixture/CLI build passed and hashes were verified before GREEN/captures. [Ledger](../../../target/zephras-water-white/ledger.md) retains commands, revisions, raw/bound inputs, flags, decoded textures and reference audit.

Matched camera/light captures under `data/diagnostics/zephras-water-white/`: `before-zephras-liquid-coast.png` shows the near-white sheet; `zephras-coast-after.png` shows blue-gray refracted water, shore and foam; `retail-coast-after.png` shows a normal-lit Kul Tiras33_30 ocean/rock shore. Both matched runs exit0 with zero engine errors/missing types; Zephras has exactly one borrowed warning for1251 and one for1279, Retail none. **This fixes demonstrated amplification/input misuse, not PBR appearance/FFT/animation parity.** Object queues remain unfinished; no settled-world/gameplay claim. Cages stopped/released.

## Other liquids

Every LiquidMaterial now has its shader, selected as in `LiquidMaterialManager.cpp` `createLiquidMaterial` (`liquid_data::LiquidShader`): 1/3 and unlisted IDs water, 2/4 magma and slime (`liquid_magma.gdshader`, with the 128x128x16 noise volume `xtextures/fx/noise.blob` 768431), 5 mercury, 10 fog, 12 ley line, 13 fel, 14 swamp and 18 azerite (`liquid_<name>.gdshader`, each a port of the matching `liquid*Mat.slang`). Rust binds one uniform contract to all of them: Float[0..17], Coefficient, Color[0..2] as `getFloatFromInt` bytes, Int[0..3], flow, slot frames (`texture_0..5`, advancing one frame per second) and the crossfaded next frame (`texture_next`, Fel/Azerite slot 5 over Float[12] s, Swamp slot 4 over Float[5] s). Azerite also binds 1797551 and 1844666. Fel and Azerite normal maps are BLP pixel format 11, BC5 (`blpFileHeader.h` `PIXEL_BC5 = 11`), now decoded by `blp::decode_rgba`.

LiquidObject vertices use their material's LVF: the parser keeps their bytes and `WaterLayer::decode_object_vertices` re-reads them. Magma objects are LVF 1. LVF 1 and 3 are separate height, UV and depth arrays (wowdev MH2O), no longer interleaved.

## WMO liquids (MLIQ)

WMO group liquids were parsed but never drawn. They now follow WebWowViewerCpp (`game_engine_core::wmo_liquid`, `godot/rust/src/terrain/wmo_liquid.rs`):

- **LiquidType** (`WmoGroupObject::setLiquidType`, `wmoGroupObject.cpp:151-201`): with MOHD flag 0x4 a group liquid n below 21 is `to_wmo_liquid(n - 1)`, any other n a LiquidType ID; without it n below 20 is `to_wmo_liquid(n)`, Green Lava (15) none, the rest n + 1. `to_wmo_liquid` keeps the low two bits: WMO Water 13 (WMO Ocean 14 with MOGP 0x80000), Ocean 14, Magma 19, Slime 20. The material is `LiquidCatalog::liquid_material(type, 0)`, as for MH2O.
- **Tiles.** A tile whose low nibble is 15 is not drawn (`SMOLTile.legacyLiquidType : 4`). The parser masked 0x3F, so tiles 0x1F/0x2F/0x3F/0x7F (e.g. 196 of Blackrock Depths group 043's 899) were drawn as slivers to unused zero-height vertices; it masks 0x0F now.
- **Surface** (`WmoGroupGeom::getWaterVertexBindings`, `wmoGroupGeom.cpp:512-616`): vertex (i, j) at MLIQ corner + 4.1667 yd × (i, j) and its own height, depth 1; triangles (0, 1, 2), (0, 2, 3). Texture coordinates are WMO-local position / 33.33 yd, or for WMO Magma the magma vertex `s, t * 3 / 256`. The liquid shaders take them from the mesh (`mesh_uv`) instead of world position × 0.06.
- **Interior colour.** `isInteriorLightingLit` (interior 0x2000 without exterior 0x8 or exterior-lit 0x40) sets the water shader's `interior`, which makes procedural WMO water (LiquidTypeXTexture Type 2, e.g. WMO Water 13) white (`liquidWaterMat.slang:162-176`).
- The surface is a `Group{g}_Liquid` child of the WMO node, so the placement and portal culling apply. `TerrainObjects` owns one WMO liquid material set, advanced by the shared material clock and relit with the terrain light.

Live (private game-server 2ceae17, UDP 5106, engine `dee54d9d`): the WMO liquid is the 3 × 4-tile WMO Water pool of WMO placement 10544 (`abbeygate01`) beside Northshire Abbey, drawn translucent teal at its MLIQ height, 1.46 yd above the WMO origin; hiding the `Group0_Liquid` node shows the grass under it (`data/diagnostics/avfix-2026-09-30/wmo-liquid/abbeygate/before-after.png`, `sheet.png`). Whether retail shows this pool here is not confirmed: it lies flat over the grass, with no basin. Stormwind's canals are WMO liquid too (groups 107520-107523, liquid 5, 0x7F/0x2F tiles common), but the city's ~16,000 objects had not finished spawning within the capture's time limit, so they were not shot.

Proof: `godot/core/tests/wmo_liquid.rs` (5 tests): the resolution table; interior lighting; Northshire Abbey's gate fountain (`abbeygate01` 108104/108105: MOHD 0x5, liquid 5 → 13, 10 of 12 tiles, tile 0x3F hidden); the Cultists' Quay delve cave (5356285/5533972) as interior WMO Water; magma vertex UVs.

## Proof

- `godot/core/tests/liquid_water.rs` (5 tests): an LVF 0 payload keeps its depths; tile 32_48 depths range 0..>64; LiquidObject 427 resolves to type 5 with its textures, floats and coefficients; ocean colours; Shallow Water wave periods [1.0, 0.4]; an unknown LiquidType is an error. Row-less LiquidObjects: see [above](#liquidobject-ids-without-db2-rows--resolved).
- `godot/tests/water_material_pixels.gd` (GPU) uses the real material at the vineyard stream at noon. It checks five things: deeper vertex depth hides more of the backing (0.48 → 0.39); depth 0 is invisible (response 1.00); backing 60 yd down is fogged (0.44 → 0.33); shallow water over black matches the retail river mix within 0.002; the clock moves 2,274 px. Run with the old shader, it fails the depth-0 check (`data/diagnostics/water-2026-09-30/fixture-red-old-shader.log`).
- `godot/tests/liquid_material_pixels.gd` (GPU) draws every non-water material from its DB2 data: all cover the view and animate over 1000→2333 ms; magma (Searing Gorge LiquidObject 413) and slime are opaque, and magma is lava-coloured (0.56, 0.15, 0.01). Screenshots: `data/diagnostics/water-2026-09-30/liquids/`. In-world magma at Searing Gorge: `data/diagnostics/water-2026-09-30/magma/sheet-gorge.png`.
- `liquid_water.rs` also covers LVF 1/3 arrays, magma/slime resolution and the Searing Gorge tile's 53 magma layers re-read as LVF 1; `blp_tests.rs` covers BC5.
- Before/after Northshire sweeps: `data/diagnostics/water-2026-09-30/{before,after}/`. Streams now show a teal-tinted, refracted stream bed near the camera instead of the opaque pale sheet.

## LiquidObject IDs without DB2 rows — resolved

**Verified: 2026-10-01** (branch `liquidobj`, `99b8bf67`, `01b80006`, `4b0b6d05`).

**Symptom.** Character select at Adventurer's Rest (map 2703, tiles 31_36 FDID 5493433 and 31_37 FDID 5493438) logged 133 `LiquidObject … has no DB2 row` errors and drew no mesh for those layers. The pairs were `(2,42)` ×47, `(5,13134)` ×5, `(5,13136)` ×13, `(5,13137)` ×3, `(81,13138)` ×26 and `(5,13139)` ×39.

**The rows do not exist.** The active 12.1.0.69933 LiquidObject DB2 starts at ID 57 and has no IDs 13103–13141. The earlier evidence, preserved below, matched the DB2 to the active root ContentKey and found no LiquidObject rows in `DBCache.bin` or the three current tmp hotfix containers. Object 42 is the ocean object: a world-wide scan of the 52,882 root ADTs in local CASC found 4,509,101 `(2,42)` layers on 458 maps. On master every one of them was dropped, so no open sea rendered natively.

**Rule 1: LiquidType of a row-less object.** The layer uses its own MH2O `liquid_type`, with no flow. This is WebWowViewerCpp `CSqliteDB::getLiquidObjectData` (`loData.liquidTypeId = fallbackliquidTypeId`) and its default `LiquidObjectRec` flow 0. It matches the data: the world-wide scan found 4,924,089 LiquidObject layers, and only 207 of them differ from their row's `LiquidTypeID`: hellfireraid62 876→869 ×182, devmapg 940→951 ×21, islands11 5→1 ×4. Present rows still decide the type.

**Rule 2: vertex format of LiquidType 2 Ocean.** These layers are flat at sea level (min and max height 0). Their vertex block is absent (3,274,235 layers) or exactly 81 bytes (1,234,866): LVF 2, depths only. LiquidType 2's material 1 says LVF 0, so reading the block that way overran it ("MH2O depthmap needs 81 bytes, has 72"). Sources for the LVF 2 reading:
- wowdev ADT/v18 SMLiquidInstance: "≥ WoD ignores value and assumes both 0.0 for LVF = 2".
- noggit3 `liquid_layer.cpp`: "lvf 2 is only used for flat water at height 0".
- WebWowViewerCpp `LiquidInstance.cpp` `createAdtVertexData` singles out `liquid_type != 2`. WebWowViewerCpp itself still reads the material LVF.

Other oceans on object 42 keep their material's LVF 0 (405-byte blocks, heights 0): Kul Tiras Ocean 947 (148,047 layers), Zandalar 1100 (29,854) and Nazjatar 1142 (15,975). So the rule is keyed on LiquidType 2, not on object 42 (`liquid_data::OCEAN_LIQUID_TYPE`).

**Residue: none.** With both rules, every one of the 4,975,880 MH2O layers resolves its LiquidType and LiquidMaterial, and every vertex block exactly fits its LVF size: 0 omitted, 0 inexact. The 4,703,325 row-less layers fall into 14 pairs:
- the six above;
- `(947,42)`, `(1100,42)` and `(1142,42)`;
- `(1177,9302)` on map 2534, `(877,4050)` in acquisitionhavoc, `(5,13675)` on map 2662, `(5,4137)` in nightmareraid and `(5,1951)` in cotwaroftheancients.

Scanner and merged counts: `data/diagnostics/liquidobj-2026-10-01/{world_scan2.py,merge.py,world_scan2_merged.json}`. 10,079 listfile root paths are not in this build's CASC.

**Proof.**
- `godot/core/tests/liquid_water.rs` lists the six IDs (`OBJECTLESS_PAIRS`). Each must resolve to its MH2O LiquidType's material: water shader, no flow, LVF 2 for ocean and 0 for the rest. The test was RED on master ("LiquidObject 42 has no DB2 row").
- Both Adventurer's Rest tiles: all layers resolve, and every block decodes in its LVF. 133 of the layers are row-less.
- Kul Tiras `kultiras_20_17` (FDID 1422588): all 256 `(947,42)` layers decode as LVF 0, heights 0 and depth 255. This test was RED under the object-42 rule.
- `game-engine-core` is 0 failed (Depot).

**Live** (private server on UDP 5130, master `6f3c6ab7` against `4b0b6d05`):
- Character select logs 133 errors on master and 0 on the branch. `Tile31_36/Water` has 0 meshes on master and 102 (5,954 triangles) on the branch; `Tile31_37/Water` has 6 and 37. The lake under the campsite waterfalls (Godot ≈ −2400, 720, −470) is empty rock on master and water on the branch: `adventurers-rest-lake-pair.png`. The 47 ocean layers lie 400–900 yd under that terrain and are not visible.
- The Stranglethorn coast west of Booty Bay (−14045, 520) logs 1,313 errors on master and 0 on the branch. The sea appears at yaw 180: `stranglethorn-coast-y{0,90,180,270}-pair.png`.
- Northshire has no row-less layers (all `(5,427)`).
- Not live-captured: Kul Tiras and the other maps, which `set-position` cannot reach. The pale look of the ocean is the open water-material work below, not this fix.

### Earlier evidence (2026-10-01, MAIN read-only reports)

- **Content identity.** Active product `wow` is `12.1.0.69933`, build key `dcfc90fffd79ba00406ae46f5f657592`. Local MD5s match the cached root ContentKeys: LiquidObject DB2 1308058 is `a2fc7df6448cecb865e2ca09db835e35`, 5493433 is `9c9f48805fd0f621026828473527d841` and 5493438 is `d9b6adbf952398ee4edfe535ac79e30e`.
- **Table.** The WDC5 has 314 stored records and 7,768 copy pairs; the in-memory export yields 8,082 rows, matching the CSV. Table hash `0xfc2a0dff` is at byte 152.
- **Hotfixes.** Retail `DBCache.bin` (XFTH9, build 69933, SHA-256 `07179f115da6ebfa46e382d912292d85d8b301ae14fc329f17d1b9dea2901301`) has 139,431 records and none for this table hash. The three current-build tmp containers (45,241, 64,874 and 121,713 records) have none either.
- **Sources.** `/tmp/claude/native-charselect-water-provenance.md`, `/tmp/claude/native-liquidobject-copy-records-seam.md`, `/tmp/claude/native-liquidobject-active-content-hotfix.md` and `/tmp/claude/native-liquidobject-hotfix-container-main.md`.

## Open

- **Specular power: unknown for retail.** The shaders use `uExteriorSpecularColor.a = 1.0` from WebWowViewerCpp `MapSceneRenderer.cpp`. Retail's `dx_5_0` water pixel shaders (`procwaterabove.bls` FDID 2977223, `water.bls` 2977281) read the exponent from `cb1[5].w`, a CPU-set constant with no reflection names, so its value is not in the shader. The Wrath 3.3.5 client uses 6.0 (solarityclient `crates/rendering/src/liquid/shader_uniform.rs:159-160`, "Native 8A38B0 uses the constant at 9E8CF8"); that is not retail evidence. With power 1, distant water gets a pale sun-coloured haze.
- Height/artistic `makeFog2`, sun attenuation (`uSunAttenuation`), classic underwater fog and wave animation for Float[16] ≠ 0 (ported, not GPU-tested).

## See Also

- [[terrain]] — MH2O geometry and streaming
- [[adt-format]] — root MH2O data
- [[db2-format]] — table schemas and decoded records
- [[retail-lighting]] — LightParams blend the water colours use
