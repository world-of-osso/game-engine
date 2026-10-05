# Forever 70205 item catalog producer

[`export_forever_item_catalog.py`](../scripts/export_forever_item_catalog.py) exports local raw DB2s into an isolated `data/db2/1.60.1.70205/items/` directory. No CASC extraction, network, server database, client Rust changes, or Retail reads/writes.

## Dependency and command

Explicit `--server-root` selects the maintained first-party server checkout containing `scripts/skyborne_data.py` (item/loadout mappings), `db2_casc.py` (including sparse ItemSparse support), and `db2_schema.sql` (positional column types). The exporter uses their pure decode functions and an in-memory schema, never `import_rows`, `project_items`, or a server database. Engine `import_forever_skyborne.parse_definition/decode_rows` handles fixed scaling/subclass rows. No new parser is implemented. Raw item/effect pins remain owned by the server importer; scaling and official DBD pins are owned by this exporter.

Main owns provisioning. Run from the engine checkout after confirming the destination does not exist:

```text
python3 scripts/export_forever_item_catalog.py --server-root /home/osso/.worktrees/game-server-skyborn --source-dir /syncthing/Sync/Projects/world-of-osso/game-engine/data/forever-1.60.1.70205/starter-kit-probe --definitions /syncthing/Sync/Projects/world-of-osso/game-engine/data/forever-1.60.1.70205/definitions
```

`--output` overrides the destination but must end in `1.60.1.70205/items`. Existing destinations are rejected, not merged/replaced. All source hashes, DBD hashes, exact matching layout blocks, required references/fields and scaling levels are checked before staging. CSVs plus manifest publish together via one sibling-directory rename; failed staging leaves no partial destination. No rollout/recovery framework. A repeat export to a fresh destination is byte-deterministic for identical source, definitions and reader code.

## CSV contract

Four item tables contain only the 30 distinct items referenced by loadouts 2373–2378, selector `(Purpose, ModID, Extra, RaceMasks_0, RaceMasks_1) = (9,75,0,0,3)`, plus their modified/ordinary appearances. `ItemSubClass` contains only referenced class/subclass labels. No unrelated labels or appearances are exported.

| File | Exact header, in order |
|---|---|
| Item.csv | ID, ClassID, SubclassID, IconFileDataID, SheatheType |
| ItemSparse.csv | ID, Display_lang, OverallQualityID, Stackable, SellPrice, Bonding, RequiredLevel, InventoryType, ItemLevel, MaxCount, Description_lang, ContainerSlots, ExpansionID, ItemDelay, DmgVariance, Flags_1, StatModifier_bonusStat_0…9, StatPercentEditor_0…9 |
| ItemSubClass.csv | ClassID, SubClassID, DisplayName_lang |
| ItemAppearance.csv | ID, DefaultIconFileDataID |
| ItemModifiedAppearance.csv | ItemID, ItemAppearanceID, OrderIndex |
| ItemArmorQuality.csv | ID, Qualitymod_0…6 |
| ItemArmorTotal.csv | ItemLevel, Cloth, Leather, Mail, Plate |
| ArmorLocation.csv | ID, Clothmodifier, Leathermodifier, Chainmodifier, Platemodifier |
| ItemArmorShield.csv; ItemDamageOneHand.csv; ItemDamageOneHandCaster.csv; ItemDamageTwoHand.csv; ItemDamageTwoHandCaster.csv | ItemLevel, Quality_0…6 |
| RandPropPoints.csv | ID, GoodF_0…4, SuperiorF_0…4, EpicF_0…4 |

Array ranges expand ascending with an underscore before the index, matching the existing Rust consumer and decoded source column names. No aliases or compatibility headers are emitted.

Nine scaling tables export every available clear source row: 100 each for armor quality/total/shield and four damage tables, 23 ArmorLocation rows, 300 RandPropPoints rows. Required selected scaling levels are 1/2; source-authored item levels remain 1/2/3/5, recorded separately. Only cloth/leather/mail/plate require ArmorLocation; robes use the chest location, as the existing consumer does. Nothing is synthesized when a field/reference is missing. All requested fields are available; no optional-field omission is needed.

`manifest.json` records build/product, FDIDs, layout/table hashes, raw/schema SHA-256, official schema source and explicit-build match, selected loadouts/items/scaling levels, authored item levels, output columns/counts/row IDs/CSV hashes and server reader/schema hashes. It contains no timestamps or absolute paths.

Concrete source values: item 2947 is Small Throwing Knife (weapon, level 3, 2000 ms); 2101 is Light Quiver (class 11, six slots); 2512 is Rough Arrow (class 6, level 5, stack 200, authored delay 3000 ms). Negative unused stat IDs remain -1, not zero-filled defaults.

## Development proof boundary

```text
scripts/agent/agent-run forever-item-producer python3 -m unittest scripts.tests.test_export_forever_item_catalog
```

Tests read the actual pinned local raw/DBD fixtures and publish only into temporary directories. `FOREVER_ITEM_SOURCE` and `FOREVER_SERVER_ROOT` can select equivalent pinned fixtures/dependencies; missing fixtures fail rather than skip. Eight development tests cover exact 30-item membership, concrete authored fields, reference/label closure, scaling values/row counts, pins, deterministic bytes, Retail sentinel preservation, raw/schema corruption, missing source/critical field, existing/Retail destination rejection, and staging-write failure cleanup. RED observed before exporter implementation; GREEN observed with these fixtures. No published data provisioning, client build/runtime acceptance, final gate, or server scaling-resource load is claimed. See [Forever system context](wiki/systems/forever-data.md).
