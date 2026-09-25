# Stormwind Dark Render

In Stormwind the district buildings rendered as black silhouettes. The cause was the WMO material, not the light or sky lookup. The district WMOs use the unified MapObj path (MOHD flag `0x02`). On that path MOCV is baked light added to the scene light. The engine instead drew every vertex-colored batch unlit as `texture * MOCV`, and MOCV is close to zero on most exterior vertices. Fixed in `120e5f36`.

## Symptom

At the Trade District (WoW `-8818, 660, 97`, near Auctioneer Fitch) the buildings were black silhouettes, while terrain, trees and NPCs rendered normally. Northshire looked plausible. Evidence: `data/diagnostics/auction-ui-20260924/r4-browse.webp`, reproduced on master `16a3d119` as `data/diagnostics/stormwind-20260925/before-trade.webp`.

## Isolation

- **Light lookup is not the cause.** Trade District is outside the falloff of every map-0 `Light.csv` row. The nearest row is 9651, 296 yd away with falloff end 14.9. So the lookup falls back to global row 1 → LightParams 12, the same as Northshire.
- **The washed-out sky is not Stormwind-specific.** The Northshire capture from the same build has the same near-white sky (`northshire-before.webp`). Sky color is a separate global issue and was not changed here.
- **WMO fog is not applied.** `WmoGroupFogVolume` entities are spawned but no system reads them.
- **The dark geometry is the district WMOs.** They are `sw_tradedistrict` (322057), `sw_oldtown` (329001), `sw_keep` (331158), `sw_magicdistrict` (321999), `sw_cathedraldistrict` (328958) and others. All have MOHD flags `0x1f` or `0x41f`: unified render path (`0x02`) plus do-not-fix-vertex-color-alpha (`0x08`). Northshire Abbey (107074) has `0x5`: not unified, and its exterior groups have no MOCV, so they take the lit StandardMaterial path.
- **The MOCV values explain the black.** Every `sw_tradedistrict` exterior group has MOCV. In group 37 (GFID 456954), mean RGB is `(38, 40, 35)/255`. About half of the 12928 vertices are below 16/255, and about 20% are exactly 127. Under the old material (`unlit: true`, color = texture × MOCV) that is about 15% of the texture, or black.

## Root cause

`wmo_standard_material` set `unlit: has_vertex_color`, and Bevy multiplies `ATTRIBUTE_COLOR` into the base color. The result was `texture * MOCV` with no scene light, for every root flag. Stock unified MapObj lighting (solarityclient `WorldModelSurfacePassPlan::lighting_mode`, unified branch, and `world_model.vert.glsl`) is `texture * (2 * MOCV + light)`, where light is:

- the daylight (sun plus ambient) for groups with `EXTERIOR` (`0x08`) or `EXTERIOR_LIT` (`0x40`), and for transition batches
- the MOHD ambient for other groups
- nothing for materials with the `UNLIT` flag

noggit and WebWowViewerCpp also add MOCV to the light rather than multiplying by it.

## Fix

`WmoUnifiedMaterial = ExtendedMaterial<StandardMaterial, WmoUnifiedLighting>` (`src/rendering/terrain/terrain_objects_wmo_unified.rs`, `assets/shaders/wmo_unified.wgsl`) is used for every batch of a unified root. The shader keeps MOCV out of the base color. In Exterior mode it adds `albedo * srgb_to_linear(2 * MOCV)` to Bevy's PBR lighting. In the other modes it outputs `albedo * srgb_to_linear(2 * MOCV [+ MOHD ambient]) + emissive`. MOCV and MOHD ambient are summed in gamma space and converted to linear once. For a pure multiplier this reproduces WoW's gamma-space result exactly. The additive Exterior term is an approximation.

Non-unified roots are unchanged: they keep the StandardMaterial path, including the vertex-color multiply. WMO spawning only holds `Assets<StandardMaterial>`, so the unified material asset is added in an `EntityCommands::queue` closure when the spawn commands apply. SIDN night-glow sync (`sync_wmo_sidn_emissive::<M>`) runs for both material types.

## Proof

- **GPU test.** `trade_district_exterior_wall_is_not_darker_than_daylight` (`src/rendering/terrain/terrain_objects_wmo_tests/unified_gpu.rs`) renders the real group-37 batch 0 through `spawn_wmo_group_batches` under a 2500 lx sun. It compares the result against the same texture on a lit StandardMaterial with no vertex color. RED: wall `[23,31,28]` vs daylight `[102,93,86]`. GREEN: `[102,94,88]` vs `[102,93,86]`.
- **Live, headless.** Screenshots on the shared :5000 server as `Stormlight` (`sw_ui`), taken from the same spot with the default camera: `before-trade.webp` (buildings black) and `after-trade.webp` (stone textures lit). `northshire-before.webp` and `northshire-after.webp` are identical apart from a passing guard.

## Follow-up

The unified-only material and its 3.3.5-derived factors were superseded by [[wmo-retail-lighting]]. Every WMO now uses the Retail light model. WebWowViewerCpp's Retail shaders (`precomputedLight = vColor.rgb * 2.0`) confirm the ×2. The duplicate placements are fixed there too.

## Still open

- **MOCV scale is not verified against Retail.** The `2 * MOCV` factor follows solarityclient. noggit and WebWowViewerCpp use a factor of 1. The additive term is linearized separately from the daylight, so exterior vertices with MOCV 127 may be brighter than in Retail. No Retail capture was compared.
- **Missing stock details.** Unified transition batches are drawn in one Exterior pass, not stock's two-pass crossfade. MOMT flattened-lighting (`0x20`) is not modeled.
- **Non-unified roots still multiply MOCV unlit.** They are missing the `* 2` that stock applies to the halved (fixed) MOCV. This is the "interior is also dim" in [[abbey-interior-black-world]], and it is unchanged.
- **Duplicate placements.** Each district WMO is spawned once per ADT tile that references it, e.g. `sw_harbordistrict` 4×. MODF uniqueId dedup is missing.
- **No WMO floor collision.** The interior capture (`after-dummies-interior.webp`) stands on terrain outside the house, because WMO floors have no collision. It shows lit unified exteriors, not the interior RootAmbient mode.
- **Sky.** The near-white sky at noon under LightParams 12 affects Northshire too.

## Sources

- `../../../data/diagnostics/stormwind-20260925/`: before/after captures, scene dumps, `capture.sh`, `create-char.js`
- `~/Repos/solarityclient/crates/rendering/src/shader/world_model_effect/pass.rs`, `world_model_spirv/source/world_model.vert.glsl` and `.frag.glsl`: stock unified lighting modes and `* 2` combine
- `~/Repos/solarityclient/crates/asset/src/world_model/map_obj_group.rs` (`fixed_vertex_color`): MOHD `0x08` keeps raw MOCV; unified default without MOCV is black
- `~/Repos/noggit3/src/glsl/wmo_frag.glsl`, `src/noggit/WMO.cpp` (`fix_vertex_color_alpha`): additive vertex color, and the halving that the stock shader doubles

## See Also

- [[abbey-interior-black-world]]: the earlier MOCV alpha/prepass bug on the same material path
- [[wmo-format]]: MOHD root flags
- [[skybox]]: the procedural sky that stays washed out
