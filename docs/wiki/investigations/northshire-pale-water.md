# Northshire pale water

Northshire's streams rendered as flat, pale white-blue sheets. The Godot water shader was the Bevy procedural placeholder. It used a generated normal map, constant base and sky colours, a hard-coded light direction and a fixed 0.08–0.85 alpha. It also never received MH2O vertex depths. Water now ports the retail water material from WebWowViewerCpp `liquidWaterMat.slang` (`calcWaterLiquidMat`), with inputs from DB2 and LightData.

## Causes and fixes

1. **Vertex depths were dropped.** Northshire's layers are LiquidType 5 "Slow Water" through LiquidObject 427. Its LiquidMaterial 1 uses LVF 0: all heights first, then all depth bytes (wowdev MH2O; WebWowViewerCpp `LiquidDataGetters.h` `getLiquidDepth`). The parser read only the heights, so every vertex got depth 1.0. `adt_tex_water.rs` now also reads the depths. Tile 32_48 has depths from 0 to 80, and depth 0 is authored on dry cells.
2. **No LiquidType material.** Colours and normals were constants. `scripts/export_db2_csv.py` now exports LiquidType, LiquidObject, LiquidMaterial and LiquidTypeXTexture from local CASC 12.1.0.69933. `game_engine_core::liquid_data::LiquidCatalog` resolves each layer following `LiquidMaterialManager.cpp`: the object gives the type, the type gives the material and LVF, and the texture slots are filled by `FrameCountTexture`. For type 5 that is slot 2 bump `newoceanbump.blp` (463849), slot 3 foam `waterchop7bw.blp` (317230), and River colours (procedural texture type 1). The shader uses Float[0..17], Coefficient[0..3] and the LiquidObject flow values. Slot 0's 30 `lake_a` frames are not read by the modern water shader.
3. **Wrong colour source.** The shader now uses the LightData RiverClose/Far (or OceanClose/Far) colours at the camera, with LightParams Water/Ocean Shallow/Deep alphas blended across the same LightParams weights (`DayNightLightHolder.cpp:928-937`, `MapSceneRenderer.cpp:194-203`). The specular colour is SunColor. Underwater fog comes from the Light's underwater LightParams slot and is inert when that slot has no keyframes (`MapSceneRenderer.cpp:297-315`).
4. **Wrong blend.** The water now reads Godot's depth and screen textures. The scene behind the water is refracted and underwater-fogged by the water's thickness, then mixed with the lit close/far colour using the depth-cubic alpha. Shore alpha and foam fade over the first yards of thickness, and depth-0 vertices are invisible.

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

Proof: `godot/core/tests/wmo_liquid.rs` (5 tests): the resolution table; interior lighting; Northshire Abbey's gate fountain (`abbeygate01` 108104/108105: MOHD 0x5, liquid 5 → 13, 10 of 12 tiles, tile 0x3F hidden); the Cultists' Quay delve cave (5356285/5533972) as interior WMO Water; magma vertex UVs.

## Proof

- `godot/core/tests/liquid_water.rs` (5 tests): an LVF 0 payload keeps its depths; tile 32_48 depths range 0..>64; LiquidObject 427 resolves to type 5 with its textures, floats and coefficients; ocean colours; Shallow Water wave periods [1.0, 0.4]; an unknown object is an error.
- `godot/tests/water_material_pixels.gd` (GPU) uses the real material at the vineyard stream at noon. It checks five things: deeper vertex depth hides more of the backing (0.48 → 0.39); depth 0 is invisible (response 1.00); backing 60 yd down is fogged (0.44 → 0.33); shallow water over black matches the retail river mix within 0.002; the clock moves 2,274 px. Run with the old shader, it fails the depth-0 check (`data/diagnostics/water-2026-09-30/fixture-red-old-shader.log`).
- `godot/tests/liquid_material_pixels.gd` (GPU) draws every non-water material from its DB2 data: all cover the view and animate over 1000→2333 ms; magma (Searing Gorge LiquidObject 413) and slime are opaque, and magma is lava-coloured (0.56, 0.15, 0.01). Screenshots: `data/diagnostics/water-2026-09-30/liquids/`. In-world magma at Searing Gorge: `data/diagnostics/water-2026-09-30/magma/sheet-gorge.png`.
- `liquid_water.rs` also covers LVF 1/3 arrays, magma/slime resolution and the Searing Gorge tile's 53 magma layers re-read as LVF 1; `blp_tests.rs` covers BC5.
- Before/after Northshire sweeps: `data/diagnostics/water-2026-09-30/{before,after}/`. Streams now show a teal-tinted, refracted stream bed near the camera instead of the opaque pale sheet.

## Open

- **Specular power: unknown for retail.** The shaders use `uExteriorSpecularColor.a = 1.0` from WebWowViewerCpp `MapSceneRenderer.cpp`. Retail's `dx_5_0` water pixel shaders (`procwaterabove.bls` FDID 2977223, `water.bls` 2977281) read the exponent from `cb1[5].w`, a CPU-set constant with no reflection names, so its value is not in the shader. The Wrath 3.3.5 client uses 6.0 (solarityclient `crates/rendering/src/liquid/shader_uniform.rs:159-160`, "Native 8A38B0 uses the constant at 9E8CF8"); that is not retail evidence. With power 1, distant water gets a pale sun-coloured haze.
- Height/artistic `makeFog2`, sun attenuation (`uSunAttenuation`), classic underwater fog and wave animation for Float[16] ≠ 0 (ported, not GPU-tested).

## See Also

- [[terrain]] — MH2O geometry and streaming
- [[retail-lighting]] — LightParams blend the water colours use
