# Item definition sources

Owned item metadata pairs the unchanged authored item ID with `shared::item_data::ItemDefinitionSource`. Client catalogs live in `godot/ui-model/src/game/item_catalog.rs`.

## What it must do

- [ ] Preserve Retail catalog data and byte semantics; source selection must not depend on race.
- [x] Resolve Forever70205 only from `data/db2/1.60.1.70205/items/`, containing the actual selected 30 kit definitions and source-local class, appearance and scaling metadata.
- [x] Keep colliding IDs 2947, 2512 and 2101 distinct in mixed-source bags, tooltips, comparisons and cursor state. Preserve equal names/icons when both products author equal values.
- [x] Missing Forever files or rows must not select Retail definitions or scaling; lookup errors identify source and ID.
- [ ] Preserve immutable GUID provenance across transfer/reload through owned inventory, equipment, bank, trade and auction-inventory consumers; recipient race must not select a definition.
- [x] Item-info IPC requires explicit query source. CLI defaults deliberately to Retail; `--source forever70205` explicitly selects Forever.
- [ ] Owned Retail equipment resolves per-model replacement texture types from root `ItemDisplayInfoModelMatRes.csv` through Retail texture resources. Forever70205 keeps source-local model skins; it must not borrow Retail replacement rows. A Forever per-model replacement export is not required by this contract.

- [x] Select equipped component models by source-local `ItemDisplayInfo.ModelResourcesID` → `ModelFileData` FDID, filtered by `ComponentModelFileData` wearer/position metadata; a listfile filename is not required to attach a selected model. The first model of an attachment slot uses that slot's attachment; additional models and body-slot models share character joints.

- [x] Modern mounts Retail's player portrait slot (60×60 at24,19) using the equipped-player renderer. Forever's FlareUI player frame has no portrait (user 2026-10-30).

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
- `godot/cli/src/tests/request_actions.rs`
- `godot/core/src/outfit_data_tests.rs`: owned Retail per-model replacements and Forever isolation.
- `godot/rust/src/assets/equipment.rs::tests::retail_head_7321940_attaches_without_a_listfile_name`: source display730376, human male, actual70391 model/skin, attachment11 and retained2702 mesh without a listfile name.
- `godot/rust/src/game/equipment/equipment_appearance_data.rs::tests::belt_keeps_buckle_and_collection_and_bracers_have_models`: source ordered buckle/collection binding and body bracer binding.
- `godot/ui-model/tests/forever_flare_frames.rs::forever_player_has_retail_portrait_without_covering_flare_bars`: shared portrait slot and retained unobscured Forever bars.

## Known gaps (current cycle)

- [ ] Main-owned native acceptance and final integration gates; current metadata CPU proof is not native E2E proof.
- [ ] Owned source-aware equipment rendering remains active; authored display-link export alone does not prove rendered gear.
- [ ] VendorItem, QuestRewardItem and loot summary tuples lack definition source; do not infer source.
- [ ] AuctionSearchQuery and browse action/grouping still filter by bare item ID; source-aware result metadata does not prove mixed-product auction selection.

## Out of scope

- Client reads of server databases, synthesized definitions, item ID renumbering and race-based catalog switching.
