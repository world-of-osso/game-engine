# WMO Retail Lighting

This page follows up [[stormwind-dark-render]]. WMO lighting, alpha testing, two-layer shaders and placement now follow Retail. Every WMO batch renders through one material, `WmoLitMaterial` (`terrain_objects_wmo_lighting.rs`, `assets/shaders/wmo_lighting.wgsl`), with one light model. The earlier solarityclient-based modes (unified Exterior/RootAmbient/Authored and the ordinary multiplicative path) are gone.

## Reference

The reference is WebWowViewerCpp (Deamon87), whose shaders follow Retail's MapObj shaders. It was read at commit `1a8cccbeffc46231c6497e6b3f5bfbf3507d8071` (2026-09-14) via `gh api`. Our assets are Retail 12.x, and solarityclient (`~/Repos/solarityclient`) replays the 3.3.5 client, so its semantics are superseded.

Files used:

- **`wowViewerLib/shaders/slang/common/commonLightFunctions.slang`**
  - `calcLight`: `matDiffuse * (ambient + precomputedLight + sun * NdotL)`. The interior and exterior parts are mixed by `interiorExteriorBlend`.
  - `applyAndMixAmbients`: adds `precomputedLight` to the ambient, horizon and ground ambient.
- **`wowViewerLib/shaders/slang/bindless/wmo/wmoshader_text.slang`**
  - `precomputedLight = vColor.rgb * 2.0`. This is the ×2 on MOCV.
  - `interiorExteriorBlend = mix(vColor.w, 1, isExteriorLit)`.
- **`wowViewerLib/shaders/slang/common/commonWMOMaterial.slang`** (`caclWMOFragMat`)
  - With a blend mode above 0, `tex.a < 0.50196` (128/255) is discarded.
  - The per-pixel-shader diffuse is chosen here: MOMT 6 TwoLayerDiffuse, MOMT 13 → TwoLayerDiffuseOpaque, MOMT 21 → MapObjLod, Metal/EnvMetal keep `matDiffuse = tex.rgb`.
- **`wowViewerLib/src/engine/objects/iWmoApi.h`**: `wmoMaterialShader`, the MOMT shader → vertex/pixel shader table.
- **`wowViewerLib/src/engine/geometry/wmoGroupGeom.cpp`**
  - `fixColorVertexAlpha`: the MOCV fixup.
  - `getVBO`: defaults. Missing MOCV is `(0,0,0,0)`; missing MOCV2 is `(0,0,0,255)`.
- **`wowViewerLib/src/engine/objects/wmo/wmoGroupObject.h`**: `isInteriorLightingLit`.
- **`wowViewerLib/src/engine/objects/wmo/wmoObject.cpp`**: `calculateAmbient` (MAVG, then MAVD, then MOHD).
- **`wowViewerLib/src/engine/persistance/header/wmoFileHeader.h`**: the MOHD flag names.

## solarityclient (3.3.5) vs Retail

