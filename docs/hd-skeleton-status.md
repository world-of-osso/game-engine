# HD Skeleton (.skel) Loading — Status Brief

## Current State

### Legacy humanmale.m2 — WORKING
- 215 inline bones, skinning with identity inverse bind poses, renders correctly
- Animation from inline MD20 bone tracks (Stand idle, etc.)
- Geoset visibility restored to pre-regression state

### HD humanmale_hd.m2 — WORKING
- 216 bones loaded from external `.skel` file (SKB1 chunk)
- Animation sequences (422) from SKS1, bone tracks from SKB1
- Full body renders correctly, animation plays

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

### Geoset regression fix (committed: 39f8412)
- Commit 85b7565 added a rule showing variant 2 for groups 7-12 (intended for HD bare skin)
- On legacy models, variant 2 for those groups is equipment geometry (shirt sleeves, leggings, tabard)
- Reverted to the working `2d39d4d` form: only `702` (ears) as special case

### Model orientation fix (uncommitted)
- Model rotated `-PI/2` around Y to face the camera at default yaw

## Known Issues

### 1. HD geoset defaults need model-aware logic
Groups 7-12 variant 2 is bare skin on HD but equipment on legacy. A single static `default_geoset_visible()` can't handle both. Future: detect model type or check which variants exist.

### 2. Missing HD textures
- `3537040.blp` and `5210142.blp` needed for HD model (download with casc-extract)

## File Changes Summary

| File | Change |
|------|--------|
| `src/asset/m2.rs` | SKID chunk, `load_skel_data()`, `parse_sks1_chunk()`, `parse_skb1_chunk()`, `load_anim_from_md20()`, geoset revert, **u32 triangle_start + cumulative index fix** |
| `src/asset/m2_anim.rs` | `parse_bones_at()`, `parse_sequences_at()`, `parse_global_sequences_at()`, `parse_bone_animations_at()` |
| `src/asset/m2_tests.rs` | Updated geoset assertions, debug tests |
| `src/main.rs` | Model rotation to face camera |
