# WMO Retail Lighting

This work follows up [[stormwind-dark-render]] and brings WMO lighting, alpha testing, two-layer shaders and placement in line with Retail. The reference is WebWowViewerCpp (Deamon87), whose shaders reproduce Retail's MapObj shaders. solarityclient, the first reference, replays the 3.3.5 client. Our assets are Retail 12.x, so 3.3.5 semantics (the multiplicative ordinary path and the 224/255 AlphaKey) were the wrong target and were replaced.

## Evidence

WebWowViewerCpp sources, read via `gh api` (`wowViewerLib/shaders/slang/...`, `wowViewerLib/src/engine/...`):

- **`commonLightFunctions.slang` `calcLight` / `applyAndMixAmbients`.**
  - The light is `matDiffuse * (ambient + precomputedLight + sun * NdotL)`.
  - The interior and exterior parts are mixed by `interiorExteriorBlend`.
- **`wmoshader_text.slang`.**
  - `precomputedLight = vColor.rgb * 2.0`. This is the ×2 on MOCV, which settles the ×2 question in favour of keeping it.
  - The blend is `mix(vColor.w, 1, isExteriorLit)`.
- **`wmoGroupGeom.cpp` `fixColorVertexAlpha`.**
  - `flag_lighten_interiors` (MOHD `0x08`) only sets the alpha.
  - `flag_skip_base_color` (MOHD `0x02`) skips the ambient subtraction.
  - Transition vertices get `(c - amb) * (1 - a) / 2`. Later vertices get `(c * a / 64 + c - amb) / 2`, with alpha `EXTERIOR | EXTERIOR_LIT ? 255 : 0`.
  - Without MOCV the color is 0 and the alpha 0. Without MOCV2 the value is `(0, 0, 0, 255)`.
- **`wmoGroupObject.h` `isInteriorLightingLit`.** A group is exterior-lit if it has `EXTERIOR` or `EXTERIOR_LIT`, or lacks `INTERIOR`.
- **`wmoObject.cpp` `calculateAmbient`.** The interior ambient is the first MAVG for an active doodad set, else the first MAVD, else the MOHD ambient.
- **`commonWMOMaterial.slang` `caclWMOFragMat` and `iWmoApi.h` `wmoMaterialShader`.**
  - MOMT 13 maps to TwoLayerDiffuseOpaque: `mix(tex2, tex, vColor2.a)`.
  - MOMT 6 is `mix(mix(tex, tex2, tex2.a), tex, vColor2.a)`.
  - MOMT 21 maps to MapObjLod: `tex`.
  - Metal/EnvMetal keep `matDiffuse = tex.rgb`.
  - With a blend mode above 0, texels with `tex.a < 0.50196` (128/255) are discarded.

## Changes (branch `wmo-parity`)

| Commit | Change |
|---|---|
| `54539726` | Retail light model and fixup, in `WmoLitMaterial`. Every MOCV batch, and every batch in an interior-lit group, uses it. Replaces `848493ad`, the 3.3.5 multiplicative ordinary path. |
| `f526cf4e`, `9f2ed3f6` | Blend 0 has no alpha test. AlphaKey tests at 128/255: `9f2ed3f6` replaces the 224/255 from 3.3.5. |
| `0b533bb0`, `0c44a2b5` | MOMT 6/13 blend in the shader by MOCV2 alpha. A custom vertex stage carries MOCV2 at location 8. Prepass pipelines keep Bevy's vertex stage. |
| `2b06d354` | Metal/EnvMetal are no longer PBR-metallic 0.85. |
| `a8a64f75` | The interior/exterior crossfade is blended in gamma space. |
| `c7e2e0a2` | A MODF uniqueId placed by several tiles is spawned once (`AdtManager.shared_wmos`). |

## Proof

**GPU tests** (`src/rendering/terrain/terrain_objects_wmo_tests/`):

- `interior_light_gpu`: the Abbey interior and exterior batches match a `texture * (ambient + 2*MOCV)` reference, `[29,11,11]` vs `[30,11,11]` and `[85,151,179]` vs `[85,151,179]`.
- `alpha_gpu`: an opaque texel at alpha 60 is kept. AlphaKey discards alpha 100 and keeps 160. RED under 224/255: the alpha-160 texel was discarded.
- `two_layer_gpu`: MOMT 13 gives blue at MOCV2 alpha 0 and red at 1. MOMT 6 gives `[127,0,128]`. RED: the second layer was ignored and the result stayed red. The test also passes with a prepass camera. Before `0c44a2b5` the prepass pipeline failed validation (location 7).
- `metal_gpu`: RED `[72,71,72]` vs diffuse `[127,126,127]`. GREEN `[128,127,128]`.
- `crossfade_gpu`: interior 11, half 79, exterior 147. RED 107 with a linear-space mix.

**Unit tests:** the lib `mocv_fixup_*` tests, 4 RED against the old fixup; `load_wmo_group_with_root_adds_blend_alpha_for_two_layer_shaders`; `terrain_shared_wmos` lifecycle.

**Live, headless.** Files are in `data/diagnostics/wmo-parity-20260925/`.

- Abbey interior: `1-before-abbey-interior.webp` is dark blue; `R-after-abbey-interior.webp` is lit with warm baked light.
- Northshire steps: unchanged.
- Trade District, alpha test: roofs return (`2-before/after-trade.webp`).
- Trade District, placement: `7-before/after-trade-tree.txt`. The `sw_*` district roots drop from 4 instances to 1, and WMO roots from 89 to 62.
- No wgpu validation errors after `0c44a2b5`.

## Still open

- The exterior daylight is Bevy PBR (sun plus ambient), not Retail's horizon/ground ambient mix. The additive MOCV term is linearized on its own.
- MAVG/MAVD horizon and ground colors (flag 1) are not used.
- Other MOCV2 shaders (7, 8, 9, 15, 18, 19) keep the CPU texel-alpha compositing. Blend modes above 1 do not use the 128/255 discard.
- `reset_streamed_terrain` clears `shared_wmos` (like `tile_doodad_entities`) without despawning them.
- No live before/after for the Metal shaders: no WMO in the captured areas uses MOMT 2 or 5.
- Many Trade District groups are `hidden` in `dump-tree` from outside. That is portal culling, and was not investigated.

## Sources

- WebWowViewerCpp, github.com/Deamon87/WebWowViewerCpp, files listed above.
- `~/Repos/solarityclient` (3.3.5 replay, superseded here).

## See Also

- [[stormwind-dark-render]]
- [[abbey-interior-black-world]]
- [[wmo-format]]