| | solarityclient (3.3.5 replay) | Retail (WebWowViewerCpp), implemented |
|---|---|---|
| Light model | Two families. The ordinary path multiplies: `tex * 2*MOCV * daylight`, and interior batches are `tex * 2*MOCV`. The unified path (MOHD `0x02`) adds: `tex * (2*MOCV + light)`, where light is Exterior, RootAmbient or Authored per group or batch. | One model: `tex * (ambient + 2*MOCV + sun)`. Interior and exterior light are blended per vertex by the fixed MOCV alpha, in gamma space. Exterior-lit groups are forced exterior. `F_UNLIT` shows the texture alone. |
| ×2 on MOCV | `diffuse * lighting * 2` in the fragment shader | `precomputedLight = vColor.rgb * 2.0` |
| Interior ambient | MOHD ambient (RootAmbient) | MAVG for the active doodad set, else the first MAVD, else MOHD |
| MOCV fixup | Transition vertices `>>1`. The rest `(c + c*a>>6) >> 1`, with alpha 255. No ambient subtraction. | Subtracts the MOHD ambient unless MOHD `0x02` (skip base color). Transition vertices get `(c-amb)*(1-a)/2` and keep their alpha. The rest get `(c*a/64 + c - amb)/2`, with alpha 255 in exterior groups and 0 in interior groups. MOHD `0x08` (lighten interiors) keeps the color raw. |
| Transition batches | Two-pass crossfade (SourceAlphaOpaque, then InverseSourceAlphaAdd) | Single pass, per-vertex blend by transition-vertex alpha |
| AlphaKey (blend 1) | test at 224/255 | discard below 128/255 |
| Two-layer shaders | 3.3.5 shader ids only (MOMT 6: `mix(t2, t1, MOCV2.a)`) | MOMT 6: `mix(mix(t1,t2,t2.a), t1, MOCV2.a)`. MOMT 13: `mix(t2, t1, MOCV2.a)`, opaque. MOMT 21 MapObjLod: `t1`. MOTV2 and MOCV2 are used whatever the MOMT flags. |
| Metal / EnvMetal | — | diffuse kept, specular/env added (this engine used PBR metallic 0.85 before) |

## Changes (branch `wmo-parity`, rebased on master `9de26672`)

| Commit | Change |
|---|---|
| `4f1acdc8`, `a0fc5294` | 3.3.5 multiplicative ordinary path and 224/255 AlphaKey. Superseded by the next two rows. |
| `e11479f1` | Retail light model and MOCV fixup. |
| `d7d9e166` | AlphaKey at 128/255. Blend 0 has no test. |
| `7e8648a1`, `3180025b` | MOMT 6/13 blend by MOCV2 in the shader. A custom vertex stage carries MOCV2 at location 8. Prepass pipelines keep Bevy's vertex stage. |
| `ca3f3b94` | Metal/EnvMetal are no longer metallic. |
| `e43aaeec` | Interior/exterior blend in gamma space (transition crossfade). |
| `dec4bc29` | Char-select test apps register `Assets<WmoLitMaterial>`. |
| `15041289` | A MODF uniqueId placed by several tiles spawns once (`AdtManager.shared_wmos`). |
| `f2b5b508` | One material path: every WMO batch uses `WmoLitMaterial`. The scene light is read only through `scene_daylight()` in `wmo_lighting.wgsl`, currently Bevy PBR sun and ambient. That is where the shared Retail scene light (sky branch) will plug in. |

## Proof

**GPU tests.** `cargo test --bin game-engine terrain_objects_wmo -- --ignored --test-threads=1` passes 12/12 at `f2b5b508`. Expected values come from the Retail equation or from an independent StandardMaterial reference, not from old pixels.

- `unified_gpu`: the Trade District wall is not darker than daylight: `[102,94,88]` vs `[102,93,86]`.
- `interior_gpu`: both Abbey prepass tests pass.
- `interior_light_gpu`: Abbey batches 0 and 13 match `tex*(ambient + 2*MOCV)`.
- `alpha_gpu`: AlphaKey discards alpha 100 and keeps 160 (RED under 224). Opaque keeps alpha 60.
- `two_layer_gpu`: MOMT 13 blue/red at MOCV2 alpha 0/1; MOMT 6 `[127,0,128]`; also with a prepass camera. RED: the second layer was ignored, and the prepass pipeline failed validation.
- `metal_gpu`: RED `[72,71,72]` vs `[127,126,127]`.
- `crossfade_gpu`: interior 11, half 79, exterior 147. RED 107 with a linear-space mix.

**Unit tests.**
- Lib `wmo`: 126, including 4 `mocv_fixup_*` tests that were RED against the old fixup.
- Bin filters: `terrain` 160, `wmo` 47, `lod` 21, `sidn` 3, `terrain_shared_wmos` 1, `char_select` 121, `char_create` 60.
- Two failures also fail on master:
  - `setup_char_select_scene_proves_render_path_via_runtime_scene_snapshot`
  - `char_create_shared_request_uses_live_name_after_next_and_category_changes`

