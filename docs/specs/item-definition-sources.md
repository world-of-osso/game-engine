# Item definition sources

Owned item metadata pairs the unchanged authored item ID with `shared::item_data::ItemDefinitionSource`. Client catalogs live in `godot/ui-model/src/game/item_catalog.rs`.

## What it must do

- [ ] Preserve Retail catalog data and byte semantics; source selection must not depend on race.
- [ ] Resolve Forever70205 only from `data/db2/1.60.1.70205/items/`, containing the actual selected 30 kit definitions and source-local class, appearance and scaling metadata.
- [ ] Keep colliding IDs 2947, 2512 and 2101 distinct in mixed-source bags, tooltips, comparisons and cursor state.
- [ ] Missing Forever files or rows must not select Retail definitions or scaling; item-info errors identify source and ID.
- [ ] Preserve source through owned inventory, equipment, bank, trade and auction-inventory consumers.
- [ ] Item-info IPC requires explicit query source. CLI defaults deliberately to Retail.

## How it works

- [Forever data](../wiki/systems/forever-data.md)

## Implementation inventory

- `godot/ui-model/src/game/item_catalog.rs`: independent source catalogs and contextual lookup.
- `godot/ui-model/src/game/item_icons.rs`: source-local appearance joins.
- `godot/ui-model/src/game/item_stats.rs`: source-local scaling tables.
- `godot/ui-model/src/game/bag_data.rs`: owned source identity.
- `godot/ui-model/src/game/item_tooltip.rs`: source-preserving formatter.
- `godot/ui-model/src/game_tooltip/item.rs`: comparison and auction-owned metadata.
- `godot/network/src/ipc/wire.rs`: explicit query identity.

## Tests asserting this spec

- `godot/ui-model/src/game/item_catalog_tests.rs`
- `godot/ui-model/src/game/cursor_item_tests.rs`

## Known gaps (current cycle)

- [ ] Main-owned export/provenance provisioning and native acceptance.
- [ ] Bare vendor, buyback, auction listing/browse, quest and loot summaries lack definition source; do not infer source.

## Out of scope

- Client reads of server databases, synthesized definitions, item ID renumbering and race-based catalog switching.
