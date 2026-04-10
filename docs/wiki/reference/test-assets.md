# Test Assets

Available test assets for the engine, all under `data/` relative to the project root.

## Models (M2)

| Path | Description | Notes |
|------|-------------|-------|
| `data/models/club_1h_torch_a_01.m2` | Textured item model | FDID 145513 + 198077; has associated BLPs |
| `data/models/humanmale.m2` + `humanmale00.skin` | Legacy character model | Minimal hair, 142KB — good for basic skeleton testing |
| `data/models/humanmale_hd.m2` + `humanmale_hd00.skin` | HD character model | FDID 1011653, 11MB, 113 submeshes, full hairstyles — use for full pipeline testing |
| `data/models/boar.m2` | Creature model | Runtime creature skin, no hardcoded BLPs — tests skin resolution path |

## Textures (BLP)

| Path | Description |
|------|-------------|
| `data/textures/145513.blp` | Torch flame texture (paired with torch M2) |
| `data/textures/198077.blp` | Torch glow texture (paired with torch M2) |
| `~/Projects/wow/Interface/` | 137K UI textures from WoW client (not model textures) |

## Terrain (ADT)

| Path | Description | Notes |
|------|-------------|-------|
| `data/terrain/azeroth_32_48.adt` | Elwynn Forest terrain tile | FDID 778027, 350KB, 256 MCNK chunks |

## Usage Notes

- **Torch** (`club_1h_torch_a_01.m2`): best asset for end-to-end texture loading — M2 + known BLP FDIDs
- **humanmale_hd**: heaviest model; use for submesh/LOD and hairstyle system testing
- **boar**: tests creature skin resolution without hardcoded texture paths
- **azeroth_32_48**: tests terrain rendering, heightmap collision, and chunk streaming

## Sources

- [test-assets.md](../../test-assets.md) — asset paths and FDIDs

## See Also

- [[collision-system]] — ADT terrain is the heightmap source for terrain collision
- [[open-source-wow-clients]] — M2/BLP format references for loading these assets
- [[character-generation]] — humanmale models are the M2 reference alongside generated glTF characters
