# Test Assets

## Authored character names

Runtime and the existing selectable-race invariant require both `data/NameGen.csv` (Retail) and `data/db2/1.60.1.70205/NameGen.csv` (Forever); both are listed in `godot/depot-test-assets.txt`. Do not replace Retail names with the Forever table.

Provision only the names export, without CASC extraction or the full asset importer:

```text
scripts/agent/agent-run skyborn-names python3 scripts/export_forever_names.py --data /syncthing/Sync/Projects/world-of-osso/game-engine/data
```

The exporter reads `forever-1.60.1.70205/skyborne-probe/1122117.db2` and `forever-1.60.1.70205/definitions/NameGen.dbd` beneath `--data`, validates their pinned SHA256 identities and layout/schema, then writes only `db2/1.60.1.70205/NameGen.csv` and `NameGen.provenance.json`. It retains all decoded rows and NameType; runtime filters first names for races 95/96. Provision those two output files into a checkout's data tree if it does not share canonical data. Do not stage data files.

The source hashes/layout and runtime behavior are documented in [Forever authored names](wiki/systems/forever-data.md#authored-skyborne-names). Main must provision the export before running the full `name_catalog` test filter through the desktop/local helper; fixture-only development proof is not asset-provisioned or native UI acceptance.

## Models and textures

- M2: `data/models/club_1h_torch_a_01.m2` — **textured** item model (FDID 145513 + 198077)
- BLP: `data/textures/145513.blp` + `198077.blp` — torch flame/glow textures
- M2: `data/models/humanmale.m2` + `humanmale00.skin` — legacy character model (minimal hair, 142KB)
- M2: `data/models/humanmale_hd.m2` + `humanmale_hd00.skin` — **HD character model** (FDID 1011653, 11MB, 113 submeshes, full hairstyles)
- M2: `data/models/boar.m2` — creature model (runtime creature skin, no hardcoded BLPs)
- ADT: `data/terrain/azeroth_32_48.adt` — Elwynn Forest terrain tile (FDID 778027, 350KB, 256 MCNK chunks)
- BLP: `~/Projects/wow/Interface/` — 137K UI textures from WoW client (not model textures)
