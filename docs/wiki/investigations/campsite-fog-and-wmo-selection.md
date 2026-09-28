# Campsite Fog and WMO Selection

Verified: 2026-09-28.

## Freywold Spring (7) and Gallagio Grand Gallery (25): flat fog colour

Both scenes use a single map-global Light (14356 on map 2847, 14358 on map 2851) whose clear-slot LightParams (5615, 6412) author `FogEnd = 0` in every keyframe. The Godot materials read `FogEnd / 36` and `FogEnd * FogScaler / 36` as a linear range, so `(end - d) / (end - start)` divided by zero and every pixel took the fog colour. 10,628 of 19,732 LightData rows have `FogEnd = 0`; 19,564 author `FogDensity > 0`.

WebWowViewerCpp never uses FogEnd as a range for this data:

- `MapSceneRenderer.cpp:225-246`: fog start `min(farPlane, 3000) * FogScaler`, end `farPlane` (at least 277.5), density `FogDensity * 0.0005`.
- `commonFogFunctions.slang` `calculateLegacyFog`: `exp(-max(0, d - start) * density)`, then `min` with the end fade `saturate(1.42857 * (1 - d / end))`.
- `DayNightLightHolder.cpp:606-649` `fixLightTimedData`: FogScaler clamps to [-1, 1] and FogEnd rises to at least 10. A keyframe without FogDensity derives one from `FogEnd - FogEnd * FogScaler`.
- `DayNightLightHolder.cpp:1020-1030`: after mixing keyframes, density is at least 0.9. FogScaler is at least -0.2 when a keyframe authored density, otherwise at least 0.

`0a4a9d6e` carries raw FogScaler/FogDensity through the shared `sky_lightdata_data` and derives `RetailFog` there. Godot binds `fog_range = (start, end)` and `fog_density`, and the m2, wmo and terrain shaders apply the legacy term. Scene 7 now gets (20, 1000, 0.002) and scene 25 gets (0, 1000, 0.0025). Far clip is WebWowViewerCpp's default 1000 (`config.h:119`). The map flag2 0x2 density override is not applied because Map.db2 is not available locally.

## Cultists' Quay (5): only untextured terrain

The tile `2837_27_31` has no MCLY layers and one MODF WMO (5356285, doodad set 1 "Warband"). The WMO origin is 161 yd from the character slot, but its extents contain the slot. The 120 yd campsite WMO radius, measured from the origin, dropped it. `ac0ef0ba` measures from the focus to the MODF extents (`wmo_within_radius`) in both Godot and Bevy.

## Still missing after these fixes

- Scene 5: the WMO's 1,663 MODD doodads (set 0 DefaultGlobal 1,612, set 1 Warband 50) are not placed. Map 2837 has no Light rows, so lighting falls back to Light 1 (LightParams 12). ZoneLight.db2 is not extracted locally.
- Scene 7: MH2O water uses the placeholder procedural shader (pale fresnel sheet), not Retail liquid colours and textures.
- Fixed in `736ed0f6`: `M2 variation 1 has zero duration` in scene 7 came from `pa_redbird_stand.m2` (FDID 588287), Stand variation 1 with duration 0 and frequency 30583. See [weighted loop variations](../systems/animation.md#weighted-loop-variations).
