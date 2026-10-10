# Disenchant item targeting

Retail spell13262 enters item spell-targeting before sending a cast. See [pipeline](../wiki/systems/disenchant.md).

## What it must do
- [ ] Spellbook/action-bar Disenchant enters an item-target cursor without casting immediately.
- [x] Cursor model emits the chosen bag GUID in `SpellCastIntent.target_item_guid`; an empty slot keeps targeting.
- [ ] Native bag clicks must invoke that model before pickup/use/sale; wired and compiled, rendered routing untested.
- [ ] Escape or right-click cancels targeting without a cast; world clicks do not substitute unit targets.
- [x] Normal spell initializers set the optional item GUID to None; compile against the companion protocol ABI.
- [ ] Personal disenchant loot opens the existing LootFrame and honors configured auto-loot/Shift inversion.

## How it works
- [Disenchant cast/cursor pipeline](../wiki/systems/disenchant.md)

## Implementation inventory
- `godot/ui-model/src/spell_targeting.rs`: item cursor state and emitted GUID intent.
- `godot/rust/src/spells/casting.rs`, `spells/ground_target.rs`, `spells.rs`: cast activation, cancellation and world-click ownership.
- `godot/rust/src/bag_cursor.rs`: targeting before bag pickup/use and cursor icon.
- `godot/rust/src/loot.rs`: personal item-loot auto-taking.
- `godot/rust/src/account.rs`, `ipc/world.rs`: ordinary-cast optional-field initializers.
- `godot/ui-model/src/ui/cast_failed_text.rs`: authentic Retail refusal strings.

## Tests asserting this spec
- `godot/ui-model/src/spell_targeting.rs`: item GUID/cancel/empty-slot cursor tests, existing ground-target tests.

## Known gaps (current cycle)
Native cursor/ground UI-model5/5 passes; matching-protocol extension compilation passes.
- [ ] Native rendered interaction proof not requested or performed; pure cursor tests do not prove pointer routing visually.

## Out of scope
- Bank targeting, new cursor art, scrap, new loot UI, server deployment and merges.
