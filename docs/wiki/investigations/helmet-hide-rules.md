# Helmet Hide Rules

Helmets suppress hair and facial geometry through at least two distinct mechanisms: the `HelmetGeosetVis` / `HelmetGeosetData` DB2 path and a base head-slot geoset path via `ItemDisplayInfo.GeosetGroup`. The two paths are independent and neither alone accounts for all visible helm behavior.

## Finding

### HelmetGeosetVis / HelmetGeosetData Path

`HelmetGeosetData` rows are keyed by `HelmetGeosetVisDataID`. Each row contains `RaceID`, `HideGeosetGroup`, `RaceBitSelection`, and an unresolved extra field (`Field_10_0_0_46047_003`). The implemented rule: for matching race rows, reset the matching character geoset group back to default variant `1`.

This path is only active when `ItemDisplayInfo.HelmetGeosetVis_0/1` are non-zero. Many helm display IDs — including the test case `1128` — have both `HelmetGeosetVis_0 = 0` and `HelmetGeosetVis_1 = 0`, so `HelmetGeosetData` is not consulted at all for them.

### ItemDisplayInfo GeosetGroup Path (head slot)

`ItemDisplayInfo` for head items drives base character geosets:
- `Head.GeosetGroup_0` → character geoset group `27`
- `Head.GeosetGroup_1` → character geoset group `21`
- Group `27` has special default behavior when `GeosetGroup_0 == 0`

This path was previously not implemented and explains why `HelmetGeosetVis` alone never accounted for all head-item behavior.

### Unresolved Extra Field in HelmetGeosetData

`Field_10_0_0_46047_003` shows a strong binary pattern: `32` appears on base face/head hide groups (0, 1, 2, 7) and `-1` on more specialized groups (3, 16, 24, 25, 31+). Likely classifies hide types but the official meaning is unknown.

### Scalp Hair Suppression

Scalp hair (geoset type `0` in `CharHairGeosets`) is probably controlled by a third path beyond `HelmetGeosetData` and `GeosetGroup`. Candidates: `CharHairGeosets.Showscalp`, a model-family-specific rule, or another DB2 keyed by head display or model resource.

## Root Cause

The `HelmetGeosetVis` path was implemented but the `ItemDisplayInfo.GeosetGroup → groups 27/21` path was missing. The combination is required to match Blizzard's head-item behavior.

## Resolution Status

- `HelmetGeosetVis` / `HelmetGeosetData` path: implemented.
- `ItemDisplayInfo.GeosetGroup_0/1 → groups 27/21`: identified, not yet fully implemented.
- Scalp hair suppression beyond both paths: still unresolved.

**Live probe:** helm display `15304` (same model family as `1128`, but with non-zero `HelmetGeosetVis = 248,306`) is the current test case for distinguishing the two paths.

## Sources

- [helmet-hair-hiding-investigation-2026-03-28.md](../../helmet-hair-hiding-investigation-2026-03-28.md) — GeosetGroup path identification, implementation status
- [helmet-geoset-extra-field-investigation-2026-03-28.md](../../helmet-geoset-extra-field-investigation-2026-03-28.md) — extra field pattern observations

## See Also

- [[character-texture-compositing]] — character body/skin texture paths during char-select
