# Campsite Fog and WMO Selection

Verified: 2026-09-28.

## Freywold Spring (7) and Gallagio Grand Gallery (25): flat fog colour

Both scenes use a single map-global Light (14356 on map 2847, 14358 on map 2851) whose clear-slot LightParams (5615, 6412) author `FogEnd = 0` in every keyframe. The Godot materials read `FogEnd / 36` and `FogEnd * FogScaler / 36` as a linear range, so `(end - d) / (end - start)` divided by zero and every pixel took the fog colour. 10,628 of 19,732 LightData rows have `FogEnd = 0`; 19,564 author `FogDensity > 0`.

WebWowViewerCpp never uses FogEnd as a range for this data:

- `MapSceneRenderer.cpp:225-246`: fog start `min(farPlane, 3000) * FogScaler`, end `farPlane` (at least 277.5), density `FogDensity * 0.0005`.
- `commonFogFunctions.slang` `calculateLegacyFog`: `exp(-max(0, d - start) * density)`, then `min` with the end fade `saturate(1.42857 * (1 - d / end))`.
- `DayNightLightHolder.cpp:606-649` `fixLightTimedData`: FogScaler clamps to [-1, 1] and FogEnd rises to at least 10. A keyframe without FogDensity derives one from `FogEnd - FogEnd * FogScaler`.
- `DayNightLightHolder.cpp:1020-1030`: after mixing keyframes, density is at least 0.9. FogScaler is at least -0.2 when a keyframe authored density, otherwise at least 0.

`0a4a9d6e` carries raw FogScaler/FogDensity through the shared `sky_lightdata_data` and derives `RetailFog` there. Godot binds `fog_range = (start, end)` and `fog_density`, and the m2, wmo and terrain shaders apply the legacy term. Scene 7 now gets (20, 1000, 0.002) and scene 25 gets (0, 1000, 0.0025). Far clip is WebWowViewerCpp's default 1000 (`config.h:119`). The map flag2 0x2 density override (`DayNightLightHolder.cpp:16`, `:1023-1026`: FogDensity = 1.0) is not ported. Map.db2 (FDID 1349477) is not in the local CASC archives, but `data/db2/12.1.0.69933/Map.csv` has all 1,184 maps and none sets `Flags_2 & 0x2`, so the override changes nothing on this build.

## Cultists' Quay (5): only untextured terrain

The tile `2837_27_31` has no MCLY layers and one MODF WMO (5356285, doodad set 1 "Warband"). The WMO origin is 161 yd from the character slot, but its extents contain the slot. The 120 yd campsite WMO radius, measured from the origin, dropped it. `ac0ef0ba` measures from the focus to the MODF extents (`wmo_within_radius`) in both Godot and Bevy.

## Still missing after these fixes

- Scene 5: the WMO's 1,663 MODD doodads (set 0 DefaultGlobal 1,612, set 1 Warband 50) are not placed.
- Scene 5: darker and less blue than Retail even with the WMO fog ported (see below).

## Cultists' Quay (5): grey instead of blue

The Light table does not cause this. With local-CASC 12.1.0.69933 data, the reference lighting rule gives the same result as the engine: Light 1, LightParams 12.

- Light.db2 (FDID 1375579, 5,355 rows) has no row for map 2837. DBCache.bin (build 69933) has no hotfixes for Light, LightParams, LightData, ZoneLight or ZoneLightPoint.
- ZoneLight.db2 (735 rows, 144 maps) has no zone on maps 2703, 2837, 2847 or 2851.
- WebWowViewerCpp has no ParentMapID fallback for lights. Without a Light row on the map, it takes the continent-0 zero-position Light (`LightParamCalculate.h:72-93`, `CSqliteDB.cpp:77-96`), which is Light 1.

The blue comes from the WMO. `11xp_arathorzealots01.wmo` (5356285) has 17 interior groups out of 18. Its data:

- MFOG record 0: flag 0x1000, end 578, start scalar 0.129, colour RGB (21, 80, 99).
- MAVG ambient: RGB (40, 55, 76).
- MOHD ambient: black.

Godot applies MAVG as the interior ambient (`wmo/scene.rs` `wmo_interior_ambient`). It does not port WMO fog, WebWowViewerCpp `WmoObject::checkFog` (`wmoObject.cpp:1631`), which `DayNightLightHolder.cpp:390-396` blends when the camera is inside a WMO group. The capture at `cf8d8153` (`data/diagnostics/zonelight-20260928/campsite-5-cf8d8153.png`) shows the cave and ship grey under LightParams 12. The Retail preview is `data/ui/campsites/cultists-quay.ktx2` (as PNG: `cultists-quay-retail.png` in the same folder).

`cf8d8153` ports the reference light selection anyway (see [[retail-lighting]]). It changes in-world lighting (Stormwind, Elwynn), not the campsites.

### WMO fog ported (`03db2144`)

`03db2144` ports the WMO fog (see [[retail-lighting]], WMO fog). At the authored camera (WarbandScene 5 Position) and at the character slot (WarbandScenePlacement 40), the camera stands in cave group 3. That group is interior (flags 0x83002a05) and has no portals, so the distance to the exit is `f32::MAX` and the weight is 1. Record 0 applies in full: legacy fog start 74.6 yd and density 0.00075 per yard (a 503 yd span, over 500, so density 1.5). The colour is RGB (21, 80, 99).

Capture `data/diagnostics/wmo-fog-20260928/campsite-5.png`: the far ship and walls move toward teal, for example (16,17,18) → (16,22,25) and (37,32,26) → (35,37,33). Near pixels do not change. Retail stays much brighter and bluer, and the fog does not explain that. Fog can only pull pixels toward (21, 80, 99), which is darker than Retail's lit ship. At this density, 150 yd of fog is about 5%. The remaining gap is lighting or content (the 1,663 unplaced MODD doodads, including light shafts), not fog selection.
- Scene 7: MH2O water uses the placeholder procedural shader (pale fresnel sheet), not Retail liquid colours and textures.
- Fixed in `736ed0f6`: `M2 variation 1 has zero duration` in scene 7 came from `pa_redbird_stand.m2` (FDID 588287), Stand variation 1 with duration 0 and frequency 30583. See [weighted loop variations](../systems/animation.md#weighted-loop-variations).
