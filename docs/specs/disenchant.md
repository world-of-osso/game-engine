# Disenchant item targeting

Retail spell13262 enters item spell-targeting before sending a cast. See [pipeline](../wiki/systems/disenchant.md).

## What it must do
- [x] Spellbook Disenchant enters an item-target cursor without casting immediately; action-bar activation shares the implementation but has no separate live proof.
- [x] Cursor model emits the chosen bag GUID in `SpellCastIntent.target_item_guid`; an empty slot keeps targeting.
- [x] Native bag clicks invoke that model before pickup/use/sale; private live casts consume eligible items in Modern and Forever.
- [ ] Escape or right-click cancels targeting without a cast; world clicks do not substitute unit targets.
- [x] Normal spell initializers set the optional item GUID to None; compile against the companion protocol ABI.
- [x] Read-only `tooltip_state().appearance_collection` exposes sorted authoritative account appearance IDs, never bag ownership, for live disposal proof.
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
- `godot/tests/appearance_collection_snapshot.gd`: empty authoritative snapshot and returned-array mutation isolation.
- `godot/rust/src/tooltips.rs`: authoritative update replacement/deduplication.

## Known gaps (current cycle)
Both-skin private live proof: spellbook→bag cast, eligible consumption, TDB entry3 materials, manual LootFrame take, grey25 refusal/retention, authoritative appearance additions214/242 and relog persistence. [Evidence](../wiki/systems/disenchant.md#private-live-proof--2026-10-10).
- [ ] Auto-loot/Shift inversion, action-bar activation and cancellation have no separate live proof.
- [ ] Higher-tier TDB rows reference16 materials absent from this Retail build; 50/100 entries affected,40 wholly unavailable. Pre-fix live753 disposal opened invisible empty loot and blocked the next cast; [server branch519a34d](../../../game-server/docs/wiki/systems/disenchant.md#stale-material-compatibility--2026-10-10) refuses unavailable/actual-empty rolls before disposal. Targeted8/8 and check/format pass; not merged or re-tested rendered.

## Out of scope
- Bank targeting, new cursor art, scrap, new loot UI, server deployment and merges.
