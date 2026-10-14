# BLP Format

BLP is Blizzard's proprietary texture format used for essentially all WoW art assets — UI textures, model skins, terrain layers, skyboxes, and more. The Godot client decodes BLPs through `game-engine-core` and uploads RGBA8 or authored DXT blocks.

## Compression

BLP files support several internal encodings:

| Type | Description |
|------|-------------|
| DXT1 | BC1 block compression, no alpha or 1-bit alpha. Used for opaque textures (terrain base layers, many model skins). A header alpha depth above 0 means the three-colour blocks' index 3 is transparent (BC1 RGBA); Godot can only upload BC1 RGB, see [[godot-dxt1-punch-through]]. |
| DXT5 | BC3 block compression, full alpha channel. Used for textures with transparency (UI elements, hair, eye overlays). |
| Uncompressed | Raw BGRA or palettized. Less common in modern assets. |

The eye reflection texture (FDID 5210142) is DXT5 at 128×128 with a small alpha coverage region (~3.9% of pixels) for the specular highlight.

## Loading Pipeline

BLP loading is handled by `godot/core/src/blp.rs` via the `image-blp` crate:

```
FDID → receipted local CASC bytes → core decode → Godot texture (RGBA8 / compressed)
```

Additional compositing helpers (`scale_2x`, `blit_region`) are implemented in `blp.rs` for building character texture atlases — blitting face, hair, and underwear overlays onto the body skin base texture.

## Palettized BGRA correction (2026-10-14)

`image-blp`1.2.0 `raw1_to_image` assigns a palette word's low byte to red, but BLP palettes store BGRA. Core normalizes `Raw1.cmap` before conversion; separately stored alpha stays unchanged. DXT paths are unaffected. Retire normalization when the dependency decodes BGRA correctly.

Authenticated70391 tabard background180071 at(64,10): palette `[0,9,129,0]` plus alpha255 must decode RGBA `[129,9,0,255]`, not `[0,9,129,255]`. Native [guild-tabard](../../specs/guild-tabards.md) pixels exposed the blue-for-red error; `guild_tabard_palette_keeps_authored_red_and_separate_alpha` covers CPU and upload decoding.

## Character Texture Compositing

Body skin textures are assembled at runtime from multiple BLP sources:
- Base body skin (type-1, 1024×512 for HD)
- Underwear overlay blitted at a fixed region offset
- Face texture (512×512) blitted into the `FACE_UPPER` region

Hair/face type-6 atlas is a separate 512×512 composite of the face skin and scalp hair BLPs.

## UI Textures

UI textures are referenced by virtual path (e.g. `Interface/Buttons/...`) and resolved through `ManifestInterfaceData.db2` to FDIDs. BLP decode applies equally — the `image-blp` crate handles both DXT1 and DXT5 paths. Some UI BLPs decode to near-black pixels that make color tinting ineffective (see the editbox focus investigation).

## Sources

- AGENTS.md (`asset/blp.rs` entry) — module description, compositing helpers
- [docs/hd-skeleton-status.md](../hd-skeleton-status.md) — `blp.rs` changes: `scale_2x`, `blit_region`, atlas compositing
- [docs/editbox-focus-texture-swap-2026-04-06.md](../editbox-focus-texture-swap-2026-04-06.md) — near-black BLP decode behavior for UI textures
- [docs/torch-halo-investigation-2026-03-30.md](../torch-halo-investigation-2026-03-30.md) — BLP used with blend mode 2 (DXT5 alpha for additive FX)

## See Also

- [[casc-format]] — BLP files extracted by FDID from CASC archives
- [[m2-format]] — BLP textures referenced by M2 TXID chunk and texture type system
- [[godot-dxt1-punch-through]] — 1-bit-alpha DXT1 drew opaque black in the Godot client
- [[db2-format]] — DB2 chains that resolve character customization BLP FDIDs
