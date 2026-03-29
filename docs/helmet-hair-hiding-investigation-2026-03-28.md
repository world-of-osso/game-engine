# Helmet Hair Hiding Investigation (2026-03-28)

## Current Status

`HelmetGeosetVis` / `HelmetGeosetData` is now implemented in the client and threaded into head equipment resolution.

That path is real, but it does not yet explain the visible helm case we were testing.

As of 2026-03-28, wowdev's `ItemDisplayInfo` docs also give the first concrete missing
head-item geoset mapping we were not applying:

- `Head.GeosetGroup_0` -> character geoset group `27`
- `Head.GeosetGroup_1` -> character geoset group `21`
- group `27` has special default behavior when `GeosetGroup_0 == 0`

That explains why `HelmetGeosetVis` alone never accounted for all visible helm behavior.
We were missing the base head-slot geoset path entirely.

## Confirmed Facts

- Visible helm display `1128` renders and is the right live probe for human male head gear.
- `1128` has `HelmetGeosetVis_0 = 0` and `HelmetGeosetVis_1 = 0` in `ItemDisplayInfo.csv`.
- `1128` also has no obvious alternate item-side hide signal:
  - `Flags = 0`
  - `GeosetGroupOverride = 0`
  - `AttachmentGeosetGroup_* = 0`
- `HelmetGeosetData` is now loaded from retail DB2 and resolved by `HelmetGeosetVisDataID`.
- The implemented rule is: for matching race rows, reset the matching character geoset group back to default variant `1`.

## Important Pattern

The same helm model family can appear both with and without `HelmetGeosetVis`.

Examples:

- Model resource `17186`:
  - `1128` uses it with `HelmetGeosetVis = 0,0`
  - `15304` uses it with `HelmetGeosetVis = 248,306`
- Model resource `17477`:
  - `718146` uses it with `HelmetGeosetVis = 0,0`
  - `173086` uses it with `HelmetGeosetVis = 245,247`
- Model resource `45870`:
  - `171441` uses it with `HelmetGeosetVis = 0,0`
  - `163793` uses it with `HelmetGeosetVis = 245,245`

So model family alone does not determine whether `HelmetGeosetVis` is present.

## Current Hypothesis

`HelmetGeosetData` appears to be only part of helmet-driven face hiding, not the full scalp-hair hiding story.

Reasons:

- The wowdev note says `HelmetGeosetData` is used for hiding certain face elements on certain races.
- Human hair styles in `CharHairGeosets.csv` are driven by geoset type `0`.
- The confirmed `HelmetGeosetData` rows we inspected for human mostly resolve to hide groups like `1`, `2`, and `3`, which look more like facial-hair / face-adjacent groups than scalp hair.
- Our group `0` handling is special-case logic in the client, not a simple "reset to variant 1" path like the other groups.

## Likely Missing Piece

Scalp hair hiding probably uses an additional rule path beyond `HelmetGeosetVis` / `HelmetGeosetData`.

Part of the previously "missing" behavior is now identified: head items also drive base
character geosets through `ItemDisplayInfo.GeosetGroup_0/1` -> groups `27/21`.

What still appears unresolved is the remaining scalp-hair suppression behavior beyond that.

Possible sources:

- another DB2/CSV keyed by head display or model resource
- a model-family-specific client rule
- a scalp/hair rule implied by `CharHairGeosets.Showscalp`
- a combination of `HelmetGeosetData` for face pieces plus a separate scalp-hair suppression rule

## Live Probe Set Up

Theron's helm was switched to display `15304`, which uses the same `helm_plate_d_02_*` family as `1128` but has non-zero `HelmetGeosetVis = 248,306`.

This is the current best live comparison:

- if `15304` changes face/hair behavior and `1128` does not, `HelmetGeosetVis` is only a partial path
- if `15304` also does not hide the relevant hair, then scalp hair is definitely controlled elsewhere

## Next Steps

1. Judge `15304` live and record exactly what changed: scalp hair, facial hair, ears, or nothing.
2. Implement and verify the base head-slot geoset mapping for groups `27` and `21`.
3. Dump the resolved hidden geoset groups for `15304` and map them to visible human male HD submeshes.
4. Investigate whether `CharHairGeosets.Showscalp` needs to participate in helmet hair suppression.
