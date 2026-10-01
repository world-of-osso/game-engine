# Retail Lighting

World materials shade with Retail's light model instead of Bevy PBR. One `RetailSceneLight` resource, computed from the LightParams blend, feeds terrain, M2 (doodads, creatures, characters, equipment) and two-texture M2 effect batches. WMOs read the same buffer (see [[wmo-retail-lighting]]). Colours stay in the space LightData and textures author them in (sRGB bytes / 255): shaders convert texels to that space, shade and fog there, and hand the linear result to Bevy.

## Source

Deamon87/WebWowViewerCpp at `1a8cccbeffc46231c6497e6b3f5bfbf3507d8071`:

- `wowViewerLib/shaders/slang/common/commonLightFunctions.slang`: `calcLight`, `applyAndMixAmbients`.
- `wowViewerLib/shaders/slang/common/commonFogFunctions.slang`: `validateFogColor` (fog colour per GxBlend).
- `wowViewerLib/shaders/slang/bindless/m2/m2shader_text.slang`: M2 IsAffectedByLight and UnFogged handling.
- `wowViewerLib/shaders/slang/bindless/adt/adtShader_text.slang`: terrain `matDiffuse = layers * 2 * vColor` and the layer-alpha specular.
- `wowViewerLib/src/engine/objects/scenes/dayNightDataHolder/DayNightLightHolder.cpp`: exterior ambient, horizon and ground ambient, and direct colours. A zero horizon or ground ambient takes the ambient.
- `wowViewerLib/src/engine/algorithms/mathHelper.cpp`: `directionalLightPhiTable` / `directionalLightThetaTable` for the sun direction.
- `wowViewerLib/src/engine/objects/scenes/m2Scene.cpp` and `m2Object.cpp` (`getM2SceneAmbientLight`): standalone M2 scenes.

## Light selection

`light_params_blend` (`src/rendering/lighting/light_lookup_data.rs`) is shared by both clients. It ports WebWowViewerCpp `calculateLightParamBlends` (`LightParamCalculate.h:42-194`, commit 1a8cccb, `cf8d8153`). It returns LightParams in overlay order: each entry is mixed over the running result by its weight.

1. **Default, weight 1.** The map's zero-position Light with the highest ID. Without one, continent 0's (Light 1). See `LightParamCalculate.h:72-93` and `CSqliteDB.cpp:77-96`. There is no ParentMapID fallback.
2. **Zone lights.** Every ZoneLight polygon on the map within 50 yd of its border and of its Zmin/Zmax range. The weight is `clamp(-(signed distance - 50) / 100)`, so it is 0.5 on the border and 1 at 50 yd inside. Zones are sorted by TransitionType, then by descending LightID (`:104-172`). The inside test and border distance are `mathHelper.cpp:680-805`. PlayerConditionID and Flags are ignored, as in the reference.
3. **Local lights.** Light spheres on the map within GameFalloffEnd. The weight is 1 inside GameFalloffStart, then fades linearly (`CSqliteDB.cpp:421-429`). They are applied strongest first (`LightParamCalculate.h:176-191`). This replaced solarityclient's farthest-first order.

Data: `data/ZoneLight.csv` and `data/ZoneLightPoint.csv` come from local CASC 12.1.0.69933 (FDIDs 1310253 and 1310256), exported by `scripts/export_db2_csv.py`. Godot's `LightingCatalog::read` requires both files. Bevy logs an error and runs without zone lights.

Effect: Stormwind (ZoneLight 1859, Light 9651) and Elwynn (ZoneLight 2471, Light 12786) now overlay LightParams 6080 at weight 1 over Light 1's LightParams 12. Campsite maps have no zone lights, so scenes 1, 5, 7 and 25 keep LightParams 12, 12, 5615 and 6412.

Known data gap: `data/Light.csv` (5,072 rows) is older than the local Light.db2 (5,355). ZoneLights 2956 and 3016 name Lights that are missing from the CSV, so they contribute nothing.

