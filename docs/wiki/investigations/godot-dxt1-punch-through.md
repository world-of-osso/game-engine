# Godot DXT1 punch-through alpha

In the Godot client, Northshire bushes drew their leaf cards as black squares from `0f36a6cb` (2026-09-27) until `fe422318`. The leaf texture was drawn, but its transparent texels came out opaque black. The cause was the texture upload format, not the M2 blend mode or the shader.

## Symptoms

- The model is `world/azeroth/elwynn/passivedoodads/bush/elwynnbush09.m2` (FDID 189700), placed about 150 times in `azeroth_32_48`, for example MDDF unique id 10333 at WoW (-8918.6, -150.3, 81.0). Its one material is flags 4, blend 1 (alpha key), and its texture is 189937 (`stumps/elwynnshrub09.blp`).
- Every quad of the bush showed black where the leaf art is transparent (`data/diagnostics/foliagealpha-2026-09-29/before/Doodad10333.png`).

## Root cause

- 189937 is a 128x128 BLP2 with DXT1 compression, alpha depth 1 and a full 8-level mip chain. 861 of its 1,024 mip-0 blocks are in the three-colour mode (colour 0 <= colour 1) and use index 3, the punch-through texel.
- `core::blp::decode_gpu` (`224eb4f8`) kept every DXT1 BLP block-compressed, and `material::shared_texture` (`0f36a6cb`) uploaded it as `Image::FORMAT_DXT1`.
- Godot 4.7.2 maps `FORMAT_DXT1` to `RD::DATA_FORMAT_BC1_RGB_UNORM_BLOCK` (`servers/rendering/renderer_rd/storage_rd/texture_storage.cpp`, `case Image::FORMAT_DXT1`). In BC1 RGB, index 3 of a three-colour block decodes to black with alpha 1. Godot has no BC1 RGBA image format.
- With alpha 1 the blend-1 alpha test (224/255) never discards, so the texel draws opaque black.
- WebWowViewerCpp selects `S3TC_RGBA_DXT1` for a DXT1 BLP with `alphaChannelBitDepth > 0` (`wowViewerLib/src/engine/texture/BlpTexture.cpp` `getTextureType`). Its Vulkan path uploads that as `VK_FORMAT_BC1_RGBA_UNORM_BLOCK` (`gapi/vulkan/textures/GBlpTextureVLK.cpp`), where index 3 has alpha 0. Before `0f36a6cb`, the Godot client decoded every BLP to RGBA8 with `image-blp`, which also gives alpha 0.

## Fix (`fe422318`)

- `decode_gpu` decodes a DXT1 BLP with alpha bits to RGBA8, with every authored mip level, using `texpresso` (the BC1 decoder that `image-blp` uses). Punch-through texels have alpha 0.
- DXT1 without alpha bits, DXT3 and DXT5 still upload block-compressed.
- Cost: 244 of the local DXT1 BLPs have alpha bits. Decoding all of them with mips takes about 58 MiB as RGBA8, compared with 7 MiB as DXT1.

## Tests

- `godot/tests/m2_dxt1_alpha_pixels.gd` loads a blend-1 M2 quad with a one-block DXT1 BLP (alpha depth 1) over a known background. The opaque texel must show its colour, and the punch-through texel must show the background. RED on the old build: the punch-through texel was `(0, 0, 0)` (`data/diagnostics/foliagealpha-2026-09-29/fixture-red.log`). GREEN: 2/2.
- `godot/core/src/blp_tests.rs` `a_one_bit_alpha_dxt1_blp_decodes_every_level_with_its_transparent_texels` checks 189937: RGBA8 with 8 levels, mip 0 equal to `decode_rgba`, and more than a quarter of the texels transparent. It has not been run yet, because agents build the Godot workspace only on Depot.
- Live Northshire captures from a private server are in `data/diagnostics/foliagealpha-2026-09-29/{before,after}/`.

## Verification on master (2026-09-29, `56a134a6`)

- No black cards remain in Northshire. Captures came from a private server: the Polymorph spot (-8966.63, -194, 80) from eight sides, eight-way sweeps from five points, and close-ups of the four bushes, including 10379 by the training dummies. An automated pass framed 294 of the 383 MDDF placements within 140 yards and counted near-black pixels in each screen box. The only flags were the burned vineyard ground behind the frame (`data/diagnostics/foliage2-2026-09-29/`).
- The Northshire trees and plants use DXT5 leaf textures (alpha depth 8, type 7), which upload as BC3 with alpha: elwynnpine01 127006, elwynntreemid01 198585, the 189927-189930 trees 464351, swampplant04/05 191275 and 190386. elwynnbush09 is the only DXT1 alpha texture there.
- Across all 25,481 local BLP2 files, no DXT1 without alpha bits (16,619 files) has a punch-through texel. So BC1 RGB against RGBA matters only for the 246 files with alpha bits.
- Black cards seen after `fe422318` come from builds without it. For example, the `godot-conversion` worktree at `e4d5f8a3` has a 02:46 extension build that predates the fix.

## Sources

- Godot 4.7.2-stable `servers/rendering/renderer_rd/storage_rd/texture_storage.cpp`
- `~/Repos/WebWowViewerCpp/wowViewerLib/src/engine/texture/BlpTexture.cpp`, `gapi/vulkan/textures/GBlpTextureVLK.cpp`

## See Also

- [[godot-texture-vram]] — the change that started uploading DXT BLPs block-compressed
- [[blp-format]] — DXT1 alpha depth