**Live, headless.** Files are in `data/diagnostics/wmo-parity-20260925/`. Every run had 0 wgpu validation errors.

- Abbey interior: `1-before-abbey-interior.webp` (master, dark blue) vs `F-after-abbey-interior.webp` (lit with warm baked light).
- Northshire steps: `1-before` vs `F-after`. The WMO is unchanged; the terrain differs because of master's terrain blend work.
- Trade District roofs return: `2-before/after-trade.webp`.
- Placement dedup: `7-before/after-trade-tree.txt`. The `sw_*` district roots drop from 4 to 1, and WMO roots from 89 to 62.

## Shared Retail scene light (branch `wmo-scene-light`)

Commit `ebde0ef7` (on sky `98196cd8`, which includes the Retail scene light `5d6ba82f`) removes Bevy PBR from WMO shading:

- `wmo_lighting.wgsl` binds `crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER`, a storage buffer at 103. It shades in authored (gamma) space with `retail_lighting.wgsl`:
  - exterior: `retail_shade(scene light with 2*MOCV added to ambient/horizon/ground, texel, N, sun shadow)`;
  - interior: `retail_shade(WMO interior ambient + 2*MOCV, no direct light, WWV's default interior sun direction (-0.30822, -0.30822, -0.9) for the sky/ground mix)`.
- The two are mixed by the MOCV alpha. `retail_apply_fog` fogs per MOMT blend unless `F_UNFOGGED`; the base material's Bevy fog is off.

GPU tests set a fixed `RetailSceneLight`, and their expected values are the Retail equation:

- `exterior_surface_shades_with_retail_scene_light` gives `107`, against `retail_light::retail_shade` = `107`.
- The Abbey batches give `[35,19,19]` vs `[35,19,19]` and `[82,145,172]` vs `[82,144,172]`, computed on the CPU from the GPU-sampled texel.

All 13 WMO GPU tests pass. Live captures in `data/diagnostics/wmo-scene-light-20260925/`: `before-*` is master, `after-*` is this branch, 0 validation errors.

## Godot standalone shader evidence

Commit `8116839b` adds `godot/shaders/wmo.gdshader` and `godot/tests/wmo_material_pixels.gd`. The actual Vulkan fixture exits 0 with 25 pixels covering MOCV interior/exterior interpolation and missing defaults; MOMT 6/13, MOCV2, UV2 and missing-UV2 repeat sampling; opaque/AlphaKey/GX alpha modes; SIDN emissive and unlit; fog/unfogged behavior; and direct light before a real caster, under its shadow, and after removal (`/tmp/claude/native-wmo-shader-green-8116839b.log`).

The initial RED is an absent shader at `/tmp/claude/native-wmo-shader-red.log`. A first repeat-UV1 oracle was wrong because the original root sampler is linear; only the fixture expectation was corrected. Compositor protocol warnings in the run are not shader `ERROR`s. This is standalone shader evidence, not native WMO scene or material binding, portal culling, water/doodad rendering, visual parity, or a final gate. Independent verifier458 is pending.

## Godot: every retail MOMT shader (commits `c23a65e0`, `75896f5f`)

The Godot client used to whitelist MOMT 0/1/4/5/6/7/13/21 and drop the whole WMO on any other id (campsite ground WMOs 4907674/4684716/4684717/5484842/4883307 on MOMT 23, Stormwind portal room 2320850 on 9/12, garrison farm 892927 on 16, campsite building 6357544 on 2/15/22). It now implements the whole WebWowViewerCpp table (reference commit `1a8cccb`, `~/Repos/WebWowViewerCpp`):

