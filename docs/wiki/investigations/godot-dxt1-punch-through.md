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

## Sources

- Godot 4.7.2-stable `servers/rendering/renderer_rd/storage_rd/texture_storage.cpp`
- `~/Repos/WebWowViewerCpp/wowViewerLib/src/engine/texture/BlpTexture.cpp`, `gapi/vulkan/textures/GBlpTextureVLK.cpp`

## See Also

- [[godot-texture-vram]] — the change that started uploading DXT BLPs block-compressed
- [[blp-format]] — DXT1 alpha depth
