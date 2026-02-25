# HD Model Loading — Status Brief

## Current State

### Legacy humanmale.m2 — WORKING
- 215 inline bones, skinning with identity inverse bind poses, renders correctly
- Animation from inline MD20 bone tracks (Stand idle, etc.)
- Geoset visibility restored to pre-regression state
- Body skin (512x512) with underwear + scalp overlays composited

### HD humanmale_hd.m2 — PARTIAL
- 216 bones loaded from external `.skel` file (SKB1 chunk) ✓
- Animation sequences (422) from SKS1, bone tracks from SKB1 ✓
- Full body renders with skin texture ✓
- Body texture (1024x512) with underwear overlay ✓
- Face texture (1027494) composited as type-6 replaceable hair texture ✓
- Scalp hair overlay (1043094) composited on top of face texture ✓
- DK eye glow (group 17) hidden by default ✓
- Eye reflection geoset (5101, FDID 5210142) renders correctly ✓ — pure white 128x128 specular map, gray under lighting is expected
- Skin bone_lookup remap applied ✓ — vertex bone_indices now correctly mapped through skin's bone table
- M2 render flags parsed ✓ — two-sided (0x04), unlit (0x01), blend modes applied per batch
- Jaw close rotation hack (-45° X) applied to jaw bone ✓ — partially closes mouth
- **Face still appears dark/transparent** — face geosets render but face skin not visible (see Known Issue #1)
- **Jaw partially closed** — -45° X helps but not enough, face skin still occluded

## What Was Done

### External skeleton loading (committed: 39f8412)
- HD M2 models store bones externally in `.skel` files instead of inline MD20
- M2 top-level `SKID` chunk contains a u32 FileDataID pointing to the `.skel` file
- `.skel` is chunked binary (same tag+size format as M2)
- `load_m2()` now: if SKID present → `load_skel_data()` from `.skel`, else → inline MD20
- Refactored `parse_bones()` into reusable `parse_bones_at(data, offset, count)` for both paths

### Full .skel data loading (uncommitted)
- `load_skel_data()` returns `SkelData` struct with bones, sequences, bone_tracks, global_sequences
- SKS1 chunk: global sequences + animation sequences (422 for humanmale_hd)
- SKB1 chunk: bones (216) + animation tracks (translation/rotation/scale per bone per sequence)
- New parsers: `parse_sequences_at()`, `parse_global_sequences_at()`, `parse_bone_animations_at()`

### Skin index overflow fix (uncommitted)
- **Root cause of HD model explosion**: skin file `indexStart` field is u16 but HD model has 147,966 total triangle indices
- From submesh 47 onward (of 113 total), `indexStart` wraps at 65536 → triangles reference wrong vertices → scattered body parts
- **Fix**: `M2Submesh.triangle_start` changed to u32, computed as cumulative sum of previous `indexCount` values instead of reading the overflowing u16 field
- Legacy models unaffected (total indices fit in u16)

### HD body texture compositing (uncommitted)
- HD body skin texture (FDID 1027767, 1024x512) resolved via `default_fdid_for_type(1, true)`
- HD underwear overlay (FDID 1027743, 256x128) composited at (256, 192) — same position as SD
- Face texture (FDID 1027494, 512x512) composited onto body atlas at (512, 0) — FACE_UPPER region
- `OverlayScale` enum replaces integer scale: `None` | `Uniform2x`
- `body_skin_overlays()` made HD-aware via `is_hd` parameter

### HD face/hair texture compositing (uncommitted)
- **Type-6 (CHARACTER_HAIR) replaceable texture** built from DB2-traced layers:
  - Base: face texture (FDID 1027494, 512x512) — face skin + inner mouth + eyeball atlas
  - Overlay: scalp hair (FDID 1043094, 512x512) — mostly transparent, hair detail at top
- All type-6 geosets share this composited 512x512 atlas
- DB2 chain traced: ChrCustomizationOption → Choice → Element → Material → TextureFileData
  - Human Male HD = ChrModelID=1
  - Face (Option 10, Choice 20): 16 skin-color variants, FDIDs 1027494–1027506 + 3 newer
  - Hair Style "Peasant" (Option 11, Choice 45): 13 hair-color variants, FDIDs 1043094–1043106
  - Bald (Choice 44): no texture, only geoset flag (GeosetID=2410)

### Geoset visibility fixes (uncommitted)
- DK eye glow (group 17) excluded from default visibility — was showing blue glow via FDID 3537040
- Eye reflection geoset (group 51, mpid 5101) correctly renders with FDID 5210142
- Eye texture (FDID 4531024) downloaded for future use (mpid 1702–1705, not visible by default)

### Geoset regression fix (committed: 39f8412)
- Commit 85b7565 added a rule showing variant 2 for groups 7-12 (intended for HD bare skin)
- On legacy models, variant 2 for those groups is equipment geometry (shirt sleeves, leggings, tabard)
- Reverted to the working `2d39d4d` form: only `702` (ears) as special case

### Model orientation fix (uncommitted)
- Model rotated `-PI/2` around Y to face the camera at default yaw

### Skin bone remap (committed: f0ef7fe)
- Skin file has bone_lookup table at offset 20 mapping local vertex bone_indices → global skeleton bone indices
- Without remap, vertices bind to wrong bones (e.g. chin at local 135/136 instead of global 88)
- Added `bone_lookup: Vec<u16>` to SkinData, `remap_bone_indices()` helper
- Applied in `collect_submesh_vertices`

### M2 render flags (committed: 1c9cf49)
- Parsed M2Material table (flags u16 + blend_mode u16) from MD20 offset 0x70
- 10 material entries for humanmale_hd: includes two-sided (0x04), unlit (0x01), blend modes
- Face batch uses render_flags_index=4: `flags=0x0004` (two-sided), `blend=0` (opaque)
- Applied per-batch: cull_mode (two-sided), unlit, alpha_mode (opaque/mask/blend/add)

### Jaw close hack (uncommitted)
- -45° X rotation applied to jaw bone (key_bone_id=7) in animation system
- Partially closes mouth but face skin still not dominant

## Known Issues

### 1. Face dark/not visible — inner mouth occlusion (HIGH)

The face geosets (mpid=5, type-6 texture) DO render and ARE textured correctly. The face texture (1027494) has 100% alpha=255 — it is fully opaque. The composited face+hair texture is correct (warm skin tones verified via runtime pixel dump).

**Root cause:** The face geoset (mpid=5, 792 vertices) contains BOTH outer face skin AND inner mouth cavity geometry in the same mesh. 462 vertices (58%) map to dark inner-mouth texture regions (luma ≤ 80), 330 (42%) map to bright skin. With the jaw wide open in bind pose, the dark inner-mouth triangles face the camera and z-occlude the bright face skin triangles.

**What was tried:**
- Unlit materials → still dark (rules out lighting/normals)
- Hiding body mesh → still dark (rules out body occlusion)
- Solid red color on face → renders correctly (geometry is fine)
- Runtime texture pixel dump → RGBA(206,154,115,255) warm skin (texture is fine)
- Face texture alpha check → 100% alpha=255, zero transparent pixels
- Jaw close -45° X rotation hack → partially helps, jaw visibly closing but face skin still not dominant

**Jaw investigation:**
- Jaw = key_bone_id=7, bone index 88, parent 39 (head)
- Stand idle: 49 jaw keyframes, ALL pure Z-yaw 2-3° breathing wiggle. No pitch (no open/close).
- Bone remap now applied (committed) — vertex bone_indices correctly remapped through skin's bone table
- -45° X rotation hack applied but insufficient

**Remaining theories:**
1. The -45° jaw close isn't enough — may need a larger angle or different axis
2. AFID chunk lists external `.anim` files — the real jaw-close data may live there
3. The face mesh may need to be split: outer face vs inner mouth rendered separately
4. Body mesh head area may have a hole that the face geosets fill — if face geosets don't cover it, background shows through

### 2. Eye geosets — specular overlay, not iris (MEDIUM)

Eye reflection geoset (mpid 5101, group 51) uses FDID 5210142: a **128x128 pure-white DXT5 texture** with 3.9% alpha coverage (small specular highlight spot). This is not the iris — it's an additive specular overlay.

**Current state:** Renders as opaque white because it's assigned as `base_color_texture` with default `Opaque` alpha mode. Should be `emissive_texture` with `AlphaBlend`.

**Proper eye rendering needs two layers:**
1. Base: iris color texture (runtime-resolved character customization, not available yet)
2. Overlay: 5210142 as emissive/additive with alpha blend for specular highlight

Eye iris texture FDID 4531024 (342KB) already downloaded but not wired up (mpid 1702-1705, not visible by default).

### 3. HD geoset defaults need model-aware logic (LOW)

Groups 7-12 variant 2 is bare skin on HD but equipment on legacy. A single static `default_geoset_visible()` can't handle both. Future: detect model type or check which variants exist.

## Type-6 Geoset Atlas Layout (humanmale_hd, 512x512)

| Geoset | mesh_part_id | UV range u | UV range v | Vertices | Description |
|--------|-------------|-----------|-----------|----------|-------------|
| Face | 5 | 0.005–0.499 | 0.005–0.984 | 792 | Left half — face skin + inner mouth |
| Eyes | 1 | 0.587–0.848 | 0.016–0.750 | 192 | Right center — eyelids, ear, temple |
| Facial hair | 102 | 0.872–0.993 | 0.451–0.703 | 195 | Far right — hair attachment |
| Facial hair | 202 | 0.851–0.991 | 0.296–0.497 | 318 | Right — beard area |
| Facial hair | 302 | 0.857–0.991 | 0.451–0.531 | 156 | Right — mustache area |
| Eyebrows | 3401 | 0.859–0.969 | 0.705–0.976 | 70 | Right — eyebrow area |

## DB2 Texture Chain (Human Male HD, ChrModelID=1)

| Option | TargetID | Description | Default FDID | Variants |
|--------|----------|-------------|-------------|----------|
| Skin Color (9) | 1 (body) | Body skin | 1027767 | 16 |
| Skin Color (9) | 13 (overlay) | Underwear | 1027743 | 16 |
| Face (10) | 5 (face) | Face skin | 1027494 | 16 per face × skin color |
| Hair Style (11) | 11 (hair) | Scalp hair | 1043094 | 13 per style × hair color |
| Hair Color | 10 (hair color) | Hair color overlay | 3582341 | N/A |

## CharComponentTextureSections Layout 173 (HD, 1024x512)

| Section | Type | X | Y | Width | Height |
|---------|------|---|---|-------|--------|
| 0 | ARM_UPPER | 0 | 0 | 256 | 128 |
| 1 | ARM_LOWER | 0 | 128 | 256 | 128 |
| 2 | HAND | 0 | 256 | 256 | 64 |
| 3 | TORSO_UPPER | 256 | 0 | 256 | 128 |
| 4 | TORSO_LOWER | 256 | 128 | 256 | 64 |
| 5 | LEGS_UPPER | 256 | 192 | 256 | 128 |
| 6 | LEGS_LOWER | 256 | 320 | 256 | 128 |
| 7 | FOOT | 256 | 448 | 256 | 64 |
| 9 | FACE_UPPER | 512 | 0 | 512 | 512 |
| 10 | FACE_LOWER | 512 | 0 | 512 | 512 |

## Texture Assets

| FDID | File | Size | Purpose |
|------|------|------|---------|
| 1027767 | body skin | 1024x512 | HD body base (type 1) |
| 1027743 | underwear | 256x128 | HD underwear overlay (type 13) |
| 1027494 | face upper | 512x512 | Face skin atlas (type 5), default face/skin |
| 1043094 | scalp hair | 512x512 | Scalp hair overlay (type 11), default style |
| 3582341 | hair color | 512x256 | Hair color tint overlay (type 10) |
| 3537040 | DK eye glow | ? | Death Knight eye glow effect (type 0) |
| 5210142 | eye reflect | ? | Eye reflection/specular (type 0) |
| 4531024 | eye texture | ? | Normal eye/iris texture (type 0) |
| 1466448 | alt face | 512x512 | Darker skin tone face variant |

## File Changes Summary

| File | Change |
|------|--------|
| `src/asset/m2.rs` | SKID chunk, `load_skel_data()`, skel parsers, geoset revert, u32 triangle_start fix, HD texture resolution, DK eyeglow fix, bone_lookup remap, `parse_materials()`, render_flags_index on M2TextureUnit |
| `src/asset/m2_anim.rs` | `parse_bones_at()`, `parse_sequences_at()`, `parse_global_sequences_at()`, `parse_bone_animations_at()` |
| `src/asset/m2_tests.rs` | Updated geoset/texture assertions, HD test cases |
| `src/asset/m2_debug_tests.rs` | UV range analysis, geoset position dumps, face vertex texture sampling, render flags dump, alpha stats |
| `src/asset/blp.rs` | `scale_2x()`, `blit_region()` compositing helpers |
| `src/animation.rs` | `jaw_bone_idx` on M2AnimData, `JAW_CLOSE_ROTATION` constant, jaw close in `apply_animation` |
| `src/main.rs` | Model rotation, `composite_overlay`, `m2_material()` with render flags, `jaw_bone_idx` wiring |
