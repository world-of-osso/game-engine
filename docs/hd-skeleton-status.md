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
- **Jaw open in bind pose** — inner mouth/teeth visible from front (animation issue)
- **Face skin tone darker than body** — needs investigation

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

## Known Issues

### 1. Jaw wide open — bind pose geometry (HIGH)

The jaw (key_bone_id=7, bone index 88, parent 39=head) is dramatically open. The bind pose vertex data itself has the mouth wide open.

**Investigation complete:**
- Stand idle has 49 jaw rotation keyframes, ALL are pure WoW Y-axis (Bevy Z-axis) oscillation 2–3° — a breathing side-to-side yaw. No pitch component at all. The animation is a subtle breathing wiggle, not a mouth open/close.
- Quaternion pipeline verified correct: `unpack_quat_component` matches WMVx exactly, `[x, z, -y, w]` conversion verified via rotation matrix decomposition. WMVx's `(-x, -z, y, w)` is the conjugate but paired with their R^T matrix → same net rotation.
- Bone flags: both head (39) and jaw (88) have only `transformed` (0x200). No billboard, no ignoreParent flags. WMVx has no special-case code for jaw (key_bone_id=7).
- No parent skeleton: SKPD chunk absent from humanmale_hd.skel. Skeleton is self-contained.
- .skel chunks: SKL1(16B), SKS1(28.5KB), SKB1(19.2MB), SKA1(2KB), AFID(608B), BFID(88B)

**Debug data:**
- Head bone 39: pivot=(-0.005, 1.848, -0.001), rot=(0.0008, 0.0024, -0.0244, 0.9997)
- Jaw bone 88: pivot=(0.058, 1.861, -0.001), rot=(0.0, 0.0, 0.0218, 0.9998)

**Impact:** Inner mouth cavity and teeth fully visible from front. Dark mouth makes face appear much darker than body.

**Remaining theories:**
1. **Bone remap table not applied** — skin file has non-empty bone-remap table (37,813 entries for HD). Chin vertices use local bone indices 135/136, NOT global bone 88 (jaw). If we pass raw vertex bone_indices without remapping through the skin's bone table, vertices attach to wrong bones. This is the most likely root cause.
2. AFID chunk lists external `.anim` files — the real jaw-close animation data may live in an external anim file, not inline in SKB1
3. Compare with WMVx rendered output to confirm whether this is expected

### 2. Eye geosets — specular overlay, not iris (MEDIUM)

Eye reflection geoset (mpid 5101, group 51) uses FDID 5210142: a **128x128 pure-white DXT5 texture** with 3.9% alpha coverage (small specular highlight spot). This is not the iris — it's an additive specular overlay.

**Current state:** Renders as opaque white because it's assigned as `base_color_texture` with default `Opaque` alpha mode. Should be `emissive_texture` with `AlphaBlend`.

**Proper eye rendering needs two layers:**
1. Base: iris color texture (runtime-resolved character customization, not available yet)
2. Overlay: 5210142 as emissive/additive with alpha blend for specular highlight

Eye iris texture FDID 4531024 (342KB) already downloaded but not wired up (mpid 1702-1705, not visible by default).

### 3. Face skin tone mismatch (MEDIUM)

Face texture (1027494) appears darker than body texture (1027767) under the same lighting. Likely caused by inner mouth geometry (dark cavity) rather than genuine tone difference. Will resolve when jaw issue is fixed.

### 4. HD geoset defaults need model-aware logic (LOW)

Groups 7-12 variant 2 is bare skin on HD but equipment on legacy. A single static `default_geoset_visible()` can't handle both. Future: detect model type or check which variants exist.

### 4. M2 render flags not parsed (LOW)

`render_flags_index` from M2Batch is read but not stored. WoW render flag bit 0x04 (two-sided) controls backface culling per batch. Currently all batches use Bevy's default (backface cull).

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
| `src/asset/m2.rs` | SKID chunk, `load_skel_data()`, skel parsers, geoset revert, u32 triangle_start fix, HD texture resolution (`is_hd`, `OverlayScale`, body/face/hair FDID constants, `body_skin_overlays` HD path, `batch_texture_type` helper), DK eyeglow visibility fix |
| `src/asset/m2_anim.rs` | `parse_bones_at()`, `parse_sequences_at()`, `parse_global_sequences_at()`, `parse_bone_animations_at()` |
| `src/asset/m2_tests.rs` | Updated geoset/texture assertions, HD test cases |
| `src/asset/m2_debug_tests.rs` | UV range analysis, geoset position dumps, texture dumps, composited head dump |
| `src/asset/blp.rs` | `scale_2x()`, `blit_region()` compositing helpers |
| `src/main.rs` | Model rotation, `composite_overlay` for `OverlayScale`, grass texture, ground clutter |