## Scene fog (Godot)

Every Godot world shader (M2, WMO, terrain, water and the seven other liquid shaders) fogs through `godot/shaders/retail_fog.gdshaderinc`, a port of WebWowViewerCpp `commonFogFunctions.slang` `makeFog2` in world space (Godot y up; the reference's view-space dot products and lengths are the same):

- **Height fog** (`calculateHeightFogFactor`): height above the FogHeight plane × FogHeightScaler, through the FogHeightCoefficients polynomial, blends the scene density toward FogHeightDensity and the fog colour toward FogHeightColor/EndFogHeightColor. FogZScalar pulls the point toward the eye's level first (`calculateAdjustedViewPosition`).
- **Legacy fog** (`calculateLegacyFog`): `exp(-max(0, d - start) * density)`, end fade at farClip, as before. **Artistic fog** (`calculateArtisticFog`): the MainFogCoefficients and HeightDensityFogCoeff polynomials over MainFogStartDist..MainFogEndDist, faded into the engine end. `LegacyFogScalar` (1 unless a main or height-density curve is authored) picks between them.
- **Colour** (`calculateFogColor`): FogColor (SkyFogColor) toward EndFogColor by ((d - FogStartOffset) / EndFogColorDistance)³, height colours by the height factor, then **sun scattering**: toward SunFogColor by ((view·sun - SunFogAngle) / (1 - SunFogAngle))³ at SunAngleBlend × SunFogStrength. Every colour goes through `validateFogColor` (GxBlend 3/10/13 black, 4 white, 5 grey).
- **CPU** (`godot/core/src/retail_fog.rs`): LightData's fog columns per keyframe, fixed as `fixLightTimedData` does (unset colours fall back to SkyFogColor/EndFogColor, no SunFogColor is no sun fog, clamps, the density heuristic), time-mixed with the custom sun blend (`calcLightParamResult` :955-980), LightParams flag 0x4 (sun fog off; `LightParams.csv` now exports `Flags`), mixed across the LightParams blend (`mixStructure<FogResult>`), then SunAngleBlend × the day curve (none before 06:30 or after 18:30). `fog_uniforms` packs it as `MapSceneRenderer.cpp:214-294` does. The sky's fog band leans toward EndFogColor by farClip / EndFogColorDistance (`getLightResultsFromDB` :850-851).
- **Sun direction:** the sun disc's (`sunPhiTable`/`sunThetaTable`, `calcSunDirForFog`), not the direct light's (`sun_fog_direction`).
- **Coefficient order (divergence from WebWowViewerCpp's code, following its comment and the data):** LightData stores the polynomials constant-first; the shader evaluates x³-first. WebWowViewerCpp's `CSqliteDB.cpp` reads them unreversed, but its unset fallback is commented "DB-ordered (0,0,0,1); stored reversed" (`DayNightLightHolder.cpp:1009`), and the authored rows run from about 0 at x = 0 to about 1 at x = 1 only constant-first (e.g. LightParams 17: -0.007 + 0.447x - 0.167x² + 0.733x³). They are reversed here.
- **Not ported:** map flag2 0x2 (density 1), LightParams sun-position/direction overrides (0x100/0x200), `and `getClampedFarClip`'s per-map minimum distances (the far clip is 1000 yd).
- **Tests:** `godot/core/tests/retail_fog.rs` (6: coefficient order, artistic selection and sun-fog day curve on LightParams 4216, keyframe fallbacks, WMO blend, packing, sun direction); GPU `godot/tests/retail_fog_pixels.gd` (8 exact cases through the M2 shader: legacy half fog, height plane above/below, FogZScalar, artistic curve, end colour, sun scattering toward/away). The legacy-only shader fails its height-plane case (`data/diagnostics/avfix-2026-09-30/retail_fog_pixels-red.log`). In-world at Northshire (08:00 and 17:00; the client's time is fixed at noon unless automation sets it): legacy-only (left) against `makeFog2` (right) in `data/diagnostics/avfix-2026-09-30/fog/before-after.png`: the high valley walls lose most of the blue legacy fog, as the height fog thins above the FogHeight plane. Those captures predate the sky dome (below): their sky is Godot's grey clear colour.

## Sky dome (Godot)

The in-world background is the retail exterior sky cone (WebWowViewerCpp `map.cpp` `skyConusVBO`, `skyConus.vert/frag.slang`), drawn as a Godot sky shader (`godot/shaders/sky_dome.gdshader`) on the `WorldLighting` environment (`godot/rust/src/lighting/mod.rs`, `bind_sky_dome`).

- **Profile:** `sky_cubemap_data::sky_dome_profile` gives the cone's seven points (top pole, rings at 15.8/8.5/2.2/1.2/-0.4 degrees, bottom pole); they equal `skyConusVBO`'s ring radii and heights. A view ray takes the colour where it crosses the cone face between two points (`edge_fraction`), as the reference's per-vertex colours interpolate across each face. The 24-segment faceting is not modelled.
- **Colours:** SkyTop, SkyMiddle, SkyBand1, SkyBand2, SkySmog and SkyFog from the LightData sample (`Map::updateBuffers` `skyColor[0..5]`), the bottom pole SkyFog again; mixed in authored (gamma) space.
- **Sun tint:** toward SunFogColor by ((view·sun - SunFogAngle) / (1 - SunFogAngle))³ × SunAngleBlend·SunFogStrength, from the fog sun direction (`skyConus.frag.slang`).
- **Stars:** `environments/stars/stars.m2` (FDID 130629, `godot/core/src/sky_bodies.rs`) draws over the dome at `starsBrightnessTable`'s alpha (byte = brightness × 254 + 1; 1 from 21:30 to 03:30, 0 from 05:00 to 20:00) × (1 − the blended LightParams 0x10 flag), only while the byte is at least 2 (`updatePlanetsAndStars`). The `LightingCatalog` worker extracts it from local CASC; `godot/rust/src/sky_model.rs` (shared with the character-select skybox) draws its batches behind world depth with a `model_alpha`, on the camera.
- **Not ported:** the 0x4 skybox fog cone (`skyMesh0x4`), sun/moon discs, LightSkybox M2s in-world, and suppressing the sky inside WMO interiors without exterior view.
- **LightSkybox models:** a LightParams' `LightSkyboxID` names a LightSkybox row (`SkyboxFileDataID`, `Flags`). `sky_bodies::skybox_draws` collects them over the LightParams blend as WebWowViewerCpp `SkyBoxCollector::addSkyBox` does: each takes its LightParams' weight (the larger when two share a model) and fades every skybox collected before it by one minus that weight. `WorldLighting` draws each with `SkyModel` on the camera; flag 0x1 holds the animation at the day's fraction (`setOverrideAnimationPerc`). Twilight Highlands (Light 2925, LightSkybox 165 `twilighthighlandssky2.m2`): `godot/tests/world_skybox_flow.gd` shows the clouds changing 125128/168788 px at noon/20:00 (`data/diagnostics/worldvis-2026-10-01/skybox.log`, `skybox/pair.png`). Not drawn: flag 0x4's extra fog-coloured sky mesh (`skyMesh0x4Sky`) and `CelestialSkyboxFileDataID`.
- **Test:** `godot/tests/world_sky_flow.gd` in the real client (private server, Northshire Valley, Human warrior spawn): at noon and midnight, nine upper-screen pixels each match the dome colour along their view ray within 4/255 (with stars shown, no darker than it), and the two times differ; the stars' alpha is 0 at noon, 1 at midnight and 128/255 at 20:45 (`data/diagnostics/worldvis-2026-10-01/stars.log`, `stars/*.png`). RED on master `a60cfa58` (background clear colour, no sky): `data/diagnostics/worldvis-2026-10-01/sky-red-master-a60cfa58.log`; GREEN with captures in `data/diagnostics/worldvis-2026-10-01/sky-green/`.

## WMO fog

`03db2144` ports WebWowViewerCpp's WMO fog (commit 1a8cccb) into the Godot client. The selection lives in the shared lib, so the Bevy client could use it too.

- **Selection** (`src/asset/wmo_format/fog.rs`, `WmoFogVolume::camera_fog`, from `WmoObject::checkFog`, `wmoObject.cpp:1631-1778`). Positions are in WMO file-local coordinates.
  - The camera's group must not be exterior (0x8) or exterior-lit (0x40).
  - Record 0 is the base. The fog is off when record 0 is the only record and lacks F_FOGVOLUME 0x1000, or when record 0 has both 0x1000 and 0x10000.
  - The group's MOGP fog ids above 0 are blended in. Records flagged 0x1 (F_IEBLEND) are skipped. So are records farther from the camera than their larger radius.
  - With 0x1000 on record 0, the result is the radial-weight average (`:1540-1545`), and record 0 fills the weight up to 1. The average colour rounds back to bytes. Without 0x1000, records are lerped into record 0 farthest first, with the byte lerp `lerpImVector` (`:1564-1581`).
  - `dist_to_exit` is the distance to the nearest MOPR portal polygon of the group (`distanceToPortalPolygon`, `:1588-1622`), or `f32::MAX` when the group has no portals.
- **Conversion** (Godot: `retail_fog::wmo_fog`; Bevy/legacy: `sky_lightdata_data.rs` `wmo_retail_fog`; from `wmoFogDataToFogResult`, `DayNightLightHolder.cpp:666-691`). The end is clamped to [30, farClip]. The start is `end * start_scalar`, floored at 0. The density comes from `calcFogDensityFromStartEnd` (`:652-660`) × 0.0005, and the fog still ends at farClip.
- **Blend** (Godot: `retail_fog::blend_wmo_fog` via `lighting::apply_wmo_fog`, `blendWmoFogIntoFogResult`; `DayNightLightHolder.cpp:491-497`, `:712-750`). The weight is `clamp(dist_to_exit * 0.04)`: the exterior fog applies at a portal and the WMO fog applies fully 25 yd inside. FogScaler and density mix linearly; sun fog, FogZScalar and the artistic curve fade out; the exterior height plane stays. The colours mix in authored space, then go back to linear uniforms.
- **Godot wiring.**
  - `WmoPortals::camera_fog` finds the camera's interior group with `camera_interior_group`: the group whose bounding box contains the camera and whose floor below it is the highest. This approximates `getGroupWmoThatCameraIsInside` (`wmoObject.cpp:1343-1404`) without its MOBN BSP. `TerrainObjects`, `CampsiteObjects` and `GlobalWmoScene` return the first matching WMO. The reference keeps the last candidate (`map.cpp:487-512`).
  - `WorldLighting::sync` applies the fog before its change check. So every `bind_model`/`bind` consumer (terrain, M2, WMO, characters, creatures) gets the same fog through the existing uniforms. The shader math does not change.
  - In-world, the camera position selects the fog, and the light is still sampled at the player. Character select uses the camera's previous-frame position.
- **Data.**
  - Cultists' Quay 5356285 has one record: 0x1000, end 578, start scalar 0.129, RGB (21, 80, 99).
  - The Stockade (108631) has record 0 plain (end 133.3, start scalar 0.1, RGB (49, 91, 143)) and record 1 F_IEBLEND. The native fixture spawn is 27.1 yd from a portal, so the weight is 1.
  - The Northshire Abbey (107074) has one plain record, so it gets no WMO fog.
- **Proof.**
  - `cargo test -p game-engine-core --lib fog::`: 7 synthetic cases.
  - `--test wmo_fog`: the real 5356285 MFOG record and the conversion values.
  - `cargo test -p game-engine-godot --lib`:
    - `cultists_quay_camera_and_character_take_the_cave_fog`: group 3, weight 1.
    - `stockade_cell_block_takes_its_blue_mfog`.
    - `cultists_quay_scene_fog_takes_the_cave_mfog`: the noon LightParams 12 sample becomes start 74.562, density 0.00075 and the cave colour.
  - The `native_transfer_fixture --global-wmo` fixture passes.
  - There is no visual capture of Stormwind or the Stockade.

## Scene light

`src/rendering/lighting/retail_light.rs` defines `RetailSceneLight`: ambient, horizon ambient, ground ambient, direct colour, sun direction and fog colour/range. `update_scene_light` samples the blend at `GameTime` and writes the resource and the world cameras' `DistanceFog`. Weather still tints and shortens the fog. `upload_retail_scene_light` writes the shading fields into one `ShaderBuffer`. It has a UUID handle (`RETAIL_SCENE_LIGHT_BUFFER`) and is updated in place, so every material binds the same GPU buffer.

The sun direction follows the client's directional-light table, not the sky dome's celestial sun. At noon it lights from 37° above the horizon; at dawn and dusk (day 0.25, 0.75), from 20°. `SkySun` (the Bevy `DirectionalLight`) survives only as the shadow caster, rotated along this direction. Its colour and illuminance shade nothing.

Character creation lights its backdrop as a Retail M2 scene. The ambient is the sum of the model's `ambient_color × ambient_intensity` terms, or white when there are none; there is no direct light.

## Equation

`assets/shaders/retail_lighting.wgsl` (mirrored by `retail_shade` in Rust):

- `hemisphere` = mix(horizon, ambient, N·up) for up-facing normals, mix(horizon, ground, −N·up) otherwise.
- `ambient` = mix(0.7·hemisphere, 1.1·hemisphere, 0.5 + 0.5·N·L).
- `result` = diffuse · (ambient + direct · N·L · sun_visibility).

`sun_visibility` is Bevy's cascaded shadow map for directional light 0. WebWowViewerCpp has no shadow map, and scaling only the direct term is this engine's choice. There are no point lights (`accumLight = 0`), no emission, and no M2 specular (WebWowViewerCpp's `calcSpec` returns 0).

Terrain adds `layer_alpha · direct · max(0, N·H)^20` (adtSpecMult 1). MCCV is decoded as byte/127, which is already the client's `2 × vColor`.

Fog uses the camera's `DistanceFog` range and colour, applied in authored space. Additive and modulating blends fog towards black, white or grey by GxBlend. Render flag 0x1 (unlit) skips shading only, and 0x2 (unfogged) skips fog.

## Materials

- `M2Material = ExtendedMaterial<StandardMaterial, RetailLit>` (`src/rendering/model/retail_m2_material.rs`, `assets/shaders/retail_m2.wgsl`). StandardMaterial supplies the texture, vertex colour and alpha. `retail_m2_material(base, render_flags, blend_mode)` disables the base fog.
- `M2EffectMaterial` (`assets/shaders/m2_effect.wgsl`): combiners in authored space, then the same lighting and fog.
- `TerrainMaterial` (`assets/shaders/terrain.wgsl`): layers blended in authored space. The MTXF cube-map reflection samples the sky cubemap. MCSH baked shadows are not rendered (WebWowViewerCpp ignores them): the ADT parser still reads MCSH, but no texture is built, uploaded or bound. The PBR roughness/reflectance are gone.

## Native Godot producer boundary

`90f91a5a` gives native `TerrainLight` a common `bind_model`/`clear_model` boundary for the nine M2 ambient/direct/sun/fog uniforms. It does not bind `environment_map`: terrain retains that cube-map binding, while the native M2 shader declares no cube uniform.

`b628997e` lets `WorldUnits` retain the current sampled light. Native creature visuals receive it on spawn or display replacement, existing visuals rebind on a live sample change, and map changes/transfers clear both material overrides and retained light; world reset drops retained light with its nodes. Bounded PASS at `0f4be666`: native `cargo fmt --all -- --check` and `cargo check -p game-engine-godot` exit 0 with no warnings; retained real-UDP headless fixture proof exits 0 through 11 phases, covering static geometry/texture, lifecycle, live sampled-light update, replacement, new-map light, and reset (`/tmp/claude/native-creature-light-green-0f4be666.log`). It asserts native resource values, not rendered pixels. The synthetic WDT has no ADT and deliberately is not `InWorld`; no readiness conclusion follows. The fixture's `authoredFogEnd / 36` input is the same shared-original value. Intermediate material clearing on map change/transfer, transfer itself, reset material values after freed nodes, and terrain cube preservation remain source-only; `clear_model` intentionally excludes `environment_map`. This establishes neither lighting correctness, visual parity, nor full conversion. Test-only review findings remain non-feature work: explicit phase state machine (cognitive 14), RGB hex authored data, and sequential `create` fixture staging.

## Display

World cameras use `Tonemapping::None` (`world_camera_tonemapping`). Measured on the GPU, TonyMcMapface turned Retail texels 200/180/150 into 173/158/135 and 250/245/235 into 195/192/186. Cameras no longer get Bevy image-based lighting (the 300-intensity `GeneratedEnvironmentMapLight`).

## Proof

- `retail_light` unit tests: noon LightParams 12 bytes, ambient fallback, sun-direction tables, `retail_shade` on a concrete texel, and hemisphere blending.
- `retail_m2_material_gpu_tests`: the texel (128,100,60) at noon renders (97,80,50) lit, unchanged unlit, (87,100,97) half-fogged, and unfogged with flag 0x2. The world-camera tonemapping leaves texels unchanged.
- `terrain_retail_gpu_tests`: the same texel renders (97,80,50), or (109,89,57) with full layer-alpha specular.
- `m2_effect_fog_gpu_tests`: the effect batch equals M2Material under one scene light.
- Screenshots: `data/diagnostics/sky-20260925/retail-*.webp`, with `retail-wmo-*` from a preview merge with the WMO scene-light branch, and `retail-with-wmo-compare.png` against `before-*`.

## Gaps

- **Point lights:** M2 and WMO point lights are still Bevy PBR lights and do not reach Retail materials (`accumLight`).
- **Fog model (Godot):** the full `makeFog2`, see "Scene fog (Godot)" below. Bevy still fogs linearly from `FogEnd / 36`, which fogs every FogEnd-0 LightParams (54% of LightData rows) completely.
- **Combiners:** single-texture M2 batches use StandardMaterial's texture × colour, not WebWowViewerCpp's `calcM2FragMaterial` pixel-shader combiners.
- **WMO fog (Godot):** underwater MFOG fog is selected but not used (Godot has no underwater fog). The camera's group comes from `camera_interior_group`, not the reference BSP query. Bevy parses MFOG and spawns `WmoGroupFogVolume` entities, but nothing reads them.
- **Map flag2 0x2:** the fog-density override is not ported. No map sets it in `data/db2/12.1.0.69933/Map.csv`.
- **LightParams sun overrides:** flags 0x100 and 0x200 (SunPolar/SunAzimuth, OverrideSunPosition) are not decoded.
- **Interpolation space:** LightData colours still interpolate in linear space, not bytes.

## See Also

- [[washed-out-sky]]: LightData decode, sky dome and light-zone blend
- [[wmo-retail-lighting]]: WMO interior/exterior light on the same scene light
- [[skybox]]: sky systems that compute the blend
- [[terrain]]: layer blending
- [[character-rendering]]: native authored M2 creature materials