- **Table.** `godot/rust/src/wmo/scene.rs` `RETAIL_WMO_SHADERS` copies `wowViewerLib/src/engine/objects/iWmoApi.h:188-309` `wmoMaterialShader` (MOMT id → WmoVertexShader, WmoPixelShader). Ids ≥ 24 are an error: the reference asserts `shader < MAX_WMO_SHADERS` (`wmoObject.cpp:1807`); release builds would fall back to 0. 10 waterWindow and 14 submarineWindow are (None, None) = (-1, -1); the reference still draws them through its `-1` branches (`commonWMOMaterial.slang:63-67`, `:320-323`): `tex * tex2`, UV1/UV2.
- **Textures.** `WmoMaterialDef::retail_texture_fdids` follows `wmoObject.cpp:1832-1853`: diffuse, env, texture_2 always; `color_2`, `flags_2`, `runTimeData[0]` for pixel 19; plus `runTimeData[1..4]` for pixel 20. FDID 0 binds a 1×1 (0,0,0,0) texel, the reference's black pixel for a null texture (`GDescriptorSet.cpp:83-90`, `GDeviceVulkan.cpp:462-464`). Every file texture goes through `assets::material::shared_texture`; slots the pixel shader never samples are not loaded.
- **Vertex streams.** The reference binds every MOTV set (up to four), the second MOCV and MOC2 the group file has, whatever the material flags, with defaults UV (1,1), MOCV2 (0,0,0,255), MOC2 (0,0,0,255) (`wmoGroupGeom.cpp:97-148`, `:393-441`). The shared parser now keeps a fourth MOTV (it used to overwrite the third) and MOC2. Godot binds UV2 = MOTV2, CUSTOM0 = (MOTV3, MOTV4), CUSTOM1 = MOC2 RGBA8, CUSTOM2.x = MOCV2 alpha. MOC2 is BGRA in memory like MOCV, so the reference's `vColorSecond.bgr` is MOC2's logical RGB.
- **Vertex shaders.** `commonWMOMaterial.slang:304-361` `calcWMOVertMat`: T1_Refl → UV2 = `reflect(normalize(pos), n).xy`; T1_Env_T2 → UV2 = `posToTexCoord` (`commonFunctions.slang:15-24`), UV3 = MOTV2; Comp_Refl → UV3 = reflect; Comp_Terrain → UV2 = view-space `pos.xy * 0.24`; all in view space, per vertex. **Not ported:** MOUV UV animation (`makeWmoUVAnimVec`); the unit of its scene time is not established.
- **Pixel shaders.** `wmo.gdshader` `wmo_fragment` ports `caclWMOFragMat` (`commonWMOMaterial.slang:29-284`) case by case in authored (gamma) space. Emissive is added after lighting and dropped by F_UNLIT, like `calcLight` (`commonLightFunctions.slang:103-198`). Specular is zero because the reference `calcSpec` returns `vec3(0)` (`:201-203`), so MapObjSpecular/Metal (1, 2) and TwoLayerTerrain (8) show diffuse only. Opacity follows each case (1.0 for 3, 4, 5, 12, 14, 19, 20). Shader 7 is now blended on the GPU (`mix(t1, t2, 1 - MOCV2.a)`, emissive `t3`), replacing the CPU composite and its overlay resizing.
- **MapObjParallax (pixel 19, MOMT 22)** is ported literally, including `contangent_frame` (`commonFunctions.slang:44-59`), its read of `tex3.b` from the top-level texture-3 sample, and the texture-3 lookup at `vColorSecond.bg`.
- **MapObjDFShader (pixel 20, MOMT 23)** is ported literally: four layers (textures 2-5 at UV1-4) weighted by MOC2 RGB and `1 - sum`, multiplied by heights from textures 6-9 alpha, sharpened and normalised; emissive = `mixed.a * env(texture 1) * mixed.rgb`; diffuse mixes toward black by MOC2 alpha. Two reference quirks are kept: `weightsMax` compares `r, g, r, a` and skips `b` (`:267`), and the ambient-occlusion colour is black because the reference's AO is a TODO (`:279`).
- **Partial WMOs.** One batch that cannot be built (id ≥ 24, blend mode > 3, missing texture file, invalid triangles) is a counted, logged `CampsiteObjects`/`WorldObjects` failure; the other batches still spawn. The Bevy client never drops a WMO for its shader: `describe_wmo_shader` maps unknown ids to plain diffuse, skips missing or different-size overlays without a message, and gives emissive shaders a constant 0.05 emission (`terrain_objects_wmo_material.rs`, `terrain_objects_wmo_surface.rs`).

