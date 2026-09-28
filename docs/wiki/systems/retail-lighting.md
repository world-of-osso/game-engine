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
- **Fog model:** Retail fog is the full `makeFog2` (exponential, height and end colour). Godot implements the legacy exponential term only (start `farClip * FogScaler`, density `FogDensity * 0.0005`, end fade at farClip; see [[campsite-fog-and-wmo-selection]]); height fog, artistic fog, end/height/sun fog colours and WMO MFOG are not ported. Bevy still fogs linearly from `FogEnd / 36`, which fogs every FogEnd-0 LightParams (54% of LightData rows) completely.
- **Combiners:** single-texture M2 batches use StandardMaterial's texture × colour, not WebWowViewerCpp's `calcM2FragMaterial` pixel-shader combiners.
- **LightParams sun overrides:** flags 0x100 and 0x200 (SunPolar/SunAzimuth, OverrideSunPosition) are not decoded.
- **Interpolation space:** LightData colours still interpolate in linear space, not bytes.

## See Also

- [[washed-out-sky]]: LightData decode, sky dome and light-zone blend
- [[wmo-retail-lighting]]: WMO interior/exterior light on the same scene light
- [[skybox]]: sky systems that compute the blend
- [[terrain]]: layer blending
- [[character-rendering]]: native authored M2 creature materials
