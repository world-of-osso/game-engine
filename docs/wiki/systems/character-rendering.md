# Character Rendering

Character rendering assembles WoW M2 character models with dynamic geoset visibility and composited textures. The pipeline resolves which submeshes are shown (driven by customization choices and equipment), composites body/face/hair textures into a single atlas, and handles HD skeleton loading for modern character models.

## Character Models and HD Skeletons

Legacy models (`humanmale.m2`) store 215 bones inline in the MD20 header. HD models (`humanmale_hd.m2`) store bones externally in a `.skel` file (referenced via the SKID chunk). The `.skel` file contains SKS1 (sequences + global sequences) and SKB1 (216 bones + animation tracks). `load_skel_data()` handles both paths transparently.

**HD skin index overflow**: the skin file's `indexStart` field is u16 but HD models have 147K+ triangle indices. From submesh 47 onward, `indexStart` wraps at 65536. Fix: `M2Submesh.triangle_start` is computed as a cumulative sum of previous `indexCount` values (u32), not the raw u16 field.

**Bone remapping**: vertex bone indices are global skeleton indices. The skin file's bone lookup table maps local (per-submesh) indices to global indices and must be applied via `remap_bone_indices()`.

## Geoset System

`mesh_part_id = (group * 100) + variant`. Variant 0 = hidden, variant 1+ = visible. Key groups:

| Group | Name | Notes |
|-------|------|-------|
| 0 | Hair/base skin | Mixed; variant 5 = hairstyle mesh |
| 4 | Gloves | Equipment |
| 5 | Boots | Equipment |
| 7 | Ears | 701=hidden, 702=visible |
| 17 | Eye glow | DK only, off by default |
| 21 | Head visibility | Driven by `ItemDisplayInfo.GeosetGroup_1` |
| 27 | Helmet variant | Driven by `ItemDisplayInfo.GeosetGroup_0`; default=2701 |
| 32 | Face (DF+) | Type-1 (body skin) texture |
| 33 | Eyes | |
| 34 | Eyebrows | |

Texture types: 0 = hardcoded TXID, 1 = body skin, 6 = face/hair atlas.

## Texture Compositing

Body skin texture (HD: 1024×512) is composited in `src/asset/char_texture.rs` (`seed_default_body_texture()`). Layers:
- Body base (type 1)
- Underwear overlay at `(256, 192)`
- Face upper + lower textures composited into the FACE_UPPER/FACE_LOWER regions

A second injection path in `m2_texture.rs` was removed — the compositor in `char_texture.rs` is now the single authoritative path.

HD face texture (FDID 1027494, 512×512): face skin + inner mouth + eyeball atlas. Scalp hair overlay (FDID 1043094) composited on top. These form the type-6 replaceable texture assigned to hair geosets.

## Helmet Geoset Hiding

Two overlapping mechanisms:

1. **HelmetGeosetVis / HelmetGeosetData**: DB2-driven per-race hide rules. For matching race rows, resets the specified character geoset group to default variant 1.
2. **ItemDisplayInfo head slot**: `GeosetGroup_0` → character group 27; `GeosetGroup_1` → character group 21. Group 27 special defaults: no helm → 2701; helm with `GeosetGroup_0 == 0` → 2702; otherwise → `2700 + GeosetGroup_0`.

Some helm models appear with `HelmetGeosetVis = 0,0` (e.g. display 1128), meaning `HelmetGeosetData` doesn't apply — only the GeosetGroup_0/1 path. Scalp hair suppression may require an additional rule path; `CharHairGeosets.Showscalp` is a candidate. See [helmet-hair-hiding-investigation-2026-03-28.md](../helmet-hair-hiding-investigation-2026-03-28.md).

`HelmetGeosetData.Field_10_0_0_46047_003` shows a binary `32`/`-1` pattern across hide groups but its meaning is unknown. See [helmet-geoset-extra-field-investigation-2026-03-28.md](../helmet-geoset-extra-field-investigation-2026-03-28.md).

## Target Circles

WoW renders selection circles procedurally (ground-projected ring tinted by unit reaction). The engine supports both procedural and BLP-textured styles. DXT1 textures (no alpha) use additive blending; DXT5 textures (real alpha) use alpha blend. Auto-detected via `is_fully_opaque()` after BLP decode. See [target-circle-styles-2026-03-30.md](../target-circle-styles-2026-03-30.md).

## Known Issues

- **HD face dark/transparent**: face geoset (mpid=5) contains both outer face skin and inner mouth cavity. With jaw in bind pose (wide open), dark inner-mouth triangles face the camera and z-occlude face skin. The jaw-close hack (-45° X rotation on key_bone_id=7) partially helps but is insufficient. Real jaw-close data may be in external `.anim` files (AFID chunk).
- **Eye rendering**: eye reflection geoset (5101) uses a specular highlight texture (pure white DXT5) — should be `emissive_texture` with AlphaBlend, not `base_color_texture` with Opaque.
- **Geoset defaults**: groups 7-12 variant 2 means bare skin on HD but equipment on legacy — the default geoset logic needs model-type awareness.

## Sources

- [character-generation.md](../character-generation.md) — original character pipeline, template skeletons, race scaling
- [hd-skeleton-status.md](../hd-skeleton-status.md) — HD model status, known issues, texture FDIDs
- [geosets.md](../geosets.md) — geoset groups, texture types, ItemDisplayInfo slot mappings
- [character-texture-debugging-2026-03-27.md](../character-texture-debugging-2026-03-27.md) — duplicate injection path cleanup
- [helmet-geoset-extra-field-investigation-2026-03-28.md](../helmet-geoset-extra-field-investigation-2026-03-28.md) — extra DB2 field observation
- [helmet-hair-hiding-investigation-2026-03-28.md](../helmet-hair-hiding-investigation-2026-03-28.md) — helmet hair hiding mechanisms
- [target-circle-styles-2026-03-30.md](../target-circle-styles-2026-03-30.md) — selection circle styles, BLP blend mode detection

## See Also

- [[rendering-pipeline]] — M2 material assembly, blend modes
- [[animation]] — bone animation, HD skeleton loading
- [[asset-pipeline]] — CASC extraction for BLP textures, DB2 chains