**Proof.**
- `cargo test -p game-engine-core --test wmo_retail_streams`: four distinct MOTV sets and MOC2 in group 4908148, nine texture FDIDs of 4907674 material 0.
- `cargo test -p game-engine-godot --lib wmo::`: 17 pass. The five campsite DF WMOs, portal room 2320850, farm 892927 and building 6357544 prepare with no batch error and every sampled texture decodes from local CASC.
- `godot/tests/wmo_material_pixels.gd` on Vulkan: every MOMT 0-21 against the reference formula, hand-derived values for MOMT 9, 12, 16, two MOMT 23 blends (MOC2-weighted heights, layer 4 at MOTV4), MOMT 22, and missing MOC2/MOTV4 defaults. With the previous shader, it fails at its first retail case.
- `native_input_fixture overlay`: streamed WMO 108238 group 38 (MOMT 7) binds its layers at authored sizes 512/128; the GPU shows texture 1 at MOCV2.a = 1 and texture 2 at 0.
- Live character select, campsites 7 and 25: 457 and 706 pixel-20 batches plus MOMT 2/7/9/11/12/13/19 batches spawn, with no WMO errors. Both scenes are hidden by fog. Their scene light has linear fog with `fog_range` (0,0), which fogs every material fully (`apply_retail_fog` divides by zero). That is a separate lighting bug. With fog off, campsite 7 shows DF-shaded stone buildings, which disappear when pixel-20 batches are hidden, and campsite 25 shows its interior.

## Godot: WMO doodad light (branch `wmodoodadcost`)

An M2 placed by a WMO is lit as WebWowViewerCpp lights it (reference commit `1a8cccb`), in `godot/rust/src/wmo/doodad_light.rs`:

- **Blend.** Every WMO doodad starts interior-lit (`wmoObject.cpp:98`, `setInteriorExteriorBlend(0)`); it is exterior-lit once a group whose MODR references it is exterior-lit (`wmoGroupObject.cpp:246-256`, the same group rule as above).
- **Interior light**, only when the first referencing group is interior-lit (`wmoObject.cpp:114-122`): `applyColorFromMOLT` (`:186-268`) and `applyLightingParamsToDoodad` (`:270-326`) turn the MODD colour, MODD flags (`0x2` no colour adjustment, `0x4` direct light from the MOLT light named by the colour alpha, `0x8` colour is the ambient, `0x10` add colour times MDDI, `0x40` no direct, `0x80` no colour in the sum), the MOLT light and the WMO ambient into interior ambient, horizon and ground colours, a direct colour, and a personal interior sun from the MOLT light or the first group's box centre toward the doodad's box centre. `fixDirectColor`/`fixAmbient1` clamp to 0x70/0x60 (their HSV value scaling scales every component alike). The WMO ambient is `calculateAmbient` (`:1923-1966`), now with MAVG/MAVD flag-1 horizon and ground colours; WMO batches use its first colour as before.
- **Shader.** `m2.gdshader` mixes `calcLight`'s interior light, `applyAndMixAmbients(interior colours) + interior_direct * N·L(interior sun)`, with the exterior light by `exterior_blend` (`commonLightFunctions.slang:103-197`). The interior part takes no sun shadow. ADT doodads keep the default blend 1.
- **Examples.** Stockade MODD 100 (flags `0x2`, colour (77, 78, 86)): direct = the MODD colour, ambient = MOHD (25, 25, 25). Stockade MODD 3 (flags `0x6`, MOLT 0): no direct colour, sun from MOLT 0. `sw_magicdistrict` Jail01 lamp MODD 32 (flags `0x10`, colour (179, 125, 95), MAVG (33, 33, 33)): direct (212, 158, 128), ambient that darkened to 0x60.

