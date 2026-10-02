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

## Zaralda native merchant fixture (2026-10-01)

[`world_zaralda_merchant_flow.gd`](../../../godot/tests/world_zaralda_merchant_flow.gd), revision `7903cb5e`, was prepared, parsed and executed against the owned private server; the initial three attempts had **0/3 passes**; these remain historical RED evidence, not the current pass ratio. Later fixture d2084de7 native-friendly exits0 / 529.10s and reaches window/title, actual ray-pick, authored244586 stock1 tooltip, backpack and close checks; independent artifact gate116 gives scoped PASS. Saved artifacts: [`data/diagnostics/zaralda-20261001/`](../../../data/diagnostics/zaralda-20261001/).

[Server Midnight investigation](../../../../game-server/docs/wiki/investigations/midnight-economy-content.md#native-acceptance-blocker) owns content/economy provenance and historical attempt boundaries; its linked catalog/CLI proof is not native proof. [Engine model/gear investigation](../investigations/npc-stance-gear.md#zaralda-rigid-waist-binding-verified-task-date-2026-10-01) owns the genuine appearance/import caveats and rigid-waist root/fix. Later native model/body-ray evidence and fixture revisions **17d77fcb/d2084de7** are recorded in that engine investigation. Actual Zaralda Buy cursor then faction-0 AutoAttack supersedes the earlier unreached pick boundary; approved faction35 now has [observed native success](../../../../game-server/docs/wiki/investigations/midnight-economy-content.md#observed-native-success-task-evidence-2026-10-01), with two PNGs and evidence JSON; no global PASS. This docs update ran no probe or build.

## Sources

- [test-assets.md](../../test-assets.md) — asset paths and FDIDs
- [Server Midnight investigation](../../../../game-server/docs/wiki/investigations/midnight-economy-content.md) — Zaralda source/data and acceptance SSOT.

## See Also

- [[collision-system]] — ADT terrain is the heightmap source for terrain collision
- [[open-source-wow-clients]] — M2/BLP format references for loading these assets
- [[character-generation]] — humanmale models are the M2 reference alongside generated glTF characters
