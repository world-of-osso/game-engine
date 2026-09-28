# Godot client texture VRAM

The Godot client's video memory grew without bound during an in-world login: every M2 placement built its own materials and uploaded its own uncompressed RGBA8 copy of every texture, and each WMO build kept a private texture map. Sharing one GPU texture per texture file and uploading DXT BLPs block-compressed with their mip chain cut the client's own VRAM from 4,994 MiB (still climbing) to 1,029 MiB (settled).

## Symptoms

- On an 8 GB VRAM carve-out (AMD Radeon 890M), game clients exhausted VRAM; amdgpu logged `Not enough memory for command submission` and the desktop compositor aborted (2026-09-27).
- Godot processes of the conversion sessions held 1.8–4.4 GiB each.

## Root cause

- `assets/material.rs` `bind_textures` created a new `ImageTexture` from decoded pixels for every material. `terrain/objects.rs` `build_doodad_model` parses each model FDID once but calls `build_model` per placement, so every lamp, barrel and torch uploaded its textures again.
- `wmo/scene.rs` cached textures only inside one `build_wmo_node` call, so repeated WMOs (Stormwind places WMO 106965 several times) uploaded their own copies.
- All uploads were `Image::Format::RGBA8` without mipmaps, 4–8x the authored DXT size.

## Fix (`224eb4f8`, `0f36a6cb`)

- `core::blp::decode_gpu` returns DXT1/3/5 BLPs as authored block data with the full mip chain (mip 0 alone when the chain is incomplete; truncated mip 0 takes the largest level the data holds). DXT3/5 alpha that is zero everywhere is made opaque, matching `decode_rgba`'s `fix_1bit_alpha`. JPEG and palettized BLPs decode to RGBA8.
- `material::shared_texture` keeps one `ImageTexture` per (texture dir, FDID) in a thread-local cache, cleared with the shared shaders at `InitStage::MainLoop` deinit. M2 batches, WMO materials and the character-select sky use it.
- Batch textures composited on the CPU (second texture unless it is an environment map, overlays) are cached per composite key; a composite that was missing a layer is not cached, so every model that uses it still reports the missing FDID.
- WMO shader-7 composites are still cached per WMO build; terrain layer textures are cached per FDID but still RGBA8.

## Measurement

`/proc/<pid>/fdinfo` `drm-memory-vram` of the Godot process, sampled every 0.2 s for 60 s after an in-world login on :5000 (the account's default character), same position both runs:

| Build | Own VRAM |
|---|---|
| `224eb4f8` (before) | 4,994 MiB at t≈45 s, rising ~0.5 GB per 5 s; stopped by a 6 GiB total watchdog |
| `0f36a6cb` (after) | settles at 1,029 MiB from t≈40 s |

## Tests

- `godot/core/src/blp_tests.rs`: DXT1 464811 keeps its 9-level chain; zero-alpha DXT5 464043 uploads opaque; torch 198077 keeps authored DXT5 alpha; palettized 1022933 decodes to RGBA8.
- `godot/tests/m2_texture_sharing.gd` (headless): models 1016191 (twice) and 1016200 share texture FDID 1006709 as one DXT1 `ImageTexture` with mipmaps. Fails on `224eb4f8` ("placements bind separate copies").
- Rendered fixtures m2_loader_pixels, m2_material_pixels, m2_skin_pixels, m2_uv_pixels, sky_m2_pixels, wmo_material_pixels and m2_assets pass.

## See Also

- [[bevy-godot-shadow-comparison]] — other Bevy/Godot resource comparisons
