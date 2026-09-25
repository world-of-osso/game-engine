# Terrain Blend Steps

Terrain texture transitions in Stormwind and Elwynn had hard, stair-stepped edges; the user compared them with solarityclient's smooth ones. There were two visible causes. First, the per-chunk MCAL alpha texture was uploaded as sRGB, so the GPU gamma-decoded the blend weights. Second, every chunk used height blending with the diffuse texture's alpha as the height. MCAL decoding also chose the storage format from the number of remaining bytes instead of the map's WDT flags. Fixed on branch `terrainblend` (2026-09-25).

## Root Causes

1. **sRGB alpha map.** `pack_alpha_map_raw` created the 64×64 alpha texture as `Rgba8UnormSrgb`. The shader therefore received `srgb_to_linear(byte / 255)`: 64 became 0.05, 128 became 0.22 and 192 became 0.53. This compressed every authored ramp into a narrow band near its top end. Blend weights are linear data, so the texture is now `Rgba8Unorm`, matching the MCSH shadow map and solarityclient's linear RGBA8 alpha atlas.
2. **Height blend with a specular mask.** `terrain.wgsl` multiplied each layer by `exp2((height - 0.5) * 3)`. When MHID had no `_h` texture, the height came from the diffuse texture's alpha. In these `_s` tilesets that alpha is a specular mask. Missing MTXP entries also defaulted to scale 1 / offset 0, so the mask acted as full-strength height. Every Stormwind/Elwynn tile has MHID all zero and no MTXP chunk (`azeroth_30_48`, `31_48` and `32_48` `_tex0`).
3. **MCAL format guessed.** Uncompressed layers were read as 8-bit whenever at least 4096 bytes remained after the layer's offset. On 4-bit maps, that misread every layer except the last. A layer was not bounded by the next layer's offset. The 63→64 edge fix ran only on the 4-bit path.

## Retail Rules Implemented

- **WDT MPHD** (wowdev WDT): `0x4 adt_has_big_alpha // shader = 2` and `0x80 adt_has_height_texturing // shader = 6`. 0x80 "controls height-based texture influence via _h+MTXP, also changing MCAL size to 4096". Azeroth (`775971.wdt`) and map 2703 both have `0x3ca`: 0x80 is set and 0x4 is not. A reader that checks only 0x4 (solarityclient's WotLK rule) would decode Retail Azeroth as 4-bit. `MphdFlags::big_alpha()` is `0x4 | 0x80`.
- **MCAL**: RLE when MCLY 0x200 is set, otherwise 4096 bytes with big alpha, otherwise 2048 4-bit bytes. A layer's bytes end at the next alpha-mapped layer's offset (solarityclient `alpha_map.rs`). Unless MCNK 0x8000 is set, the last row and column copy the previous ones on every format. Every Stormwind/Elwynn chunk has 0x8000 set, so this edge fix is correctness-only for those tiles.
- **Blend modes** (`TerrainBlendMode`, `config.y`):
  - Layered (4-bit maps): `mix` chain.
  - Weighted (big alpha): `base = 1 - saturate(a1+a2+a3)`, layer N = aN. This matches solarityclient `terrain.frag.glsl` `weighted_blending`, which was verified against the native D3D9 client.
  - Height-weighted (MPHD 0x80), from the wowdev ADT/v18 MTXP shader: `pct = weights * (h_tex.a * heightScale + heightOffset)`, `pct *= 1 - saturate(max(pct) - pct)`, `pct /= sum(pct)`.
- **MTXP defaults**: heightScale 0 and heightOffset 1 ("it will not load a _h texture"). A layer without an MHID `_h` texture gets scale 0, so its height is its offset and the diffuse alpha never contributes.

## UV Inset (not adopted)

solarityclient samples alpha at texel centres (`u * 63/64 + 0.5/64`). This is a WotLK "64 samples, shared edge" convention, and it also prevents bleeding between neighbouring chunks in its atlas. Our alpha maps are separate 64×64 per-chunk textures with ClampToEdge, so they cannot bleed. The Retail data does not support the shared-edge convention. For the same texture across horizontal chunk seams, `A[63] == B[0]` held on 45% of varying rows in `azeroth_32_48` and 69% in `azeroth_31_48`. The mean seam difference (27.9 and 13.6) was close to the interior step (24.6 and 24.8). That is what texel-coverage data looks like, not duplicated edge samples. The 0..1 per-chunk UV stays.

## Evidence

Headless client (`terrain_ui`, `Terrainblend`), same position and camera angles before (master `4221297c`) and after (`terrainblend`). Files are in `data/diagnostics/terrainblend-20260925/`:
- `compare-northshire-path-crop.png` at (-8914, -135, 80.5), yaw 90, pitch -45. Before, grass showed through the cobble path. After, the path is opaque and the grass-to-dirt fringe is a wide, smooth ramp.
- `compare-trade-district-{90,180}-crop.png` at (-8818, 660, 94.7), pitch -55. The duskwood dirt runs between cobble patches are wider and softer.
- `*-tex0-log.txt` shows `MPHD 0x3ca` loaded for every tile.

The user's exact Stormwind spot was not recovered. At these capture distances the "after" shots do not show the stair-stepping, but a close-up at the user's spot has not been checked.

## Tests

- `asset::wdt::tests::*`: MPHD parse and flag semantics.
- `adt_tex::tests::small_alpha_map_decodes_first_of_two_uncompressed_layers_as_4_bit`, `height_textured_map_decodes_uncompressed_layers_as_8_bit`, `compressed_layer_cannot_read_past_the_next_layers_offset`, `edge_fix_applies_to_big_and_compressed_alpha_maps`.
- `terrain_material::tests::alpha_map_texture_hands_the_shader_authored_mcal_weights_unchanged`: `get_color_at().to_linear()` equals byte/255, and the format is `Rgba8Unorm`.
- `terrain_material::tests::chunk_without_height_textures_has_no_height_influence`, `layer_with_height_texture_keeps_mtxp_scale_and_one_without_keeps_only_offset`, `map_flags_select_the_terrain_blend_mode`.

The shader formulas themselves have no unit test. The GPU shared-clock test compiles `terrain.wgsl` and renders the layered path.

## Sources

- wowdev.wiki ADT/v18 (MCAL, MTXP heightblend shader) and WDT (MPHD flags)
- `~/Repos/solarityclient/crates/asset/src/terrain/alpha_map.rs`, `crates/rendering/src/shader/terrain_spirv/source/terrain.frag.glsl`, `crates/rendering/src/terrain/mesh.rs`

## See Also

- [[terrain]]: tile loading and the `_tex0` companion
- [[adt-format]]: MCLY/MCAL layout
