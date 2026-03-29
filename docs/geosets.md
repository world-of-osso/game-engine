# WoW HD Character Geosets

Reference: WMVx `GameConstants.h`, `CharacterCustomization.cpp`

## Geoset Groups

mesh_part_id = (group * 100) + variant. Variant 0 = hidden, variant 1+ = visible options.

| Group | WMVx Enum | Name | Notes |
|-------|-----------|------|-------|
| 0 | CG_SKIN_OR_HAIRSTYLE | Base skin / hair | Mixed group. On human male HD: mpid 0=hands/base, 1=bald cap, 5=visible hairstyle mesh |
| 1 | CG_GEOSET100 | Facial hair 1 | e.g. goatee |
| 2 | CG_GEOSET200 | Facial hair 2 | e.g. sideburns |
| 3 | CG_GEOSET300 | Facial hair 3 | e.g. mustache |
| 4 | CG_GLOVES | Gloves | Equipment |
| 5 | CG_BOOTS | Boots | Equipment |
| 7 | CG_EARS | Ears | 701=hidden, 702=visible |
| 13 | CG_WRISTBANDS | Wristbands | Equipment |
| 17 | CG_EYEGLOW | Eye glow | DK-only (off by default) |
| 18 | CG_BELT | Belt | Equipment |
| 20 | CG_FEET | Feet | Equipment |
| 21 | ? | Head show/hide | Head items use this via `ItemDisplayInfo.GeosetGroup_1`; wowdev documents `0 = no geoset`, `1 = show head` |
| 22 | CG_CHEST | Chest | Equipment |
| 27 | ? | Head / helmet variant | Head items use this via `ItemDisplayInfo.GeosetGroup_0`; special default behavior when the value is `0` |
| 32 | CG_FACE | Face | DF+ face geometry |
| 33 | CG_EYES | Eyes | |
| 34 | CG_EYEBROWS | Eyebrows | |
| 35 | CG_EARRINGS | Earrings/Piercings | |

## ItemDisplayInfo Slot Mappings

These are item-side selectors that map onto character geoset groups. Do not apply raw
`ItemDisplayInfo.GeosetGroup_*` values directly without slot-aware translation.

### Head

wowdev's `ItemDisplayInfo` docs identify two head-slot mappings:

- `GeosetGroup_0` -> character geoset group `27`
- `GeosetGroup_1` -> character geoset group `21`

Head group `27` has special default behavior:

- no helmet equipped -> `2701`
- helmet equipped and `GeosetGroup_0 == 0` -> `2702`
- helmet equipped and `GeosetGroup_0 > 0` -> `2700 + GeosetGroup_0`

Head group `21` behaves like a head visibility toggle:

- `GeosetGroup_1 == 0` -> no `21xx` geoset
- `GeosetGroup_1 == 1` -> `2101`

This matters because some helms do not rely on a spawned runtime M2 model to "appear".
They depend on these character geoset switches instead.

## Texture Types (tex_type in M2)

| Type | Texture Source | Used By |
|------|---------------|---------|
| 0 | Hardcoded (TXID FDID) | Eyes (mpid 5101) |
| 1 | Body skin | Body (mpid 0), face (mpid 3201), arms, legs, etc. |
| 6 | Face/hair texture | Bald cap (mpid 1), hair (mpid 5), facial hair (102/202/302) |

## Bone Indices

M2 vertex bone indices are **global** skeleton indices — NOT local per-geoset indices.
The skin file's bone table (offset 20, `properties` field) is NOT used for remapping.
WMVx confirms this: vertices index directly into the global bone array.
Do NOT apply skin bone lookup remapping — it breaks animation (wrong bone bindings).

## Face Rendering

The face is **not** part of the type-6 (hair) texture system.

- **Face geometry**: geoset group 32, mpid 3201 (for variant 1)
- **Face texture type**: type-1 (body skin)
- **Face texture application**: face upper + face lower textures are composited as layers onto the body skin texture at `CharacterRegion::FACE_UPPER` and `CharacterRegion::FACE_LOWER` regions

Different face choices in character creation swap the face texture FDIDs, not the geometry. The face mesh (mpid 32xx) stays the same.

### WMVx Face Texture Compositing

```
Body skin base texture
  + face[0] → FACE_LOWER region (layer 1)
  + face[1] → FACE_UPPER region (layer 1)
```

Source: `CharacterCustomization.cpp:337-347`

### Modern Models (DF+)

WMVx forces face visible for modern models:
```cpp
state.setVisibility(core::CharacterGeosets::CG_FACE, 1);
```

## Human Male HD — Observed Geosets

From `humanmale_hd.m2` + skin file:

### Type-1 (body skin)
| mpid | Group | Variant | Verts | Description |
|------|-------|---------|-------|-------------|
| 0 | 0 | 0 | 929 | Upper arms. Live-confirmed via single-mesh charselect debug probe. |
| 401 | 4 | 1 | 180 | Forearms |
| 404 | 4 | 4 | ? | Part of a glove. Live-confirmed via dedicated geoset debug screen. |
| 501 | 5 | 1 | 216 | Boots |
| 701 | 7 | 1 | 66 | Ears (hidden variant) |
| 702 | 7 | 2 | 134 | Ears (visible variant) |
| 1301 | 13 | 1 | 266 | Underwear / upper legs |
| 1801 | 18 | 1 | 64 | Waist |
| 1801 | 18 | 1 | 64 | Belt |
| 2001 | 20 | 1 | 444 | Feet |
| 2201 | 22 | 1 | 452 | Chest |
| 3201 | 32 | 1 | 63 | Face |

### Type-6 (face/hair texture)
| mpid | Group | Variant | Verts | Description |
|------|-------|---------|-------|-------------|
| 1 | 0 | 1 | 192 | Flat helmet-compatible hairstyle / scalp closure. Live-confirmed via single-mesh charselect debug probe. |
| 5 | 0 | 5 | 792 | Hairstyle with two big front ponytails. Live-confirmed via dedicated geoset debug screen. |
| 16 | 0 | 16 | ? | Hairstyle with ponytail. Live-confirmed via dedicated geoset debug screen. |
| 17 | 0 | 17 | ? | Hairstyle with long hair at the back. Live-confirmed via dedicated geoset debug screen. |
| 102 | 1 | 2 | 195 | Beard. Live-confirmed via dedicated geoset debug screen. |
| 202 | 2 | 2 | 318 | Facial hair 2 |
| 302 | 3 | 2 | 156 | Facial hair 3 |
| 3401 | 34 | 1 | 70 | Eyebrows |

### Type-0 (hardcoded texture)
| mpid | Group | Variant | Verts | Description |
|------|-------|---------|-------|-------------|
| 5101 | 51 | 1 | 114 | Eyes |
