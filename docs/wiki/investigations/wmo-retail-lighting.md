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

## Still open

- The scene light is still Bevy PBR, not the Retail ambient/horizon/ground mix. It moves to the shared Retail scene light through `scene_daylight()`.
- MAVG/MAVD horizon and ground colors (flag 1) are not used.
- MOCV2 shaders 7, 8, 9, 15, 18 and 19 still use CPU texel-alpha compositing. Blend modes above 1 do not get the 128/255 discard.
- `reset_streamed_terrain` clears `shared_wmos` without despawning, like `tile_doodad_entities`.
- No live Metal before/after: no captured area uses MOMT 2 or 5.
- Portal culling hides many Trade District groups from outside. Not investigated.

## See Also

- [[stormwind-dark-render]]
- [[abbey-interior-black-world]]
- [[wmo-format]]