Proof: `cargo test -p game-engine-godot --lib doodad_light` 4/4 (the three examples, hand-derived from the file bytes, plus an exterior-group doodad at blend 1). `godot/tests/m2_material_pixels.gd` on Vulkan: at blend 0 under a real shadow caster the pixel is `0.4 * (0.2 * 1.1 + 0.5)` = 0.286 (expected 0.288), at blend 0.5 the half mix; the old shader gives the shadowed exterior 0.235. `godot/tests/wmo_doodads_flow.gd`: the spawned Stockade MODD 100 batches carry blend 0 and those colours.

Live (Fbfps at the indoor Stormwind spot, WoW (-8785.9, 820.7, 97.65); `data/diagnostics/wmodoodadcost-20260928/`): `master-15b3715c.png` vs `after-6cb884ec.png` differ in 20,234 pixels, all on WMO doodads (banners, lamps, crates and sacks, the weapon rack; `lighting-diff-mask.png`); WMO walls, floor, terrain and the player are unchanged. The props near the lamps turn warmer and the weapon rack darker (`lighting-compare.png`, left master, right branch). No Retail capture of this spot exists to compare against.

Not ported: the reference's per-group MOCV sampling for doodads (`wmoGroupObject.cpp` `assignInteriorParams`, commented out there), M2 fog's interior sun mix, and MNLD/MOLT point lights (MOLP: below).

## Godot: WMO group point lights (MOLP, branch `worldvis`)

Each group's MOLP lights (44-byte `map_object_point_light`: BGRA colour, position, attenuation start/end, intensity) in the MLSP (offset, count) range of each active doodad set (set 0 and the placement's) become `Group<g>_Light<i>` `OmniLight3D` children of the WMO node (`godot/rust/src/wmo/point_lights.rs`, core `wmo::active_point_lights`), encoded as the M2 point lights: colour x intensity, range = attenuation end, start / end in the specular parameter, retail's squared ramp in the opaque shaders (WebWowViewerCpp `wmoGroupObject.cpp:270-287`, `CPointLight.cpp`, `pointLight.frag.slang`). A group with MOLP but no MLSP (Stormwind harbor docks 248081) lights nothing, as in the reference. The Stockade's `stormwindjail_001` set 0 has 12 orange torches. Not ported: MOP2/MLSK animated lights and MNLD (`CEngineLight`), spot lights (MOLS), and placement scale on the range.

## WMO fog

The Godot client applies WMO MFOG fog inside interior groups (`03db2144`). See [[retail-lighting]], WMO fog.

## Still open

- WMO batches: MAVG/MAVD horizon and ground colors (flag 1) are not used (WMO doodads use them).
- Bevy: MOCV2 shaders 7, 8, 9, 15, 18 and 19 still use CPU texel-alpha compositing. In both clients, blend modes above 1 do not get the reference's 128/255 discard (`wmoObject.cpp:1875` enables it for every blend mode above 0).
- Godot: MOUV UV animation is not ported. Blend modes above 3 are rejected per batch.
- `reset_streamed_terrain` clears `shared_wmos` without despawning, like `tile_doodad_entities`.
- No live Metal before/after: no captured area uses MOMT 2 or 5.
- Portal culling hides many Trade District groups from outside. Not investigated.

## See Also

- [[stormwind-dark-render]]
- [[abbey-interior-black-world]]
- [[wmo-format]]
