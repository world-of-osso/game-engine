# WoW HD Character Geosets

Reference: WMVx `GameConstants.h`, `CharacterCustomization.cpp`

## Geoset Groups

mesh_part_id = (group * 100) + variant. Variant 0 = hidden, variant 1+ = visible options.

| Group | WMVx Enum | Name | Notes |
|-------|-----------|------|-------|
| 0 | CG_SKIN_OR_HAIRSTYLE | Base skin / hair | mpid 0=arms/hands, 1=bald cap, 5=hair only |
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
| 22 | CG_CHEST | Chest | Equipment |
| 32 | CG_FACE | Face | DF+ face geometry |
| 33 | CG_EYES | Eyes | |
| 34 | CG_EYEBROWS | Eyebrows | |
| 35 | CG_EARRINGS | Earrings/Piercings | |

## Texture Types (tex_type in M2)

| Type | Texture Source | Used By |
|------|---------------|---------|
| 0 | Hardcoded (TXID FDID) | Eyes (mpid 5101) |
| 1 | Body skin | Body (mpid 0), face (mpid 3201), arms, legs, etc. |
| 6 | Face/hair texture | Bald cap (mpid 1), hair (mpid 5), facial hair (102/202/302) |

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
| 0 | 0 | 0 | 929 | Hands only (NOT arms/forearms) |
| 401 | 4 | 1 | 180 | Forearms |
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
| 1 | 0 | 1 | 192 | Bald cap (scalp closure) |
| 5 | 0 | 5 | 792 | Hair style |
| 102 | 1 | 2 | 195 | Facial hair 1 |
| 202 | 2 | 2 | 318 | Facial hair 2 |
| 302 | 3 | 2 | 156 | Facial hair 3 |
| 3401 | 34 | 1 | 70 | Eyebrows |

### Type-0 (hardcoded texture)
| mpid | Group | Variant | Verts | Description |
|------|-------|---------|-------|-------------|
| 5101 | 51 | 1 | 114 | Eyes |
