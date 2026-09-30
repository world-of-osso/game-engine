# Northshire pale water

Northshire's streams rendered as flat, pale white-blue sheets. The Godot water shader was the Bevy procedural placeholder. It used a generated normal map, constant base and sky colours, a hard-coded light direction and a fixed 0.08–0.85 alpha. It also never received MH2O vertex depths. Water now ports the retail water material from WebWowViewerCpp `liquidWaterMat.slang` (`calcWaterLiquidMat`), with inputs from DB2 and LightData.

## Causes and fixes

1. **Vertex depths were dropped.** Northshire's layers are LiquidType 5 "Slow Water" through LiquidObject 427. Its LiquidMaterial 1 uses LVF 0: all heights first, then all depth bytes (wowdev MH2O; WebWowViewerCpp `LiquidDataGetters.h` `getLiquidDepth`). The parser read only the heights, so every vertex got depth 1.0. `adt_tex_water.rs` now also reads the depths. Tile 32_48 has depths from 0 to 80, and depth 0 is authored on dry cells.
2. **No LiquidType material.** Colours and normals were constants. `scripts/export_db2_csv.py` now exports LiquidType, LiquidObject, LiquidMaterial and LiquidTypeXTexture from local CASC 12.1.0.69933. `game_engine_core::liquid_data::LiquidCatalog` resolves each layer following `LiquidMaterialManager.cpp`: the object gives the type, the type gives the material and LVF, and the texture slots are filled by `FrameCountTexture`. For type 5 that is slot 2 bump `newoceanbump.blp` (463849), slot 3 foam `waterchop7bw.blp` (317230), and River colours (procedural texture type 1). The shader uses Float[0..17], Coefficient[0..3] and the LiquidObject flow values. Slot 0's 30 `lake_a` frames are not read by the modern water shader.
3. **Wrong colour source.** The shader now uses the LightData RiverClose/Far (or OceanClose/Far) colours at the camera, with LightParams Water/Ocean Shallow/Deep alphas blended across the same LightParams weights (`DayNightLightHolder.cpp:928-937`, `MapSceneRenderer.cpp:194-203`). The specular colour is SunColor. Underwater fog comes from the Light's underwater LightParams slot and is inert when that slot has no keyframes (`MapSceneRenderer.cpp:297-315`).
4. **Wrong blend.** The water now reads Godot's depth and screen textures. The scene behind the water is refracted and underwater-fogged by the water's thickness, then mixed with the lit close/far colour using the depth-cubic alpha. Shore alpha and foam fade over the first yards of thickness, and depth-0 vertices are invisible.

Unported LiquidMaterials (2, 4, 5, 10, 12, 13, 14, 18: magma, slime, mercury, fog, ley line, fel, swamp, azerite) are reported per layer and not drawn. LiquidObjects whose material is not LVF 0 are also reported, because the parser reads LiquidObject vertices as LVF 0.

## Proof

- `godot/core/tests/liquid_water.rs` (5 tests): an LVF 0 payload keeps its depths; tile 32_48 depths range 0..>64; LiquidObject 427 resolves to type 5 with its textures, floats and coefficients; ocean colours; Shallow Water wave periods [1.0, 0.4]; an unknown object is an error.
- `godot/tests/water_material_pixels.gd` (GPU) uses the real material at the vineyard stream at noon. It checks five things: deeper vertex depth hides more of the backing (0.48 → 0.39); depth 0 is invisible (response 1.00); backing 60 yd down is fogged (0.44 → 0.33); shallow water over black matches the retail river mix within 0.002; the clock moves 2,274 px. Run with the old shader, it fails the depth-0 check (`data/diagnostics/water-2026-09-30/fixture-red-old-shader.log`).
- Before/after Northshire sweeps: `data/diagnostics/water-2026-09-30/{before,after}/`. Streams now show a teal-tinted, refracted stream bed near the camera instead of the opaque pale sheet.

## Open

- **Specular power.** `MapSceneRenderer.cpp:204` binds `uExteriorSpecularColor.a = 1.0`, the power used by the ported shader. With power 1 the SunColor highlight adds about +0.2 to every channel at grazing angles, which makes distant water pale grey. The retail value is unknown.
- Height/artistic `makeFog2`, sun attenuation (`uSunAttenuation`), WMO liquids, the other LiquidMaterials, LVF 1/3 array layout (the parser reads them interleaved; wowdev lists separate arrays), and wave animation for types with Float[16] ≠ 0 (ported but not GPU-tested).

## See Also

- [[terrain]] — MH2O geometry and streaming
- [[retail-lighting]] — LightParams blend the water colours use
