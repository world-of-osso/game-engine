# HelmetGeosetData Extra Field Investigation (2026-03-28)

## Status

This note documents an observed pattern in `HelmetGeosetData.Field_10_0_0_46047_003`.

We do **not** know the field's meaning yet.

## Background

`HelmetGeosetData` currently exposes these columns in WoWDBDefs:

- `ID`
- `RaceID`
- `HideGeosetGroup`
- `HelmetGeosetVisDataID`
- `RaceBitSelection`
- `Field_10_0_0_46047_003`

The client currently handles:

- `RaceID`
- `HideGeosetGroup`
- `HelmetGeosetVisDataID`
- `RaceBitSelection`

The unnamed extra field is not handled yet.

## Observation

Dumping real retail `HelmetGeosetData.db2` rows for interesting `HelmetGeosetVisDataID` values showed a strong binary-looking pattern in the extra field:

- value `32`
- value `-1`

This does not look like random padding.

## Observed Correlation

For the vis IDs inspected (`245`, `247`, `248`, `306`, `307`, `616`):

- `extra = 32` commonly appears on base face/head hide groups:
  - `0`
  - `1`
  - `2`
  - often `7`
- `extra = -1` commonly appears on more specialized hide groups:
  - `3`
  - `16`
  - `24`
  - `25`
  - `31`
  - `35`
  - `37`
  - `39`
  - `42`
  - `46`

## Examples

### `HelmetGeosetVisDataID = 247`

- `hide 1` -> `extra = 32`
- `hide 2` -> `extra = 32`
- `hide 3` -> `extra = -1`

### `HelmetGeosetVisDataID = 248`

- `hide 0` -> `extra = 32`
- `hide 1` -> `extra = 32`
- `hide 2` -> `extra = 32`
- `hide 3` -> `extra = -1`
- `hide 7` -> `extra = 32`

### `HelmetGeosetVisDataID = 307`

- `hide 0` -> `extra = 32`
- `hide 7` -> `extra = 32`
- `hide 25` -> `extra = -1`
- `hide 35` -> `extra = -1`
- `hide 37` -> `extra = -1`
- `hide 39` -> `extra = -1`

### `HelmetGeosetVisDataID = 616`

- `hide 39` -> `extra = -1`

## What We Can Safely Say

- the field is probably meaningful
- the field appears to classify different kinds of helmet geoset hides
- `32` and `-1` are not distributed randomly across hide groups

## What We Cannot Say Yet

- we do not know the field's official name
- we do not know whether `32` is a bitmask, enum value, or tag
- we do not know whether `-1` means "none", "default", "full hide", or something else
- we do not know how Blizzard uses this field at runtime

## Important Limitation

This field still does **not** explain helm display `1128` by itself.

`1128` has:

- `HelmetGeosetVis_0 = 0`
- `HelmetGeosetVis_1 = 0`

So even if we decode this extra field correctly, it only matters for helms that already participate in the `HelmetGeosetVis` / `HelmetGeosetData` path.

## Next Step

If needed, the next investigation should:

1. parse the extra field in the runtime loader instead of ignoring it
2. correlate it with visible results on helms that do use non-zero `HelmetGeosetVis`
3. determine whether `32` corresponds to a scalp-visible or face-only reset mode
