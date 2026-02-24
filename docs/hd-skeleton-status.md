# HD Skeleton (.skel) Loading — Status Brief

## What Was Done

### External skeleton loading (committed: 39f8412)
- HD M2 models (e.g. `humanmale_hd.m2`) store bones externally in `.skel` files instead of inline MD20
- M2 top-level `SKID` chunk contains a u32 FileDataID pointing to the `.skel` file
- `.skel` is chunked binary (same tag+size format as M2); bones are in the `SKB1` chunk
- `load_m2()` now: parse inline bones → if empty + SKID present → load `{stem}.skel` → parse SKB1
- Refactored `parse_bones()` into reusable `parse_bones_at(data, offset, count)` for both paths
- HD humanmale .skel (FDID 2138400) downloaded to `data/models/humanmale_hd.skel` (18MB, 216 bones)

### Geoset regression fix (same commit)
- Commit 85b7565 added a rule showing variant 2 for groups 7-12 (intended for HD bare skin)
- On legacy models, variant 2 for those groups is equipment geometry (shirt sleeves, leggings, tabard)
- This caused legacy humanmale to render with equipment + bare skin superposed
- Reverted to the working `2d39d4d` form: only `702` (ears) as special case

### Model orientation fix (uncommitted)
- Model now rotated `-PI/2` around Y to face the camera at default yaw

## Current State

### Legacy humanmale.m2 — WORKING
- 215 inline bones, skinning with identity transforms, renders correctly
- Geoset visibility restored to pre-regression state

### HD humanmale_hd.m2 — PARTIALLY WORKING
- 216 bones loaded from .skel, body has full shape (no longer thin spine)
- **Body parts appear exploded/scattered** when animation system is active

## Known Issues

### 1. HD model skinning is broken
The `spawn_skeleton()` in main.rs uses `Mat4::IDENTITY` for all inverse bind poses and `Transform::IDENTITY` for all joint entities. The animation system (`animation.rs`) then applies bone transforms using the pivot-based formula. This works for legacy models but produces wrong results for HD because:
- The animation `bone_tracks` come from inline MD20 data (which has 0 bones for HD)
- The `.skel` bones are loaded but their animation tracks are NOT (animations may be in separate `.anim` files or in the `SKA1` chunk)
- The animation system reads `bone_tracks[bone_idx]` which may be empty or mismatched

### 2. HD geoset defaults need model-aware logic
Groups 7-12 variant 2 is bare skin on HD but equipment on legacy. A single static `default_geoset_visible()` can't handle both. Future: detect model type or check which variants exist.

### 3. Missing HD textures
- `3537040.blp` and `5210142.blp` needed for HD model (download with casc-extract)

## File Changes Summary

| File | Change |
|------|--------|
| `src/asset/m2.rs` | SKID chunk parsing, `load_skel_bones()`, skel fallback in `load_m2()`, geoset revert |
| `src/asset/m2_anim.rs` | Extract `parse_bones_at()` from `parse_bones()` |
| `src/asset/m2_tests.rs` | Updated geoset assertions, added debug tests |
| `src/main.rs` | Model rotation to face camera (uncommitted) |

## Next Steps

1. **Fix HD skinning**: Either disable animation for skel-loaded bones, or load animation data from the .skel's `SKA1`/`SKS1` chunks
2. **Proper inverse bind poses**: Compute from bone pivots instead of identity
3. **Model-aware geosets**: Detect HD vs legacy for groups 7-12 default visibility
4. **Download missing textures**: `3537040.blp`, `5210142.blp` via casc-extract
